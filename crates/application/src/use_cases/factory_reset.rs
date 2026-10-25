//! Factory-reset use cases (TASK-BRK-091, AUD-043 §3).
//!
//! Engine reset that returns the database to first-run state with a
//! verified backup enforced inside the same operation. Wipe-by-DELETE is
//! impossible (`BEFORE DELETE` abort-triggers guard every ledger and log
//! table: migrations 0001/0004/0008/0009), so the reset builds a FRESH
//! migrated database file, carries `backups_log` across, appends a
//! chain-linked `RESET_PERFORMED` audit event as the first row, and
//! activates it through the restore-style quarantine-swap protocol.
//! No schema change was needed: the fresh file is produced by the embedded
//! migration runner, never by DDL written here.
//!
//! Ordering (crash-safety): preconditions (reads) -> typed-token compare
//! -> writability probe -> verified backup -> fresh-file build + swap.
//! A crash before the swap leaves the old database intact AND the fresh
//! backup present, so the operation is retryable; nothing is ever wiped
//! before the backup verifies.

use std::path::Path;

use baraka_domain::backup::{BackupTriggerType, FactoryResetOutcome};
use baraka_domain::errors::DomainError;

use crate::errors::ApplicationError;
use crate::ports::{
    BackupLogEntry, BackupRepository, CashSessionRepository, FactoryResetRepository, SessionContext,
};
use crate::use_cases::backup::{CreateBackupUseCase, VerifyBackupIntegrityUseCase};

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// Constant-shape string comparison (no early exit on first mismatch).
///
/// The typed token echoes a displayed value (SafeDeleteModal convention);
/// comparing in constant shape avoids leaking the expected value's prefix
/// through timing. Lengths still differ observably — accepted: the token
/// space is a store name, not a secret.
fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for index in 0..left.len() {
        diff |= left[index] ^ right[index];
    }
    diff == 0
}

/// Filesystem writability probe for the backup directory.
///
/// Creates and removes a probe file; maps ANY failure to
/// `RESET_BACKUP_UNWRITABLE`. Performs zero database writes.
fn probe_dir_writable(dir: &Path) -> Result<(), ApplicationError> {
    let probe = dir.join(".factory-reset-write-probe");
    if std::fs::write(&probe, b"1").is_err() {
        return Err(coded("RESET_BACKUP_UNWRITABLE"));
    }
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

/// Read-only precheck report: every gate the execute path will enforce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetPrecheck {
    /// An OPEN or CLOSING cash session exists for this terminal.
    pub open_session: bool,
    /// The backup directory accepted a write probe.
    pub backup_dir_writable: bool,
    /// Newest `backups_log` row, if any.
    pub newest_backup: Option<BackupLogEntry>,
    /// True only when every gate passes.
    pub eligible: bool,
    /// Failed gates, in check order (codes only; UI translates).
    pub reasons: Vec<String>,
}

/// Read-only factory-reset precheck (zero writes to any file or database).
pub struct PrecheckFactoryResetUseCase<'a, B, C>
where
    B: ?Sized,
    C: ?Sized,
{
    backups: &'a B,
    cash: &'a C,
}

impl<'a, B, C> PrecheckFactoryResetUseCase<'a, B, C>
where
    B: BackupRepository + ?Sized,
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(backups: &'a B, cash: &'a C) -> Self {
        Self { backups, cash }
    }

    /// Evaluate every execute gate without writing anything.
    pub fn execute(
        &self,
        _session: &SessionContext,
        terminal_id: &str,
        backup_dir: &Path,
    ) -> Result<ResetPrecheck, ApplicationError> {
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let open_session = self.cash.get_active_session(terminal_id.trim())?.is_some();
        let backup_dir_writable = probe_dir_writable(backup_dir).is_ok();
        let newest_backup = self.backups.list_backups(1)?.into_iter().next();
        let mut reasons = Vec::new();
        if open_session {
            reasons.push("RESET_OPEN_SESSION".to_string());
        }
        if !backup_dir_writable {
            reasons.push("RESET_BACKUP_UNWRITABLE".to_string());
        }
        Ok(ResetPrecheck {
            open_session,
            backup_dir_writable,
            newest_backup,
            eligible: reasons.is_empty(),
            reasons,
        })
    }
}

