//! Spooler transport selection facts (TASK-BRK-089-C1).
//!
//! The document-details print path can reach the printer over serial
//! (COM) or through the Windows spooler (RAW to a named queue). Which
//! arm is active is a persisted merchant setting; these are the domain
//! rules for that setting. The OS call itself lives in `spooler_raw`.

use crate::errors::DomainError;

/// Persisted `printer.transport` value for the serial (COM) arm.
pub const TRANSPORT_SERIAL: &str = "serial";

/// Persisted `printer.transport` value for the spooler (RAW queue) arm.
pub const TRANSPORT_SPOOLER: &str = "spooler";

/// Ticket render mode: engine ESC/POS bytes (true receipt printers).
pub const TICKET_RENDER_ESC_POS: &str = "escpos";

/// Ticket render mode: OS-graphics HTML ticket (label-class USB printers,
/// ADR-104). The engine creates the job; the worker emits zero bytes.
pub const TICKET_RENDER_GRAPHIQUE: &str = "graphique";

/// Maximum spooler queue-name length in characters (mirrors the FFI
/// boundary; validated again at the call site).
pub const MAX_QUEUE_NAME_CHARS: usize = 220;

/// Strict validation for the persisted transport selector: only the two
/// known arms; anything else is rejected before any write.
pub fn validate_printer_transport(value: &str) -> Result<&str, DomainError> {
    match value.trim() {
        TRANSPORT_SERIAL => Ok(TRANSPORT_SERIAL),
        TRANSPORT_SPOOLER => Ok(TRANSPORT_SPOOLER),
        other => Err(DomainError::InvalidCanonicalString(format!(
            "printer.transport must be 'serial' or 'spooler', got '{other}'"
        ))),
    }
}

/// Strict validation for the persisted ticket render mode: only the two
/// known modes; anything else is rejected before any write.
pub fn validate_ticket_render_mode(value: &str) -> Result<&str, DomainError> {
    match value.trim() {
        TICKET_RENDER_ESC_POS => Ok(TICKET_RENDER_ESC_POS),
        TICKET_RENDER_GRAPHIQUE => Ok(TICKET_RENDER_GRAPHIQUE),
        other => Err(DomainError::InvalidCanonicalString(format!(
            "ticket_render_mode must be 'escpos' or 'graphique', got '{other}'"
        ))),
    }
}

/// Strict validation for the persisted spooler queue name: non-empty after
/// trimming, at most 220 characters. Unicode names are allowed (the FFI
/// boundary encodes wide strings).
pub fn validate_spooler_queue_name(value: &str) -> Result<String, DomainError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DomainError::InvalidCanonicalString(
            "printer.queue_name must not be empty".to_string(),
        ));
    }
    if trimmed.chars().count() > MAX_QUEUE_NAME_CHARS {
        return Err(DomainError::InvalidCanonicalString(format!(
            "printer.queue_name exceeds {MAX_QUEUE_NAME_CHARS} characters"
        )));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_both_render_modes() {
        assert_eq!(validate_ticket_render_mode("escpos"), Ok("escpos"));
        assert_eq!(validate_ticket_render_mode("graphique"), Ok("graphique"));
        assert_eq!(
            validate_ticket_render_mode("  graphique  "),
            Ok("graphique")
        );
    }

    #[test]
    fn rejects_unknown_render_mode() {
        assert!(validate_ticket_render_mode("tspl").is_err());
        assert!(validate_ticket_render_mode("").is_err());
        assert!(validate_ticket_render_mode("ESC POS").is_err());
    }
    #[test]
    fn accepts_both_arms() {
        assert_eq!(validate_printer_transport("serial"), Ok("serial"));
        assert_eq!(validate_printer_transport("spooler"), Ok("spooler"));
    }

    #[test]
    fn rejects_unknown_transport() {
        assert!(validate_printer_transport("usb").is_err());
        assert!(validate_printer_transport("").is_err());
    }

    #[test]
    fn accepts_exact_queue_names() {
        assert_eq!(
            validate_spooler_queue_name("  XPrinter XP-410B  ").as_deref(),
            Ok("XPrinter XP-410B")
        );
        assert!(validate_spooler_queue_name("طابعة المطبخ").is_ok());
    }

    #[test]
    fn rejects_empty_and_overlong_queue_names() {
        assert!(validate_spooler_queue_name("   ").is_err());
        let long: String = "x".repeat(MAX_QUEUE_NAME_CHARS + 1);
        assert!(validate_spooler_queue_name(&long).is_err());
        let boundary: String = "x".repeat(MAX_QUEUE_NAME_CHARS);
        assert!(validate_spooler_queue_name(&boundary).is_ok());
    }
}
