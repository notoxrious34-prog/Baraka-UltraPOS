//! Database backup metadata (TASK-BRK-051, ADR-010).
//!
//! Pure value types only: what triggered a backup, how it ended, and the
//! tamper-evidence binding it (SHA-256 over the snapshot bytes). No I/O,
//! no SQLite, no filesystem here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What triggered a backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackupTriggerType {
    /// Operator-initiated from the UI.
    Manual,
    /// Automatic capture at cash-session close.
    ShiftClose,
    /// Automatic capture on a timer.
    Scheduled,
}

/// Terminal lifecycle state of a backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackupStatus {
    /// Snapshot written, verified, checksummed, and logged.
    Success,
    /// Snapshot or verification failed (file removed, log retained).
    Failed,
    /// Stored file later failed verification (tamper or bit-rot signal).
    Corrupted,
}

/// One backup snapshot and its tamper-evidence binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Stable backup identifier.
    pub id: Uuid,
    /// Snapshot file name (`baraka_pos_backup_<ts>_<short>.db`).
    pub file_name: String,
    /// Absolute snapshot path at creation time.
    pub file_path: String,
    /// Snapshot size in bytes.
    pub file_size_bytes: u64,
    /// SHA-256 hex over the exact snapshot bytes.
    pub sha256_checksum: String,
    /// What triggered the backup.
    pub trigger_type: BackupTriggerType,
    /// Terminal lifecycle state.
    pub status: BackupStatus,
    /// Embedded migration count the snapshot was taken under.
    pub schema_version: u32,
    /// Failure detail, if any.
    pub error_message: Option<String>,
    /// Capture instant.
    pub created_at: DateTime<Utc>,
}

/// Ledger reconciled read-only against its cached balances (TASK-BRK-054).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerScope {
    /// `customers.current_balance_centimes` vs `customer_ledger`.
    Customer,
    /// `suppliers.current_balance_centimes` vs `supplier_ledger`.
    Supplier,
    /// `stock_levels` vs `stock_movements`.
    Stock,
}

/// Non-blocking restore advisory surfaced in the precheck report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreWarning {
    /// The backup holds a cash session that was never closed.
    OpenCashSessionInBackup {
        /// Session opening instant (RFC3339 UTC).
        opened_at: String,
    },
    /// The backup predates the running binary; migrations run on activate.
    OlderSchemaWillMigrate {
        /// Migration count recorded inside the backup.
        from_version: u32,
        /// Migration count of the running binary.
        to_version: u32,
    },
    /// Live documents newer than the backup survive only in quarantine.
    NewerActivityWillBeQuarantined {
        /// Live documents with `created_at` after the backup instant.
        document_count: u64,
    },
}

/// The eight read-only verification gates of the restore protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreChecks {
    /// Backup file exists on disk and is non-empty.
    pub file_exists: bool,
    /// Streaming SHA-256 matches the `backups_log` binding.
    pub sha256_matches: bool,
    /// `PRAGMA integrity_check` returns exactly `"ok"`.
    pub integrity_ok: bool,
    /// `PRAGMA foreign_key_check` returns zero rows.
    pub foreign_keys_ok: bool,
    /// The full audit hash chain recomputes cleanly.
    pub audit_chain_ok: bool,
    /// Every customer balance equals its signed ledger sum.
    pub customer_ledger_ok: bool,
    /// Every supplier balance equals its signed ledger sum.
    pub supplier_ledger_ok: bool,
    /// Every stock level equals its signed movement sum.
    pub stock_levels_ok: bool,
}

/// Human-scale content of the backup under review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupContentSummary {
    /// Stored product rows.
    pub products: u64,
    /// Stored customer rows.
    pub customers: u64,
    /// Stored document rows.
    pub documents_total: u64,
    /// Newest document instant, if any (RFC3339 UTC).
    pub last_document_at: Option<String>,
}

