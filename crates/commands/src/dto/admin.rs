//! Administration DTOs: backups, restore, factory reset, users, import.
use serde::{Deserialize, Serialize};

/// Backup creation request (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateBackupRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Trigger selector (`MANUAL`, `SHIFT_CLOSE`, `SCHEDULED`; absent or
    /// blank defaults to `MANUAL`).
    pub trigger_type: Option<String>,
}

/// Backup metadata view (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupMetadataDto {
    /// Backup id (UUID string).
    pub id: String,
    /// Snapshot file name.
    pub file_name: String,
    /// Snapshot size in bytes (`None` = legacy row; UI shows "—").
    pub file_size_bytes: Option<u64>,
    /// SHA-256 hex over the exact snapshot bytes.
    pub sha256_checksum: String,
    /// Trigger code (`MANUAL`, `SHIFT_CLOSE`, `SCHEDULED`).
    pub trigger_type: String,
    /// Lifecycle state (`SUCCESS`, `FAILED`, `CORRUPTED`, `PRUNED`).
    pub status: String,
    /// Embedded migration count at capture.
    pub schema_version: u32,
    /// Failure detail, if any.
    pub error_message: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
}

/// Automatic-backup schedule view (TASK-BRK-092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupScheduleDto {
    /// Cadence code (`disabled`, `daily`, `weekly`).
    pub cadence: String,
    /// Local wall-clock `HH:MM`, or `None` when disabled/absent.
    pub time: Option<String>,
}

/// Automatic-backup due-check report (TASK-BRK-092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledBackupReportDto {
    /// Cadence code.
    pub cadence: String,
    /// Local wall-clock `HH:MM`, or `None`.
    pub time: Option<String>,
    /// A backup is past due and none succeeded inside the window.
    pub due: bool,
    /// Latest SUCCESS backup instant (RFC3339 UTC), if any.
    pub last_backup_at: Option<String>,
    /// Next local run instant (RFC3339 UTC), if any.
    pub next_run_at: Option<String>,
}

/// Durable last-backup status (TASK-BRK-092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastBackupStatusDto {
    /// `SUCCESS`, `FAILED`, `CORRUPTED`, `PRUNED`, or empty.
    pub status: String,
    /// Newest logged snapshot file name, if any.
    pub file_name: Option<String>,
    /// Newest logged capture instant (RFC3339 UTC), if any.
    pub created_at: Option<String>,
    /// Failure detail, if any.
    pub error_message: Option<String>,
}

/// USB copy outcome view (TASK-BRK-092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsbCopyOutcomeDto {
    /// Directory the copy landed in.
    pub destination: String,
    /// Snapshot file name on the removable medium.
    pub file_name: String,
    /// Copied byte count.
    pub file_size_bytes: u64,
    /// Copy verified by size + SHA-256 re-check.
    pub verified: bool,
}

/// Combined automatic-backup status read (TASK-BRK-092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupHardeningStatusDto {
    /// Stored cadence + time.
    pub schedule: BackupScheduleDto,
    /// Removable destination directory, if set.
    pub usb_destination: Option<String>,
    /// Durable last-backup state (success or failure).
    pub last_status: LastBackupStatusDto,
    /// Due-check report.
    pub due: ScheduledBackupReportDto,
}

/// `backup_hardening_configure` request (TASK-BRK-092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupHardeningConfigureRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Cadence code (`disabled`, `daily`, `weekly`).
    pub cadence: String,
    /// Local wall-clock `HH:MM` (required unless disabled).
    pub time: Option<String>,
    /// Removable USB destination directory (blank clears it).
    pub usb_destination: Option<String>,
}

/// Scheduled-backup run outcome (TASK-BRK-092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledRunOutcomeDto {
    /// Verified local snapshot metadata.
    pub backup: BackupMetadataDto,
    /// USB mirror outcome, or `None` when unset/failed.
    pub usb: Option<UsbCopyOutcomeDto>,
}

/// Backup verification request (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyBackupRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Backup id (file name as logged).
    pub backup_id: String,
}

/// Backup verification outcome (TASK-BRK-052).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyBackupResponseDto {
    /// Backup id as requested.
    pub backup_id: String,
    /// True only when the file exists, matches its hash, and passes pragmas.
    pub is_valid: bool,
    /// Live SHA-256 hex over the file bytes.
    pub sha256_checksum: String,
    /// Lifecycle state after verification.
    pub status: String,
    /// Human detail, if any.
    pub message: Option<String>,
}

