//! Two-till sync SQLite repository (TASK-BRK-124, ADR-123 subset).
//!
//! Export is read-only: locally created FINALIZED sale documents
//! (TICKET/BL/FACTURE, `origin_number IS NULL`) become a canonical v1
//! package (grammar in `baraka_domain::sync`, SHA-256 over the body).
//!
//! Import is ONE atomic transaction: verify (size cap, grammar, hash),
//! resolve catalog identity by SKU/barcode and customers by code
//! fail-closed, allocate the IMPORTER's gapless numbers (`allocate_in_tx`),
//! preserve `origin_number`, apply stock moves to the importer's warehouse
//! (importer-wins on negative, collected as warnings), recompute ledger
//! legs locally, and keep imported payments out of every cash session
//! (money stays put). Re-import aborts on the sync-identity index —
//! fail closed, zero double-count. Returns/credit-memos are NOT in v1:
//! their cumulative chains reference source documents the importer cannot
//! resolve, so the exporter excludes them loudly (documented in the UI).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{SyncExportPort, SyncImportOutcome, SyncImportPort, SyncRenumber};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::StockMovementType;
use baraka_domain::sales::document::DocumentType;
use baraka_domain::sync as sync_format;
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, Transaction};
use uuid::Uuid;

use crate::adapters::sqlite_document_sequence_repository::allocate_in_tx;
use crate::adapters::sqlite_stock_repository::{apply_one, map_engine_error, sentinel, tx_failure};
use crate::persistence::sha256::Sha256;
use crate::persistence::transaction::run_transaction;

fn coded(code: &str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

fn is_unique_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

/// SQLite sync repository over the shared connection.
pub struct SqliteSyncRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteSyncRepository {
    /// Wrap the shared connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApplicationError> {
        self.db
            .lock()
            .map_err(|_| ApplicationError::Port("database lock poisoned".to_string()))
    }
}

/// One exportable document row (joined identity in SQL, grammar in domain).
struct ExportRow {
    doc: sync_format::SyncDoc,
    items: Vec<sync_format::SyncItem>,
    pays: Vec<sync_format::SyncPay>,
}