/// Why a backup must not be activated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreIneligibleReason {
    /// Snapshot file missing or zero-length.
    BackupFileMissing,
    /// File bytes drifted from the recorded SHA-256.
    HashMismatch,
    /// Physical page structure is damaged.
    IntegrityCheckFailed,
    /// Dangling references exist.
    ForeignKeyViolation,
    /// The audit hash chain does not recompute.
    AuditChainBroken,
    /// Customer balances diverge from their ledgers.
    CustomerLedgerMismatch {
        /// Violating customer rows.
        mismatch_count: u64,
    },
    /// Supplier balances diverge from their ledgers.
    SupplierLedgerMismatch {
        /// Violating supplier rows.
        mismatch_count: u64,
    },
    /// Stock levels diverge from their movements.
    StockLevelMismatch {
        /// Violating `(warehouse, product)` pairs.
        mismatch_count: u64,
    },
    /// The backup was written by a newer binary; downgrade is forbidden.
    SchemaNewerThanApp {
        /// Migration count recorded inside the backup.
        backup_version: u32,
        /// Migration count of the running binary.
        app_version: u32,
    },
}

/// Full read-only precheck report (Phase P of the restore protocol).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreReport {
    /// Logged backup id (snapshot file name).
    pub backup_id: String,
    /// Snapshot file name.
    pub file_name: String,
    /// Snapshot size in bytes (0 when the file is absent).
    pub file_size_bytes: u64,
    /// Capture instant from `backups_log` (RFC3339 UTC).
    pub created_at: String,
    /// Migration count recorded inside the backup.
    pub schema_version_backup: u32,
    /// Migration count of the running binary.
    pub schema_version_app: u32,
    /// The eight verification gates.
    pub checks: PreRestoreChecks,
    /// Human-scale content of the backup.
    pub content_summary: BackupContentSummary,
    /// Non-blocking advisories.
    pub warnings: Vec<RestoreWarning>,
    /// True only when every check passes and no reason is recorded.
    pub eligible: bool,
    /// Every failed gate, in protocol order.
    pub ineligible_reasons: Vec<RestoreIneligibleReason>,
}

/// Outcome of a successful verified restore (Phase R).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreOutcome {
    /// Logged backup id that was activated.
    pub restored_backup_id: String,
    /// Quarantine base file name of the previous active database.
    pub quarantined_to: String,
    /// Activation instant (RFC3339 UTC).
    pub restored_at: String,
    /// Always true: the operator must restart before resuming work.
    pub restart_required: bool,
}

/// Verified factory-reset outcome (TASK-BRK-091, AUD-043 §3).
///
/// The reset builds a fresh migrated database file (first-run state),
/// carries the `backups_log` history across, appends a chain-linked
/// `RESET_PERFORMED` audit event as the first row, quarantines the
/// previous live database, and activates the fresh file. Wipe-by-DELETE
/// is impossible: `BEFORE DELETE` abort-triggers guard every ledger and
/// log table (0001/0004/0008/0009), so no schema change was needed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryResetOutcome {
    /// Snapshot file name of the mandatory pre-reset verified backup.
    pub reset_backup_file_name: String,
    /// Quarantine base file name of the previous live database.
    pub quarantined_db_name: String,
    /// Activation instant (RFC3339 UTC).
    pub reset_at: String,
    /// Always true: the operator must restart before resuming work.
    pub restart_required: bool,
}

/// Typed failure of the restore protocol (TASK-BRK-054, ADR-062).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreError {
    /// Caller lacks `SystemSettingsManage`.
    Unauthorized,
    /// No `backups_log` row for the id.
    BackupNotFound,
    /// Snapshot file missing or zero-length.
    BackupFileMissing,
    /// SHA-256 mismatch (tamper or bit-rot).
    BackupTampered,
    /// Not a valid SQLite database, or `integrity_check` failed.
    BackupCorrupted,
    /// `foreign_key_check` returned rows.
    BackupFkViolation,
    /// Backup schema is newer than the running binary.
    BackupSchemaNewer {
        /// Migration count recorded inside the backup.
        backup_version: u32,
        /// Migration count of the running binary.
        app_version: u32,
    },
    /// The audit hash chain does not recompute.
    BackupAuditChainBroken,
    /// Cached balances diverge from their ledger.
    BackupLedgerInconsistent {
        /// Reconciled ledger.
        scope: LedgerScope,
        /// Violating rows.
        mismatch_count: u64,
    },
    /// Another restore holds the process-wide guard.
    RestoreInProgress,
    /// A post-quarantine step failed; the previous database was moved back.
    SwapFailedAndRolledBack {
        /// Operator-safe failure detail (never crosses the IPC boundary).
        detail: String,
    },
    /// Local I/O or lock failure before any byte moved.
    Io(String),
}
