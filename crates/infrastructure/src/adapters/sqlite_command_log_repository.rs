//! SQLite idempotency log repository.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{CommandLogRepository, StoredCommand};
use baraka_domain::user::UserId;
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;

use crate::persistence::transaction::run_transaction;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("command log store failure".to_string())
}

/// Idempotency log over a shared SQLite connection.
pub struct SqliteCommandLogRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteCommandLogRepository {
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

impl CommandLogRepository for SqliteCommandLogRepository {
    fn lookup(&self, command_id: &str) -> Result<Option<StoredCommand>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT command_id, request_hash, response_payload FROM command_log WHERE command_id = ?1",
                [command_id],
                |row| {
                    Ok(StoredCommand {
                        command_id: row.get(0)?,
                        request_hash: row.get(1)?,
                        response_payload: row.get(2)?,
                    })
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn append_completed(
        &self,
        command_id: &str,
        request_hash: &str,
        command_name: &str,
        response_payload: &str,
        actor: UserId,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "INSERT INTO command_log
                 (command_id, request_hash, command_type, response_payload, created_at, created_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    command_id,
                    request_hash,
                    command_name,
                    response_payload,
                    stamp,
                    actor.to_string()
                ],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }
}
