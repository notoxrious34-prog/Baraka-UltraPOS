//! SQLite return repository: cumulative return reads plus the single
//! atomic return bundle (TASK-BRK-046).
//!
//! One `run_transaction` (`BEGIN IMMEDIATE`): idempotency lookup-or-replay,
//! source FINALIZED re-check, gapless AVOIR number, AVOIR header +
//! source-linked items, `RETURN_IN` leaf restock via the sale engine's own
//! `apply_one`, drawer `REFUND_OUT` or ledger `CREDIT` settlement, audit
//! event, print outbox job, command log completion. Any failure rolls back
//! everything: no AVOIR, no movements, no seal drift, no consumed sequence.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{PersistReturnOrder, PersistReturnReceipt, ReturnRepository};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::StockMovementType;
use baraka_domain::sales::{format_number, DocumentId, DocumentStatus, DocumentType};
use chrono::{Datelike, SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_cash_audit_row;
use crate::adapters::sqlite_cash_session_repository::{
    append_or_replay, evaluate_stored_json, lookup_command,
};
use crate::adapters::sqlite_document_sequence_repository::allocate_in_tx;
use crate::adapters::sqlite_print_job_repository::enqueue_in_tx;
use crate::adapters::sqlite_stock_repository::{apply_one, map_engine_error, sentinel, tx_failure};
use crate::persistence::migration::checksum_of;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("return store failure".to_string())
}

/// In-transaction sentinel codes local to the return bundle.
const BAD_SOURCE_STATUS: &str = "RETURN_BAD_SOURCE_STATUS";

/// Receipt stored for idempotent replay (serializable, like conversions).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredReturnReceipt {
    return_document_id: String,
    return_document_number: String,
}

/// Receipt persistence over a shared SQLite connection.
pub struct SqliteReturnRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteReturnRepository {
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

/// Translate return-bundle sentinels; everything else rides the shared
/// engine mapper (stock/idempotency/concurrency sentinels).
fn map_return_error(error: InfrastructureError) -> ApplicationError {
    if let InfrastructureError::Migration(ref message) = error {
        if message == BAD_SOURCE_STATUS {
            return ApplicationError::Validation(DomainError::InvalidConversionSourceStatus);
        }
    }
    map_engine_error(error)
}

impl ReturnRepository for SqliteReturnRepository {
    fn cumulative_returned_quantities(
        &self,
        source_document_id: DocumentId,
    ) -> Result<HashMap<String, i64>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT di.source_item_id, COALESCE(SUM(di.quantity_micro), 0)
                 FROM document_items AS di
                 JOIN documents AS d ON di.document_id = d.id
                 WHERE d.source_document_id = ?1
                   AND d.status = 'FINALIZED'
                   AND di.source_item_id IS NOT NULL
                 GROUP BY di.source_item_id",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([source_document_id.to_string()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|_| store_failure())?;
        let mut out = HashMap::new();
        for row in rows {
            let (item_id, total) = row.map_err(|_| store_failure())?;
            out.insert(item_id, total);
        }
        Ok(out)
    }

    fn persist_return(
        &self,
        order: &PersistReturnOrder,
    ) -> Result<PersistReturnReceipt, ApplicationError> {
        let mut guard = self.lock()?;
        persist_return_atomic(&mut guard, order).map_err(map_return_error)
    }
}

