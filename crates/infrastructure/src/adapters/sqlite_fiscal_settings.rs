//! SQLite fiscal stamp settings (`fiscal.*` keys in the `settings` table).
//!
//! Zero schema changes (ADR-092): the toggle persists as the canonical
//! `"true"` / `"false"` string under `fiscal.timbre_enabled`; an absent
//! key reads as enabled.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::FiscalSettingsPort;
use rusqlite::Connection;

/// Settings key for the Algerian fiscal stamp toggle.
pub const TIMBRE_ENABLED_KEY: &str = "fiscal.timbre_enabled";

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("fiscal settings store failure".to_string())
}

/// Fiscal stamp settings persistence over a shared SQLite connection.
pub struct SqliteFiscalSettings {
    db: Arc<Mutex<Connection>>,
}

impl SqliteFiscalSettings {
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

impl FiscalSettingsPort for SqliteFiscalSettings {
    fn timbre_enabled(&self) -> Result<bool, ApplicationError> {
        let guard = self.lock()?;
        let value: Option<String> = guard
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [TIMBRE_ENABLED_KEY],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())?;
        Ok(value.as_deref().map(str::trim) != Some("false"))
    }

    fn set_timbre_enabled(&self, enabled: bool) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let value = if enabled { "true" } else { "false" };
        guard
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                rusqlite::params![TIMBRE_ENABLED_KEY, value, stamp],
            )
            .map_err(|_| store_failure())?;
        Ok(())
    }
}
