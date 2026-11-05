//! Atomic stock movement engine over SQLite.
//!
//! `stock_movements` is the append-only source of truth; `stock_levels` is
//! a derived read cache updated in the SAME transaction under
//! `BEGIN IMMEDIATE`. Quantities persist as positive magnitudes; direction
//! derives exclusively from the movement type.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    NewStockMovement, StockMovementCommand, StockMovementRecord, StockRepository, TransferCommand,
    TransferResult,
};
use baraka_domain::catalog::convert_to_base_micros;
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::inventory::{apply_delta, ReferenceType, StockMovementType, WarehouseId};
use baraka_domain::primitives::FactorMicro;
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, Transaction};
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_audit_row;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;

/// Sentinel error codes crossing the transaction boundary (mapped 1:1 to
/// `ApplicationError` at the repository edge; shared with checkout).
pub(crate) mod sentinel {
    pub const KIT_PRODUCT: &str = "KIT_PRODUCT";
    pub const CONCURRENCY: &str = "CONCURRENCY";
    pub const MISSING_PRODUCT: &str = "MISSING_PRODUCT";
    pub const MISSING_UNIT: &str = "MISSING_UNIT";
    pub const OVERFLOW: &str = "OVERFLOW";
    pub const INSUFFICIENT_PREFIX: &str = "INSUFFICIENT:";
    /// Explicit-confirmation demand for privileged sales on shortage.
    /// Payload uses unit separators (product names may contain anything):
    /// `NEGCONFIRM:<product_id>\x1F<product_name>\x1F<available>\x1F<requested>`.
    pub const NEGCONFIRM_PREFIX: &str = "NEGCONFIRM:";
    pub const NEGCONFIRM_SEP: char = '\u{1F}';
    pub const IDEMPOTENCY_CONFLICT: &str = "IDEMPOTENCY_CONFLICT";
    pub const SEQUENCE_OVERFLOW: &str = "SEQUENCE_OVERFLOW";
    pub const CREDIT_LIMIT_PREFIX: &str = "CREDIT_LIMIT_EXCEEDED:";

    pub fn insufficient(available: i64, requested: i64) -> String {
        format!("{INSUFFICIENT_PREFIX}{available}:{requested}")
    }

    pub fn negconfirm(
        product_id: &str,
        product_name: &str,
        available: i64,
        requested: i64,
    ) -> String {
        format!(
            "{NEGCONFIRM_PREFIX}{product_id}{SEP}{product_name}{SEP}{available}{SEP}{requested}",
            SEP = NEGCONFIRM_SEP
        )
    }

    pub fn credit_limit_exceeded(limit: i64, proposed: i64) -> String {
        format!("{CREDIT_LIMIT_PREFIX}{limit}:{proposed}")
    }
}

/// Build a sentinel transaction failure (shared with the checkout engine).
pub(crate) fn tx_failure(message: String) -> InfrastructureError {
    InfrastructureError::Migration(message)
}

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("stock store failure".to_string())
}

/// Shared boundary mapper: sentinel `Migration` codes become typed
/// application errors (also used by the atomic checkout engine).
pub(crate) fn map_engine_error(error: InfrastructureError) -> ApplicationError {
    match error {
        InfrastructureError::Sqlite(_) => store_failure(),
        InfrastructureError::Migration(message) => map_sentinel(&message),
        InfrastructureError::BusyTimeout(_)
        | InfrastructureError::ChecksumMismatch { .. }
        | InfrastructureError::DatabaseNewerThanBinary { .. }
        | InfrastructureError::Config(_) => store_failure(),
    }
}