/// Execute the full return bundle inside one `run_transaction`.
#[allow(clippy::too_many_lines)]
fn persist_return_atomic(
    conn: &mut Connection,
    order: &PersistReturnOrder,
) -> Result<PersistReturnReceipt, InfrastructureError> {
    let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let fiscal_year = Utc::now().year();
    let request_hash = checksum_of(&order.request_canonical);
    let actor = order.actor.to_string();
    let source_id = order.source_document_id.to_string();
    run_transaction(conn, |tx| {
        // a. Idempotency fast path (read committed state).
        if let Some((stored_hash, payload)) = lookup_command(tx, &order.idempotency_key)? {
            if let Some(stored) =
                evaluate_stored_json::<StoredReturnReceipt>(&stored_hash, &payload, &request_hash)?
            {
                return Ok(PersistReturnReceipt {
                    return_document_id: stored.return_document_id,
                    return_document_number: stored.return_document_number,
                });
            }
        }
        // b. Authoritative source re-check (status may have moved since the
        // use-case pre-check; the source itself is never sealed by returns).
        let (source_status, source_warehouse): (String, String) = tx
            .query_row(
                "SELECT status, warehouse_id FROM documents WHERE id = ?1",
                [&source_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => tx_failure("SOURCE_NOT_FOUND".to_string()),
                other => InfrastructureError::from(other),
            })?;
        if source_status != DocumentStatus::Finalized.code() {
            return Err(tx_failure(BAD_SOURCE_STATUS.to_string()));
        }
        if source_warehouse != order.warehouse_id.to_string() {
            return Err(tx_failure(BAD_SOURCE_STATUS.to_string()));
        }
        // c. Source party linkage for the AVOIR header.
        let (party_type, party_id): (Option<String>, Option<String>) = tx.query_row(
            "SELECT party_type, party_id FROM documents WHERE id = ?1",
            [&source_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        // d. Gapless AVOIR number (same allocator as every document kind).
        let sequence = allocate_in_tx(tx, DocumentType::Avoir.code(), fiscal_year).map_err(
            |error| match error {
                InfrastructureError::Migration(message)
                    if message == sentinel::SEQUENCE_OVERFLOW =>
                {
                    tx_failure(sentinel::SEQUENCE_OVERFLOW.to_string())
                }
                other => other,
            },
        )?;
        let number = format_number(DocumentType::Avoir.prefix(), fiscal_year, sequence);
        let avoir_id = Uuid::new_v4().to_string();
        // e. AVOIR header (FINALIZED at birth: immutable from here on).
        tx.execute(
            "INSERT INTO documents
             (id, number, fiscal_year, type, party_type, party_id, source_document_id,
              warehouse_id, document_language, language, subtotal_ht_centimes,
              vat_total_centimes, global_discount_centimes, timbre_centimes,
              rounding_centimes, ttc_centimes, paid_amount_centimes,
              change_amount_centimes, payment_method, status, created_at,
              created_by, revision, origin_device_id, notes)
             VALUES (?1, ?2, ?3, 'AVOIR', ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, 0, 0, 0,
                     ?11, 0, 0, 'REFUND', 'FINALIZED', ?12, ?13, 1, ?14, ?15)",
            rusqlite::params![
                avoir_id,
                number,
                i64::from(fiscal_year),
                party_type,
                party_id,
                source_id,
                order.warehouse_id.to_string(),
                order.document_language.to_string(),
                order.subtotal_ht_centimes,
                order.vat_total_centimes,
                order.ttc_centimes,
                stamp,
                actor,
                &order.terminal_id,
                order.notes.as_deref(),
            ],
        )?;
        // f. Source-linked items.
        for line in &order.lines {
            tx.execute(
                "INSERT INTO document_items
                 (id, document_id, product_id, unit_id, source_item_id,
                  quantity_micro, quantity_base_micro, unit_price_centimes,
                  discount_centimes, tva_rate_micro, line_ht_centimes,
                  line_tva_centimes, line_ttc_centimes, cost_price_centimes,
                  sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    avoir_id,
                    line.product_id.to_string(),
                    line.unit_id.to_string(),
                    line.source_item_id,
                    line.quantity_micro,
                    line.quantity_base_micro,
                    line.unit_price_centimes,
                    line.discount_centimes,
                    line.line_ht_centimes,
                    line.line_tva_centimes,
                    line.line_ttc_centimes,
                    line.cost_price_centimes,
                    line.sort_order,
                ],
            )?;
        }
        // g. Restock legs via the sale engine's own mechanism (RETURN_IN
        // only grows levels; negative authorization is never requested).
        for line in &order.lines {
            apply_one(
                tx,
                &line.product_id.to_string(),
                &order.warehouse_id.to_string(),
                &line.unit_id.to_string(),
                line.quantity_micro,
                StockMovementType::ReturnIn,
                Some("RETURN"),
                Some(avoir_id.as_str()),
                None,
                false,
                line.reason.as_deref(),
                &actor,
                &stamp,
            )?;
        }
        // h. Drawer refund leg (strictly negative REFUND_OUT).
        if let Some(cash) = &order.cash_refund {
            if cash.amount_centimes > 0 {
                let session_status: String = tx
                    .query_row(
                        "SELECT status FROM cash_sessions WHERE id = ?1",
                        [&cash.cash_session_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| match error {
                        rusqlite::Error::QueryReturnedNoRows => {
                            tx_failure("NO_OPEN_CASH_SESSION".to_string())
                        }
                        other => InfrastructureError::from(other),
                    })?;
                if session_status != "OPEN" {
                    return Err(tx_failure("NO_OPEN_CASH_SESSION".to_string()));
                }
                let refunded = cash
                    .amount_centimes
                    .checked_neg()
                    .ok_or_else(|| tx_failure(sentinel::OVERFLOW.to_string()))?;
                tx.execute(
                    "INSERT INTO cash_movements
                     (id, cash_session_id, movement_type, amount_centimes,
                      reference_type, reference_id, description, created_at, created_by)
                     VALUES (?1, ?2, 'REFUND_OUT', ?3, 'AVOIR', ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        cash.cash_session_id,
                        refunded,
                        avoir_id,
                        format!("Remboursement Avoir {number}"),
                        stamp,
                        actor,
                    ],
                )?;
            }
        }
        // i. Ledger CREDIT leg (debt reduction) with cached-balance refresh.
        if let Some(credit) = &order.credit_refund {
            if credit.amount_centimes > 0 {
                let current: Option<(i64, i64)> = tx
                    .query_row(
                        "SELECT current_balance_centimes, revision FROM customers WHERE id = ?1",
                        [credit.customer_id.to_string()],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map(Some)
                    .or_else(|error| match error {
                        rusqlite::Error::QueryReturnedNoRows => Ok(None),
                        other => Err(other),
                    })?;
                let (balance, revision) =
                    current.ok_or_else(|| tx_failure("CUSTOMER_MISSING".to_string()))?;
                let new_balance = balance
                    .checked_sub(credit.amount_centimes)
                    .ok_or_else(|| tx_failure(sentinel::OVERFLOW.to_string()))?;
                let sequence: i64 = tx.query_row(
                    "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM customer_ledger",
                    [],
                    |row| row.get(0),
                )?;
                let entry_id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO customer_ledger
                     (id, sequence_no, customer_id, entry_type, amount_centimes,
                      running_balance_centimes, document_type, document_id,
                      created_by, created_at)
                     VALUES (?1, ?2, ?3, 'CREDIT', ?4, ?5, 'AVOIR', ?6, ?7, ?8)",
                    rusqlite::params![
                        entry_id,
                        sequence,
                        credit.customer_id.to_string(),
                        credit.amount_centimes,
                        new_balance,
                        avoir_id,
                        actor,
                        stamp,
                    ],
                )?;
                let changed = tx.execute(
                    "UPDATE customers
                     SET current_balance_centimes = ?1, revision = revision + 1, updated_at = ?2
                     WHERE id = ?3 AND revision = ?4",
                    rusqlite::params![new_balance, stamp, credit.customer_id.to_string(), revision],
                )?;
                if changed != 1 {
                    return Err(tx_failure("CONCURRENCY".to_string()));
                }
            }
        }
        // j. Audit event with TerminalContext origin.
        let audit_json = format!(
            "{{\"source_document_id\":\"{source_id}\",\"target_document_id\":\"{avoir_id}\",\"actor_user_id\":\"{actor}\",\"origin_device_id\":\"{origin}\",\"timestamp\":\"{stamp}\",\"total_ttc_centimes\":{total}}}",
            origin = order.terminal_id,
            total = order.ttc_centimes,
        );
        insert_cash_audit_row(
            tx,
            "PROCESS_RETURN",
            "DOCUMENT",
            &avoir_id,
            None,
            &audit_json,
            &actor,
            &stamp,
        )?;
        // k. Print outbox job (checkout RECEIPT pattern).
        let job_payload = serde_json::json!({
            "document_id": avoir_id,
            "number": number,
            "total_ttc_centimes": order.ttc_centimes,
        })
        .to_string();
        enqueue_in_tx(tx, Some(avoir_id.as_str()), "RECEIPT", &job_payload, &stamp)?;
        // l. Idempotency completion with lost-race replay.
        let receipt = PersistReturnReceipt {
            return_document_id: avoir_id.clone(),
            return_document_number: number.clone(),
        };
        let stored = StoredReturnReceipt {
            return_document_id: avoir_id,
            return_document_number: number,
        };
        append_or_replay(
            tx,
            &order.idempotency_key,
            &request_hash,
            "sales.process_return",
            &stored,
            &stamp,
            &actor,
        )?;
        Ok(receipt)
    })
}