fn read_export_rows(
    tx: &Transaction,
    terminal_id: &str,
    since_created_at: &str,
    limit: usize,
) -> Result<Vec<ExportRow>, rusqlite::Error> {
    let mut docs_stmt = tx.prepare(
        "SELECT d.number, d.fiscal_year, d.type, d.document_language, c.code,
                d.subtotal_ht_centimes, d.vat_total_centimes,
                d.global_discount_centimes, d.timbre_centimes, d.rounding_centimes,
                d.ttc_centimes, d.paid_amount_centimes, d.change_amount_centimes,
                d.payment_method, d.created_at
         FROM documents d
         LEFT JOIN customers c ON c.id = d.party_id
         WHERE d.status = 'FINALIZED'
           AND d.type IN ('TICKET', 'BL', 'FACTURE')
           AND d.origin_device_id = ?1
           AND d.origin_number IS NULL
           AND d.created_at > ?2
         ORDER BY d.created_at ASC
         LIMIT ?3",
    )?;
    let doc_metas = docs_stmt.query_map(
        params![terminal_id, since_created_at, limit as i64],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, i64>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
            ))
        },
    )?;
    let mut rows = Vec::new();
    for meta in doc_metas {
        let (
            number,
            fiscal_year,
            type_code,
            language,
            party_code,
            subtotal,
            vat,
            discount,
            timbre,
            rounding,
            ttc,
            paid,
            change,
            payment_method,
            created_at,
        ) = meta?;
        let mut items_stmt = tx.prepare(
            "SELECT i.quantity_micro, i.quantity_base_micro, i.unit_price_centimes,
                    i.discount_centimes, i.tva_rate_micro, i.line_ttc_centimes,
                    i.cost_price_centimes, i.sort_order, p.sku, u.barcode
             FROM document_items i
             JOIN products p ON p.id = i.product_id
             LEFT JOIN product_units u ON u.id = i.unit_id
             WHERE i.document_id = (SELECT id FROM documents WHERE number = ?1 AND type = ?2
                                    AND fiscal_year = ?3 AND origin_device_id = ?4
                                    AND origin_number IS NULL)
             ORDER BY i.sort_order ASC",
        )?;
        // Resolve the local document id once for the item/payment joins.
        let doc_id: String = tx.query_row(
            "SELECT id FROM documents WHERE number = ?1 AND type = ?2
             AND fiscal_year = ?3 AND origin_device_id = ?4 AND origin_number IS NULL",
            params![number, type_code, fiscal_year, terminal_id],
            |row| row.get(0),
        )?;
        let item_rows = items_stmt.query_map(
            params![number, type_code, fiscal_year, terminal_id],
            |row| {
                Ok(sync_format::SyncItem {
                    doc_origin_number: number.clone(),
                    sku: row.get::<_, String>(8)?,
                    quantity_micro: row.get(0)?,
                    quantity_base_micro: row.get(1)?,
                    unit_price_centimes: row.get(2)?,
                    discount_centimes: row.get(3)?,
                    tva_rate_micro: row.get(4)?,
                    line_ttc_centimes: row.get(5)?,
                    cost_price_centimes: row.get(6)?,
                    unit_barcode: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                    sort_order: row.get(7)?,
                })
            },
        )?;
        let mut items = Vec::new();
        for item in item_rows {
            items.push(item?);
        }
        let mut pays_stmt = tx.prepare(
            "SELECT p.payment_method, p.amount_centimes, c.code
             FROM payments p
             LEFT JOIN customers c ON c.id = p.customer_id
             WHERE p.document_id = ?1
             ORDER BY p.sequence_no ASC",
        )?;
        let pay_rows = pays_stmt.query_map(params![doc_id], |row| {
            Ok(sync_format::SyncPay {
                doc_origin_number: number.clone(),
                method_code: row.get::<_, String>(0)?,
                customer_code: row.get::<_, Option<String>>(2)?,
                amount_centimes: row.get(1)?,
            })
        })?;
        let mut pays = Vec::new();
        for pay in pay_rows {
            pays.push(pay?);
        }
        rows.push(ExportRow {
            doc: sync_format::SyncDoc {
                doc_type_code: type_code,
                fiscal_year: i32::try_from(fiscal_year).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Integer,
                        "fiscal_year out of range".into(),
                    )
                })?,
                origin_number: number,
                language,
                party_code,
                subtotal_ht_centimes: subtotal,
                vat_total_centimes: vat,
                global_discount_centimes: discount,
                timbre_centimes: timbre,
                rounding_centimes: rounding,
                ttc_centimes: ttc,
                paid_amount_centimes: paid,
                change_amount_centimes: change,
                payment_method,
                created_at,
            },
            items,
            pays,
        });
    }
    Ok(rows)
}

impl SyncExportPort for SqliteSyncRepository {
    fn export_package(
        &self,
        terminal_id: &str,
        since_created_at: &str,
        limit: usize,
    ) -> Result<(String, usize), ApplicationError> {
        let mut guard = self.lock()?;
        let rows = run_transaction(&mut guard, |tx| {
            read_export_rows(tx, terminal_id, since_created_at, limit)
                .map_err(crate::persistence::InfrastructureError::from)
        })
        .map_err(map_engine_error)?;
        let mut body: Vec<String> = Vec::new();
        let mut doc_count = 0_usize;
        for row in &rows {
            if row.items.is_empty() {
                continue;
            }
            body.push(sync_format::build_doc_line(&row.doc));
            for item in &row.items {
                body.push(sync_format::build_item_line(item));
            }
            for pay in &row.pays {
                body.push(sync_format::build_pay_line(pay));
            }
            doc_count += 1;
        }
        let body_text = body.join("\n");
        let mut hasher = Sha256::new();
        hasher.update(body_text.as_bytes());
        let hash_hex = hasher.finish_hex();
        let exported_at = Utc::now()
            .to_rfc3339_opts(SecondsFormat::Millis, true)
            .to_string();
        let header = sync_format::build_header(terminal_id, &exported_at, doc_count, &hash_hex);
        Ok((format!("{header}\n{body_text}"), doc_count))
    }
}

