//! SQLite purchase engine: BC orders, BR receipts, FF invoices, payments.
//!
//! Every mutating method runs inside ONE `run_transaction` with
//! Lookup-then-Append idempotency against `command_log` (shared helpers with
//! the cash engine). ADR-029 is enforced structurally: an FF sourced from a
//! BR never touches stock; a standalone FF performs the PURCHASE_IN
//! movements itself, atomically, exactly once. ADR-030: CASH payments
//! require the OPEN session (shared precondition helper) and insert one
//! negative SUPPLIER_PAYMENT_OUT movement; OTHER payments never touch the
//! drawer. All money is `i64` centimes; no float exists in this module.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    PurchaseDocumentView, PurchaseLineInput, PurchaseLineView, PurchaseRepository,
    SupplierPaymentOutcome,
};
use baraka_domain::catalog::convert_to_base_micros;
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{StockMovementType, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money, QtyMicro};
use baraka_domain::sales::{format_number, DocumentType};
use baraka_domain::user::UserId;
use chrono::{Datelike, SecondsFormat, Utc};
use rusqlite::{params, Connection, Transaction};
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_cash_audit_row;
use crate::adapters::sqlite_cash_session_repository::{
    append_or_replay, evaluate_stored_json, insert_supplier_payment_out_in_tx, lookup_command,
    require_open_session_in_tx,
};
use crate::adapters::sqlite_document_sequence_repository::allocate_in_tx;
use crate::adapters::sqlite_stock_repository::{apply_one, map_engine_error, sentinel, tx_failure};
use crate::persistence::migration::checksum_of;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;

/// Purchase-document header insert (shared columns for BC/BR/FF).
/// Purchase admin documents are never printed; `FR` is an explicit literal
/// choice for the layout column, not a sale-time receipt capture.
const DOCUMENT_INSERT_SQL: &str = "INSERT INTO documents
     (id, number, fiscal_year, type, party_type, party_id, source_document_id,
      warehouse_id, document_language, language, subtotal_ht_centimes,
      vat_total_centimes, global_discount_centimes, timbre_centimes,
      rounding_centimes, ttc_centimes, paid_amount_centimes,
      change_amount_centimes, payment_method, status, created_at, created_by,
      revision, origin_device_id, notes)
     VALUES (?1, ?2, ?3, ?4, 'SUPPLIER', ?5, ?6, ?7, 'FR', 'FR', ?8,
             0, 0, 0, 0, ?8, 0, 0, ?9, ?10, ?11, ?12, 1, ?13, NULL)";

/// Purchase line snapshot insert (TVA fields are zero pre tax engine).
const ITEM_INSERT_SQL: &str = "INSERT INTO document_items
     (id, document_id, product_id, unit_id, source_item_id, quantity_micro,
      quantity_base_micro, unit_price_centimes, discount_centimes,
      tva_rate_micro, tax_exemption_code, line_ht_centimes, line_tva_centimes,
      line_ttc_centimes, cost_price_centimes, sort_order)
     VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, 0, 0, NULL, ?8, 0, ?8, ?7, ?9)";

// Local sentinel codes (mapped 1:1 to typed domain errors at the edge).
const SUPPLIER_MISSING: &str = "SUPPLIER_MISSING";
const SUPPLIER_INACTIVE: &str = "SUPPLIER_INACTIVE";
const PURCHASE_ORDER_NOT_FOUND: &str = "PURCHASE_ORDER_NOT_FOUND";
const PURCHASE_ORDER_NOT_DRAFT: &str = "PURCHASE_ORDER_NOT_DRAFT";
const PURCHASE_ORDER_NOT_OPEN: &str = "PURCHASE_ORDER_NOT_OPEN";
const SUPPLIER_MISMATCH: &str = "SUPPLIER_MISMATCH";
const PURCHASE_LINE_NOT_ON_ORDER: &str = "PURCHASE_LINE_NOT_ON_ORDER";
const PURCHASE_LINE_MISMATCH: &str = "PURCHASE_LINE_MISMATCH";
const PURCHASE_RECEIPT_EXCEEDS: &str = "PURCHASE_RECEIPT_EXCEEDS";
const FF_ALREADY_SOURCED: &str = "FF_ALREADY_SOURCED";