/// Destructive factory-reset executor.
pub struct ExecuteFactoryResetUseCase<'a, B, C, F>
where
    B: ?Sized,
    C: ?Sized,
    F: ?Sized,
{
    backups: &'a B,
    cash: &'a C,
    reset: &'a F,
}

/// Execution parameters (bundled: clippy caps free functions at 7 args).
pub struct ResetExecution<'a> {
    /// Sovereign terminal id (managed state, never client input).
    pub terminal_id: &'a str,
    /// Directory holding backup snapshot files.
    pub backup_dir: &'a Path,
    /// Newest SUCCESS rows to keep after rotation.
    pub retention: usize,
    /// Running binary version recorded in the backup row.
    pub app_version: &'a str,
    /// Retyped confirmation token (constant-shape compare).
    pub typed_token: &'a str,
    /// Operator id recorded in `RESET_PERFORMED`.
    pub actor: &'a str,
}

impl<'a, B, C, F> ExecuteFactoryResetUseCase<'a, B, C, F>
where
    B: BackupRepository + ?Sized,
    C: CashSessionRepository + ?Sized,
    F: FactoryResetRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(backups: &'a B, cash: &'a C, reset: &'a F) -> Self {
        Self {
            backups,
            cash,
            reset,
        }
    }

    /// Enforce preconditions, capture + verify the mandatory backup, then
    /// build, quarantine-swap, and activate the fresh database.
    ///
    /// Validation (token) runs before ANY write, including the backup:
    /// a wrong token leaves zero rows and zero files behind.
    pub fn execute(
        &self,
        session: &SessionContext,
        params: &ResetExecution<'_>,
    ) -> Result<FactoryResetOutcome, ApplicationError> {
        let terminal = params.terminal_id.trim();
        if terminal.is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        // Gate 1 (reads only): no open cash session on this terminal.
        if self.cash.get_active_session(terminal)?.is_some() {
            return Err(coded("RESET_OPEN_SESSION"));
        }
        // Gate 2 (validation before any write): typed token must match the
        // expected value (store name, or DEP-01 when blank) in constant shape.
        let expected = self.reset.expected_reset_token()?;
        if !constant_time_eq(params.typed_token.trim(), expected.trim()) {
            return Err(coded("RESET_TOKEN_MISMATCH"));
        }
        // Gate 3 (filesystem only, zero DB writes): backup dir writable.
        probe_dir_writable(params.backup_dir)?;
        // Step 1: mandatory verified backup through the existing use cases
        // (Manual trigger: no backup-protocol change — the RESET_PERFORMED
        // audit row in the fresh DB references this snapshot by id).
        let backup = CreateBackupUseCase::new(self.backups)
            .execute(
                session,
                BackupTriggerType::Manual,
                params.backup_dir,
                params.retention,
                terminal,
                params.app_version,
            )
            .map_err(|_| coded("RESET_BACKUP_FAILED"))?;
        VerifyBackupIntegrityUseCase::new(self.backups)
            .execute(session, params.backup_dir, &backup.file_name)
            .map_err(|_| coded("RESET_BACKUP_FAILED"))?;
        // Step 2: build + swap + activate. Crash before the swap leaves the
        // old database intact with the fresh backup present (retryable).
        self.reset.execute_reset(params.actor, &backup.file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn token_compare_is_exact_and_constant_shape() {
        assert!(constant_time_eq("Boutique", "Boutique"));
        assert!(!constant_time_eq("boutique", "Boutique"));
        assert!(!constant_time_eq("Boutique ", "Boutique"));
        assert!(!constant_time_eq("", "DEP-01"));
        assert!(constant_time_eq("", ""));
    }
}
