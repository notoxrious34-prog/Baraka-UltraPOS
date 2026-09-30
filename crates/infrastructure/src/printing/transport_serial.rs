//! Serial (COM) printer transport over the `serialport` crate.
//!
//! Configuration is read fresh from the `settings` table on every call so
//! merchant changes apply without a restart. Pure Rust on Windows.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::PrinterTransportPort;
use baraka_domain::errors::DomainError;
use rusqlite::Connection;

/// Write timeout per physical send.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// Serial printer transport over a shared connection (settings source).
pub struct SerialPrinterTransport {
    db: Arc<Mutex<Connection>>,
}

impl SerialPrinterTransport {
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

    /// Configured COM port, or `None` when unconfigured.
    fn port_name(&self) -> Option<String> {
        self.setting("printer.serial_port").and_then(|port| {
            let trimmed = port.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    /// Configured baud rate (falls back to 9600 on garbage).
    fn baud_rate(&self) -> u32 {
        self.setting("printer.serial_baud")
            .and_then(|text| text.trim().parse::<u32>().ok())
            .unwrap_or(9_600)
    }
}

impl PrinterTransportPort for SerialPrinterTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        let port_name = self.port_name().ok_or(ApplicationError::Validation(
            DomainError::PrinterTransportUnavailable,
        ))?;
        let mut port = serialport::new(port_name, self.baud_rate())
            .timeout(WRITE_TIMEOUT)
            .open()
            .map_err(|_| ApplicationError::Validation(DomainError::PrinterTransportUnavailable))?;
        use std::io::Write;
        port.write_all(bytes)
            .and_then(|()| port.flush())
            .map_err(|_| ApplicationError::Validation(DomainError::PrinterTransportUnavailable))
    }

    fn is_available(&self) -> bool {
        let Some(port_name) = self.port_name() else {
            return false;
        };
        serialport::new(port_name, self.baud_rate())
            .timeout(Duration::from_millis(300))
            .open()
            .is_ok()
    }
}
