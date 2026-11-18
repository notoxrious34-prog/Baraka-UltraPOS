//! Transport selection router (TASK-BRK-089-C1, extended BRK-113).
//!
//! Dual-printer routing: the counter **Receipt** role and the dedicated
//! **Label** role each carry their own transport arm plus a primary /
//! fallback spooler-queue pair (`printer.{receipt,label}.*` keys). Keys are
//! read fresh on every call; a missing role key falls back to the legacy
//! global for the receipt role only (single-printer backward
//! compatibility). Unknown stored spellings fail LOUDLY — writes are
//! validated upstream, so only a hand-edited DB can hold garbage.
//! The serial (COM) arm stays global (single-COM limitation).

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::PrinterTransportPort;
use baraka_domain::printing::{
    resolve_role_queues, resolve_role_transport, should_try_fallback, PrintRole, TRANSPORT_SPOOLER,
};
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

    /// Read one trimmed non-empty setting, or `None` when absent/blank.
    fn setting(&self, key: &str) -> Option<String> {
        let guard = self.db.lock().ok()?;
        guard
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .ok()
            .map(|value: String| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    /// Resolve one role's transport arm (domain rule; legacy seeds receipt).
    /// Read posture is tolerant for the RECEIPT role only: pre-BRK-113
    /// databases carry the seeded uppercase default, and the old router
    /// treated any non-`"spooler"` spelling as serial — an invalid stored
    /// arm falls back to serial (writes are validated LOUDLY upstream, so
    /// only a hand-edited DB can hold garbage). The LABEL role is strict:
    /// its keys are new, so an invalid spelling is a real error.
    fn role_arm(&self, role: PrintRole) -> Result<String, ApplicationError> {
        let role_value = self.setting(&role.transport_key());
        let legacy = if role == PrintRole::Receipt {
            self.setting("printer.transport")
        } else {
            None
        };
        match resolve_role_transport(role, role_value.as_deref(), legacy.as_deref()) {
            Ok(arm) => Ok(arm),
            Err(error) if role == PrintRole::Label => Err(ApplicationError::Validation(error)),
            Err(_) => Ok("serial".to_string()),
        }
    }

    /// Send on the spooler arm with per-role fallback (primary, then the
    /// configured fallback queue when it is distinct).
    fn send_spooler_for_role(&self, role: PrintRole, bytes: &[u8]) -> Result<(), ApplicationError> {
        let queues = resolve_role_queues(
            self.setting(&role.queue_key()).as_deref(),
            self.setting(&role.fallback_queue_key()).as_deref(),
        )
        .map_err(|_| {
            ApplicationError::Validation(
                baraka_domain::errors::DomainError::PrinterTransportUnavailable,
            )
        })?;
        match self.spooler.send_to_queue(&queues.primary, bytes) {
            Ok(()) => Ok(()),
            Err(primary_error) if should_try_fallback(&queues) => self
                .spooler
                // `fallback` is `Some` here by `should_try_fallback`.
                .send_to_queue(queues.fallback.as_deref().unwrap_or(""), bytes)
                .map_err(|_| primary_error),
            Err(primary_error) => Err(primary_error),
        }
    }

    /// Role-scoped availability: any configured queue probes clean, or the
    /// serial arm answers.
    fn role_available(&self, role: PrintRole) -> bool {
        let arm = self.role_arm(role).unwrap_or_else(|_| "serial".to_string());
        if arm == TRANSPORT_SPOOLER {
            let primary = self.setting(&role.queue_key());
            let fallback = self.setting(&role.fallback_queue_key());
            primary
                .as_deref()
                .map(|queue| self.spooler.queue_available(queue))
                .unwrap_or(false)
                || fallback
                    .as_deref()
                    .map(|queue| self.spooler.queue_available(queue))
                    .unwrap_or(false)
        } else {
            self.serial.is_available()
        }
    }
}

impl PrinterTransportPort for RoutingPrinterTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        // Legacy single-arm path = the Receipt role (exact prior behavior:
        // receipt inherits the legacy global keys).
        self.send_raster_for_role(PrintRole::Receipt, bytes)
    }

    fn is_available(&self) -> bool {
        self.is_available_for_role(PrintRole::Receipt)
    }

    fn send_raster_for_role(&self, role: PrintRole, bytes: &[u8]) -> Result<(), ApplicationError> {
        if self.role_arm(role)? == TRANSPORT_SPOOLER {
            self.send_spooler_for_role(role, bytes)
        } else {
            self.serial.send_raster(bytes)
        }
    }

    fn is_available_for_role(&self, role: PrintRole) -> bool {
        self.role_available(role)
    }
}