/// Translate one sentinel code; unknown codes stay opaque.
fn map_sentinel(message: &str) -> ApplicationError {
    use baraka_domain::errors::DomainError;
    if message == sentinel::KIT_PRODUCT {
        ApplicationError::CannotMoveStockForKitProduct
    } else if message == sentinel::CONCURRENCY {
        ApplicationError::ConcurrencyInvariantViolated
    } else if message == sentinel::MISSING_PRODUCT {
        ApplicationError::NotFound("product".to_string())
    } else if message == sentinel::MISSING_UNIT {
        ApplicationError::NotFound("product unit".to_string())
    } else if message == sentinel::IDEMPOTENCY_CONFLICT {
        ApplicationError::IdempotencyConflict
    } else if message == sentinel::SEQUENCE_OVERFLOW {
        ApplicationError::Validation(DomainError::GaplessSequenceOverflow)
    } else if message == "CREDIT_DISABLED" {
        ApplicationError::Validation(DomainError::CreditDisabled)
    } else if message == "CUSTOMER_MISSING" || message == "CREDIT_CUSTOMER_REQUIRED" {
        ApplicationError::NotFound("customer".to_string())
    } else if message == "NO_OPEN_CASH_SESSION" {
        ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
    } else if message == "INVALID_PAYMENT_STATE" {
        ApplicationError::Validation(DomainError::InvalidPaymentState)
    } else if message == "CASH_SESSION_ALREADY_OPEN" {
        ApplicationError::Validation(DomainError::CashSessionAlreadyOpenForTerminal)
    } else if message == "CASH_SESSION_NOT_OPEN" {
        ApplicationError::Validation(DomainError::CashSessionNotOpen)
    } else if message == "CASH_SESSION_NOT_IN_CLOSING" {
        ApplicationError::Validation(DomainError::CashSessionNotInClosing)
    } else if message == "CASH_SESSION_NOT_FOUND" {
        ApplicationError::Validation(DomainError::CashSessionNotFound)
    } else if message == "BOOTSTRAP_ALREADY_COMPLETED" {
        ApplicationError::Validation(DomainError::BootstrapAlreadyCompleted)
    } else if let Some(rest) = message.strip_prefix(sentinel::INSUFFICIENT_PREFIX) {
        let mut parts = rest.splitn(2, ':');
        let available = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let requested = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        ApplicationError::InsufficientStock {
            available_base_micro: available,
            requested_base_micro: requested,
        }
    } else if let Some(rest) = message.strip_prefix(sentinel::NEGCONFIRM_PREFIX) {
        let mut parts = rest.split(sentinel::NEGCONFIRM_SEP);
        let product_id = parts.next().unwrap_or_default().to_string();
        let product_name = parts.next().unwrap_or_default().to_string();
        let available = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let requested = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        ApplicationError::NegativeStockConfirmationRequired {
            product_id,
            product_name,
            available_micro: available,
            requested_micro: requested,
        }
    } else if let Some(rest) = message.strip_prefix(sentinel::CREDIT_LIMIT_PREFIX) {
        let mut parts = rest.splitn(2, ':');
        let limit = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let proposed = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        ApplicationError::CreditLimitExceeded {
            limit_centimes: limit,
            proposed_centimes: proposed,
        }
    } else {
        store_failure()
    }
}