/// Translate one purchase sentinel; anything else falls through to the
/// shared engine mapper (stock/cash/idempotency sentinels).
fn map_purchase_error(error: InfrastructureError) -> ApplicationError {
    if let InfrastructureError::Migration(ref message) = error {
        let mapped = match message.as_str() {
            SUPPLIER_MISSING => Some(DomainError::SupplierNotFound),
            SUPPLIER_INACTIVE => Some(DomainError::SupplierInactive),
            PURCHASE_ORDER_NOT_FOUND | SUPPLIER_MISMATCH => {
                Some(DomainError::PurchaseOrderNotFound)
            }
            PURCHASE_ORDER_NOT_DRAFT | PURCHASE_ORDER_NOT_OPEN => {
                Some(DomainError::PurchaseOrderNotDraft)
            }
            PURCHASE_LINE_NOT_ON_ORDER | PURCHASE_LINE_MISMATCH => {
                Some(DomainError::PurchaseLineQuantityInvalid)
            }
            PURCHASE_RECEIPT_EXCEEDS => Some(DomainError::PurchaseReceiptQuantityExceedsOrdered),
            FF_ALREADY_SOURCED => Some(DomainError::PurchaseInvoiceAlreadySourced),
            _ => None,
        };
        if let Some(domain) = mapped {
            return ApplicationError::Validation(domain);
        }
    }
    map_engine_error(error)
}

/// Purchase persistence over a shared SQLite connection.
pub struct SqlitePurchaseRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqlitePurchaseRepository {
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

/// `(is_active, current_balance_centimes)` for a supplier row.
fn supplier_row(conn: &Connection, supplier_id: &str) -> Result<(bool, i64), InfrastructureError> {
    conn.query_row(
        "SELECT is_active, current_balance_centimes FROM suppliers WHERE id = ?1",
        [supplier_id],
        |row| Ok((row.get::<_, i64>(0)? != 0, row.get(1)?)),
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => tx_failure(SUPPLIER_MISSING.to_string()),
        other => InfrastructureError::from(other),
    })
}

