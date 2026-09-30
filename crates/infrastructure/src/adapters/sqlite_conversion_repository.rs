//! Single-shot DEVIS-to-BL conversion engine (ADR-040, TASK-BRK-014A).
//!
//! One `run_transaction` (`BEGIN IMMEDIATE`): idempotency lookup-or-replay,
//! source lock + strict in-transaction validation, gapless BL number, BL
//! header + byte-copy items, leaf stock deductions (`SALE_OUT`, the exact
//! movement type the Atomic Sale Engine uses), source seal to `CONVERTED`,
//! `DOCUMENT_CONVERTED` audit event, print outbox job, command log
//! completion. Any failure rolls back everything: no BL, no movements, no
//! seal, no consumed sequence number.
//!
//! Kit lines are expanded by the use case before this boundary (same split
//! as checkout: verify + expand pre-transaction, apply in-transaction), so
//! this module only ever applies leaf deductions via [`apply_one`].

use rusqlite::{Connection, Transaction};
use uuid::Uuid;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CheckoutPayment, ConvertBlToFactureCommand, ConvertBlToFactureResult, ConvertDevisToBlCommand,
    ConvertDevisToBlResult,
};
use baraka_domain::errors::DomainError;
use baraka_domain::sales::DocumentType;
use chrono::{Datelike, SecondsFormat, Utc};

use crate::adapters::sqlite_audit_log::insert_cash_audit_row;
use crate::adapters::sqlite_cash_session_repository::{
    append_or_replay, evaluate_stored_json, lookup_command, require_open_session_in_tx,
};
use crate::adapters::sqlite_document_sequence_repository::allocate_in_tx;
use crate::adapters::sqlite_print_job_repository::enqueue_in_tx;
use crate::adapters::sqlite_stock_repository::{
    apply_one, map_engine_error, next_sequence_no, sentinel, tx_failure,
};
use crate::persistence::migration::checksum_of;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;
use baraka_domain::inventory::StockMovementType;
use baraka_domain::sales::{format_number, DocumentStatus};

// Local sentinel codes (mapped 1:1 to typed domain errors at the edge).
const SOURCE_NOT_FOUND: &str = "DEVIS_NOT_FOUND";
const ALREADY_CONVERTED: &str = "DEVIS_ALREADY_CONVERTED";
const BAD_SOURCE_STATUS: &str = "DEVIS_BAD_STATUS";
const BAD_SOURCE_TYPE: &str = "DEVIS_BAD_TYPE";
const EMPTY_SOURCE: &str = "DEVIS_EMPTY";
// Tender lines do not cover the source total exactly (either direction);
// surfaces as the checkout tender-coverage error (no change concept here).
const TENDER_MISMATCH: &str = "TENDER_MISMATCH";