/// Restore precheck request (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Backup id (file name as logged).
    pub backup_id: String,
}

/// Destructive restore request (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreBackupRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Backup id (file name as logged).
    pub backup_id: String,
}

/// Non-blocking restore advisory view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RestoreWarningDto {
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

/// The eight verification gates view (TASK-BRK-054).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreChecksDto {
    /// Backup file exists on disk and is non-empty.
    pub file_exists: bool,
    /// Streaming SHA-256 matches the recorded binding.
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

/// Backup content summary view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupContentSummaryDto {
    /// Stored product rows.
    pub products: u64,
    /// Stored customer rows.
    pub customers: u64,
    /// Stored document rows.
    pub documents_total: u64,
    /// Newest document instant, if any (RFC3339 UTC).
    pub last_document_at: Option<String>,
}

/// Restore ineligibility reason view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RestoreIneligibleReasonDto {
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

/// Full read-only precheck report view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRestoreReportDto {
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
    pub checks: PreRestoreChecksDto,
    /// Human-scale content of the backup.
    pub content_summary: BackupContentSummaryDto,
    /// Non-blocking advisories.
    pub warnings: Vec<RestoreWarningDto>,
    /// True only when every check passes and no reason is recorded.
    pub eligible: bool,
    /// Every failed gate, in protocol order.
    pub ineligible_reasons: Vec<RestoreIneligibleReasonDto>,
}

/// Verified restore outcome view (TASK-BRK-054).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreOutcomeDto {
    /// Logged backup id that was activated.
    pub restored_backup_id: String,
    /// Quarantine base file name of the previous active database.
    pub quarantined_to: String,
    /// Activation instant (RFC3339 UTC).
    pub restored_at: String,
    /// Always true: the operator must restart before resuming work.
    pub restart_required: bool,
}

/// Factory-reset precheck request (TASK-BRK-091).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryResetPrecheckRequestDto {
    /// Authorizing session token.
    pub session_token: String,
}

/// One backup row for the reset precheck view (subset of the log row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetBackupRefDto {
    /// Snapshot file name.
    pub file_name: String,
    /// Capture instant (RFC3339 UTC).
    pub created_at: String,
    /// Lifecycle state (`SUCCESS`, `FAILED`, `CORRUPTED`, `PRUNED`).
    pub status: String,
}

/// Read-only factory-reset precheck view (TASK-BRK-091).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetPrecheckDto {
    /// An OPEN or CLOSING cash session exists for this terminal.
    pub open_session: bool,
    /// The backup directory accepted a write probe.
    pub backup_dir_writable: bool,
    /// Newest logged backup, if any.
    pub newest_backup: Option<ResetBackupRefDto>,
    /// True only when every gate passes.
    pub eligible: bool,
    /// Failed gates in check order (codes only; UI translates).
    pub reasons: Vec<String>,
}

/// Destructive factory-reset request (TASK-BRK-091).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryResetExecuteRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Retyped confirmation token (store name, or `DEP-01` when blank).
    pub typed_token: String,
}

/// Verified factory-reset outcome view (TASK-BRK-091).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactoryResetOutcomeDto {
    /// Snapshot file name of the mandatory pre-reset verified backup.
    pub reset_backup_file_name: String,
    /// Quarantine base file name of the previous live database.
    pub quarantined_to: String,
    /// Activation instant (RFC3339 UTC).
    pub reset_at: String,
    /// Always true: the operator must restart before resuming work.
    pub restart_required: bool,
}

/// Managed operator view (mirrors application `UserRecord`; never carries hashes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummaryDto {
    /// Operator id (UUID string).
    pub id: String,
    /// Login name.
    pub username: String,
    /// Role code (`VENDEUR`, `CAISSIER`, `GERANT`, `ADMIN`).
    pub role: String,
    /// Active flag.
    pub is_active: bool,
    /// True when a PIN hash slot is configured.
    pub has_pin: bool,
    /// Capture instant (RFC3339 millis).
    pub created_at: String,
    /// Last mutation instant (RFC3339 millis).
    pub updated_at: String,
}

