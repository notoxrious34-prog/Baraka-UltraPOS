//! USB spooler (RAW) printer transport over `spooler_raw` (TASK-BRK-089-C1).
//!
//! Configuration is read fresh from the `settings` table on every call so
//! merchant changes apply without a restart. No `unsafe` here — all Win32
//! FFI lives in the `spooler-raw` crate behind its safe API.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::PrinterTransportPort;
use baraka_domain::errors::DomainError;
use rusqlite::Connection;

/// Spooler RAW transport over a shared connection (settings source).
pub struct SpoolerRawTransport {
    db: Arc<Mutex<Connection>>,
}

impl SpoolerRawTransport {
    /// Wrap the shared connection used for settings reads.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    /// Read one `printer.*` setting; `None` when the key is absent.
    fn setting(&self, key: &str) -> Option<String> {
        let guard = self.db.lock().ok()?;
        guard
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .ok()
    }

    /// Configured queue name, or `None` when unconfigured.
    fn queue_name(&self) -> Option<String> {
        self.setting("printer.queue_name").and_then(|queue| {
            let trimmed = queue.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    /// Map a `spooler_raw` failure onto the existing error taxonomy.
    /// Unconfigured/empty queue mirrors the serial missing-port case;
    /// overlong names and OS failures carry their diagnostic detail.
    fn map_error(error: spooler_raw::SpoolerError) -> ApplicationError {
        match error {
            spooler_raw::SpoolerError::InvalidQueueName(detail)
                if detail == "queue name is empty" =>
            {
                ApplicationError::Validation(DomainError::PrinterTransportUnavailable)
            }
            other => {
                ApplicationError::Validation(DomainError::InvalidCanonicalString(other.to_string()))
            }
        }
    }
}

impl PrinterTransportPort for SpoolerRawTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        let queue_name = self.queue_name().ok_or(ApplicationError::Validation(
            DomainError::PrinterTransportUnavailable,
        ))?;
        let queue = spooler_raw::SpoolerQueue::new(&queue_name).map_err(Self::map_error)?;
        spooler_raw::submit_raw(&queue, bytes).map_err(Self::map_error)
    }

    fn is_available(&self) -> bool {
        let Some(queue_name) = self.queue_name() else {
            return false;
        };
        let Ok(queue) = spooler_raw::SpoolerQueue::new(&queue_name) else {
            return false;
        };
        // Probe opens + closes without starting a document: no empty
        // job is ever spooled by an availability check.
        queue.probe().is_ok()
    }
}