/// Authoritative in-transaction credit check for financial closings.
///
/// Mirrors the checkout engine's credit gate line-for-line (policy,
/// activity, LIMITED cap with override grant) and returns the new balance.
/// Duplicated here — rather than importing the checkout module's private
/// helper — because TASK-BRK-014B forbids modifying checkout files.
fn authorize_credit_for_closing(
    tx: &Transaction,
    customer_id: &str,
    credit_amount: i64,
    override_granted: bool,
) -> Result<i64, InfrastructureError> {
    let profile: Option<(String, i64, i64, bool)> = tx
        .query_row(
            "SELECT credit_policy, credit_limit_centimes, current_balance_centimes, is_active
             FROM customers WHERE id = ?1",
            [customer_id],
            |row| {
                let policy: String = row.get(0)?;
                let limit: i64 = row.get(1)?;
                let balance: i64 = row.get(2)?;
                let active: bool = row.get::<_, i64>(3)? != 0;
                Ok((policy, limit, balance, active))
            },
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    let (policy, limit, balance, active) =
        profile.ok_or_else(|| tx_failure("CUSTOMER_MISSING".to_string()))?;
    if !active || policy == "DISABLED" {
        return Err(tx_failure("CREDIT_DISABLED".to_string()));
    }
    let proposed = (i128::from(balance) + i128::from(credit_amount))
        .try_into()
        .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
    if policy == "LIMITED" {
        let proposed_value: i64 = proposed;
        if proposed_value > limit && !override_granted {
            return Err(tx_failure(sentinel::credit_limit_exceeded(
                limit,
                proposed_value,
            )));
        }
    }
    Ok(proposed)
}

/// Translate one conversion sentinel; anything else falls through to the
/// shared engine mapper (stock/idempotency sentinels).
fn map_conversion_error(error: InfrastructureError) -> ApplicationError {
    // Insufficient-stock aborts carry the failing product identity inline
    // (`INSUFFICIENT_STOCK_FOR:<label>:<shortfall>`); surface the full
    // message as validation so the product is named end to end.
    if let InfrastructureError::Migration(ref message) = error {
        if message.starts_with("INSUFFICIENT_STOCK_FOR:") {
            return ApplicationError::Validation(DomainError::InvalidCanonicalString(
                message.clone(),
            ));
        }
    }
    match &error {
        InfrastructureError::Migration(message) if message == SOURCE_NOT_FOUND => {
            ApplicationError::Validation(DomainError::SourceDocumentNotFound)
        }
        InfrastructureError::Migration(message) if message == ALREADY_CONVERTED => {
            ApplicationError::Validation(DomainError::DocumentAlreadyConverted)
        }
        InfrastructureError::Migration(message) if message == BAD_SOURCE_STATUS => {
            ApplicationError::Validation(DomainError::InvalidConversionSourceStatus)
        }
        InfrastructureError::Migration(message) if message == BAD_SOURCE_TYPE => {
            ApplicationError::Validation(DomainError::InvalidConversionSourceType)
        }
        InfrastructureError::Migration(message) if message == EMPTY_SOURCE => {
            ApplicationError::Validation(DomainError::EmptySourceDocument)
        }
        InfrastructureError::Migration(message) if message == TENDER_MISMATCH => {
            ApplicationError::Validation(DomainError::InsufficientPayment)
        }
        _ => map_engine_error(error),
    }
}

/// Frozen source snapshot read strictly inside the transaction.
#[allow(clippy::too_many_arguments)]
struct SourceSnapshot {
    id: String,
    doc_type_code: String,
    party_type: Option<String>,
    party_id: Option<String>,
    warehouse_id: String,
    language: String,
    subtotal_ht: i64,
    vat_total: i64,
    global_discount: i64,
    timbre: i64,
    ttc: i64,
    payment_method: String,
    status_code: String,
    notes: Option<String>,
}

/// One stored source line, byte-copy payload for the BL item row.
#[allow(clippy::too_many_arguments)]
struct SourceLine {
    product_id: String,
    unit_id: String,
    quantity_micro: i64,
    quantity_base_micro: i64,
    unit_price_centimes: i64,
    discount_centimes: i64,
    tva_rate_micro: i64,
    line_ht_centimes: i64,
    line_tva_centimes: i64,
    line_ttc_centimes: i64,
    cost_price_centimes: i64,
    sort_order: i64,
}

/// Read the source header strictly inside the transaction (row lock held
/// by `BEGIN IMMEDIATE`; re-validated here, never trusted from outside).
fn read_source(tx: &Transaction, source_id: &str) -> Result<SourceSnapshot, InfrastructureError> {
    tx.query_row(
        "SELECT id, type, party_type, party_id, warehouse_id,
                document_language, subtotal_ht_centimes, vat_total_centimes,
                global_discount_centimes, timbre_centimes, ttc_centimes,
                payment_method, status, notes
         FROM documents WHERE id = ?1",
        [source_id],
        |row| {
            Ok(SourceSnapshot {
                id: row.get(0)?,
                doc_type_code: row.get(1)?,
                party_type: row.get(2)?,
                party_id: row.get(3)?,
                warehouse_id: row.get(4)?,
                language: row.get(5)?,
                subtotal_ht: row.get(6)?,
                vat_total: row.get(7)?,
                global_discount: row.get(8)?,
                timbre: row.get(9)?,
                ttc: row.get(10)?,
                payment_method: row.get(11)?,
                status_code: row.get(12)?,
                notes: row.get(13)?,
            })
        },
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
    .map_err(InfrastructureError::from)?
    .ok_or_else(|| tx_failure(SOURCE_NOT_FOUND.to_string()))
}

/// Read stored source lines in display order, strictly inside the txn.
fn read_source_lines(
    tx: &Transaction,
    source_id: &str,
) -> Result<Vec<SourceLine>, InfrastructureError> {
    let mut statement = tx.prepare(
        "SELECT product_id, unit_id, quantity_micro, quantity_base_micro,
                unit_price_centimes, discount_centimes, tva_rate_micro,
                line_ht_centimes, line_tva_centimes, line_ttc_centimes,
                cost_price_centimes, sort_order
         FROM document_items WHERE document_id = ?1 ORDER BY sort_order",
    )?;
    let rows = statement
        .query_map([source_id], |row| {
            Ok(SourceLine {
                product_id: row.get(0)?,
                unit_id: row.get(1)?,
                quantity_micro: row.get(2)?,
                quantity_base_micro: row.get(3)?,
                unit_price_centimes: row.get(4)?,
                discount_centimes: row.get(5)?,
                tva_rate_micro: row.get(6)?,
                line_ht_centimes: row.get(7)?,
                line_tva_centimes: row.get(8)?,
                line_ttc_centimes: row.get(9)?,
                cost_price_centimes: row.get(10)?,
                sort_order: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Display name (`name_fr`) plus SKU for override/shortfall messages.
fn product_label(tx: &Transaction, product_id: &str) -> Result<String, InfrastructureError> {
    tx.query_row(
        "SELECT sku, name_fr FROM products WHERE id = ?1",
        [product_id],
        |row| {
            let sku: String = row.get(0)?;
            let name: String = row.get(1)?;
            Ok(format!("{sku} ({name})"))
        },
    )
    .map_err(InfrastructureError::from)
}

/// Format base micros (scale 10^4) as a trimmed decimal unit string.
fn format_base_units(shortfall_base_micro: i64) -> String {
    let sign = if shortfall_base_micro < 0 { "-" } else { "" };
    let magnitude: u64 = shortfall_base_micro.unsigned_abs();
    let whole = magnitude / 10_000;
    let frac = magnitude % 10_000;
    if frac == 0 {
        format!("{sign}{whole}")
    } else {
        let mut frac_text = format!("{frac:04}");
        while frac_text.ends_with('0') {
            frac_text.pop();
        }
        format!("{sign}{whole}.{frac_text}")
    }
}

/// Execute the full atomic conversion inside one `run_transaction`.
pub(crate) fn convert_devis_to_bl_atomic(
    conn: &mut Connection,
    command: &ConvertDevisToBlCommand,
) -> Result<ConvertDevisToBlResult, ApplicationError> {
    let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let fiscal_year = Utc::now().year();
    let request_hash = checksum_of(&command.request_canonical);
    let actor = command.actor.to_string();
    run_transaction(conn, |tx| {
        // a. Idempotency fast path (read committed state).
        if let Some((stored_hash, payload)) = lookup_command(tx, &command.command_id)? {
            if let Some(result) = evaluate_stored_json::<ConvertDevisToBlResult>(
                &stored_hash,
                &payload,
                &request_hash,
            )? {
                return Ok(result);
            }
        }
        // c. Fetch source strictly inside the transaction.
        let source_id = command.source_document_id.to_string();
        let source = read_source(tx, &source_id)?;
        // d/e. Type + status validation (single-shot gate, ADR-040).
        let doc_type = DocumentType::from_code(&source.doc_type_code)
            .map_err(|_| tx_failure(BAD_SOURCE_TYPE.to_string()))?;
        if doc_type != DocumentType::Devis {
            return Err(tx_failure(BAD_SOURCE_TYPE.to_string()));
        }
        let status = DocumentStatus::from_code(&source.status_code)
            .map_err(|_| tx_failure(BAD_SOURCE_STATUS.to_string()))?;
        // Authoritative in-transaction gate: only Finalized converts.
        if status != DocumentStatus::Finalized {
            if status == DocumentStatus::Converted {
                return Err(tx_failure(ALREADY_CONVERTED.to_string()));
            }
            return Err(tx_failure(BAD_SOURCE_STATUS.to_string()));
        }
        // f. Non-empty source.
        let source_lines = read_source_lines(tx, &source_id)?;
        if source_lines.is_empty() {
            return Err(tx_failure(EMPTY_SOURCE.to_string()));
        }
        // h. Gapless BL number.
        let sequence = allocate_in_tx(tx, DocumentType::Bl.code(), fiscal_year)?;
        let number = format_number(DocumentType::Bl.prefix(), fiscal_year, sequence);
        let bl_id = Uuid::new_v4().to_string();
        // i. BL header: frozen snapshot copies, never recomputed.
        tx.execute(
            "INSERT INTO documents
             (id, number, fiscal_year, type, party_type, party_id, source_document_id,
              warehouse_id, document_language, language, subtotal_ht_centimes,
              vat_total_centimes, global_discount_centimes, timbre_centimes,
              rounding_centimes, ttc_centimes, paid_amount_centimes,
              change_amount_centimes, payment_method, status, created_at,
              created_by, revision, origin_device_id, notes)
             VALUES (?1, ?2, ?3, 'BL', ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, ?12, 0,
                     ?13, 0, 0, ?14, 'FINALIZED', ?15, ?16, 1, ?17, ?18)",
            rusqlite::params![
                bl_id,
                number,
                i64::from(fiscal_year),
                source.party_type,
                source.party_id,
                source.id,
                source.warehouse_id,
                source.language,
                source.subtotal_ht,
                source.vat_total,
                source.global_discount,
                source.timbre,
                source.ttc,
                source.payment_method,
                stamp,
                actor,
                command.terminal_id,
                source.notes,
            ],
        )?;
        // j. Byte-for-byte item copies (no recalculation, no re-rounding).
        for line in &source_lines {
            tx.execute(
                "INSERT INTO document_items
                 (id, document_id, product_id, unit_id, quantity_micro,
                  quantity_base_micro, unit_price_centimes, discount_centimes,
                  tva_rate_micro, line_ht_centimes, line_tva_centimes,
                  line_ttc_centimes, cost_price_centimes, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    bl_id,
                    line.product_id,
                    line.unit_id,
                    line.quantity_micro,
                    line.quantity_base_micro,
                    line.unit_price_centimes,
                    line.discount_centimes,
                    line.tva_rate_micro,
                    line.line_ht_centimes,
                    line.line_tva_centimes,
                    line.line_ttc_centimes,
                    line.cost_price_centimes,
                    line.sort_order,
                ],
            )?;
        }
        // g/k/l. Leaf deductions via the sale engine's own mechanism.
        let mut warnings: Vec<String> = Vec::new();
        for leaf in &command.leaf_deductions {
            match apply_one(
                tx,
                &leaf.product_id.to_string(),
                &source.warehouse_id,
                &leaf.unit_id.to_string(),
                leaf.quantity_base_micro,
                StockMovementType::SaleOut,
                Some("SALE"),
                Some(bl_id.as_str()),
                None,
                command.authorize_negative_stock,
                None,
                &actor,
                &stamp,
            ) {
                Ok((_, new_quantity, override_applied)) => {
                    if override_applied {
                        // Shortfall = requested - on-hand = -(resulting level).
                        let shortfall = new_quantity.checked_neg().unwrap_or(i64::MAX);
                        let label = product_label(tx, &leaf.product_id.to_string())
                            .unwrap_or_else(|_| leaf.product_id.to_string());
                        warnings.push(format!(
                            "Stock négatif autorisé pour {label}: {} unités manquantes",
                            format_base_units(shortfall),
                        ));
                    }
                }
                Err(InfrastructureError::Migration(message))
                    if message.starts_with(sentinel::INSUFFICIENT_PREFIX) =>
                {
                    // No override grant: abort everything, naming the product.
                    let mut parts = message[sentinel::INSUFFICIENT_PREFIX.len()..].splitn(2, ':');
                    let available: i64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let requested: i64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let shortfall = requested.saturating_sub(available);
                    let label = product_label(tx, &leaf.product_id.to_string())
                        .unwrap_or_else(|_| leaf.product_id.to_string());
                    return Err(tx_failure(format!(
                        "INSUFFICIENT_STOCK_FOR:{}:{}",
                        label,
                        format_base_units(shortfall),
                    )));
                }
                Err(other) => return Err(other),
            }
        }
        // m. Seal the source (guarded: exactly one row flips Finalized).
        let sealed = tx.execute(
            "UPDATE documents SET status = 'CONVERTED' WHERE id = ?1 AND status = 'FINALIZED'",
            [&source_id],
        )?;
        if sealed != 1 {
            return Err(tx_failure(ALREADY_CONVERTED.to_string()));
        }
        // n. Audit event with TerminalContext origin (ADR-028).
        let audit_json = format!(
            "{{\"source_document_id\":\"{source_id}\",\"target_document_id\":\"{bl_id}\",\"actor_user_id\":\"{actor}\",\"origin_device_id\":\"{origin}\",\"timestamp\":\"{stamp}\"}}",
            origin = command.terminal_id,
        );
        insert_cash_audit_row(
            tx,
            "DOCUMENT_CONVERTED",
            "documents",
            &bl_id,
            None,
            &audit_json,
            &actor,
            &stamp,
        )?;
        // o. Print outbox job (checkout RECEIPT pattern).
        let job_payload = serde_json::json!({
            "document_id": bl_id,
            "number": number,
            "total_ttc_centimes": source.ttc,
        })
        .to_string();
        enqueue_in_tx(tx, &bl_id, "RECEIPT", &job_payload, &stamp)?;
        // p. Idempotency completion with lost-race replay.
        let items_count = u32::try_from(source_lines.len())
            .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
        let result = ConvertDevisToBlResult {
            new_document_id: bl_id.clone(),
            new_document_number: number.clone(),
            new_document_status: DocumentStatus::Finalized.code().to_string(),
            total_ttc_centimes: source.ttc,
            items_count,
            warnings,
        };
        append_or_replay(
            tx,
            &command.command_id,
            &request_hash,
            "sales.convert_devis_to_bl",
            &result,
            &stamp,
            &actor,
        )
    })
    .map_err(map_conversion_error)
}

/// Execute the full BL-to-FACTURE financial closing inside one
/// `run_transaction`.
///
/// ADR-046 HARD INVARIANT: this method performs ZERO stock writes. Physical
/// stock left the warehouse when the source BL was finalized, so no
/// `stock_movements` INSERT and no `stock_levels` UPDATE may occur here
/// under any circumstance. (Do not "fix" the absence of stock calls as a
/// bug: it is the specified financial-only semantic, mirroring FF-from-BR.)
#[allow(clippy::too_many_lines)]
pub(crate) fn convert_bl_to_facture_atomic(
    conn: &mut Connection,
    command: &ConvertBlToFactureCommand,
) -> Result<ConvertBlToFactureResult, ApplicationError> {
    let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let fiscal_year = Utc::now().year();
    let request_hash = checksum_of(&command.request_canonical);
    let actor = command.actor.to_string();
    run_transaction(conn, |tx| {
        // a. Idempotency fast path (read committed state).
        if let Some((stored_hash, payload)) = lookup_command(tx, &command.command_id)? {
            if let Some(result) = evaluate_stored_json::<ConvertBlToFactureResult>(
                &stored_hash,
                &payload,
                &request_hash,
            )? {
                return Ok(result);
            }
        }
        // c. Fetch source strictly inside the transaction.
        let source_id = command.source_document_id.to_string();
        let source = read_source(tx, &source_id)?;
        // d. BL-only gate for this closing.
        let doc_type = DocumentType::from_code(&source.doc_type_code)
            .map_err(|_| tx_failure(BAD_SOURCE_TYPE.to_string()))?;
        if doc_type != DocumentType::Bl {
            return Err(tx_failure(BAD_SOURCE_TYPE.to_string()));
        }
        // e. Authoritative in-transaction status gate.
        let status = DocumentStatus::from_code(&source.status_code)
            .map_err(|_| tx_failure(BAD_SOURCE_STATUS.to_string()))?;
        if status != DocumentStatus::Finalized {
            if status == DocumentStatus::Converted {
                return Err(tx_failure(ALREADY_CONVERTED.to_string()));
            }
            return Err(tx_failure(BAD_SOURCE_STATUS.to_string()));
        }
        // f. Non-empty source.
        let source_lines = read_source_lines(tx, &source_id)?;
        if source_lines.is_empty() {
            return Err(tx_failure(EMPTY_SOURCE.to_string()));
        }
        // g. Exact tender settlement (no change concept in a closing).
        let mut tendered: i128 = 0;
        let mut cash_total: i128 = 0;
        for payment in &command.payments {
            tendered = tendered
                .checked_add(i128::from(payment.amount_centimes))
                .ok_or_else(|| tx_failure(sentinel::OVERFLOW.to_string()))?;
            if payment.method_code == "CASH" {
                cash_total = cash_total
                    .checked_add(i128::from(payment.amount_centimes))
                    .ok_or_else(|| tx_failure(sentinel::OVERFLOW.to_string()))?;
            }
        }
        if tendered != i128::from(source.ttc) {
            return Err(tx_failure(TENDER_MISMATCH.to_string()));
        }
        let cash_total = i64::try_from(cash_total)
            .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
        // h. Cash-session gate (ADR-025), only when cash is tendered.
        let cash_session_id = if cash_total > 0 {
            Some(require_open_session_in_tx(tx, &command.terminal_id)?)
        } else {
            None
        };
        // i. Credit settlement with authoritative re-check + invariant proof.
        // The FACTURE id is allocated before any write so every reference
        // (ledger, payments, audit) binds it directly — no fix-up updates.
        let facture_id = Uuid::new_v4().to_string();
        let mut customer_balance: Option<i64> = None;
        if command.credit_amount_centimes > 0 {
            let customer_id = source.party_id.as_deref().ok_or_else(|| {
                tx_failure("CREDIT_CUSTOMER_REQUIRED".to_string())
            })?;
            let new_balance = authorize_credit_for_closing(
                tx,
                customer_id,
                command.credit_amount_centimes,
                command.credit_override_granted,
            )?;
            let entry_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO customer_ledger
                 (id, sequence_no, customer_id, entry_type, amount_centimes,
                  running_balance_centimes, document_type, document_id,
                  created_by, created_at)
                 VALUES (?1, (SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM customer_ledger),
                         ?2, 'DEBIT', ?3, ?4, 'SALE', ?5, ?6, ?7)",
                rusqlite::params![
                    entry_id,
                    customer_id,
                    command.credit_amount_centimes,
                    new_balance,
                    facture_id,
                    actor,
                    stamp,
                ],
            )?;
            tx.execute(
                "UPDATE customers
                 SET current_balance_centimes = ?1, revision = revision + 1, updated_at = ?2
                 WHERE id = ?3",
                rusqlite::params![new_balance, stamp, customer_id],
            )?;
            // Domain invariant: cached balance equals ledger recomputation.
            let recomputed: Option<i64> = tx.query_row(
                "SELECT SUM(CASE WHEN entry_type = 'DEBIT' THEN amount_centimes
                                 ELSE -amount_centimes END)
                 FROM customer_ledger WHERE customer_id = ?1",
                [customer_id],
                |row| row.get(0),
            )?;
            if recomputed.unwrap_or(0) != new_balance {
                return Err(tx_failure(sentinel::CONCURRENCY.to_string()));
            }
            customer_balance = Some(new_balance);
        }
        // j. Gapless FACTURE number.
        let sequence = allocate_in_tx(tx, DocumentType::Facture.code(), fiscal_year)?;
        let number = format_number(DocumentType::Facture.prefix(), fiscal_year, sequence);
        // k. FACTURE header + byte-copy items (frozen snapshot, no math).
        tx.execute(
            "INSERT INTO documents
             (id, number, fiscal_year, type, party_type, party_id, source_document_id,
              warehouse_id, document_language, language, subtotal_ht_centimes,
              vat_total_centimes, global_discount_centimes, timbre_centimes,
              rounding_centimes, ttc_centimes, paid_amount_centimes,
              change_amount_centimes, payment_method, status, created_at,
              created_by, revision, origin_device_id, notes)
             VALUES (?1, ?2, ?3, 'FACTURE', ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, ?12, 0,
                     ?13, ?13, 0, ?14, 'FINALIZED', ?15, ?16, 1, ?17, ?18)",
            rusqlite::params![
                facture_id,
                number,
                i64::from(fiscal_year),
                source.party_type,
                source.party_id,
                source.id,
                source.warehouse_id,
                command.document_language.to_string(),
                source.subtotal_ht,
                source.vat_total,
                source.global_discount,
                source.timbre,
                source.ttc,
                payment_method_label(&command.payments),
                stamp,
                actor,
                command.terminal_id,
                source.notes,
            ],
        )?;
        for line in &source_lines {
            tx.execute(
                "INSERT INTO document_items
                 (id, document_id, product_id, unit_id, quantity_micro,
                  quantity_base_micro, unit_price_centimes, discount_centimes,
                  tva_rate_micro, line_ht_centimes, line_tva_centimes,
                  line_ttc_centimes, cost_price_centimes, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    facture_id,
                    line.product_id,
                    line.unit_id,
                    line.quantity_micro,
                    line.quantity_base_micro,
                    line.unit_price_centimes,
                    line.discount_centimes,
                    line.tva_rate_micro,
                    line.line_ht_centimes,
                    line.line_tva_centimes,
                    line.line_ttc_centimes,
                    line.cost_price_centimes,
                    line.sort_order,
                ],
            )?;
        }
        // l. Cash drawer leg (checkout SALE_CASH_IN pattern, exact portion).
        if cash_total > 0 {
            let session_id = cash_session_id
                .clone()
                .ok_or_else(|| tx_failure(sentinel::CONCURRENCY.to_string()))?;
            tx.execute(
                "INSERT INTO cash_movements
                 (id, cash_session_id, movement_type, amount_centimes,
                  reference_type, reference_id, description, created_at, created_by)
                 VALUES (?1, ?2, 'SALE_CASH_IN', ?3, 'DOCUMENT', ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    session_id,
                    cash_total,
                    facture_id,
                    number,
                    stamp,
                    actor,
                ],
            )?;
        }
        // n. Payment rows mirroring the checkout population pattern.
        for payment in &command.payments {
            let sequence_no = next_sequence_no(tx)?;
            tx.execute(
                "INSERT INTO payments
                 (id, sequence_no, document_id, customer_id, amount_centimes,
                  payment_method, cash_session_id, created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    sequence_no,
                    facture_id,
                    if payment.method_code == "CREDIT" {
                        source.party_id.clone()
                    } else {
                        None::<String>
                    },
                    payment.amount_centimes,
                    payment.method_code,
                    if payment.method_code == "CASH" {
                        cash_session_id.clone()
                    } else {
                        None::<String>
                    },
                    actor,
                    stamp,
                ],
            )?;
        }
        // o. Seal the source (guarded: exactly one row flips Finalized).
        let sealed = tx.execute(
            "UPDATE documents SET status = 'CONVERTED' WHERE id = ?1 AND status = 'FINALIZED'",
            [&source_id],
        )?;
        if sealed != 1 {
            return Err(tx_failure(ALREADY_CONVERTED.to_string()));
        }
        // p. Audit event with TerminalContext origin (ADR-028).
        let audit_json = format!(
            "{{\"source_document_id\":\"{source_id}\",\"target_document_id\":\"{facture_id}\",\"actor_user_id\":\"{actor}\",\"origin_device_id\":\"{origin}\",\"timestamp\":\"{stamp}\"}}",
            origin = command.terminal_id,
        );
        insert_cash_audit_row(
            tx,
            "DOCUMENT_CONVERTED",
            "documents",
            &facture_id,
            None,
            &audit_json,
            &actor,
            &stamp,
        )?;
        // q. Print outbox job (checkout RECEIPT pattern, ADR-044).
        let job_payload = serde_json::json!({
            "document_id": facture_id,
            "number": number,
            "total_ttc_centimes": source.ttc,
        })
        .to_string();
        enqueue_in_tx(tx, &facture_id, "RECEIPT", &job_payload, &stamp)?;
        // r. Idempotency completion with lost-race replay.
        let items_count = u32::try_from(source_lines.len())
            .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
        let result = ConvertBlToFactureResult {
            new_document_id: facture_id.clone(),
            new_document_number: number.clone(),
            new_document_status: DocumentStatus::Finalized.code().to_string(),
            total_ttc_centimes: source.ttc,
            items_count,
            customer_new_balance_centimes: customer_balance,
        };
        append_or_replay(
            tx,
            &command.command_id,
            &request_hash,
            "sales.convert_bl_to_facture",
            &result,
            &stamp,
            &actor,
        )
    })
    .map_err(map_conversion_error)
}

/// Checkout-style tender summary label (`CASH`, `CREDIT`, or `MIXED`).
fn payment_method_label(payments: &[CheckoutPayment]) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for payment in payments {
        if !seen.contains(&payment.method_code.as_str()) {
            seen.push(payment.method_code.as_str());
        }
    }
    match seen.as_slice() {
        [] => "NONE",
        [single] => single,
        _ => "MIXED",
    }
    .to_string()
}
