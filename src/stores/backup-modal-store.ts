/**
 * Backup cockpit store (TASK-BRK-053).
 *
 * Backup list retrieval, manual snapshot creation, and on-disk
 * verification through the Admin-gated backup commands. File sizes stay
 * numeric (bytes); human formatting lives in the modal. The backend
 * re-validates everything (session, role, integrity); client-side guards
 * are UX-only.
 */

import { create } from "zustand";
import {
  createBackup,
  listBackups,
  precheckRestoreBackup,
  restoreBackup,
  verifyBackup,
  type BackupMetadata,
  type PreRestoreReport,
  type RestoreIneligibleReason,
  type RestoreOutcome,
  type RestoreWarning,
  type VerifyBackupResponse,
} from "@/lib/ipc";
import { t, type I18nKey } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";

/** Human-readable byte count (`850 KB`, `2.45 MB`, …). */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) {
    return "…";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0] ?? "KB";
  for (const next of units.slice(1)) {
    if (value < 1024) {
      break;
    }
    value /= 1024;
    unit = next;
  }
  return `${value.toFixed(value < 10 ? 2 : 1)} ${unit}`;
}

/** Restore sub-state machine phase (TASK-BRK-054, spec §4). */
export type RestorePhase =
  | "idle"
  | "precheck"
  | "report"
  | "restoring"
  | "success"
  | "restore-error";

/**
 * Whether the restore action may be rendered for this role string.
 * The role is the only client-side permission signal; the Rust gate on
 * `SystemSettingsManage` (ADMIN-only per the role matrix) stays
 * authoritative. UI gating is defense-in-depth.
 */
export function canRestoreForRole(role: string | null | undefined): boolean {
  return role === "ADMIN";
}

/** Exact, case-sensitive, locale-independent confirmation gate. */
export function isRestoreConfirmationValid(input: string): boolean {
  return input.trim() === "RESTORE";
}

/** Esc / X dismissal is allowed only outside the locked phases. */
export function isRestorePhaseDismissable(phase: RestorePhase): boolean {
  return phase === "idle" || phase === "report" || phase === "restore-error";
}

/** Destructive CTA enablement: eligible report plus exact confirmation. */
export function isRestoreExecutable(
  report: PreRestoreReport | null,
  confirmationInput: string,
): boolean {
  return report !== null && report.eligible && isRestoreConfirmationValid(confirmationInput);
}

const RESTORE_ERROR_I18N_KEYS: Record<string, I18nKey> = {
  UNAUTHORIZED: "backups.restore.error.UNAUTHORIZED",
  BACKUP_NOT_FOUND: "backups.restore.error.BACKUP_NOT_FOUND",
  BACKUP_MISSING: "backups.restore.error.BACKUP_FILE_MISSING",
  BACKUP_FILE_MISSING: "backups.restore.error.BACKUP_FILE_MISSING",
  BACKUP_CHECKSUM_MISMATCH: "backups.restore.error.BACKUP_TAMPERED",
  BACKUP_TAMPERED: "backups.restore.error.BACKUP_TAMPERED",
  BACKUP_CORRUPTED: "backups.restore.error.BACKUP_CORRUPTED",
  BACKUP_FK_VIOLATION: "backups.restore.error.BACKUP_FK_VIOLATION",
  BACKUP_SCHEMA_NEWER: "backups.restore.error.BACKUP_SCHEMA_NEWER",
  BACKUP_AUDIT_CHAIN_BROKEN: "backups.restore.error.BACKUP_AUDIT_CHAIN_BROKEN",
  BACKUP_LEDGER_INCONSISTENT: "backups.restore.error.BACKUP_LEDGER_INCONSISTENT",
  RESTORE_IN_PROGRESS: "backups.restore.error.RESTORE_IN_PROGRESS",
  RESTORE_SWAP_FAILED: "backups.restore.error.RESTORE_SWAP_FAILED",
  IO: "backups.restore.error.IO",
};

