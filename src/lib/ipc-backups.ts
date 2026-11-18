/**
 * Backups bridge: snapshots, verification, restore precheck, factory reset.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import { call } from "./ipc-kernel";

/** Backup trigger selector (mirrors the Rust trigger codes). */
export type BackupTriggerType = "MANUAL" | "SHIFT_CLOSE" | "SCHEDULED";

/** Backup metadata view (mirrors BackupMetadataDto). */
export interface BackupMetadata {
  id: string;
  fileName: string;
  fileSizeBytes: number | null;
  sha256Checksum: string;
  triggerType: string;
  status: string;
  schemaVersion: number;
  errorMessage: string | null;
  createdAt: string;
}

interface BackupMetadataWire {
  id: string;
  file_name: string;
  file_size_bytes: number | null;
  sha256_checksum: string;
  trigger_type: string;
  status: string;
  schema_version: number;
  error_message: string | null;
  created_at: string;
}

function toBackupMetadata(wire: BackupMetadataWire): BackupMetadata {
  return {
    id: wire.id,
    fileName: wire.file_name,
    fileSizeBytes: wire.file_size_bytes,
    sha256Checksum: wire.sha256_checksum,
    triggerType: wire.trigger_type,
    status: wire.status,
    schemaVersion: wire.schema_version,
    errorMessage: wire.error_message,
    createdAt: wire.created_at,
  };
}

/**
 * Capture a verified online backup snapshot (Admin-only server-side).
 * Rejects with the backend wire code verbatim (UNAUTHORIZED,
 * BACKUP_VERIFICATION_FAILED, …).
 */
export async function createBackup(
  sessionToken: string,
  triggerType?: BackupTriggerType,
): Promise<BackupMetadata> {
  const wire = await call<BackupMetadataWire>("backups_create", {
    request: {
      session_token: sessionToken,
      trigger_type: triggerType ?? "MANUAL",
    },
  });
  return toBackupMetadata(wire);
}

/**
 * List recent backup audit rows, newest first (Admin-only server-side).
 */
export async function listBackups(sessionToken: string): Promise<BackupMetadata[]> {
  const wire = await call<BackupMetadataWire[]>("backups_list", {
    session_token: sessionToken,
  });
  return wire.map(toBackupMetadata);
}

/** Backup verification outcome (mirrors VerifyBackupResponseDto). */
export interface VerifyBackupResponse {
  backupId: string;
  isValid: boolean;
  sha256Checksum: string;
  status: string;
  message: string | null;
}

/**
 * Re-verify one logged snapshot on disk (Admin-only server-side).
 * Transport-level failures reject; content failures surface as
 * BACKUP_* wire codes (BACKUP_MISSING, BACKUP_CHECKSUM_MISMATCH,
 * BACKUP_CORRUPTED).
 */
export async function verifyBackup(
  sessionToken: string,
  backupId: string,
): Promise<VerifyBackupResponse> {
  interface VerifyBackupWire {
    backup_id: string;
    is_valid: boolean;
    sha256_checksum: string;
    status: string;
    message: string | null;
  }
  const wire = await call<VerifyBackupWire>("backups_verify", {
    request: { session_token: sessionToken, backup_id: backupId },
  });
  return {
    backupId: wire.backup_id,
    isValid: wire.is_valid,
    sha256Checksum: wire.sha256_checksum,
    status: wire.status,
    message: wire.message,
  };
}

/** One of the eight restore verification gates (mirrors PreRestoreChecksDto). */
export interface PreRestoreChecks {
  fileExists: boolean;
  sha256Matches: boolean;
  integrityOk: boolean;
  foreignKeysOk: boolean;
  auditChainOk: boolean;
  customerLedgerOk: boolean;
  supplierLedgerOk: boolean;
  stockLevelsOk: boolean;
}

/** Non-blocking restore advisory (mirrors RestoreWarningDto). */
export type RestoreWarning =
  | { type: "open_cash_session_in_backup"; openedAt: string }
  | { type: "older_schema_will_migrate"; fromVersion: number; toVersion: number }
  | { type: "newer_activity_will_be_quarantined"; documentCount: number };

/** Restore ineligibility reason (mirrors RestoreIneligibleReasonDto). */
export type RestoreIneligibleReason =
  | { type: "backup_file_missing" }
  | { type: "hash_mismatch" }
  | { type: "integrity_check_failed" }
  | { type: "foreign_key_violation" }
  | { type: "audit_chain_broken" }
  | { type: "customer_ledger_mismatch"; mismatchCount: number }
  | { type: "supplier_ledger_mismatch"; mismatchCount: number }
  | { type: "stock_level_mismatch"; mismatchCount: number }
  | { type: "schema_newer_than_app"; backupVersion: number; appVersion: number };