/// Resolved catalog identity for one import item.
struct ResolvedItem {
    product_id: String,
    unit_id: String,
}

fn resolve_customer_id(
    tx: &Transaction,
    code: &str,
) -> Result<String, crate::persistence::InfrastructureError> {
    tx.query_row("SELECT id FROM customers WHERE code = ?1", [code], |row| {
        row.get(0)
    })
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => tx_failure(format!("SYNC_UNKNOWN_CUSTOMER:{code}")),
        other => crate::persistence::InfrastructureError::from(other),
    })
}

fn resolve_item(
    tx: &Transaction,
    item: &sync_format::SyncItem,
) -> Result<ResolvedItem, crate::persistence::InfrastructureError> {
    let product_id: String = tx
        .query_row(
            "SELECT id FROM products WHERE sku = ?1",
            [&item.sku],
            |row| row.get(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                tx_failure(format!("SYNC_UNKNOWN_SKU:{}", item.sku))
            }
            other => crate::persistence::InfrastructureError::from(other),
        })?;
    // Unit identity: origin barcode when present, else the importer's
    // default selling unit. The factor MUST reproduce the origin base
    // micros exactly — otherwise the tills' catalogs drifted and the
    // package aborts instead of posting drift.
    let (unit_id, factor): (String, i64) = if item.unit_barcode.is_empty() {
        tx.query_row(
            "SELECT id, factor_micro FROM product_units
             WHERE product_id = ?1 AND is_default_selling = 1",
            [&product_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                tx_failure(format!("SYNC_UNKNOWN_UNIT:{}", item.sku))
            }
            other => crate::persistence::InfrastructureError::from(other),
        })?
    } else {
        tx.query_row(
            "SELECT id, factor_micro FROM product_units
             WHERE product_id = ?1 AND barcode = ?2",
            params![product_id, item.unit_barcode],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                tx_failure(format!("SYNC_UNKNOWN_UNIT:{}", item.sku))
            }
            other => crate::persistence::InfrastructureError::from(other),
        })?
    };
    let rebuilt = baraka_domain::catalog::unit::convert_to_base_micros(
        item.quantity_micro,
        baraka_domain::primitives::FactorMicro::from_micros(factor),
    )
    .map_err(|_| tx_failure(format!("SYNC_UNIT_DRIFT:{}", item.sku)))?;
    if rebuilt != item.quantity_base_micro {
        return Err(tx_failure(format!("SYNC_UNIT_DRIFT:{}", item.sku)));
    }
    Ok(ResolvedItem {
        product_id,
        unit_id,
    })
}

