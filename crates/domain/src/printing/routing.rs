//! Dual-printer role routing (TASK-BRK-113).
//!
//! Two independent print roles — the counter **Receipt** printer and the
//! dedicated **Label** (`XP-410B`) printer — each with its own transport
//! arm plus a primary/fallback spooler-queue pair. Settings are additive
//! key/value rows (no migration): a missing role key falls back to the
//! legacy global (`printer.transport` / `printer.queue_name`), so
//! single-printer merchants keep working with zero reconfiguration.
//!
//! Fallback rule (per-role fallback queues): attempt the primary queue;
//! on transport failure attempt the fallback queue when it is configured,
//! non-empty, and different from the primary. Anything else surfaces the
//! original error — silent rerouting to an unconfigured queue is never
//! allowed. The serial (COM) port stays global (single-COM limitation).

use crate::errors::DomainError;

use super::transport::{
    validate_printer_transport, validate_spooler_queue_name, TRANSPORT_SERIAL, TRANSPORT_SPOOLER,
};

/// Counter receipt role: tickets, debt receipts, reports.
pub const PRINT_ROLE_RECEIPT: &str = "receipt";

/// Dedicated label role: 40x20 shelf labels (`XP-410B`).
pub const PRINT_ROLE_LABEL: &str = "label";

/// Print role selector for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintRole {
    Receipt,
    Label,
}

impl PrintRole {
    /// Strict parse of a persisted role spelling.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value.trim() {
            PRINT_ROLE_RECEIPT => Ok(PrintRole::Receipt),
            PRINT_ROLE_LABEL => Ok(PrintRole::Label),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "print role must be 'receipt' or 'label', got '{other}'"
            ))),
        }
    }

    /// Canonical wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            PrintRole::Receipt => PRINT_ROLE_RECEIPT,
            PrintRole::Label => PRINT_ROLE_LABEL,
        }
    }

    /// Settings key holding this role's transport arm.
    pub fn transport_key(self) -> String {
        format!("printer.{}.transport", self.as_str())
    }

    /// Settings key holding this role's primary spooler queue.
    pub fn queue_key(self) -> String {
        format!("printer.{}.queue_name", self.as_str())
    }

    /// Settings key holding this role's fallback spooler queue.
    pub fn fallback_queue_key(self) -> String {
        format!("printer.{}.queue_name_fallback", self.as_str())
    }
}

/// Resolved queue pair for one role: primary plus optional fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleQueues {
    pub primary: String,
    pub fallback: Option<String>,
}

/// Resolve one role's transport arm. The role key always wins. The receipt
/// role falls back to the legacy global (single-printer backward
/// compatibility, then serial). The label role NEVER inherits the legacy
/// arm — a COM receipt printer must not silently receive TSPL label bytes;
/// an unconfigured label role means the spooler arm (the `XP-410B`
/// default) with an empty queue, which fails LOUDLY as unavailable.
/// Unknown spellings are rejected, never coerced.
pub fn resolve_role_transport(
    role: PrintRole,
    role_value: Option<&str>,
    legacy_value: Option<&str>,
) -> Result<String, DomainError> {
    let role_raw = role_value.map(str::trim).filter(|value| !value.is_empty());
    if let Some(raw) = role_raw {
        return Ok(validate_printer_transport(raw)?.to_string());
    }
    match role {
        PrintRole::Receipt => {
            let legacy_raw = legacy_value
                .map(str::trim)
                .filter(|value| !value.is_empty());
            match legacy_raw {
                Some(raw) => Ok(validate_printer_transport(raw)?.to_string()),
                None => Ok(TRANSPORT_SERIAL.to_string()),
            }
        }
        PrintRole::Label => Ok(TRANSPORT_SPOOLER.to_string()),
    }
}

/// Resolve one role's queue pair. Empty/absent primary is an error (the
/// caller maps it to `PrinterTransportUnavailable`); empty fallback means
/// no fallback; a fallback identical to the primary is dropped (it would
/// retry the same queue instead of rerouting).
pub fn resolve_role_queues(
    primary_value: Option<&str>,
    fallback_value: Option<&str>,
) -> Result<RoleQueues, DomainError> {
    let primary_raw = primary_value
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(primary_raw) = primary_raw else {
        return Err(DomainError::InvalidCanonicalString(
            "role queue_name must not be empty".to_string(),
        ));
    };
    let primary = validate_spooler_queue_name(primary_raw)?;
    let fallback = fallback_value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(validate_spooler_queue_name)
        .transpose()?
        .filter(|candidate| *candidate != primary);
    Ok(RoleQueues { primary, fallback })
}

