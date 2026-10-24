//! Transport selection router (TASK-BRK-089-C1).
//!
//! Reads `printer.transport` fresh on every call (`"serial"` default,
//! `"spooler"` for the USB RAW arm) and delegates to the matching
//! transport. Unknown or absent values fall back to serial — the legacy
//! behavior — because only validated values can be persisted.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::PrinterTransportPort;
use rusqlite::Connection;

use super::transport_serial::SerialPrinterTransport;
use super::transport_spooler::SpoolerRawTransport;

/// Routes print traffic to the configured transport arm.
pub struct RoutingPrinterTransport {
    serial: SerialPrinterTransport,
    spooler: SpoolerRawTransport,
    db: Arc<Mutex<Connection>>,
}

impl RoutingPrinterTransport {
    /// Wrap the shared connection; both arms read settings from it.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self {
            serial: SerialPrinterTransport::new(Arc::clone(&db)),
            spooler: SpoolerRawTransport::new(Arc::clone(&db)),
            db,
        }
    }

    /// True when the spooler arm is selected.
    fn use_spooler(&self) -> bool {
        let selection: Option<String> = self
            .db
            .lock()
            .ok()
            .and_then(|guard| {
                guard
                    .query_row(
                        "SELECT value FROM settings WHERE key = ?1",
                        ["printer.transport"],
                        |row| row.get::<_, String>(0),
                    )
                    .ok()
            })
            .map(|value| value.trim().to_string());
        selection.as_deref() == Some(baraka_domain::printing::TRANSPORT_SPOOLER)
    }
}

impl PrinterTransportPort for RoutingPrinterTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        if self.use_spooler() {
            self.spooler.send_raster(bytes)
        } else {
            self.serial.send_raster(bytes)
        }
    }

    fn is_available(&self) -> bool {
        if self.use_spooler() {
            self.spooler.is_available()
        } else {
            self.serial.is_available()
        }
    }
}