/// Stock repository + atomic engine over a shared SQLite connection.
pub struct SqliteStockRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteStockRepository {
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

fn stamp_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Fetch `(quantity, revision)`, inserting a zero row when absent.
fn fetch_or_create_level(
    tx: &Transaction,
    product_id: &str,
    warehouse_id: &str,
) -> Result<(i64, i64), InfrastructureError> {
    let found: Option<(i64, i64)> = tx
        .query_row(
            "SELECT quantity_base_micro, revision FROM stock_levels
             WHERE product_id = ?1 AND warehouse_id = ?2",
            rusqlite::params![product_id, warehouse_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    match found {
        Some(level) => Ok(level),
        None => {
            tx.execute(
                "INSERT INTO stock_levels
                 (warehouse_id, product_id, quantity_micro, quantity_base_micro, revision)
                 VALUES (?1, ?2, 0, 0, 0)",
                rusqlite::params![warehouse_id, product_id],
            )?;
            Ok((0, 0))
        }
    }
}

/// Next global movement sequence number (inside the write transaction).
pub(crate) fn next_sequence_no(tx: &Transaction) -> Result<i64, InfrastructureError> {
    tx.query_row(
        "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM stock_movements",
        [],
        |row| row.get(0),
    )
    .map_err(InfrastructureError::from)
}

/// Single movement application shared with the atomic checkout engine.
/// Returns `(movement_id, new_quantity, override_applied)`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_one(
    tx: &Transaction,
    product_id: &str,
    warehouse_id: &str,
    unit_id: &str,
    quantity_micro: i64,
    movement_type: StockMovementType,
    reference_type: Option<&str>,
    reference_id: Option<&str>,
    transfer_batch_id: Option<&str>,
    authorize_negative_stock: bool,
    reason: Option<&str>,
    actor: &str,
    stamp: &str,
) -> Result<(String, i64, bool), InfrastructureError> {
    let is_kit: bool = tx
        .query_row(
            "SELECT is_kit FROM products WHERE id = ?1",
            [product_id],
            |row| row.get::<_, i64>(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?
        .ok_or_else(|| tx_failure(sentinel::MISSING_PRODUCT.to_string()))?
        != 0;
    if is_kit {
        return Err(tx_failure(sentinel::KIT_PRODUCT.to_string()));
    }
    let factor: i64 = tx
        .query_row(
            "SELECT factor_micro FROM product_units WHERE id = ?1 AND product_id = ?2",
            rusqlite::params![unit_id, product_id],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?
        .ok_or_else(|| tx_failure(sentinel::MISSING_UNIT.to_string()))?;
    let quantity_base = convert_to_base_micros(quantity_micro, FactorMicro::from_micros(factor))
        .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
    let (current, revision) = fetch_or_create_level(tx, product_id, warehouse_id)?;
    let new_quantity = apply_delta(current, movement_type.signed_delta(quantity_base))
        .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
    let mut override_applied = false;
    if new_quantity < 0 {
        let allowed = movement_type != StockMovementType::TransferOut && authorize_negative_stock;
        if !allowed {
            return Err(tx_failure(sentinel::insufficient(current, quantity_base)));
        }
        override_applied = true;
        insert_audit_row(tx, product_id, warehouse_id, new_quantity, actor, stamp)?;
    }
    let changed = tx.execute(
        "UPDATE stock_levels
         SET quantity_base_micro = ?1, revision = revision + 1, updated_at = ?2
         WHERE product_id = ?3 AND warehouse_id = ?4 AND revision = ?5",
        rusqlite::params![new_quantity, stamp, product_id, warehouse_id, revision],
    )?;
    if changed != 1 {
        return Err(tx_failure(sentinel::CONCURRENCY.to_string()));
    }
    let movement_id = Uuid::new_v4().to_string();
    let sequence_no = next_sequence_no(tx)?;
    tx.execute(
        "INSERT INTO stock_movements
         (id, sequence_no, warehouse_id, product_id, movement_type, unit_id,
          quantity_micro, quantity_base_micro, reference_type, reference_id,
          transfer_batch_id, authorized_negative_stock, reason,
          created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        rusqlite::params![
            movement_id,
            sequence_no,
            warehouse_id,
            product_id,
            movement_type.code(),
            unit_id,
            quantity_micro,
            quantity_base,
            reference_type,
            reference_id,
            transfer_batch_id,
            i64::from(override_applied),
            reason,
            actor,
            stamp,
        ],
    )?;
    Ok((movement_id, new_quantity, override_applied))
}

impl StockRepository for SqliteStockRepository {
    fn get_stock_level(
        &self,
        product_id: ProductId,
        warehouse_id: WarehouseId,
    ) -> Result<i64, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT quantity_base_micro FROM stock_levels
                 WHERE product_id = ?1 AND warehouse_id = ?2",
                rusqlite::params![product_id.to_string(), warehouse_id.to_string()],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
            .map(|level| level.unwrap_or(0))
    }

    fn upsert_stock_level_with_delta(
        &self,
        product_id: ProductId,
        warehouse_id: WarehouseId,
        signed_delta_base_micro: i64,
    ) -> Result<i64, ApplicationError> {
        let mut guard = self.lock()?;
        run_transaction(&mut guard, |tx| {
            let (current, revision) =
                fetch_or_create_level(tx, &product_id.to_string(), &warehouse_id.to_string())?;
            let new_quantity =
                i64::try_from(i128::from(current) + i128::from(signed_delta_base_micro))
                    .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
            write_level(
                tx,
                &product_id.to_string(),
                &warehouse_id.to_string(),
                revision,
                new_quantity,
            )?;
            Ok(new_quantity)
        })
        .map_err(map_engine_error)
    }

    fn insert_stock_movement(
        &self,
        movement: &NewStockMovement,
    ) -> Result<String, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = stamp_now();
        run_transaction(&mut guard, |tx| {
            let movement_id = Uuid::new_v4().to_string();
            let sequence_no = next_sequence_no(tx)?;
            tx.execute(
                "INSERT INTO stock_movements
                 (id, sequence_no, warehouse_id, product_id, movement_type, unit_id,
                  quantity_micro, quantity_base_micro, reference_type, reference_id,
                  transfer_batch_id, authorized_negative_stock, reason,
                  created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                rusqlite::params![
                    movement_id,
                    sequence_no,
                    movement.warehouse_id.to_string(),
                    movement.product_id.to_string(),
                    movement.movement_type.code(),
                    movement.unit_id.to_string(),
                    movement.quantity_micro,
                    movement.quantity_base_micro,
                    movement.reference_type.map(|kind| kind.code().to_string()),
                    movement.reference_id,
                    movement.transfer_batch_id,
                    i64::from(movement.authorized_negative_stock),
                    movement.reason,
                    movement.actor.to_string(),
                    stamp,
                ],
            )?;
            Ok(movement_id)
        })
        .map_err(map_engine_error)
    }

    fn get_product_unit_conversion(
        &self,
        product_id: ProductId,
        unit_id: UnitId,
    ) -> Result<FactorMicro, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT factor_micro FROM product_units WHERE id = ?1 AND product_id = ?2",
                rusqlite::params![unit_id.to_string(), product_id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .map(FactorMicro::from_micros)
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    ApplicationError::NotFound("product unit".to_string())
                }
                _ => store_failure(),
            })
    }

    fn apply_stock_movement(
        &self,
        command: &StockMovementCommand,
    ) -> Result<StockMovementRecord, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = stamp_now();
        run_transaction(&mut guard, |tx| {
            let (movement_id, new_quantity, override_applied) = apply_one(
                tx,
                &command.product_id.to_string(),
                &command.warehouse_id.to_string(),
                &command.unit_id.to_string(),
                command.quantity_micro,
                command.movement_type,
                command.reference_type.map(|kind| kind.code()),
                command.reference_id.as_deref(),
                command.transfer_batch_id.as_deref(),
                command.authorize_negative_stock,
                command.reason.as_deref(),
                &command.actor.to_string(),
                &stamp,
            )?;
            Ok(StockMovementRecord {
                movement_id,
                new_quantity_base_micro: new_quantity,
                negative_override_applied: override_applied,
            })
        })
        .map_err(map_engine_error)
    }

    fn transfer_stock(
        &self,
        command: &TransferCommand,
    ) -> Result<TransferResult, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = stamp_now();
        run_transaction(&mut guard, |tx| {
            let (out_id, source_new, _) = apply_one(
                tx,
                &command.product_id.to_string(),
                &command.source_warehouse_id.to_string(),
                &command.unit_id.to_string(),
                command.quantity_micro,
                StockMovementType::TransferOut,
                Some(ReferenceType::Transfer.code()),
                None,
                Some(command.transfer_batch_id.as_str()),
                false,
                command.reason.as_deref(),
                &command.actor.to_string(),
                &stamp,
            )?;
            let (in_id, destination_new, _) = apply_one(
                tx,
                &command.product_id.to_string(),
                &command.destination_warehouse_id.to_string(),
                &command.unit_id.to_string(),
                command.quantity_micro,
                StockMovementType::TransferIn,
                Some(ReferenceType::Transfer.code()),
                None,
                Some(command.transfer_batch_id.as_str()),
                false,
                command.reason.as_deref(),
                &command.actor.to_string(),
                &stamp,
            )?;
            Ok(TransferResult {
                transfer_batch_id: command.transfer_batch_id.clone(),
                out_movement_id: out_id,
                in_movement_id: in_id,
                source_new_quantity_base_micro: source_new,
                destination_new_quantity_base_micro: destination_new,
            })
        })
        .map_err(map_engine_error)
    }
}

/// Revision-guarded level write shared by the granular helper.
fn write_level(
    tx: &Transaction,
    product_id: &str,
    warehouse_id: &str,
    revision: i64,
    new_quantity: i64,
) -> Result<(), InfrastructureError> {
    let stamp = stamp_now();
    let changed = tx.execute(
        "UPDATE stock_levels
         SET quantity_base_micro = ?1, revision = revision + 1, updated_at = ?2
         WHERE product_id = ?3 AND warehouse_id = ?4 AND revision = ?5",
        rusqlite::params![new_quantity, stamp, product_id, warehouse_id, revision],
    )?;
    if changed != 1 {
        return Err(tx_failure(sentinel::CONCURRENCY.to_string()));
    }
    Ok(())
}