/** Backup content summary (mirrors BackupContentSummaryDto). */
export interface BackupContentSummary {
  products: number;
  customers: number;
  documentsTotal: number;
  lastDocumentAt: string | null;
}

/** Full read-only precheck report (mirrors PreRestoreReportDto). */
export interface PreRestoreReport {
  backupId: string;
  fileName: string;
  fileSizeBytes: number;
  createdAt: string;
  schemaVersionBackup: number;
  schemaVersionApp: number;
  checks: PreRestoreChecks;
  contentSummary: BackupContentSummary;
  warnings: RestoreWarning[];
  eligible: boolean;
  ineligibleReasons: RestoreIneligibleReason[];
}

/** Verified restore outcome (mirrors RestoreOutcomeDto). */
export interface RestoreOutcome {
  restoredBackupId: string;
  quarantinedTo: string;
  restoredAt: string;
  restartRequired: boolean;
}

type RestoreWarningWire =
  | { type: "open_cash_session_in_backup"; opened_at: string }
  | { type: "older_schema_will_migrate"; from_version: number; to_version: number }
  | { type: "newer_activity_will_be_quarantined"; document_count: number };

type RestoreIneligibleReasonWire =
  | { type: "backup_file_missing" }
  | { type: "hash_mismatch" }
  | { type: "integrity_check_failed" }
  | { type: "foreign_key_violation" }
  | { type: "audit_chain_broken" }
  | { type: "customer_ledger_mismatch"; mismatch_count: number }
  | { type: "supplier_ledger_mismatch"; mismatch_count: number }
  | { type: "stock_level_mismatch"; mismatch_count: number }
  | { type: "schema_newer_than_app"; backup_version: number; app_version: number };

interface PreRestoreReportWire {
  backup_id: string;
  file_name: string;
  file_size_bytes: number;
  created_at: string;
  schema_version_backup: number;
  schema_version_app: number;
  checks: {
    file_exists: boolean;
    sha256_matches: boolean;
    integrity_ok: boolean;
    foreign_keys_ok: boolean;
    audit_chain_ok: boolean;
    customer_ledger_ok: boolean;
    supplier_ledger_ok: boolean;
    stock_levels_ok: boolean;
  };
  content_summary: {
    products: number;
    customers: number;
    documents_total: number;
    last_document_at: string | null;
  };
  warnings: RestoreWarningWire[];
  eligible: boolean;
  ineligible_reasons: RestoreIneligibleReasonWire[];
}

function toRestoreWarning(wire: RestoreWarningWire): RestoreWarning {
  switch (wire.type) {
    case "open_cash_session_in_backup":
      return { type: wire.type, openedAt: wire.opened_at };
    case "older_schema_will_migrate":
      return { type: wire.type, fromVersion: wire.from_version, toVersion: wire.to_version };
    case "newer_activity_will_be_quarantined":
      return { type: wire.type, documentCount: wire.document_count };
  }
}

function toRestoreIneligibleReason(wire: RestoreIneligibleReasonWire): RestoreIneligibleReason {
  switch (wire.type) {
    case "backup_file_missing":
    case "hash_mismatch":
    case "integrity_check_failed":
    case "foreign_key_violation":
    case "audit_chain_broken":
      return { type: wire.type };
    case "customer_ledger_mismatch":
    case "supplier_ledger_mismatch":
    case "stock_level_mismatch":
      return { type: wire.type, mismatchCount: wire.mismatch_count };
    case "schema_newer_than_app":
      return { type: wire.type, backupVersion: wire.backup_version, appVersion: wire.app_version };
  }
}

function toPreRestoreReport(wire: PreRestoreReportWire): PreRestoreReport {
  return {
    backupId: wire.backup_id,
    fileName: wire.file_name,
    fileSizeBytes: wire.file_size_bytes,
    createdAt: wire.created_at,
    schemaVersionBackup: wire.schema_version_backup,
    schemaVersionApp: wire.schema_version_app,
    checks: {
      fileExists: wire.checks.file_exists,
      sha256Matches: wire.checks.sha256_matches,
      integrityOk: wire.checks.integrity_ok,
      foreignKeysOk: wire.checks.foreign_keys_ok,
      auditChainOk: wire.checks.audit_chain_ok,
      customerLedgerOk: wire.checks.customer_ledger_ok,
      supplierLedgerOk: wire.checks.supplier_ledger_ok,
      stockLevelsOk: wire.checks.stock_levels_ok,
    },
    contentSummary: {
      products: wire.content_summary.products,
      customers: wire.content_summary.customers,
      documentsTotal: wire.content_summary.documents_total,
      lastDocumentAt: wire.content_summary.last_document_at,
    },
    warnings: wire.warnings.map(toRestoreWarning),
    eligible: wire.eligible,
    ineligibleReasons: wire.ineligible_reasons.map(toRestoreIneligibleReason),
  };
}

