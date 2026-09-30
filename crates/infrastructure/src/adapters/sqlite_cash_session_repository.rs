//! SQLite cash session engine: lifecycle, idempotency, Z-report math.
//!
//! Every financial method runs inside ONE `run_transaction` with
//! Lookup-then-Append idempotency against `command_log` (never
//! Claim-then-Complete). Status transitions are revision-guarded and
//! carry the expected current status in the WHERE clause; zero affected
//! rows surface as concurrent-modification errors. All money is `i64`
//! centimes; no float exists in this module.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CashMovementTotal, CashMovementView, CashSessionReport, CashSessionRepository, CashSessionView,
};
use baraka_domain::cash::cash_session::compute_variance;
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::user::UserId;
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, Transaction};
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_cash_audit_row;
use crate::adapters::sqlite_stock_repository::{map_engine_error, sentinel, tx_failure};
use crate::persistence::migration::checksum_of;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;

/// Cash session persistence over a shared SQLite connection.
pub struct SqliteCashSessionRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteCashSessionRepository {
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

fn is_unique_violation(error: &rusqlite::Error) -> bool {
    match error {
        rusqlite::Error::SqliteFailure(inner, _) => {
            inner.code == rusqlite::ErrorCode::ConstraintViolation
        }
        _ => false,
    }
}

pub(crate) fn lookup_command(
    tx: &Transaction,
    command_id: &str,
) -> Result<Option<(String, String)>, InfrastructureError> {
    tx.query_row(
        "SELECT request_hash, response_payload FROM command_log WHERE command_id = ?1",
        [command_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
    .map_err(InfrastructureError::from)
}

pub(crate) fn evaluate_stored_json<T>(
    stored_hash: &str,
    payload: &str,
    request_hash: &str,
) -> Result<Option<T>, InfrastructureError>
where
    T: serde::de::DeserializeOwned,
{
    if stored_hash != request_hash {
        return Err(tx_failure(sentinel::IDEMPOTENCY_CONFLICT.to_string()));
    }
    if payload.is_empty() {
        return Ok(None);
    }
    let value: T =
        serde_json::from_str(payload).map_err(|_| tx_failure("REPLAY_CORRUPT".to_string()))?;
    Ok(Some(value))
}

fn append_completed(
    tx: &Transaction,
    command_id: &str,
    request_hash: &str,
    command_type: &str,
    response_payload: &str,
    stamp: &str,
    actor: &str,
) -> Result<(), InfrastructureError> {
    tx.execute(
        "INSERT INTO command_log
         (command_id, request_hash, command_type, response_payload, created_at, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            command_id,
            request_hash,
            command_type,
            response_payload,
            stamp,
            actor
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn append_or_replay<T>(
    tx: &Transaction,
    command_id: &str,
    request_hash: &str,
    command_type: &str,
    value: &T,
    stamp: &str,
    actor: &str,
) -> Result<T, InfrastructureError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + Clone,
{
    let payload_json =
        serde_json::to_string(value).map_err(|_| tx_failure("RESPONSE_ENCODE".to_string()))?;
    match append_completed(
        tx,
        command_id,
        request_hash,
        command_type,
        &payload_json,
        stamp,
        actor,
    ) {
        Ok(()) => Ok(value.clone()),
        Err(InfrastructureError::Sqlite(inner)) if is_unique_violation(&inner) => {
            match lookup_command(tx, command_id)? {
                Some((stored_hash, payload)) => {
                    match evaluate_stored_json::<T>(&stored_hash, &payload, request_hash)? {
                        Some(replay) => Ok(replay),
                        None => Err(tx_failure("CLAIM_RACE".to_string())),
                    }
                }
                None => Err(tx_failure("CLAIM_RACE".to_string())),
            }
        }
        Err(other) => Err(other),
    }
}

#[allow(clippy::too_many_arguments)]
fn row_to_session(
    id: String,
    warehouse_id: String,
    terminal_id: String,
    status: String,
    opened_by: String,
    opened_by_username: String,
    opened_at: String,
    opening_float_centimes: i64,
    closed_by: Option<String>,
    closed_by_username: Option<String>,
    closed_at: Option<String>,
    actual_counted_centimes: Option<i64>,
    system_expected_centimes: Option<i64>,
    variance_centimes: Option<i64>,
    closing_notes: Option<String>,
    revision: i64,
    created_at: String,
    updated_at: String,
) -> CashSessionView {
    CashSessionView {
        id,
        warehouse_id,
        terminal_id,
        status,
        opened_by,
        opened_by_username,
        opened_at,
        opening_float_centimes,
        closed_by,
        closed_by_username,
        closed_at,
        actual_counted_centimes,
        system_expected_centimes,
        variance_centimes,
        closing_notes,
        revision,
        created_at,
        updated_at,
    }
}

fn load_session_by_id(
    tx: &Transaction,
    session_id: &str,
) -> Result<Option<CashSessionView>, InfrastructureError> {
    #[allow(clippy::type_complexity)]
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        i64,
        String,
        String,
    )> = tx
        .query_row(
            "SELECT s.id, s.warehouse_id, s.terminal_id, s.status,
                    s.opened_by, COALESCE(u1.username, s.opened_by), s.opened_at,
                    s.opening_float_centimes,
                    s.closed_by, u2.username, s.closed_at,
                    s.actual_counted_centimes, s.system_expected_centimes,
                    s.variance_centimes, s.closing_notes, s.revision,
                    s.created_at, s.updated_at
             FROM cash_sessions s
             LEFT JOIN users u1 ON u1.id = s.opened_by
             LEFT JOIN users u2 ON u2.id = s.closed_by
             WHERE s.id = ?1",
            [session_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                ))
            },
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    Ok(row.map(
        |(
            id,
            warehouse_id,
            terminal_id,
            status,
            opened_by,
            opened_by_username,
            opened_at,
            opening_float,
            closed_by,
            closed_by_username,
            closed_at,
            actual,
            expected,
            variance,
            notes,
            revision,
            created_at,
            updated_at,
        )| {
            row_to_session(
                id,
                warehouse_id,
                terminal_id,
                status,
                opened_by,
                opened_by_username,
                opened_at,
                opening_float,
                closed_by,
                closed_by_username,
                closed_at,
                actual,
                expected,
                variance,
                notes,
                revision,
                created_at,
                updated_at,
            )
        },
    ))
}

fn load_active_for_terminal(
    tx: &Transaction,
    terminal_id: &str,
) -> Result<Option<CashSessionView>, InfrastructureError> {
    #[allow(clippy::type_complexity)]
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        i64,
        String,
        String,
    )> = tx
        .query_row(
            "SELECT s.id, s.warehouse_id, s.terminal_id, s.status,
                    s.opened_by, COALESCE(u1.username, s.opened_by), s.opened_at,
                    s.opening_float_centimes,
                    s.closed_by, u2.username, s.closed_at,
                    s.actual_counted_centimes, s.system_expected_centimes,
                    s.variance_centimes, s.closing_notes, s.revision,
                    s.created_at, s.updated_at
             FROM cash_sessions s
             LEFT JOIN users u1 ON u1.id = s.opened_by
             LEFT JOIN users u2 ON u2.id = s.closed_by
             WHERE s.terminal_id = ?1 AND s.status IN ('OPEN','CLOSING')
             ORDER BY s.opened_at DESC LIMIT 1",
            [terminal_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                ))
            },
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    Ok(row.map(
        |(
            id,
            warehouse_id,
            terminal_id,
            status,
            opened_by,
            opened_by_username,
            opened_at,
            opening_float,
            closed_by,
            closed_by_username,
            closed_at,
            actual,
            expected,
            variance,
            notes,
            revision,
            created_at,
            updated_at,
        )| {
            row_to_session(
                id,
                warehouse_id,
                terminal_id,
                status,
                opened_by,
                opened_by_username,
                opened_at,
                opening_float,
                closed_by,
                closed_by_username,
                closed_at,
                actual,
                expected,
                variance,
                notes,
                revision,
                created_at,
                updated_at,
            )
        },
    ))
}

fn sum_movements(tx: &Transaction, session_id: &str) -> Result<i64, InfrastructureError> {
    let total: Option<i64> = tx.query_row(
        "SELECT SUM(amount_centimes) FROM cash_movements WHERE cash_session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?;
    Ok(total.unwrap_or(0))
}

fn map_cash_error(error: InfrastructureError) -> ApplicationError {
    match &error {
        InfrastructureError::Migration(message) if message == "CASH_SESSION_ALREADY_OPEN" => {
            ApplicationError::Validation(DomainError::CashSessionAlreadyOpenForTerminal)
        }
        InfrastructureError::Migration(message) if message == "CASH_SESSION_NOT_OPEN" => {
            ApplicationError::Validation(DomainError::CashSessionNotOpen)
        }
        InfrastructureError::Migration(message) if message == "CASH_SESSION_NOT_IN_CLOSING" => {
            ApplicationError::Validation(DomainError::CashSessionNotInClosing)
        }
        InfrastructureError::Migration(message) if message == "NO_OPEN_CASH_SESSION" => {
            ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
        }
        InfrastructureError::Migration(message) if message == "CASH_SESSION_NOT_FOUND" => {
            ApplicationError::Validation(DomainError::CashSessionNotFound)
        }
        _ => map_engine_error(error),
    }
}

impl CashSessionRepository for SqliteCashSessionRepository {
    fn open_session(
        &self,
        warehouse_id: WarehouseId,
        opening_float_centimes: i64,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        if opening_float_centimes < 0 {
            return Err(ApplicationError::Validation(
                DomainError::CashSessionInvalidOpeningFloat,
            ));
        }
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let warehouse_text = warehouse_id.to_string();
        let terminal_text = terminal_id.to_string();
        let command_text = command_id.to_string();
        run_transaction(&mut guard, |tx| {
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<CashSessionView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            // Friendly pre-check for the common single-session case.
            if load_active_for_terminal(tx, &terminal_text)?.is_some() {
                return Err(tx_failure("CASH_SESSION_ALREADY_OPEN".to_string()));
            }
            // Warehouse must exist and be active.
            let active: Option<i64> = tx
                .query_row(
                    "SELECT is_active FROM warehouses WHERE id = ?1",
                    [&warehouse_text],
                    |row| row.get(0),
                )
                .map(Some)
                .or_else(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    other => Err(other),
                })?;
            match active {
                Some(1) => {}
                Some(_) => return Err(tx_failure("WAREHOUSE_INACTIVE".to_string())),
                None => return Err(tx_failure("WAREHOUSE_MISSING".to_string())),
            }
            let session_id = Uuid::new_v4().to_string();
            let origin: String = tx
                .query_row(
                    "SELECT value FROM settings WHERE key = 'device_id'",
                    [],
                    |row| row.get(0),
                )
                .map(Some)
                .or_else(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    other => Err(other),
                })?
                .unwrap_or_else(|| terminal_text.clone());
            let insert = tx.execute(
                "INSERT INTO cash_sessions
                 (id, warehouse_id, terminal_id, status, opened_by, opened_at,
                  opening_float_centimes, closed_by, closed_at,
                  actual_counted_centimes, system_expected_centimes, variance_centimes,
                  closing_notes, revision, origin_device_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'OPEN', ?4, ?5, ?6, NULL, NULL, NULL, NULL, NULL, NULL, 1, ?7, ?5, ?5)",
                rusqlite::params![
                    session_id,
                    warehouse_text,
                    terminal_text,
                    actor_text,
                    stamp,
                    opening_float_centimes,
                    origin,
                ],
            );
            if let Err(error) = insert {
                if is_unique_violation(&error) {
                    return Err(tx_failure("CASH_SESSION_ALREADY_OPEN".to_string()));
                }
                return Err(InfrastructureError::from(error));
            }
            tx.execute(
                "INSERT INTO cash_movements
                 (id, cash_session_id, movement_type, amount_centimes,
                  reference_type, reference_id, description, created_at, created_by)
                 VALUES (?1, ?2, 'OPENING_FLOAT', ?3, NULL, NULL, 'Opening float', ?4, ?5)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    session_id,
                    opening_float_centimes,
                    stamp,
                    actor_text,
                ],
            )?;
            let new_json = format!(
                "{{\"session_id\":\"{session_id}\",\"warehouse_id\":\"{warehouse_text}\",\"opening_float_centimes\":{opening_float_centimes}}}"
            );
            insert_cash_audit_row(
                tx,
                "CASH_SESSION_OPENED",
                "cash_sessions",
                &session_id,
                None,
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let view = load_session_by_id(tx, &session_id)?
                .ok_or_else(|| tx_failure("CASH_SESSION_NOT_FOUND".to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "cash.open",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_cash_error)
    }

    fn begin_closing(
        &self,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let terminal_text = terminal_id.to_string();
        let command_text = command_id.to_string();
        run_transaction(&mut guard, |tx| {
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<CashSessionView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let active = load_active_for_terminal(tx, &terminal_text)?;
            let current = match active {
                Some(view) if view.status == "OPEN" => view,
                _ => return Err(tx_failure("CASH_SESSION_NOT_OPEN".to_string())),
            };
            let expected = sum_movements(tx, &current.id)?;
            let changed = tx.execute(
                "UPDATE cash_sessions
                 SET status = 'CLOSING', system_expected_centimes = ?1,
                     revision = revision + 1, updated_at = ?2
                 WHERE id = ?3 AND status = 'OPEN'",
                rusqlite::params![expected, stamp, current.id],
            )?;
            if changed != 1 {
                return Err(tx_failure("CASH_SESSION_NOT_OPEN".to_string()));
            }
            let new_json = format!(
                "{{\"session_id\":\"{}\",\"system_expected_centimes\":{expected}}}",
                current.id
            );
            insert_cash_audit_row(
                tx,
                "CASH_SESSION_CLOSING_BEGUN",
                "cash_sessions",
                &current.id,
                Some("{\"status\":\"OPEN\"}"),
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let view = load_session_by_id(tx, &current.id)?
                .ok_or_else(|| tx_failure("CASH_SESSION_NOT_FOUND".to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "cash.beginClosing",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_cash_error)
    }

    fn cancel_closing(
        &self,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let terminal_text = terminal_id.to_string();
        let command_text = command_id.to_string();
        run_transaction(&mut guard, |tx| {
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<CashSessionView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let active = load_active_for_terminal(tx, &terminal_text)?;
            let current = match active {
                Some(view) if view.status == "CLOSING" => view,
                _ => return Err(tx_failure("CASH_SESSION_NOT_IN_CLOSING".to_string())),
            };
            let changed = tx.execute(
                "UPDATE cash_sessions
                 SET status = 'OPEN', system_expected_centimes = NULL,
                     revision = revision + 1, updated_at = ?1
                 WHERE id = ?2 AND status = 'CLOSING'",
                rusqlite::params![stamp, current.id],
            )?;
            if changed != 1 {
                return Err(tx_failure("CASH_SESSION_NOT_IN_CLOSING".to_string()));
            }
            insert_cash_audit_row(
                tx,
                "CASH_SESSION_CLOSING_CANCELLED",
                "cash_sessions",
                &current.id,
                Some("{\"status\":\"CLOSING\"}"),
                "{\"status\":\"OPEN\"}",
                &actor_text,
                &stamp,
            )?;
            let view = load_session_by_id(tx, &current.id)?
                .ok_or_else(|| tx_failure("CASH_SESSION_NOT_FOUND".to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "cash.cancelClosing",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_cash_error)
    }

    fn confirm_closing(
        &self,
        session_id: &str,
        actual_counted_centimes: i64,
        closing_notes: Option<String>,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        if actual_counted_centimes < 0 {
            return Err(ApplicationError::Validation(
                DomainError::CashSessionInvalidCountedAmount,
            ));
        }
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let session_text = session_id.to_string();
        let terminal_text = terminal_id.to_string();
        let command_text = command_id.to_string();
        run_transaction(&mut guard, |tx| {
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<CashSessionView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let current = load_session_by_id(tx, &session_text)?
                .ok_or_else(|| tx_failure("CASH_SESSION_NOT_FOUND".to_string()))?;
            if current.terminal_id != terminal_text || current.status != "CLOSING" {
                return Err(tx_failure("CASH_SESSION_NOT_IN_CLOSING".to_string()));
            }
            let expected = current.system_expected_centimes.unwrap_or(0);
            let variance = compute_variance(actual_counted_centimes, expected)
                .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
            let changed = tx.execute(
                "UPDATE cash_sessions
                 SET status = 'CLOSED', closed_by = ?1, closed_at = ?2,
                     actual_counted_centimes = ?3, variance_centimes = ?4,
                     closing_notes = ?5, revision = revision + 1, updated_at = ?2
                 WHERE id = ?6 AND status = 'CLOSING'",
                rusqlite::params![
                    actor_text,
                    stamp,
                    actual_counted_centimes,
                    variance.centimes(),
                    closing_notes,
                    session_text,
                ],
            )?;
            if changed != 1 {
                return Err(tx_failure("CASH_SESSION_NOT_IN_CLOSING".to_string()));
            }
            let new_json = format!(
                "{{\"session_id\":\"{session_text}\",\"actual_counted_centimes\":{actual_counted_centimes},\"system_expected_centimes\":{expected},\"variance_centimes\":{},\"variance_nonzero\":{}}}",
                variance.centimes(),
                if variance.centimes() != 0 {
                    "true"
                } else {
                    "false"
                }
            );
            insert_cash_audit_row(
                tx,
                "CASH_SESSION_CLOSED",
                "cash_sessions",
                &session_text,
                Some("{\"status\":\"CLOSING\"}"),
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let view = load_session_by_id(tx, &session_text)?
                .ok_or_else(|| tx_failure("CASH_SESSION_NOT_FOUND".to_string()))?;
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "cash.confirmClosing",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(map_cash_error)
    }

    fn get_active_session(
        &self,
        terminal_id: &str,
    ) -> Result<Option<CashSessionView>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT s.id, s.warehouse_id, s.terminal_id, s.status,
                        s.opened_by, COALESCE(u1.username, s.opened_by), s.opened_at,
                        s.opening_float_centimes,
                        s.closed_by, u2.username, s.closed_at,
                        s.actual_counted_centimes, s.system_expected_centimes,
                        s.variance_centimes, s.closing_notes, s.revision,
                        s.created_at, s.updated_at
                 FROM cash_sessions s
                 LEFT JOIN users u1 ON u1.id = s.opened_by
                 LEFT JOIN users u2 ON u2.id = s.closed_by
                 WHERE s.terminal_id = ?1 AND s.status IN ('OPEN','CLOSING')
                 ORDER BY s.opened_at DESC LIMIT 1",
                [terminal_id],
                |row| {
                    Ok(row_to_session(
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                        row.get(13)?,
                        row.get(14)?,
                        row.get(15)?,
                        row.get(16)?,
                        row.get(17)?,
                    ))
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| ApplicationError::Port("cash store failure".to_string()))
    }

    fn record_expense(
        &self,
        amount_centimes: i64,
        description: &str,
        terminal_id: &str,
        command_id: &str,
        request_canonical: &str,
        actor: UserId,
    ) -> Result<CashMovementView, ApplicationError> {
        if amount_centimes <= 0 {
            return Err(ApplicationError::Validation(
                DomainError::CashExpenseInvalidAmount,
            ));
        }
        if description.trim().is_empty() || description.trim().chars().count() > 255 {
            return Err(ApplicationError::Validation(
                DomainError::CashExpenseDescriptionRequired,
            ));
        }
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_hash = checksum_of(request_canonical);
        let actor_text = actor.to_string();
        let terminal_text = terminal_id.to_string();
        let command_text = command_id.to_string();
        let description_text = description.trim().to_string();
        run_transaction(&mut guard, |tx| {
            if let Some((stored_hash, payload)) = lookup_command(tx, &command_text)? {
                if let Some(replay) =
                    evaluate_stored_json::<CashMovementView>(&stored_hash, &payload, &request_hash)?
                {
                    return Ok(replay);
                }
            }
            let active = load_active_for_terminal(tx, &terminal_text)?;
            let current = match active {
                Some(view) if view.status == "OPEN" => view,
                _ => return Err(tx_failure("NO_OPEN_CASH_SESSION".to_string())),
            };
            let movement_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO cash_movements
                 (id, cash_session_id, movement_type, amount_centimes,
                  reference_type, reference_id, description, created_at, created_by)
                 VALUES (?1, ?2, 'EXPENSE_OUT', ?3, NULL, NULL, ?4, ?5, ?6)",
                rusqlite::params![
                    movement_id,
                    current.id,
                    -amount_centimes,
                    description_text,
                    stamp,
                    actor_text,
                ],
            )?;
            let new_json = format!(
                "{{\"session_id\":\"{}\",\"amount_centimes\":{amount_centimes}}}",
                current.id
            );
            insert_cash_audit_row(
                tx,
                "CASH_EXPENSE_RECORDED",
                "cash_movements",
                &movement_id,
                None,
                &new_json,
                &actor_text,
                &stamp,
            )?;
            let view = CashMovementView {
                id: movement_id.clone(),
                cash_session_id: current.id.clone(),
                movement_type: "EXPENSE_OUT".to_string(),
                amount_centimes: -amount_centimes,
                reference_type: None,
                reference_id: None,
                description: Some(description_text.clone()),
                created_at: stamp.clone(),
                created_by: actor_text.clone(),
                created_by_username: actor_text.clone(),
            };
            append_or_replay(
                tx,
                &command_text,
                &request_hash,
                "cash.recordExpense",
                &view,
                &stamp,
                &actor_text,
            )
        })
        .map_err(|error| match &error {
            InfrastructureError::Migration(message) if message == "NO_OPEN_CASH_SESSION" => {
                ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
            }
            _ => map_cash_error(error),
        })
    }

    fn get_session_report(&self, session_id: &str) -> Result<CashSessionReport, ApplicationError> {
        let guard = self.lock()?;
        let session = guard
            .query_row(
                "SELECT s.id, s.warehouse_id, s.terminal_id, s.status,
                        s.opened_by, COALESCE(u1.username, s.opened_by), s.opened_at,
                        s.opening_float_centimes,
                        s.closed_by, u2.username, s.closed_at,
                        s.actual_counted_centimes, s.system_expected_centimes,
                        s.variance_centimes, s.closing_notes, s.revision,
                        s.created_at, s.updated_at
                 FROM cash_sessions s
                 LEFT JOIN users u1 ON u1.id = s.opened_by
                 LEFT JOIN users u2 ON u2.id = s.closed_by
                 WHERE s.id = ?1",
                [session_id],
                |row| {
                    Ok(row_to_session(
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                        row.get(13)?,
                        row.get(14)?,
                        row.get(15)?,
                        row.get(16)?,
                        row.get(17)?,
                    ))
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| ApplicationError::Port("cash store failure".to_string()))?
            .ok_or(ApplicationError::Validation(
                DomainError::CashSessionNotFound,
            ))?;
        let mut statement = guard
            .prepare(
                "SELECT m.id, m.cash_session_id, m.movement_type, m.amount_centimes,
                        m.reference_type, m.reference_id, m.description, m.created_at,
                        m.created_by, COALESCE(u.username, m.created_by)
                 FROM cash_movements m
                 LEFT JOIN users u ON u.id = m.created_by
                 WHERE m.cash_session_id = ?1
                 ORDER BY m.created_at ASC, m.rowid ASC",
            )
            .map_err(|_| ApplicationError::Port("cash store failure".to_string()))?;
        let rows = statement
            .query_map([session_id], |row| {
                Ok(CashMovementView {
                    id: row.get(0)?,
                    cash_session_id: row.get(1)?,
                    movement_type: row.get(2)?,
                    amount_centimes: row.get(3)?,
                    reference_type: row.get(4)?,
                    reference_id: row.get(5)?,
                    description: row.get(6)?,
                    created_at: row.get(7)?,
                    created_by: row.get(8)?,
                    created_by_username: row.get(9)?,
                })
            })
            .map_err(|_| ApplicationError::Port("cash store failure".to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ApplicationError::Port("cash store failure".to_string()))?;
        let mut totals: Vec<CashMovementTotal> = Vec::new();
        for movement in &rows {
            if let Some(existing) = totals
                .iter_mut()
                .find(|total| total.movement_type == movement.movement_type)
            {
                existing.total_centimes = existing
                    .total_centimes
                    .checked_add(movement.amount_centimes)
                    .ok_or_else(|| ApplicationError::Port("cash total overflow".to_string()))?;
                existing.count += 1;
            } else {
                totals.push(CashMovementTotal {
                    movement_type: movement.movement_type.clone(),
                    total_centimes: movement.amount_centimes,
                    count: 1,
                });
            }
        }
        totals.sort_by(|a, b| a.movement_type.cmp(&b.movement_type));
        let is_final = session.status == "CLOSED";
        Ok(CashSessionReport {
            session,
            movements: rows,
            totals_by_type: totals,
            is_final,
        })
    }
}

/// Active OPEN session id for a terminal (checkout/credit/supplier-payment
/// precondition: ADR-025/ADR-030).
pub(crate) fn require_open_session_in_tx(
    tx: &Transaction,
    terminal_id: &str,
) -> Result<String, InfrastructureError> {
    tx.query_row(
        "SELECT id FROM cash_sessions WHERE terminal_id = ?1 AND status = 'OPEN' LIMIT 1",
        [terminal_id],
        |row| row.get(0),
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => tx_failure("NO_OPEN_CASH_SESSION".to_string()),
        other => InfrastructureError::from(other),
    })
}

/// Insert one SUPPLIER_PAYMENT_OUT movement inside the CALLER's
/// transaction (ADR-030: CASH supplier payments only).
/// `amount_centimes` is the positive payment magnitude; the stored row is
/// signed negative per the movement CHECK. The session precondition is the
/// caller's responsibility (call `require_open_session_in_tx` first).
pub(crate) fn insert_supplier_payment_out_in_tx(
    tx: &Transaction,
    cash_session_id: &str,
    amount_centimes: i64,
    reference_id: &str,
    description: Option<&str>,
    stamp: &str,
    actor: &str,
) -> Result<String, InfrastructureError> {
    let movement_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO cash_movements
         (id, cash_session_id, movement_type, amount_centimes,
          reference_type, reference_id, description, created_at, created_by)
         VALUES (?1, ?2, 'SUPPLIER_PAYMENT_OUT', ?3, 'SUPPLIER_PAYMENT', ?4, ?5, ?6, ?7)",
        rusqlite::params![
            movement_id,
            cash_session_id,
            -amount_centimes,
            reference_id,
            description,
            stamp,
            actor,
        ],
    )?;
    Ok(movement_id)
}
