//! Database backup use cases (TASK-BRK-051, ADR-010).
//!
//! Consistent online snapshots with self-verification, SHA-256
//! tamper-evidence, `backups_log` audit rows, and retention rotation.
//! The use cases orchestrate; all SQLite and filesystem work lives behind
//! `BackupRepository`. Any authenticated session may invoke these —
//! role gating (when ordered) belongs at the future command edge.

use std::path::{Path, PathBuf};

use baraka_domain::backup::{
    BackupMetadata, BackupStatus, BackupTriggerType, PreRestoreReport, RestoreError, RestoreOutcome,
};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Permission;
use chrono::Utc;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::ports::{BackupLogEntry, BackupRepository, SessionContext};

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// Snapshot file stem timestamp (`YYYYMMDDTHHMMSS`, UTC).
fn stamp_now() -> String {
    Utc::now().format("%Y%m%dT%H%M%S").to_string()
}

/// Snapshot file name for a fresh backup id.
fn file_name_for(id: &Uuid) -> String {
    let short = id.to_string().replace('-', "");
    format!("baraka_pos_backup_{}_{}.db", stamp_now(), &short[..8])
}

/// Create a verified backup snapshot with rotation.
pub struct CreateBackupUseCase<'a, R>
where
    R: ?Sized,
{
    backups: &'a R,
}

impl<'a, R> CreateBackupUseCase<'a, R>
where
    R: BackupRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(backups: &'a R) -> Self {
        Self { backups }
    }

    /// Capture, verify, hash, log, and rotate.
    ///
    /// `retention` keeps that many newest SUCCESS rows (minimum 1);
    /// older SUCCESS snapshots are deleted from disk and marked PRUNED.
    /// A failed verification records FAILED, removes the file, and
    /// returns the failure — nothing unverified is ever kept.
    pub fn execute(
        &self,
        _session: &SessionContext,
        trigger: BackupTriggerType,
        dest_dir: &Path,
        retention: usize,
        terminal_id: &str,
        app_version: &str,
    ) -> Result<BackupMetadata, ApplicationError> {
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let keep = retention.max(1);
        let id = Uuid::new_v4();
        let file_name = file_name_for(&id);
        let dest = dest_dir.join(&file_name);
        let created_at = Utc::now();
        let schema_version = self.backups.schema_version();
        self.backups.snapshot_to(&dest)?;
        let verification = self.backups.verify_snapshot(&dest)?;
        if !verification.quick_check_ok || verification.foreign_key_violations != 0 {
            let _ = std::fs::remove_file(&dest);
            self.backups.record_backup(&BackupLogEntry {
                id: id.to_string(),
                backup_id: file_name.clone(),
                file_name: file_name.clone(),
                app_version: app_version.to_string(),
                schema_version,
                created_at: created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                source_device_id: terminal_id.trim().to_string(),
                database_hash: String::new(),
                cipher_version: "NONE".to_string(),
                status: "FAILED".to_string(),
                file_size_bytes: None,
                trigger: trigger.as_code().to_string(),
            })?;
            return Err(coded("BACKUP_VERIFICATION_FAILED"));
        }
        let sha256_checksum = self.backups.sha256_of(&dest)?;
        let file_size_bytes = self.backups.file_size_of(&dest)?;
        self.backups.record_backup(&BackupLogEntry {
            id: id.to_string(),
            backup_id: file_name.clone(),
            file_name: file_name.clone(),
            app_version: app_version.to_string(),
            schema_version,
            created_at: created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            source_device_id: terminal_id.trim().to_string(),
            database_hash: sha256_checksum.clone(),
            cipher_version: "NONE".to_string(),
            status: "SUCCESS".to_string(),
            file_size_bytes: Some(file_size_bytes),
            trigger: trigger.as_code().to_string(),
        })?;
        self.enforce_retention(dest_dir, keep)?;
        Ok(BackupMetadata {
            id,
            file_name,
            file_path: dest.to_string_lossy().to_string(),
            file_size_bytes: Some(file_size_bytes),
            sha256_checksum,
            trigger_type: trigger,
            status: BackupStatus::Success,
            schema_version: u32::try_from(schema_version).unwrap_or(u32::MAX),
            error_message: None,
            created_at,
        })
    }

    /// Delete SUCCESS snapshots beyond the newest `keep`, marking PRUNED.
    fn enforce_retention(&self, dest_dir: &Path, keep: usize) -> Result<(), ApplicationError> {
        let rows = self.backups.list_backups(keep.saturating_add(10_000))?;
        let mut survivors = 0usize;
        for row in rows {
            if row.status != "SUCCESS" {
                continue;
            }
            survivors += 1;
            if survivors > keep {
                let _ = self.backups.delete_file(&dest_dir.join(&row.file_name));
                self.backups.mark_status(&row.backup_id, "PRUNED")?;
            }
        }
        Ok(())
    }
}

/// List recent backups, newest first.
pub struct ListBackupsUseCase<'a, R>
where
    R: ?Sized,
{
    backups: &'a R,
}