/**
 * i18n key for a restore wire error code. The swap-failure detail string
 * is never rendered (generic localized message); unknown codes fall back
 * to a generic message so raw internals never reach the user.
 */
export function restoreErrorI18nKey(code: string): I18nKey {
  return RESTORE_ERROR_I18N_KEYS[code] ?? "backups.restore.error.UNKNOWN";
}

/** Localized one-line description of an ineligibility reason. */
export function describeIneligibleReason(
  reason: RestoreIneligibleReason,
  locale: Locale,
): string {
  switch (reason.type) {
    case "backup_file_missing":
      return t(locale, "backups.restore.ineligible.backup_file_missing");
    case "hash_mismatch":
      return t(locale, "backups.restore.ineligible.hash_mismatch");
    case "integrity_check_failed":
      return t(locale, "backups.restore.ineligible.integrity_check_failed");
    case "foreign_key_violation":
      return t(locale, "backups.restore.ineligible.foreign_key_violation");
    case "audit_chain_broken":
      return t(locale, "backups.restore.ineligible.audit_chain_broken");
    case "customer_ledger_mismatch":
      return t(locale, "backups.restore.ineligible.customer_ledger_mismatch").replace(
        "{count}",
        String(reason.mismatchCount),
      );
    case "supplier_ledger_mismatch":
      return t(locale, "backups.restore.ineligible.supplier_ledger_mismatch").replace(
        "{count}",
        String(reason.mismatchCount),
      );
    case "stock_level_mismatch":
      return t(locale, "backups.restore.ineligible.stock_level_mismatch").replace(
        "{count}",
        String(reason.mismatchCount),
      );
    case "schema_newer_than_app":
      return t(locale, "backups.restore.ineligible.schema_newer_than_app")
        .replace("{backup}", String(reason.backupVersion))
        .replace("{app}", String(reason.appVersion));
  }
}

/** Localized one-line description of a restore warning. */
export function describeRestoreWarning(warning: RestoreWarning, locale: Locale): string {
  switch (warning.type) {
    case "open_cash_session_in_backup":
      return t(locale, "backups.restore.warning.openCashSession");
    case "older_schema_will_migrate":
      return t(locale, "backups.restore.warning.olderSchema");
    case "newer_activity_will_be_quarantined":
      return t(locale, "backups.restore.warning.newerActivity").replace(
        "{count}",
        String(warning.documentCount),
      );
  }
}

interface BackupModalStoreState {
  isOpen: boolean;
  isLoading: boolean;
  isCreating: boolean;
  verifyingId: string | null;
  backups: BackupMetadata[];
  verificationResults: Record<string, VerifyBackupResponse>;
  errorMessage: string | null;
  successMessage: string | null;
  restorePhase: RestorePhase;
  restoreReport: PreRestoreReport | null;
  confirmationInput: string;
  restoreOutcome: RestoreOutcome | null;
  restoreError: string | null;
  openModal: () => Promise<void>;
  closeModal: () => void;
  fetchBackups: () => Promise<void>;
  triggerManualBackup: () => Promise<boolean>;
  verifyBackupIntegrity: (backupId: string) => Promise<boolean>;
  startPrecheck: (backupId: string) => Promise<boolean>;
  resetToIdle: () => void;
  setConfirmationInput: (value: string) => void;
  executeRestore: (backupId: string) => Promise<boolean>;
  reset: () => void;
}

const initialState = {
  isOpen: false,
  isLoading: false,
  isCreating: false,
  verifyingId: null as string | null,
  backups: [] as BackupMetadata[],
  verificationResults: {} as Record<string, VerifyBackupResponse>,
  errorMessage: null as string | null,
  successMessage: null as string | null,
  restorePhase: "idle" as RestorePhase,
  restoreReport: null as PreRestoreReport | null,
  confirmationInput: "",
  restoreOutcome: null as RestoreOutcome | null,
  restoreError: null as string | null,
};