/// `users_create` request (plaintext secrets; hashed inside the use case).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateUserRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Login name (3..=30, alphanumeric + underscores).
    pub username: String,
    /// Plaintext password (min 6 chars).
    pub password: String,
    /// Optional plaintext PIN (4..=6 digits).
    pub pin: Option<String>,
    /// Role code.
    pub role: String,
}

/// `users_update` request (role/active-flag patch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateUserRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target operator id (UUID string).
    pub user_id: String,
    /// New role code, or `None` to keep the current one.
    pub role: Option<String>,
    /// New active flag, or `None` to keep the current one.
    pub is_active: Option<bool>,
}

/// `users_reset_credentials` request (plaintext replacements).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetUserCredentialsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target operator id (UUID string).
    pub user_id: String,
    /// Replacement plaintext password, or `None` to keep the current one.
    pub password: Option<String>,
    /// Replacement plaintext PIN, or `None` to keep the current one.
    pub pin: Option<String>,
}

/// One batch-import row (`catalog_import_products`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProductRowDto {
    /// Optional SKU; trimmed + uppercased, generated when absent.
    pub sku: Option<String>,
    /// Optional base-unit barcode; cleared when already assigned.
    pub barcode: Option<String>,
    /// Required French designation.
    pub name_fr: String,
    /// Arabic designation; falls back to `name_fr`.
    pub name_ar: Option<String>,
    /// Owning category UUID string; detached when unknown.
    pub category_id: Option<String>,
    /// Base-unit French name (`Pièce` default).
    pub base_unit_name_fr: Option<String>,
    /// Base-unit Arabic name (`قطعة` default).
    pub base_unit_name_ar: Option<String>,
    /// Purchase centimes; zero when absent.
    pub purchase_price_centimes: Option<i64>,
    /// Sale centimes; must be non-negative.
    pub sale_price_centimes: i64,
    /// Opening quantity in base micros; provisioned when positive.
    pub initial_quantity_micro: Option<i64>,
}

/// `catalog_import_products` request: atomic batch ingestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProductsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Rows to ingest (clamped to 1,000 per call).
    pub rows: Vec<ImportProductRowDto>,
    /// Warehouse code or id (default `DEP-01`, else first active depot).
    pub warehouse_id: Option<String>,
    /// `skip` (default) or `error` on duplicate SKU.
    pub on_duplicate_sku: Option<String>,
}

/// Per-row failure entry of the import execution report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRowErrorDto {
    /// Zero-based input row index.
    pub row_index: usize,
    /// Resolved (or supplied) SKU when known.
    pub sku: Option<String>,
    /// Row designation for operator display.
    pub name_fr: String,
    /// Stable machine-readable reason.
    pub error: String,
}

/// `catalog_import_products` response: structured execution report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProductsResponseDto {
    /// Rows ingested (after clamping).
    pub total_rows: usize,
    /// Rows fully created with base unit (and stock when requested).
    pub imported_count: usize,
    /// Rows overwritten in place (`update` policy).
    pub updated_count: usize,
    /// Rows skipped on duplicate SKU (`skip` policy).
    pub skipped_count: usize,
    /// Per-row validation and skip entries, in row order.
    pub errors: Vec<ImportRowErrorDto>,
    /// Wall-clock execution time in milliseconds.
    pub duration_ms: u64,
}

/// Dual-track deletion request (session + target id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteEntityRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target entity id (UUID string).
    pub id: String,
    /// Operator reason (mandatory, min 3 chars, on irreversible purges;
    /// recorded best-effort on deactivations).
    pub reason: String,
}

/// Dual-track deletion outcome (mirrors `DeleteEntityOutcome`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteEntityResultDto {
    /// Deleted or deactivated entity id.
    pub id: String,
    /// `"hard_deleted"` or `"deactivated"`.
    pub mode: String,
    /// Stable wire message (French copy resolves in the frontend).
    pub message: String,
}

/// `warehouses_update` request: sparse rename and/or (de)activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateWarehouseRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target warehouse id (UUID string).
    pub id: String,
    /// Replacement display name.
    pub name: Option<String>,
    /// Replacement active flag.
    pub is_active: Option<bool>,
}

/// Fiscal stamp settings view (mirrors the `fiscal.*` settings keys).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiscalSettingsDto {
    /// Algerian fiscal stamp toggle.
    pub timbre_enabled: bool,
}

/// `fiscal_settings_update` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateFiscalSettingsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Algerian fiscal stamp toggle.
    pub timbre_enabled: bool,
}
