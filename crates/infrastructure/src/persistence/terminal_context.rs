//! Terminal context: single persisted device identifier.
//!
//! Investigation (TASK-BRK-010 mandatory step): no persisted local device
//! identifier existed prior to that task; every `origin_device_id` write in
//! the codebase used a fixed placeholder string, and the `settings` table
//! held only printer keys. This module created the sovereign identifier
//! under the `settings` key `device_id` (UUID v4, generated once, persisted
//! permanently) and treats the same value as both `origin_device_id` and
//! `terminal_id` for the entire system going forward (ADR-028).

use rusqlite::{Connection, Transaction};
use uuid::Uuid;

use super::InfrastructureError;

/// Settings key holding the sovereign device identifier.
pub const DEVICE_ID_KEY: &str = "device_id";

/// Resolve the device identifier, generating and persisting it once when absent.
pub fn resolve_device_id(conn: &Connection) -> Result<String, InfrastructureError> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [DEVICE_ID_KEY],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    if let Some(value) = existing {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    let fresh = Uuid::new_v4().to_string();
    let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![DEVICE_ID_KEY, fresh, stamp],
    )?;
    Ok(fresh)
}

/// Resolve the device identifier inside a write transaction.
pub fn resolve_device_id_tx(tx: &Transaction) -> Result<String, InfrastructureError> {
    let existing: Option<String> = tx
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [DEVICE_ID_KEY],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    if let Some(value) = existing {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    let fresh = Uuid::new_v4().to_string();
    let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    tx.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![DEVICE_ID_KEY, fresh, stamp],
    )?;
    Ok(fresh)
}

/// Managed terminal context shared via Tauri state (never client input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalContext {
    /// Sovereign terminal identifier (== device identifier).
    pub terminal_id: String,
}

impl TerminalContext {
    /// Build from a resolved identifier.
    pub fn new(terminal_id: String) -> Self {
        Self { terminal_id }
    }

    /// Borrow the identifier.
    #[must_use]
    pub fn terminal_id(&self) -> &str {
        &self.terminal_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::connection::open_in_memory;
    use crate::persistence::migration::run_migrations;

    #[test]
    fn device_id_is_stable_across_resolutions() {
        let mut conn = open_in_memory().expect("memory");
        run_migrations(&mut conn).expect("migrate");
        let first = resolve_device_id(&conn).expect("first");
        assert_eq!(first.len(), 36);
        assert!(Uuid::parse_str(&first).is_ok());
        let second = resolve_device_id(&conn).expect("second");
        assert_eq!(first, second);
    }
}