export const useBackupModalStore = create<BackupModalStoreState>()((set, get) => ({
  ...initialState,

  openModal: async () => {
    set({ ...initialState, isOpen: true });
    await get().fetchBackups();
  },

  closeModal: () => {
    if (get().isCreating) {
      return;
    }
    if (!isRestorePhaseDismissable(get().restorePhase)) {
      return;
    }
    set({ ...initialState });
  },

  fetchBackups: async () => {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      set({ errorMessage: "IPC_NO_ACTIVE_SESSION", backups: [] });
      return;
    }
    set({ isLoading: true, errorMessage: null });
    try {
      const backups = await listBackups(token);
      set({ backups });
    } catch (failure: unknown) {
      set({
        errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR",
        backups: [],
      });
    } finally {
      set({ isLoading: false });
    }
  },

  triggerManualBackup: async () => {
    const state = get();
    if (state.isCreating || state.isLoading) {
      return false;
    }
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
      return false;
    }
    // Synchronous lock before the first await (TASK-BRK-032 precedent).
    set({ isCreating: true, errorMessage: null, successMessage: null });
    try {
      const created = await createBackup(token, "MANUAL");
      set((current) => ({
        backups: [created, ...current.backups],
        successMessage: created.fileName,
      }));
      return true;
    } catch (failure: unknown) {
      set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
      return false;
    } finally {
      set({ isCreating: false });
    }
  },

  verifyBackupIntegrity: async (backupId) => {
    const state = get();
    if (state.verifyingId !== null) {
      return false;
    }
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
      return false;
    }
    set({ verifyingId: backupId, errorMessage: null });
    try {
      const result = await verifyBackup(token, backupId);
      set((current) => ({
        verificationResults: { ...current.verificationResults, [backupId]: result },
      }));
      return result.isValid;
    } catch (failure: unknown) {
      set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
      return false;
    } finally {
      set({ verifyingId: null });
    }
  },

  reset: () => {
    set({ ...initialState });
  },

  startPrecheck: async (backupId) => {
    const state = get();
    if (state.restorePhase === "precheck" || state.restorePhase === "restoring") {
      return false;
    }
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      set({ restorePhase: "restore-error", restoreError: "IPC_NO_ACTIVE_SESSION" });
      return false;
    }
    set({
      restorePhase: "precheck",
      restoreReport: null,
      confirmationInput: "",
      restoreOutcome: null,
      restoreError: null,
    });
    try {
      const report = await precheckRestoreBackup(token, backupId);
      set({ restorePhase: "report", restoreReport: report });
      return report.eligible;
    } catch (failure: unknown) {
      set({
        restorePhase: "restore-error",
        restoreError: typeof failure === "string" ? failure : "INTERNAL_ERROR",
      });
      return false;
    }
  },

  resetToIdle: () => {
    set({
      restorePhase: "idle",
      restoreReport: null,
      confirmationInput: "",
      restoreOutcome: null,
      restoreError: null,
    });
  },

  setConfirmationInput: (value) => {
    set({ confirmationInput: value });
  },

  executeRestore: async (backupId) => {
    const state = get();
    if (state.restorePhase !== "report") {
      return false;
    }
    if (!isRestoreExecutable(state.restoreReport, state.confirmationInput)) {
      return false;
    }
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      set({ restorePhase: "restore-error", restoreError: "IPC_NO_ACTIVE_SESSION" });
      return false;
    }
    set({ restorePhase: "restoring", restoreOutcome: null, restoreError: null });
    try {
      const outcome = await restoreBackup(token, backupId);
      set({ restorePhase: "success", restoreOutcome: outcome });
      return true;
    } catch (failure: unknown) {
      set({
        restorePhase: "restore-error",
        restoreError: typeof failure === "string" ? failure : "INTERNAL_ERROR",
      });
      return false;
    }
  },
}));
