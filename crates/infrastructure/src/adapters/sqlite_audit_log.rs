//! Minimal single-purpose audit writer for negative-stock overrides.
//!
//! No generalized audit subsystem exists (or is built) in this task: this
//! module appends exactly one event type into the hash-chained `audit_log`
//! table from migration 0001. Chain rule: `record_hash = SHA256(prev_hash
//! | timestamp | actor | action | entity | entity_id | new_json)`; the first
//! record chains from `"GENESIS"`.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::AuditPort;
use baraka_domain::catalog::ProductId;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::user::UserId;
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, Transaction};

use crate::persistence::migration::checksum_of;
use crate::persistence::transaction::run_transaction;

/// Hash-chained audit writer over a shared SQLite connection.
pub struct SqliteAuditLog {
    db: Arc<Mutex<Connection>>,
}

impl SqliteAuditLog {
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

/// Append one override event inside the CALLER's transaction (atomicity).
///
/// Must run in the same `run_transaction` as the stock write it records.
pub fn insert_audit_row(
    tx: &Transaction,
    product_id: &str,
    warehouse_id: &str,
    resulting_quantity_base_micro: i64,
    actor: &str,
    stamp: &str,
) -> Result<(), crate::persistence::InfrastructureError> {
    let prev_hash: String = tx
        .query_row(
            "SELECT record_hash FROM audit_log ORDER BY sequence_no DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?
        .unwrap_or_else(|| "GENESIS".to_string());
    let entity_id = format!("{warehouse_id}:{product_id}");
    let new_json = format!("{{\"quantity_base_micro\":{resulting_quantity_base_micro}}}");
    let canonical = format!(
        "{prev_hash}|{stamp}|{actor}|NEGATIVE_STOCK_OVERRIDE|stock_levels|{entity_id}|{new_json}"
    );
    let record_hash = checksum_of(&canonical);
    tx.execute(
        "INSERT INTO audit_log
         (prev_hash, record_hash, timestamp, user_id, action,
          entity_type, entity_id, old_value_json, new_value_json)
         VALUES (?1, ?2, ?3, ?4, 'NEGATIVE_STOCK_OVERRIDE', 'stock_levels', ?5, NULL, ?6)",
        rusqlite::params![prev_hash, record_hash, stamp, actor, entity_id, new_json],
    )?;
    Ok(())
}

/// Append one dual-track delete/deactivate event inside the CALLER's
/// transaction (TASK-BRK-080). Same hash chain as the other writers.
/// `action` is `DISABLE` or `PURGE`; for `PURGE` the newest prior `DISABLE`
/// row for the entity is linked via `prior_disable_seq` (null when the
/// entity purged without a prior disable, e.g. never referenced).
pub fn insert_delete_audit_row(
    tx: &Transaction,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    audit: &baraka_application::ports::DeleteAudit,
    stamp: &str,
) -> Result<(), crate::persistence::InfrastructureError> {
    let prior_disable_seq: Option<i64> = if action == "PURGE" {
        tx.query_row(
            "SELECT MAX(sequence_no) FROM audit_log
             WHERE action = 'DISABLE' AND entity_type = ?1 AND entity_id = ?2",
            rusqlite::params![entity_type, entity_id],
            |row| row.get(0),
        )?
    } else {
        None
    };
    let new_json = match prior_disable_seq {
        Some(seq) => format!(
            "{{\"reason\":\"{}\",\"prior_disable_seq\":{seq}}}",
            audit.reason.replace('\\', "\\\\").replace('"', "\\\"")
        ),
        None => format!(
            "{{\"reason\":\"{}\",\"prior_disable_seq\":null}}",
            audit.reason.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    };
    insert_cash_audit_row(
        tx,
        action,
        entity_type,
        entity_id,
        Some(&audit.old_json),
        &new_json,
        &audit.actor,
        stamp,
    )
}

/// Append one generic cash audit event inside the CALLER's transaction.
///
/// Same hash chain as [`insert_audit_row`]; variance and other business
/// fields travel inside `new_json` (flagged prominently when non-zero).
#[allow(clippy::too_many_arguments)]
pub fn insert_cash_audit_row(
    tx: &Transaction,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    old_json: Option<&str>,
    new_json: &str,
    actor: &str,
    stamp: &str,
) -> Result<(), crate::persistence::InfrastructureError> {
    let prev_hash: String = tx
        .query_row(
            "SELECT record_hash FROM audit_log ORDER BY sequence_no DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?
        .unwrap_or_else(|| "GENESIS".to_string());
    let canonical =
        format!("{prev_hash}|{stamp}|{actor}|{action}|{entity_type}|{entity_id}|{new_json}");
    let record_hash = checksum_of(&canonical);
    tx.execute(
        "INSERT INTO audit_log
         (prev_hash, record_hash, timestamp, user_id, action,
          entity_type, entity_id, old_value_json, new_value_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            prev_hash,
            record_hash,
            stamp,
            actor,
            action,
            entity_type,
            entity_id,
            old_json,
            new_json
        ],
    )?;
    Ok(())
}

impl AuditPort for SqliteAuditLog {
    fn record_negative_stock_override(
        &self,
        product_id: ProductId,
        warehouse_id: WarehouseId,
        resulting_quantity_base_micro: i64,
        actor: UserId,
        at: DateTime<Utc>,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = at.to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            insert_audit_row(
                tx,
                &product_id.to_string(),
                &warehouse_id.to_string(),
                resulting_quantity_base_micro,
                &actor.to_string(),
                &stamp,
            )
        })
        .map_err(|_| ApplicationError::Port("audit store failure".to_string()))
    }
}