impl<'a, R> ListBackupsUseCase<'a, R>
where
    R: BackupRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(backups: &'a R) -> Self {
        Self { backups }
    }

    /// Load rows and rejoin each file name with `dest_dir`.
    pub fn execute(
        &self,
        _session: &SessionContext,
        dest_dir: &Path,
        limit: usize,
    ) -> Result<Vec<BackupMetadata>, ApplicationError> {
        let rows = self.backups.list_backups(limit.clamp(1, 10_000))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(BackupMetadata {
                id: Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil()),
                file_name: row.file_name.clone(),
                file_path: dest_dir.join(&row.file_name).to_string_lossy().to_string(),
                file_size_bytes: row.file_size_bytes,
                sha256_checksum: row.database_hash.clone(),
                trigger_type: BackupTriggerType::from_code(&row.trigger),
                status: match row.status.as_str() {
                    "SUCCESS" => BackupStatus::Success,
                    "CORRUPTED" => BackupStatus::Corrupted,
                    _ => BackupStatus::Failed,
                },
                schema_version: u32::try_from(row.schema_version).unwrap_or(u32::MAX),
                error_message: None,
                created_at: row
                    .created_at
                    .parse::<chrono::DateTime<Utc>>()
                    .unwrap_or_else(|_| Utc::now()),
            });
        }
        Ok(out)
    }
}

/// Re-verify one logged backup against its file on disk.
pub struct VerifyBackupIntegrityUseCase<'a, R>
where
    R: ?Sized,
{
    backups: &'a R,
}

impl<'a, R> VerifyBackupIntegrityUseCase<'a, R>
where
    R: BackupRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(backups: &'a R) -> Self {
        Self { backups }
    }

    /// Fail `BACKUP_MISSING` (no file), `BACKUP_CHECKSUM_MISMATCH`
    /// (bytes drifted), or `BACKUP_CORRUPTED` (pragma failure); mark the
    /// row CORRUPTED on pragma failure and return the metadata otherwise.
    pub fn execute(
        &self,
        _session: &SessionContext,
        dest_dir: &Path,
        backup_id: &str,
    ) -> Result<BackupMetadata, ApplicationError> {
        let row = self
            .backups
            .get_backup(backup_id.trim())?
            .ok_or_else(|| coded("BACKUP_MISSING"))?;
        let path: PathBuf = dest_dir.join(&row.file_name);
        if !path.is_file() {
            return Err(coded("BACKUP_MISSING"));
        }
        let actual = self.backups.sha256_of(&path)?;
        if actual != row.database_hash {
            return Err(coded("BACKUP_CHECKSUM_MISMATCH"));
        }
        let verification = self.backups.verify_snapshot(&path)?;
        if !verification.quick_check_ok || verification.foreign_key_violations != 0 {
            self.backups.mark_status(&row.backup_id, "CORRUPTED")?;
            return Err(coded("BACKUP_CORRUPTED"));
        }
        let size = self.backups.file_size_of(&path)?;
        // Verify re-stats and corrects drift: a row whose stored size no
        // longer matches the file it points at is repaired in place.
        if row.file_size_bytes != Some(size) {
            self.backups.correct_file_size(&row.backup_id, size)?;
        }
        Ok(BackupMetadata {
            id: Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::nil()),
            file_name: row.file_name.clone(),
            file_path: path.to_string_lossy().to_string(),
            file_size_bytes: Some(size),
            sha256_checksum: actual,
            trigger_type: BackupTriggerType::from_code(&row.trigger),
            status: BackupStatus::Success,
            schema_version: u32::try_from(row.schema_version).unwrap_or(u32::MAX),
            error_message: None,
            created_at: row
                .created_at
                .parse::<chrono::DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

/// Require the settings grant; restore commands are Admin-only.
fn require_restore_admin(session: &SessionContext) -> Result<(), RestoreError> {
    if session
        .role
        .has_permission(Permission::SystemSettingsManage)
    {
        Ok(())
    } else {
        Err(RestoreError::Unauthorized)
    }
}

/// Read-only restore precheck (TASK-BRK-054, ADR-062).
///
/// Permission check + delegation only: every verification lives behind
/// `BackupRepository`, and this use case holds zero infrastructure
/// knowledge. The report is advisory; `RestoreBackupUseCase` re-runs every
/// verification internally and never trusts this output.
pub struct PrecheckRestoreUseCase<'a, R>
where
    R: ?Sized,
{
    backups: &'a R,
}

impl<'a, R> PrecheckRestoreUseCase<'a, R>
where
    R: BackupRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(backups: &'a R) -> Self {
        Self { backups }
    }

    /// Return the full verification report for one logged backup.
    pub fn execute(
        &self,
        session: &SessionContext,
        dest_dir: &Path,
        backup_id: &str,
    ) -> Result<PreRestoreReport, RestoreError> {
        require_restore_admin(session)?;
        self.backups.precheck_restore(backup_id.trim(), dest_dir)
    }
}

/// Verified destructive restore (TASK-BRK-054, ADR-062).
///
/// Permission check + delegation only: the port implementation re-runs
/// every verification internally, quarantines the active database, and
/// rolls back on any post-quarantine failure.
pub struct RestoreBackupUseCase<'a, R>
where
    R: ?Sized,
{
    backups: &'a R,
}

impl<'a, R> RestoreBackupUseCase<'a, R>
where
    R: BackupRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(backups: &'a R) -> Self {
        Self { backups }
    }

    /// Execute the verified restore; `actor` feeds the audit trail.
    pub fn execute(
        &self,
        session: &SessionContext,
        dest_dir: &Path,
        backup_id: &str,
    ) -> Result<RestoreOutcome, RestoreError> {
        require_restore_admin(session)?;
        self.backups
            .restore_backup(backup_id.trim(), dest_dir, &session.user_id.to_string())
    }
}