/**
 * Full read-only verification report for one logged backup
 * (Admin-only server-side). Rejects with the backend wire code verbatim
 * (UNAUTHORIZED, BACKUP_NOT_FOUND, BACKUP_MISSING, …).
 */
export async function precheckRestoreBackup(
  sessionToken: string,
  backupId: string,
): Promise<PreRestoreReport> {
  const wire = await call<PreRestoreReportWire>("backups_restore_precheck", {
    request: { session_token: sessionToken, backup_id: backupId },
  });
  return toPreRestoreReport(wire);
}

/**
 * Verified destructive restore with quarantine and rollback safety
 * (Admin-only server-side). The backend re-verifies everything
 * internally; transport failures reject with the wire code verbatim
 * (BACKUP_CHECKSUM_MISMATCH, BACKUP_CORRUPTED, RESTORE_SWAP_FAILED, …).
 */
export async function restoreBackup(
  sessionToken: string,
  backupId: string,
): Promise<RestoreOutcome> {
  interface RestoreOutcomeWire {
    restored_backup_id: string;
    quarantined_to: string;
    restored_at: string;
    restart_required: boolean;
  }
  const wire = await call<RestoreOutcomeWire>("backups_restore", {
    request: { session_token: sessionToken, backup_id: backupId },
  });
  return {
    restoredBackupId: wire.restored_backup_id,
    quarantinedTo: wire.quarantined_to,
    restoredAt: wire.restored_at,
    restartRequired: wire.restart_required,
  };
}

/** One backup row of the factory-reset precheck view. */
export interface ResetBackupRef {
  fileName: string;
  createdAt: string;
  status: string;
}

/** Read-only factory-reset precheck view (mirrors ResetPrecheckDto). */
export interface FactoryResetPrecheck {
  openSession: boolean;
  backupDirWritable: boolean;
  newestBackup: ResetBackupRef | null;
  eligible: boolean;
  reasons: string[];
}

/** Verified factory-reset outcome view (mirrors FactoryResetOutcomeDto). */
export interface FactoryResetOutcome {
  resetBackupFileName: string;
  quarantinedTo: string;
  resetAt: string;
  restartRequired: boolean;
}

/**
 * Read-only factory-reset gates (TASK-DSN-094). Admin-enforced server-side;
 * rejects with the backend wire code verbatim (UNAUTHORIZED, …).
 */
export async function getFactoryResetPrecheck(
  sessionToken: string,
): Promise<FactoryResetPrecheck> {
  interface ResetPrecheckWire {
    open_session: boolean;
    backup_dir_writable: boolean;
    newest_backup: { file_name: string; created_at: string; status: string } | null;
    eligible: boolean;
    reasons: string[];
  }
  const wire = await call<ResetPrecheckWire>("factory_reset_precheck", {
    request: { session_token: sessionToken },
  });
  return {
    openSession: wire.open_session,
    backupDirWritable: wire.backup_dir_writable,
    newestBackup:
      wire.newest_backup === null
        ? null
        : {
            fileName: wire.newest_backup.file_name,
            createdAt: wire.newest_backup.created_at,
            status: wire.newest_backup.status,
          },
    eligible: wire.eligible,
    reasons: wire.reasons,
  };
}

/**
 * Destructive factory reset (TASK-DSN-094). Admin-enforced server-side with
 * validation before any write; rejects with the backend wire code verbatim
 * (RESET_OPEN_SESSION, RESET_TOKEN_MISMATCH, RESET_BACKUP_UNWRITABLE,
 * RESET_BACKUP_FAILED, RESET_SWAP_FAILED, …).
 */
export async function executeFactoryReset(
  sessionToken: string,
  typedToken: string,
): Promise<FactoryResetOutcome> {
  interface FactoryResetOutcomeWire {
    reset_backup_file_name: string;
    quarantined_to: string;
    reset_at: string;
    restart_required: boolean;
  }
  const wire = await call<FactoryResetOutcomeWire>("factory_reset_execute", {
    request: { session_token: sessionToken, typed_token: typedToken },
  });
  return {
    resetBackupFileName: wire.reset_backup_file_name,
    quarantinedTo: wire.quarantined_to,
    resetAt: wire.reset_at,
    restartRequired: wire.restart_required,
  };
}