/// True when a fallback attempt is allowed after a primary failure: a
/// distinct fallback queue is configured.
pub fn should_try_fallback(queues: &RoleQueues) -> bool {
    queues.fallback.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_roles_and_rejects_unknown() {
        assert_eq!(PrintRole::parse("receipt"), Ok(PrintRole::Receipt));
        assert_eq!(PrintRole::parse("  label "), Ok(PrintRole::Label));
        assert!(PrintRole::parse("ticket").is_err());
        assert!(PrintRole::parse("").is_err());
    }

    #[test]
    fn role_keys_follow_the_printer_role_namespace() {
        assert_eq!(
            PrintRole::Receipt.transport_key(),
            "printer.receipt.transport"
        );
        assert_eq!(PrintRole::Label.queue_key(), "printer.label.queue_name");
        assert_eq!(
            PrintRole::Label.fallback_queue_key(),
            "printer.label.queue_name_fallback"
        );
    }

    #[test]
    fn role_transport_prefers_role_key_then_legacy_then_serial() {
        assert_eq!(
            resolve_role_transport(PrintRole::Receipt, Some("spooler"), Some("serial")),
            Ok("spooler".to_string())
        );
        assert_eq!(
            resolve_role_transport(PrintRole::Receipt, None, Some("spooler")),
            Ok("spooler".to_string())
        );
        assert_eq!(
            resolve_role_transport(PrintRole::Receipt, None, None),
            Ok("serial".to_string())
        );
    }

    #[test]
    fn blank_role_keys_fall_through_to_legacy() {
        assert_eq!(
            resolve_role_transport(PrintRole::Receipt, Some("   "), Some("spooler")),
            Ok("spooler".to_string())
        );
    }

    #[test]
    fn label_defaults_to_spooler_when_nothing_is_configured() {
        assert_eq!(
            resolve_role_transport(PrintRole::Label, None, None),
            Ok("spooler".to_string())
        );
    }

    #[test]
    fn label_role_key_wins_over_legacy() {
        assert_eq!(
            resolve_role_transport(PrintRole::Label, Some("serial"), Some("spooler")),
            Ok("serial".to_string())
        );
    }

    #[test]
    fn label_never_inherits_the_legacy_arm() {
        assert_eq!(
            resolve_role_transport(PrintRole::Label, None, Some("serial")),
            Ok("spooler".to_string())
        );
    }

    #[test]
    fn rejects_unknown_transport_spellings() {
        assert!(resolve_role_transport(PrintRole::Receipt, Some("usb"), None).is_err());
        assert!(resolve_role_transport(PrintRole::Receipt, None, Some("usb")).is_err());
    }

    #[test]
    fn resolves_primary_and_drops_identical_fallback() {
        let queues = resolve_role_queues(Some("  XP-410B  "), Some("XP-410B")).unwrap();
        assert_eq!(queues.primary, "XP-410B");
        assert_eq!(queues.fallback, None);
        assert!(!should_try_fallback(&queues));
    }

    #[test]
    fn keeps_a_distinct_fallback_queue() {
        let queues = resolve_role_queues(Some("XP-410B"), Some("  Backup-Label ")).unwrap();
        assert_eq!(queues.primary, "XP-410B");
        assert_eq!(queues.fallback, Some("Backup-Label".to_string()));
        assert!(should_try_fallback(&queues));
    }

    #[test]
    fn empty_primary_is_an_error_and_empty_fallback_means_none() {
        assert!(resolve_role_queues(None, Some("Q")).is_err());
        assert!(resolve_role_queues(Some("   "), None).is_err());
        let queues = resolve_role_queues(Some("Q"), Some("   ")).unwrap();
        assert_eq!(queues.fallback, None);
    }

    #[test]
    fn overlong_queue_names_are_rejected_per_arm() {
        let long: String = "x".repeat(221);
        assert!(resolve_role_queues(Some(&long), None).is_err());
    }
}