/// One purchase line computed snapshot: (base micros, line total centimes).
fn compute_line(
    conn: &Connection,
    line: &PurchaseLineInput,
) -> Result<(i64, i64), InfrastructureError> {
    let factor: Option<i64> = conn
        .query_row(
            "SELECT factor_micro FROM product_units WHERE id = ?1 AND product_id = ?2",
            params![line.unit_id.to_string(), line.product_id.to_string()],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    let factor = factor.ok_or_else(|| tx_failure(sentinel::MISSING_UNIT.to_string()))?;
    let base_micro = convert_to_base_micros(line.quantity_micro, FactorMicro::from_micros(factor))
        .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
    let total_centimes = Money::from_centimes(line.unit_cost_centimes)
        .checked_mul_qty(QtyMicro::from_micros(line.quantity_micro))
        .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?
        .centimes();
    Ok((base_micro, total_centimes))
}

/// Insert one document_items snapshot row; returns the stored line view.
fn insert_item(
    conn: &Connection,
    document_id: &str,
    line: &PurchaseLineInput,
    sort_order: i64,
) -> Result<PurchaseLineView, InfrastructureError> {
    let (base_micro, total_centimes) = compute_line(conn, line)?;
    let item_id = Uuid::new_v4().to_string();
    conn.execute(
        ITEM_INSERT_SQL,
        params![
            item_id,
            document_id,
            line.product_id.to_string(),
            line.unit_id.to_string(),
            line.quantity_micro,
            base_micro,
            line.unit_cost_centimes,
            total_centimes,
            sort_order,
        ],
    )?;
    Ok(PurchaseLineView {
        id: item_id,
        product_id: line.product_id.to_string(),
        unit_id: line.unit_id.to_string(),
        quantity_micro: line.quantity_micro,
        quantity_base_micro: base_micro,
        unit_cost_centimes: line.unit_cost_centimes,
        line_total_centimes: total_centimes,
    })
}

/// Grouped quantities `product|unit -> sum(micros)` for one document.
fn grouped_quantities(
    conn: &Connection,
    document_id: &str,
) -> Result<HashMap<String, i64>, InfrastructureError> {
    let mut statement = conn.prepare(
        "SELECT product_id, unit_id, SUM(quantity_micro)
         FROM document_items WHERE document_id = ?1 GROUP BY product_id, unit_id",
    )?;
    let rows = statement
        .query_map([document_id], |row| {
            let product: String = row.get(0)?;
            let unit: String = row.get(1)?;
            let quantity: i64 = row.get(2)?;
            Ok((format!("{product}|{unit}"), quantity))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows.into_iter().collect())
}

/// Cumulative quantity already received for one BC line group.
fn received_for_group(
    conn: &Connection,
    bc_id: &str,
    product_id: &str,
    unit_id: &str,
) -> Result<i64, InfrastructureError> {
    conn.query_row(
        "SELECT COALESCE(SUM(di.quantity_micro), 0)
         FROM documents d
         JOIN document_items di ON di.document_id = d.id
         WHERE d.source_document_id = ?1 AND d.type = 'BR'
           AND di.product_id = ?2 AND di.unit_id = ?3",
        params![bc_id, product_id, unit_id],
        |row| row.get(0),
    )
    .map_err(InfrastructureError::from)
}

/// Load one purchase document (header + ordered lines). Transaction
/// handles deref to `Connection`, so this serves both call sites.
fn load_purchase_document(
    conn: &Connection,
    document_id: &str,
) -> Result<Option<PurchaseDocumentView>, InfrastructureError> {
    // 10-tuple mirrors the SELECT list exactly (same narrowly-scoped
    // allowance already used for the cash-session header tuples).
    #[allow(clippy::type_complexity)]
    let header: Option<(
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
        String,
        String,
    )> = conn
        .query_row(
            "SELECT id, number, type, status, party_id, warehouse_id,
                    source_document_id, ttc_centimes, created_at, created_by
             FROM documents WHERE id = ?1 AND type IN ('BC','BR','FF')",
            [document_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    let Some((
        id,
        number,
        doc_type,
        status,
        supplier_id,
        warehouse_id,
        source,
        total,
        created_at,
        created_by,
    )) = header
    else {
        return Ok(None);
    };
    let mut statement = conn.prepare(
        "SELECT id, product_id, unit_id, quantity_micro, quantity_base_micro,
                unit_price_centimes, line_ht_centimes
         FROM document_items WHERE document_id = ?1 ORDER BY sort_order",
    )?;
    let lines = statement
        .query_map([id.as_str()], |row| {
            Ok(PurchaseLineView {
                id: row.get(0)?,
                product_id: row.get(1)?,
                unit_id: row.get(2)?,
                quantity_micro: row.get(3)?,
                quantity_base_micro: row.get(4)?,
                unit_cost_centimes: row.get(5)?,
                line_total_centimes: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(PurchaseDocumentView {
        id,
        number,
        doc_type,
        status,
        supplier_id,
        warehouse_id,
        source_document_id: source,
        total_centimes: total,
        lines,
        created_at,
        created_by,
    }))
}

/// Allocate a gapless number and insert the purchase document header.
#[allow(clippy::too_many_arguments)]
fn insert_purchase_document(
    tx: &Transaction,
    document_id: &str,
    doc_type: DocumentType,
    supplier_id: &str,
    warehouse_id: &str,
    source_document_id: Option<&str>,
    total_centimes: i64,
    payment_method: &str,
    status: &str,
    stamp: &str,
    actor: &str,
    origin_device_id: &str,
) -> Result<String, InfrastructureError> {
    let fiscal_year = Utc::now().year();
    let sequence = allocate_in_tx(tx, doc_type.code(), fiscal_year)?;
    let number = format_number(doc_type.prefix(), fiscal_year, sequence);
    tx.execute(
        DOCUMENT_INSERT_SQL,
        params![
            document_id,
            number,
            i64::from(fiscal_year),
            doc_type.code(),
            supplier_id,
            source_document_id,
            warehouse_id,
            total_centimes,
            payment_method,
            status,
            stamp,
            actor,
            origin_device_id,
        ],
    )?;
    Ok(number)
}

impl PurchaseRepository for SqlitePurchaseRepository {
    #[allow(clippy::too_many_arguments)]
    fn create_order(
        &self,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: &[PurchaseLineInput],
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let supplier_text = supplier_id.trim().to_string();
        let warehouse_text = warehouse_id.to_string();
        let terminal_text = terminal_id.trim().to_string();
        let command_text = command_id.trim().to_string();
        let lines = lines.to_vec();
        run_transaction(&mut guard, |tx| {
            let conn: &Connection = tx;
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<PurchaseDocumentView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let (is_active, _) = supplier_row(conn, &supplier_text)?;
            if !is_active {
                return Err(tx_failure(SUPPLIER_INACTIVE.to_string()));
            }
            // Precompute totals before the header insert (line snapshots
            // need only reads; item rows land after the parent document).
            let mut computed = Vec::with_capacity(lines.len());
            let mut total: i128 = 0;
            for line in &lines {
                let snapshot = compute_line(conn, line)?;
                total += i128::from(snapshot.1);
                computed.push(snapshot);
            }
            let total = i64::try_from(total).map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
            let document_id = Uuid::new_v4().to_string();
            let number = insert_purchase_document(
                tx,
                &document_id,
                DocumentType::Bc,
                &supplier_text,
                &warehouse_text,
                None,
                total,
                "NONE",
                "DRAFT",
                &stamp,
                &actor_text,
                &terminal_text,
            )?;
            for (index, (line, (base_micro, line_total))) in
                lines.iter().zip(computed.iter()).enumerate()
            {
                let item_id = Uuid::new_v4().to_string();
                conn.execute(
                    ITEM_INSERT_SQL,
                    params![
                        item_id,
                        document_id,
                        line.product_id.to_string(),
                        line.unit_id.to_string(),
                        line.quantity_micro,
                        base_micro,
                        line.unit_cost_centimes,
                        line_total,
                        index as i64,
                    ],
                )?;
            }
            let new_json =
                format!("{{\"document_id\":\"{document_id}\",\"number\":\"{number}\",\"total_centimes\":{total}}}");
            insert_cash_audit_row(
                tx,
                "PURCHASE_ORDER_CREATED",
                "documents",
                &document_id,
                None,
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let view = load_purchase_document(conn, &document_id)?
                .ok_or_else(|| tx_failure(PURCHASE_ORDER_NOT_FOUND.to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "purchases.createOrder",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_purchase_error)
    }

    fn update_order(
        &self,
        document_id: &str,
        lines: &[PurchaseLineInput],
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let terminal_text = terminal_id.trim().to_string();
        let document_text = document_id.trim().to_string();
        let command_text = command_id.trim().to_string();
        let lines = lines.to_vec();
        run_transaction(&mut guard, |tx| {
            let conn: &Connection = tx;
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) = evaluate_stored_json::<PurchaseDocumentView>(
                    &stored_hash,
                    &payload,
                    &request_hash,
                )? {
                    return Ok(replay);
                }
            }
            let status: String = conn
                .query_row(
                    "SELECT status FROM documents WHERE id = ?1 AND type = 'BC'",
                    [&document_text],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => {
                        tx_failure(PURCHASE_ORDER_NOT_FOUND.to_string())
                    }
                    other => InfrastructureError::from(other),
                })?;
            if status != "DRAFT" {
                return Err(tx_failure(PURCHASE_ORDER_NOT_DRAFT.to_string()));
            }
            let mut total: i128 = 0;
            let mut computed = Vec::with_capacity(lines.len());
            for line in &lines {
                let snapshot = compute_line(conn, line)?;
                total += i128::from(snapshot.1);
                computed.push(snapshot);
            }
            let total =
                i64::try_from(total).map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
            // DRAFT item replacement is trigger-safe (parent is not finalized).
            conn.execute(
                "DELETE FROM document_items WHERE document_id = ?1",
                [&document_text],
            )?;
            for (index, (line, (base_micro, line_total))) in
                lines.iter().zip(computed.iter()).enumerate()
            {
                conn.execute(
                    ITEM_INSERT_SQL,
                    params![
                        Uuid::new_v4().to_string(),
                        document_text,
                        line.product_id.to_string(),
                        line.unit_id.to_string(),
                        line.quantity_micro,
                        base_micro,
                        line.unit_cost_centimes,
                        line_total,
                        index as i64,
                    ],
                )?;
            }
            let changed = conn.execute(
                "UPDATE documents
                 SET subtotal_ht_centimes = ?1, ttc_centimes = ?1,
                     revision = revision + 1
                 WHERE id = ?2 AND status = 'DRAFT'",
                params![total, document_text],
            )?;
            if changed != 1 {
                return Err(tx_failure(PURCHASE_ORDER_NOT_DRAFT.to_string()));
            }
            let new_json = format!(
                "{{\"document_id\":\"{document_text}\",\"total_centimes\":{total},\"terminal_id\":\"{terminal_text}\"}}"
            );
            insert_cash_audit_row(
                tx,
                "PURCHASE_ORDER_UPDATED",
                "documents",
                &document_text,
                None,
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let view = load_purchase_document(conn, &document_text)?
                .ok_or_else(|| tx_failure(PURCHASE_ORDER_NOT_FOUND.to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "purchases.updateOrder",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_purchase_error)
    }

    #[allow(clippy::too_many_arguments)]
    fn receive_goods(
        &self,
        source_bc_id: Option<&str>,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: &[PurchaseLineInput],
        override_allowed: bool,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let supplier_text = supplier_id.trim().to_string();
        let warehouse_text = warehouse_id.to_string();
        let terminal_text = terminal_id.trim().to_string();
        let command_text = command_id.trim().to_string();
        let source_text = source_bc_id.map(str::trim).map(str::to_string);
        let lines = lines.to_vec();
        run_transaction(&mut guard, |tx| {
            let conn: &Connection = tx;
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<PurchaseDocumentView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let (is_active, _) = supplier_row(conn, &supplier_text)?;
            if !is_active {
                return Err(tx_failure(SUPPLIER_INACTIVE.to_string()));
            }
            // --- BC guards (ADR-029: BR is the physical fact; BC is intent).
            let mut over_receipt_detected = false;
            if let Some(bc_id) = source_text.as_deref() {
                let (bc_status, bc_party): (String, Option<String>) = conn
                    .query_row(
                        "SELECT status, party_id FROM documents WHERE id = ?1 AND type = 'BC'",
                        [bc_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| match error {
                        rusqlite::Error::QueryReturnedNoRows => {
                            tx_failure(PURCHASE_ORDER_NOT_FOUND.to_string())
                        }
                        other => InfrastructureError::from(other),
                    })?;
                if bc_party.as_deref() != Some(supplier_text.as_str()) {
                    return Err(tx_failure(SUPPLIER_MISMATCH.to_string()));
                }
                if !matches!(bc_status.as_str(), "DRAFT" | "PARTIALLY_RECEIVED") {
                    return Err(tx_failure(PURCHASE_ORDER_NOT_OPEN.to_string()));
                }
                let ordered = grouped_quantities(conn, bc_id)?;
                // Batch the incoming quantities per (product|unit) group.
                let mut batch: HashMap<String, i64> = HashMap::new();
                for line in &lines {
                    let key = format!("{}|{}", line.product_id, line.unit_id);
                    *batch.entry(key).or_insert(0) += line.quantity_micro;
                }
                for (key, quantity) in &batch {
                    let Some(ordered_quantity) = ordered.get(key) else {
                        return Err(tx_failure(PURCHASE_LINE_NOT_ON_ORDER.to_string()));
                    };
                    let (product, unit) = key
                        .split_once('|')
                        .ok_or_else(|| tx_failure(PURCHASE_LINE_NOT_ON_ORDER.to_string()))?;
                    let already = received_for_group(conn, bc_id, product, unit)?;
                    let expected_max = *ordered_quantity;
                    if already + *quantity > expected_max {
                        if !override_allowed {
                            return Err(tx_failure(PURCHASE_RECEIPT_EXCEEDS.to_string()));
                        }
                        over_receipt_detected = true;
                    }
                }
            }
            // --- BR header + items.
            let document_id = Uuid::new_v4().to_string();
            let mut total: i128 = 0;
            let mut computed = Vec::with_capacity(lines.len());
            for line in &lines {
                let snapshot = compute_line(conn, line)?;
                total += i128::from(snapshot.1);
                computed.push(snapshot);
            }
            let total = i64::try_from(total).map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
            let number = insert_purchase_document(
                tx,
                &document_id,
                DocumentType::Br,
                &supplier_text,
                &warehouse_text,
                source_text.as_deref(),
                total,
                "NONE",
                "FINALIZED",
                &stamp,
                &actor_text,
                &terminal_text,
            )?;
            for (index, (line, (base_micro, line_total))) in
                lines.iter().zip(computed.iter()).enumerate()
            {
                conn.execute(
                    ITEM_INSERT_SQL,
                    params![
                        Uuid::new_v4().to_string(),
                        document_id,
                        line.product_id.to_string(),
                        line.unit_id.to_string(),
                        line.quantity_micro,
                        base_micro,
                        line.unit_cost_centimes,
                        line_total,
                        index as i64,
                    ],
                )?;
                // Stock effect of the physical receipt (engine is the only
                // writer: level delta + append-only movement row).
                apply_one(
                    tx,
                    &line.product_id.to_string(),
                    &warehouse_text,
                    &line.unit_id.to_string(),
                    line.quantity_micro,
                    StockMovementType::PurchaseIn,
                    Some("BR"),
                    Some(document_id.as_str()),
                    None,
                    false,
                    Some("Goods received"),
                    &actor_text,
                    &stamp,
                )?;
            }
            // --- BC rollup after the BR committed in this transaction.
            if let Some(bc_id) = source_text.as_deref() {
                let ordered = grouped_quantities(conn, bc_id)?;
                let mut statement = conn.prepare(
                    "SELECT di.product_id, di.unit_id, SUM(di.quantity_micro)
                     FROM documents d
                     JOIN document_items di ON di.document_id = d.id
                     WHERE d.source_document_id = ?1 AND d.type = 'BR'
                     GROUP BY di.product_id, di.unit_id",
                )?;
                let rows = statement
                    .query_map([bc_id], |row| {
                        let product: String = row.get(0)?;
                        let unit: String = row.get(1)?;
                        let quantity: i64 = row.get(2)?;
                        Ok((format!("{product}|{unit}"), quantity))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                let received: HashMap<String, i64> = rows.into_iter().collect();
                let fully = ordered.iter().all(|(key, ordered_quantity)| {
                    received.get(key).copied().unwrap_or(0) >= *ordered_quantity
                });
                let new_status = if fully { "FULLY_RECEIVED" } else { "PARTIALLY_RECEIVED" };
                let changed = conn.execute(
                    "UPDATE documents
                     SET status = ?1, revision = revision + 1
                     WHERE id = ?2 AND status IN ('DRAFT','PARTIALLY_RECEIVED')",
                    params![new_status, bc_id],
                )?;
                if changed != 1 {
                    return Err(tx_failure(sentinel::CONCURRENCY.to_string()));
                }
            }
            let new_json = format!(
                "{{\"document_id\":\"{document_id}\",\"number\":\"{number}\",\"source_bc\":{:?},\"total_centimes\":{total}}}",
                source_text
            );
            insert_cash_audit_row(
                tx,
                "GOODS_RECEIVED",
                "documents",
                &document_id,
                None,
                &new_json,
                &actor_text,
                &stamp,
            )?;
            if over_receipt_detected {
                let override_json = format!(
                    "{{\"OVERRIDE\":\"QUANTITY_EXCEEDED\",\"bc_id\":{:?},\"br_id\":\"{document_id}\"}}",
                    source_text
                );
                insert_cash_audit_row(
                    tx,
                    "PURCHASE_RECEIPT_QUANTITY_OVERRIDE",
                    "documents",
                    &document_id,
                    None,
                    &override_json,
                    &actor_text,
                    &stamp,
                )?;
            }
            let view = load_purchase_document(conn, &document_id)?
                .ok_or_else(|| tx_failure(PURCHASE_ORDER_NOT_FOUND.to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "purchases.receiveGoods",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_purchase_error)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_invoice(
        &self,
        source_br_id: Option<&str>,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: &[PurchaseLineInput],
        invoice_total_centimes: i64,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let supplier_text = supplier_id.trim().to_string();
        let warehouse_text = warehouse_id.to_string();
        let terminal_text = terminal_id.trim().to_string();
        let command_text = command_id.trim().to_string();
        let source_text = source_br_id.map(str::trim).map(str::to_string);
        let lines = lines.to_vec();
        run_transaction(&mut guard, |tx| {
            let conn: &Connection = tx;
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<PurchaseDocumentView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let (is_active, balance) = supplier_row(conn, &supplier_text)?;
            if !is_active {
                return Err(tx_failure(SUPPLIER_INACTIVE.to_string()));
            }
            if let Some(br_id) = source_text.as_deref() {
                let br_party: Option<String> = conn
                    .query_row(
                        "SELECT party_id FROM documents WHERE id = ?1 AND type = 'BR'",
                        [br_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| match error {
                        rusqlite::Error::QueryReturnedNoRows => {
                            tx_failure(PURCHASE_ORDER_NOT_FOUND.to_string())
                        }
                        other => InfrastructureError::from(other),
                    })?;
                if br_party.as_deref() != Some(supplier_text.as_str()) {
                    return Err(tx_failure(SUPPLIER_MISMATCH.to_string()));
                }
                // Double-invoicing guard (ADR-029): at most one FF per BR.
                let existing: Option<String> = conn
                    .query_row(
                        "SELECT id FROM documents WHERE source_document_id = ?1 AND type = 'FF' LIMIT 1",
                        [br_id],
                        |row| row.get(0),
                    )
                    .map(Some)
                    .or_else(|error| match error {
                        rusqlite::Error::QueryReturnedNoRows => Ok(None),
                        other => Err(other),
                    })?;
                if existing.is_some() {
                    return Err(tx_failure(FF_ALREADY_SOURCED.to_string()));
                }
                // Financial reconciliation: quantities must match the BR.
                let br_groups = grouped_quantities(conn, br_id)?;
                let mut input_groups: HashMap<String, i64> = HashMap::new();
                for line in &lines {
                    *input_groups
                        .entry(format!("{}|{}", line.product_id, line.unit_id))
                        .or_insert(0) += line.quantity_micro;
                }
                if br_groups != input_groups {
                    return Err(tx_failure(PURCHASE_LINE_MISMATCH.to_string()));
                }
            }
            let document_id = Uuid::new_v4().to_string();
            let number = insert_purchase_document(
                tx,
                &document_id,
                DocumentType::Ff,
                &supplier_text,
                &warehouse_text,
                source_text.as_deref(),
                invoice_total_centimes,
                "PENDING",
                "FINALIZED",
                &stamp,
                &actor_text,
                &terminal_text,
            )?;
            for (index, line) in lines.iter().enumerate() {
                insert_item(conn, &document_id, line, index as i64)?;
                // ADR-029 guard branch: stock moves here ONLY for a
                // standalone FF (no source BR). A BR-sourced FF must not
                // touch stock — the goods entered at BR time already.
                if source_text.is_none() {
                    apply_one(
                        tx,
                        &line.product_id.to_string(),
                        &warehouse_text,
                        &line.unit_id.to_string(),
                        line.quantity_micro,
                        StockMovementType::PurchaseIn,
                        Some("FF"),
                        Some(document_id.as_str()),
                        None,
                        false,
                        Some("Standalone purchase invoice"),
                        &actor_text,
                        &stamp,
                    )?;
                }
            }
            // Single ledger DEBIT for the financial fact.
            let entry_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO supplier_ledger
                 (id, supplier_id, entry_type, amount_centimes,
                  reference_type, reference_id, description, created_at, created_by)
                 VALUES (?1, ?2, 'DEBIT', ?3, 'PURCHASE_INVOICE', ?4, NULL, ?5, ?6)",
                params![
                    entry_id,
                    supplier_text,
                    invoice_total_centimes,
                    document_id,
                    stamp,
                    actor_text,
                ],
            )?;
            let new_balance = i128::from(balance) + i128::from(invoice_total_centimes);
            let new_balance =
                i64::try_from(new_balance).map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
            let changed = conn.execute(
                "UPDATE suppliers SET current_balance_centimes = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_balance, stamp, supplier_text],
            )?;
            if changed != 1 {
                return Err(tx_failure(SUPPLIER_MISSING.to_string()));
            }
            let new_json = format!(
                "{{\"document_id\":\"{document_id}\",\"number\":\"{number}\",\"source_br\":{:?},\"stock_touched\":{},\"ledger_entry_id\":\"{entry_id}\",\"invoice_total_centimes\":{invoice_total_centimes}}}",
                source_text,
                source_text.is_none()
            );
            insert_cash_audit_row(
                tx,
                "SUPPLIER_INVOICE_RECORDED",
                "documents",
                &document_id,
                None,
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let view = load_purchase_document(conn, &document_id)?
                .ok_or_else(|| tx_failure(PURCHASE_ORDER_NOT_FOUND.to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "purchases.recordInvoice",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_purchase_error)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_payment(
        &self,
        supplier_id: &str,
        amount_centimes: i64,
        payment_method: &str,
        description: Option<String>,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<SupplierPaymentOutcome, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let supplier_text = supplier_id.trim().to_string();
        let terminal_text = terminal_id.trim().to_string();
        let command_text = command_id.trim().to_string();
        let method = payment_method.trim().to_ascii_uppercase();
        let description_text = description.clone();
        run_transaction(&mut guard, |tx| {
            let conn: &Connection = tx;
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<SupplierPaymentOutcome>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let (is_active, balance) = supplier_row(conn, &supplier_text)?;
            if !is_active {
                return Err(tx_failure(SUPPLIER_INACTIVE.to_string()));
            }
            if amount_centimes <= 0 {
                return Err(tx_failure("SUPPLIER_PAYMENT_INVALID_AMOUNT".to_string()));
            }
            // ADR-030: only CASH payments touch the drawer (and require the
            // OPEN session, ADR-025). OTHER never checks or writes cash.
            let cash_session_id = match method.as_str() {
                "CASH" => Some(require_open_session_in_tx(tx, &terminal_text)?),
                "OTHER" => None,
                _ => return Err(tx_failure("SUPPLIER_PAYMENT_METHOD_INVALID".to_string())),
            };
            let entry_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO supplier_ledger
                 (id, supplier_id, entry_type, amount_centimes,
                  reference_type, reference_id, description, created_at, created_by)
                 VALUES (?1, ?2, 'CREDIT', ?3, 'PAYMENT', ?4, ?5, ?6, ?7)",
                params![
                    entry_id,
                    supplier_text,
                    amount_centimes,
                    entry_id,
                    description_text,
                    stamp,
                    actor_text,
                ],
            )?;
            let new_balance = i128::from(balance) - i128::from(amount_centimes);
            let new_balance =
                i64::try_from(new_balance).map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
            let changed = conn.execute(
                "UPDATE suppliers SET current_balance_centimes = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_balance, stamp, supplier_text],
            )?;
            if changed != 1 {
                return Err(tx_failure(SUPPLIER_MISSING.to_string()));
            }
            let cash_movement_id = match cash_session_id {
                Some(session_id) => Some(insert_supplier_payment_out_in_tx(
                    tx,
                    &session_id,
                    amount_centimes,
                    &entry_id,
                    description_text.as_deref(),
                    &stamp,
                    &actor_text,
                )?),
                None => None,
            };
            let new_json = format!(
                "{{\"ledger_entry_id\":\"{entry_id}\",\"supplier_id\":\"{supplier_text}\",\"amount_centimes\":{amount_centimes},\"method\":\"{method}\",\"balance_negative\":{},\"cash_movement_id\":{:?}}}",
                new_balance < 0,
                cash_movement_id
            );
            insert_cash_audit_row(
                tx,
                "SUPPLIER_PAYMENT_RECORDED",
                "suppliers",
                &supplier_text,
                None,
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let outcome = SupplierPaymentOutcome {
                ledger_entry_id: entry_id,
                new_balance_centimes: new_balance,
                cash_movement_id,
            };
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "purchases.recordPayment",
                &outcome,
                &stamp,
                &actor_text,
            )
        })
        .map_err(|error| {
            // By-value match: no field stays borrowed when the fall-through
            // mapper consumes the owned error.
            if let InfrastructureError::Migration(ref message) = error {
                match message.as_str() {
                    "SUPPLIER_PAYMENT_INVALID_AMOUNT" => {
                        return ApplicationError::Validation(
                            DomainError::SupplierPaymentInvalidAmount,
                        );
                    }
                    "SUPPLIER_PAYMENT_METHOD_INVALID" => {
                        return ApplicationError::Validation(
                            DomainError::SupplierPaymentMethodInvalid,
                        );
                    }
                    _ => {}
                }
            }
            map_purchase_error(error)
        })
    }

    fn get_document(
        &self,
        document_id: &str,
    ) -> Result<Option<PurchaseDocumentView>, ApplicationError> {
        let guard = self.lock()?;
        load_purchase_document(&guard, document_id.trim()).map_err(map_purchase_error)
    }
}