impl SyncImportPort for SqliteSyncRepository {
    fn import_package(
        &self,
        package_text: &str,
        warehouse_id: &str,
        actor_user_id: &str,
        terminal_id: &str,
        now_rfc3339: &str,
    ) -> Result<SyncImportOutcome, ApplicationError> {
        if package_text.len() > sync_format::MAX_PACKAGE_BYTES {
            return Err(coded("SYNC_PACKAGE_TOO_LARGE"));
        }
        let mut lines = package_text.lines();
        let header_line = lines.next().ok_or_else(|| coded("SYNC_EMPTY_PACKAGE"))?;
        let (origin_terminal, _exported_at, doc_count, hash_hex) =
            sync_format::parse_header(header_line).map_err(ApplicationError::Validation)?;
        if origin_terminal == terminal_id {
            return Err(coded("SYNC_SELF_IMPORT"));
        }
        let body_lines: Vec<String> = lines.map(str::to_string).collect();
        if body_lines.len() > sync_format::MAX_PACKAGE_DOCS * 4 {
            return Err(coded("SYNC_PACKAGE_TOO_LARGE"));
        }
        // Integrity first: the hash covers the exact body bytes.
        let mut hasher = Sha256::new();
        hasher.update(body_lines.join("\n").as_bytes());
        if hasher.finish_hex() != hash_hex {
            return Err(coded("SYNC_HASH_MISMATCH"));
        }
        // Grammar second: parse everything fail-closed BEFORE any write.
        let mut docs: Vec<sync_format::SyncDoc> = Vec::new();
        let mut items: Vec<sync_format::SyncItem> = Vec::new();
        let mut pays: Vec<sync_format::SyncPay> = Vec::new();
        for line in &body_lines {
            if line.starts_with("DOC|") {
                docs.push(sync_format::parse_doc_line(line).map_err(ApplicationError::Validation)?);
            } else if line.starts_with("ITEM|") {
                items.push(
                    sync_format::parse_item_line(line).map_err(ApplicationError::Validation)?,
                );
            } else if line.starts_with("PAY|") {
                pays.push(sync_format::parse_pay_line(line).map_err(ApplicationError::Validation)?);
            } else if line.trim().is_empty() {
                continue;
            } else {
                return Err(coded("SYNC_UNKNOWN_LINE"));
            }
        }
        if docs.len() != doc_count {
            return Err(coded("SYNC_COUNT_MISMATCH"));
        }
        if docs.is_empty() {
            return Err(coded("SYNC_EMPTY_PACKAGE"));
        }
        if docs.len() > sync_format::MAX_PACKAGE_DOCS {
            return Err(coded("SYNC_PACKAGE_TOO_LARGE"));
        }
        let mut items_by_doc: HashMap<&str, Vec<&sync_format::SyncItem>> = HashMap::new();
        for item in &items {
            items_by_doc
                .entry(item.doc_origin_number.as_str())
                .or_default()
                .push(item);
        }
        let mut pays_by_doc: HashMap<&str, Vec<&sync_format::SyncPay>> = HashMap::new();
        for pay in &pays {
            pays_by_doc
                .entry(pay.doc_origin_number.as_str())
                .or_default()
                .push(pay);
        }
        for doc in &docs {
            match DocumentType::from_code(&doc.doc_type_code) {
                Ok(DocumentType::Ticket) | Ok(DocumentType::Bl) | Ok(DocumentType::Facture) => {}
                _ => return Err(coded("SYNC_UNSUPPORTED_TYPE")),
            }
            if !items_by_doc.contains_key(doc.origin_number.as_str()) {
                return Err(coded("SYNC_DOC_WITHOUT_ITEMS"));
            }
            if let Some(doc_pays) = pays_by_doc.get(doc.origin_number.as_str()) {
                for pay in doc_pays {
                    let pay_customer = pay.customer_code.as_deref().unwrap_or("");
                    let doc_customer = doc.party_code.as_deref().unwrap_or("");
                    if pay.method_code == "CREDIT" && pay_customer != doc_customer {
                        return Err(coded("SYNC_CREDIT_MISMATCH"));
                    }
                }
            }
        }

        let mut guard = self.lock()?;
        let outcome = run_transaction(&mut guard, |tx| {
            // Warehouse must exist (importer chooses the pit, not the package).
            let warehouse_ok: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM warehouses WHERE id = ?1 AND is_active = 1",
                    [warehouse_id],
                    |row| row.get(0),
                )
                .map_err(crate::persistence::InfrastructureError::from)?;
            if warehouse_ok != 1 {
                return Err(tx_failure("SYNC_UNKNOWN_WAREHOUSE".to_string()));
            }
            // Renumber plan from the IMPORTER's live sequences (pure domain).
            let mut nexts: HashMap<(String, i32), i64> = HashMap::new();
            for doc in &docs {
                let key = (doc.doc_type_code.clone(), doc.fiscal_year);
                if let std::collections::hash_map::Entry::Vacant(slot) = nexts.entry(key) {
                    let last: Option<i64> = tx
                        .query_row(
                            "SELECT last_sequence FROM document_sequences
                             WHERE type = ?1 AND fiscal_year = ?2",
                            params![doc.doc_type_code, doc.fiscal_year],
                            |row| row.get(0),
                        )
                        .map(Some)
                        .or_else(|error| match error {
                            rusqlite::Error::QueryReturnedNoRows => Ok(None),
                            other => Err(other),
                        })?;
                    slot.insert(last.map_or(1, |last| last + 1).max(1));
                }
            }
            let planned = sync_format::plan_renumbers(&docs, &nexts, &|code| {
                DocumentType::from_code(code)
                    .ok()
                    .map(|kind| kind.prefix().to_string())
            })
            .map_err(|error| tx_failure(format!("SYNC_PLAN:{error}")))?;
            let mut renumbered = Vec::with_capacity(docs.len());
            let mut warnings: Vec<(String, String)> = Vec::new();
            for (doc, plan) in docs.iter().zip(planned.iter()) {
                let doc_type = DocumentType::from_code(&doc.doc_type_code)
                    .map_err(|error| tx_failure(error.to_string()))?;
                // Gapless allocation enforces the plan (exclusive tx: no drift).
                let sequence = allocate_in_tx(tx, doc_type.code(), doc.fiscal_year)?;
                let expected: i64 = nexts
                    .get(&(doc.doc_type_code.clone(), doc.fiscal_year))
                    .copied()
                    .unwrap_or(1);
                // Consume the planned slot.
                nexts.insert((doc.doc_type_code.clone(), doc.fiscal_year), expected + 1);
                if sequence != expected {
                    return Err(tx_failure("SYNC_SEQUENCE_DRIFT".to_string()));
                }
                let customer_id: Option<String> = match &doc.party_code {
                    Some(code) => Some(
                        tx.query_row("SELECT id FROM customers WHERE code = ?1", [code], |row| {
                            row.get(0)
                        })
                        .map_err(|error| match error {
                            rusqlite::Error::QueryReturnedNoRows => {
                                tx_failure(format!("SYNC_UNKNOWN_CUSTOMER:{code}"))
                            }
                            other => crate::persistence::InfrastructureError::from(other),
                        })?,
                    ),
                    None => None,
                };
                let document_id = Uuid::new_v4().to_string();
                let stamp = now_rfc3339;
                let insert = tx.execute(
                    "INSERT INTO documents
                     (id, number, fiscal_year, type, party_type, party_id, warehouse_id,
                      document_language, language, subtotal_ht_centimes, vat_total_centimes,
                      global_discount_centimes, timbre_centimes, rounding_centimes,
                      ttc_centimes, paid_amount_centimes, change_amount_centimes,
                      payment_method, status, created_at, created_by, revision,
                      origin_device_id, origin_number, credit_solde_before_centimes,
                      credit_solde_after_centimes, credit_delta_centimes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, ?12, ?13,
                             ?14, ?15, ?16, ?17, 'FINALIZED', ?18, ?19, 1, ?20, ?21,
                             NULL, NULL, NULL)",
                    params![
                        document_id,
                        plan.new_number,
                        i64::from(doc.fiscal_year),
                        doc.doc_type_code,
                        customer_id.as_ref().map(|_| "CUSTOMER"),
                        customer_id,
                        warehouse_id,
                        doc.language,
                        doc.subtotal_ht_centimes,
                        doc.vat_total_centimes,
                        doc.global_discount_centimes,
                        doc.timbre_centimes,
                        doc.rounding_centimes,
                        doc.ttc_centimes,
                        doc.paid_amount_centimes,
                        doc.change_amount_centimes,
                        doc.payment_method,
                        doc.created_at,
                        actor_user_id,
                        origin_terminal,
                        doc.origin_number,
                    ],
                );
                if let Err(error) = insert {
                    if is_unique_violation(&error) {
                        return Err(tx_failure("SYNC_ALREADY_IMPORTED".to_string()));
                    }
                    return Err(crate::persistence::InfrastructureError::from(error));
                }
                // Items (resolved identity, verbatim money).
                if let Some(doc_items) = items_by_doc.get(doc.origin_number.as_str()) {
                    for item in doc_items {
                        let resolved = resolve_item(tx, item)?;
                        tx.execute(
                            "INSERT INTO document_items
                             (id, document_id, product_id, unit_id, quantity_micro,
                              quantity_base_micro, unit_price_centimes, discount_centimes,
                              discount_per_unit_centimes, tva_rate_micro, line_ht_centimes,
                              line_tva_centimes, line_ttc_centimes, cost_price_centimes,
                              sort_order)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, 0, 0, ?10, ?11, ?12)",
                            params![
                                Uuid::new_v4().to_string(),
                                document_id,
                                resolved.product_id,
                                resolved.unit_id,
                                item.quantity_micro,
                                item.quantity_base_micro,
                                item.unit_price_centimes,
                                item.discount_centimes,
                                item.tva_rate_micro,
                                item.line_ttc_centimes,
                                item.cost_price_centimes,
                                item.sort_order,
                            ],
                        )?;
                        // Stock move on the IMPORTER's warehouse (importer wins).
                        let (_, _, override_applied) = apply_one(
                            tx,
                            &resolved.product_id,
                            warehouse_id,
                            &resolved.unit_id,
                            item.quantity_micro,
                            StockMovementType::SaleOut,
                            Some("SYNC_SALE"),
                            Some(document_id.as_str()),
                            None,
                            true,
                            Some("two-till sync import"),
                            actor_user_id,
                            stamp,
                        )?;
                        if override_applied {
                            warnings.push((item.sku.clone(), plan.new_number.clone()));
                        }
                    }
                }
                // Payments (never into a cash session: money stays put).
                if let Some(doc_pays) = pays_by_doc.get(doc.origin_number.as_str()) {
                    for pay in doc_pays {
                        let pay_sequence: i64 = tx.query_row(
                            "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM payments",
                            [],
                            |row| row.get(0),
                        )?;
                        let pay_customer: Option<String> = match &pay.customer_code {
                            Some(code) => Some(resolve_customer_id(tx, code)?),
                            None => None,
                        };
                        tx.execute(
                            "INSERT INTO payments
                             (id, sequence_no, document_id, customer_id, amount_centimes,
                              payment_method, cash_session_id, created_by, created_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)",
                            params![
                                Uuid::new_v4().to_string(),
                                pay_sequence,
                                document_id,
                                pay_customer,
                                pay.amount_centimes,
                                pay.method_code,
                                actor_user_id,
                                stamp,
                            ],
                        )?;
                    }
                }
                // Credit leg recomputed locally (origin authorized it; the
                // importer does NOT re-enforce the limit — dropping a real
                // debt over a limit technicality would falsify the ledger).
                let credit = doc.ttc_centimes.saturating_sub(doc.paid_amount_centimes);
                if credit > 0 {
                    let customer = customer_id
                        .ok_or_else(|| tx_failure("SYNC_CREDIT_WITHOUT_CUSTOMER".to_string()))?;
                    let balance: i64 = tx.query_row(
                        "SELECT current_balance_centimes FROM customers WHERE id = ?1",
                        [&customer],
                        |row| row.get(0),
                    )?;
                    let new_balance = balance
                        .checked_add(credit)
                        .ok_or_else(|| tx_failure(sentinel::OVERFLOW.to_string()))?;
                    let ledger_sequence: i64 = tx.query_row(
                        "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM customer_ledger",
                        [],
                        |row| row.get(0),
                    )?;
                    tx.execute(
                        "INSERT INTO customer_ledger
                         (id, sequence_no, customer_id, entry_type, amount_centimes,
                          running_balance_centimes, document_type, document_id,
                          created_by, created_at)
                         VALUES (?1, ?2, ?3, 'DEBIT', ?4, ?5, 'SALE', ?6, ?7, ?8)",
                        params![
                            Uuid::new_v4().to_string(),
                            ledger_sequence,
                            customer,
                            credit,
                            new_balance,
                            document_id,
                            actor_user_id,
                            stamp,
                        ],
                    )?;
                    tx.execute(
                        "UPDATE customers
                         SET current_balance_centimes = ?1, revision = revision + 1,
                             updated_at = ?2
                         WHERE id = ?3",
                        params![new_balance, stamp, customer],
                    )?;
                }
                renumbered.push(SyncRenumber {
                    origin_number: doc.origin_number.clone(),
                    new_number: plan.new_number.clone(),
                });
            }
            Ok(SyncImportOutcome {
                imported_docs: renumbered.len(),
                renumbered,
                negative_stock_warnings: warnings,
            })
        });
        // SYNC_* codes cross the boundary verbatim (fail-closed UI contract);
        // every other engine sentinel keeps the shared static mapping.
        outcome.map_err(|error| match error {
            crate::persistence::InfrastructureError::Migration(message)
                if message.starts_with("SYNC_") =>
            {
                coded(&message)
            }
            other => map_engine_error(other),
        })
    }
}
