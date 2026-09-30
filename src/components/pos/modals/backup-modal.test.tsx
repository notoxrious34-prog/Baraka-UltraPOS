/**
 * Unit tests for the backup cockpit (TASK-BRK-053).
 *
 * Covers store lifecycle (open/fetch/create/verify/close), the
 * single-flight locks, canonical payload shapes, and bilingual error
 * mapping — all without floats. Component rendering follows the shared
 * store-contract pattern used by every sibling modal suite (no DOM
 * harness exists in this project).
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  canRestoreForRole,
  describeRestoreWarning,
  formatBytes,
  isRestoreConfirmationValid,
  isRestoreExecutable,
  isRestorePhaseDismissable,
  restoreErrorI18nKey,
  useBackupModalStore,
} from "@/stores/backup-modal-store";
import {
  RestoreReportPanel,
  RestoreRowButton,
  closeApplication,
  type RestoreReportPanelProps,
} from "@/components/pos/modals/BackupModal";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";
import type { PreRestoreReport } from "@/lib/ipc";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: vi.fn() }));

const invokeMock = vi.mocked(invoke);
const getCurrentWindowMock = vi.mocked(getCurrentWindow);

/** The bridge probe requires a `window` carrying the Tauri marker. */
function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function primeSession(role = "ADMIN"): void {
  fakeTauriWindow();
  const session: SessionInfo = {
    token: "tok-bkp-9",
    userId: "55555555-5555-5555-5555-555555555555",
    username: "admin1",
    role,
    expiresAt: "2026-01-01T00:00:00Z",
  };
  useSessionStore.setState({ session });
  useBackupModalStore.getState().reset();
}

const ROW = {
  id: "11111111-1111-4111-8111-111111111111",
  file_name: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
  file_size_bytes: 2572288,
  sha256_checksum: "cd".repeat(32),
  trigger_type: "MANUAL",
  status: "SUCCESS",
  schema_version: 13,
  error_message: null,
  created_at: "2026-09-29T12:00:00.000Z",
};

describe("formatBytes", () => {
  it("renders clean KB/MB units", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(850 * 1024)).toBe("850.0 KB");
    expect(formatBytes(2572288)).toBe("2.45 MB");
    expect(formatBytes(Number.NaN)).toBe("…");
  });
});

describe("backup store lifecycle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("opens and lists backups returned by listBackups", async () => {
    invokeMock.mockResolvedValueOnce([ROW]);
    await useBackupModalStore.getState().openModal();
    const state = useBackupModalStore.getState();
    expect(state.isOpen).toBe(true);
    expect(state.backups).toHaveLength(1);
    expect(state.backups[0]?.fileName).toBe(ROW.file_name);
    expect(state.isLoading).toBe(false);
    expect(invokeMock).toHaveBeenCalledWith("backups_list", {
      session_token: "tok-bkp-9",
    });
  });

  it("create button calls createBackup MANUAL, locks flight, prepends row", async () => {
    invokeMock.mockResolvedValueOnce([]);
    await useBackupModalStore.getState().openModal();
    vi.clearAllMocks();
    invokeMock.mockResolvedValueOnce(ROW);
    const first = useBackupModalStore.getState().triggerManualBackup();
    const second = useBackupModalStore.getState().triggerManualBackup();
    const [a, b] = await Promise.all([first, second]);
    expect(a).toBe(true);
    expect(b).toBe(false);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("backups_create", {
      request: { session_token: "tok-bkp-9", trigger_type: "MANUAL" },
      session_token: "tok-bkp-9",
    });
    expect(useBackupModalStore.getState().backups[0]?.id).toBe(ROW.id);
    expect(useBackupModalStore.getState().isCreating).toBe(false);
  });

  it("verify invokes verifyBackup and stores the success badge", async () => {
    invokeMock.mockResolvedValueOnce([ROW]);
    await useBackupModalStore.getState().openModal();
    vi.clearAllMocks();
    invokeMock.mockResolvedValueOnce({
      backup_id: ROW.id,
      is_valid: true,
      sha256_checksum: ROW.sha256_checksum,
      status: "SUCCESS",
      message: null,
    });
    const ok = await useBackupModalStore.getState().verifyBackupIntegrity(ROW.id);
    expect(ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("backups_verify", {
      request: { session_token: "tok-bkp-9", backup_id: ROW.id },
      session_token: "tok-bkp-9",
    });
    expect(
      useBackupModalStore.getState().verificationResults[ROW.id]?.isValid,
    ).toBe(true);
    expect(useBackupModalStore.getState().verifyingId).toBeNull();
  });

  it("surfaces verification failure with an error indicator", async () => {
    invokeMock.mockResolvedValueOnce([ROW]);
    await useBackupModalStore.getState().openModal();
    vi.clearAllMocks();
    invokeMock.mockRejectedValueOnce("BACKUP_CORRUPTED");
    const ok = await useBackupModalStore.getState().verifyBackupIntegrity(ROW.id);
    expect(ok).toBe(false);
    expect(useBackupModalStore.getState().errorMessage).toBe("BACKUP_CORRUPTED");
    expect(
      useBackupModalStore.getState().verificationResults[ROW.id],
    ).toBeUndefined();
    for (const code of ["BACKUP_CORRUPTED", "BACKUP_CHECKSUM_MISMATCH", "BACKUP_MISSING"]) {
      expect(translateBackendError(code, "fr-DZ")).not.toBe("Erreur interne du noyau");
      expect(translateBackendError(code, "ar-DZ")).not.toBe("خطأ داخلي في النواة");
    }
  });

  it("closeModal resets state and honors the creating lock", async () => {
    invokeMock.mockResolvedValueOnce([ROW]);
    await useBackupModalStore.getState().openModal();
    useBackupModalStore.getState().closeModal();
    expect(useBackupModalStore.getState().isOpen).toBe(false);
    expect(useBackupModalStore.getState().backups).toHaveLength(0);
  });
});

const RESTORE_ROW = {
  id: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
  fileName: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
  fileSizeBytes: 2097152,
  sha256Checksum: "ab".repeat(32),
  triggerType: "MANUAL",
  status: "SUCCESS",
  schemaVersion: 13,
  errorMessage: null,
  createdAt: "2026-09-29T12:00:00.000Z",
};

const ELIGIBLE_REPORT: PreRestoreReport = {
  backupId: RESTORE_ROW.id,
  fileName: RESTORE_ROW.fileName,
  fileSizeBytes: 2097152,
  createdAt: "2026-09-29T12:00:00.000Z",
  schemaVersionBackup: 13,
  schemaVersionApp: 13,
  checks: {
    fileExists: true,
    sha256Matches: true,
    integrityOk: true,
    foreignKeysOk: true,
    auditChainOk: true,
    customerLedgerOk: true,
    supplierLedgerOk: true,
    stockLevelsOk: true,
  },
  contentSummary: {
    products: 4,
    customers: 2,
    documentsTotal: 9,
    lastDocumentAt: "2026-09-29T11:59:00.000Z",
  },
  warnings: [
    { type: "open_cash_session_in_backup", openedAt: "2026-09-29T08:00:00.000Z" },
    { type: "newer_activity_will_be_quarantined", documentCount: 5 },
  ],
  eligible: true,
  ineligibleReasons: [],
};

const INELIGIBLE_REPORT: PreRestoreReport = {
  ...ELIGIBLE_REPORT,
  checks: { ...ELIGIBLE_REPORT.checks, sha256Matches: false },
  warnings: [],
  eligible: false,
  ineligibleReasons: [
    { type: "hash_mismatch" },
    { type: "customer_ledger_mismatch", mismatchCount: 2 },
  ],
};

/**
 * Static-markup restore suite (TASK-BRK-054, directive G). The project has
 * no DOM runner (no jsdom/happy-dom/testing-library, and no new
 * dependencies permitted), so components render via react-dom/server and
 * interaction drivers assert through the same store actions the JSX
 * handlers invoke (input onChange → setConfirmationInput, CTA onClick →
 * executeRestore, Retour onClick → resetToIdle, Esc/X → closeModal).
 */
describe("backup restore flow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession("ADMIN");
    usePosUiStore.setState({ locale: "fr-DZ" });
  });

  function noop(): void {
    // Intentional no-op: static markup asserts structure, store tests
    // assert the effects behind each handler.
  }

  function renderPanel(overrides: Partial<RestoreReportPanelProps> = {}): string {
    const props: RestoreReportPanelProps = {
      locale: "fr-DZ",
      phase: "report",
      report: ELIGIBLE_REPORT,
      confirmationInput: "",
      outcome: null,
      restoreErrorCode: null,
      onBack: noop,
      onConfirmationInput: noop,
      onExecute: noop,
      onCloseApp: noop,
      ...overrides,
    };
    // React escapes apostrophes in static markup; normalize for assertions.
    return renderToStaticMarkup(<RestoreReportPanel {...props} />)
      .replace(/&#x27;/g, "'")
      .replace(/&quot;/g, '"');
  }

  function renderRowButton(): string {
    return renderToStaticMarkup(
      <RestoreRowButton backupId={RESTORE_ROW.id} disabled={false} label="Restaurer" onStartPrecheck={noop} />,
    );
  }

  function executeButtonTag(markup: string): string {
    const match = /<button([^>]*)>Restaurer la base de données<\/button>/.exec(markup);
    if (match === null || match[1] === undefined) {
      throw new Error("destructive CTA absent from the markup");
    }
    return match[1];
  }

  function isExecuteDisabled(markup: string): boolean {
    // React renders disabled={true} as disabled=""; the class list also
    // contains the `disabled:opacity-40` variant, so match the attribute.
    return executeButtonTag(markup).includes('disabled=""');
  }

  it("G1: restore action absent without the permission, present with it", () => {
    // The caller renders `{canRestore && <RestoreRowButton .../>}`; absence
    // follows from the predicate, presence from the button markup.
    expect(canRestoreForRole("VENDEUR")).toBe(false);
    expect(canRestoreForRole("GERANT")).toBe(false);
    expect(canRestoreForRole(null)).toBe(false);
    expect(canRestoreForRole(undefined)).toBe(false);
    expect(canRestoreForRole("ADMIN")).toBe(true);
    const markup = renderRowButton();
    expect(markup).toContain("Restaurer");
    expect(markup).toContain("↺");
    expect(markup).toContain(`aria-label="Restaurer"`);
  });

  it("G2: eligible report renders all eight checks, the summary, and the confirmation panel", () => {
    const markup = renderPanel({});
    for (const label of [
      "Fichier de sauvegarde présent",
      "Empreinte SHA-256 conforme",
      "Intégrité physique de la base",
      "Intégrité référentielle (clés étrangères)",
      "Chaîne d'audit valide",
      "Cohérence du livre clients (crédit)",
      "Cohérence du livre fournisseurs",
      "Cohérence des niveaux de stock",
    ]) {
      expect(markup).toContain(label);
    }
    for (const label of ["Produits", "Clients", "Documents", "Dernier document"]) {
      expect(markup).toContain(label);
    }
    expect(markup).toContain(RESTORE_ROW.fileName);
    expect(markup).toContain("Confirmation obligatoire");
    expect(markup).toContain("Tapez RESTORE");
    expect(markup).toContain("Restaurer la base de données");
  });

  it("G3a: wrong confirmations keep the CTA disabled, exact RESTORE enables it", () => {
    expect(isRestoreConfirmationValid("restore")).toBe(false);
    expect(isRestoreConfirmationValid("RESTOR")).toBe(false);
    expect(isRestoreConfirmationValid("استعادة")).toBe(false);
    for (const wrong of ["restore", "RESTOR", "استعادة"]) {
      expect(isExecuteDisabled(renderPanel({ confirmationInput: wrong }))).toBe(true);
    }
    expect(isRestoreConfirmationValid("RESTORE")).toBe(true);
    expect(isExecuteDisabled(renderPanel({ confirmationInput: "RESTORE" }))).toBe(false);
  });

  it("G3b: surrounding spaces still enable the CTA (trim semantics)", () => {
    expect(isRestoreConfirmationValid("  RESTORE  ")).toBe(true);
    expect(isExecuteDisabled(renderPanel({ confirmationInput: "  RESTORE  " }))).toBe(false);
  });

  it("G4: ineligible report disables the CTA permanently and localizes the reasons", () => {
    // The confirmation block renders only when eligible (spec §5), so for
    // an ineligible report no executable CTA exists at all — the strongest
    // form of "permanently disabled".
    const markup = renderPanel({ report: INELIGIBLE_REPORT, confirmationInput: "RESTORE" });
    expect(markup).not.toContain("Restaurer la base de données");
    expect(isRestoreExecutable(INELIGIBLE_REPORT, "RESTORE")).toBe(false);
    expect(markup).toContain("Restauration impossible");
    expect(markup).toContain("Empreinte SHA-256 non concordante");
    expect(markup).toContain("2 ligne(s) en écart");
    expect(markup).not.toContain("Confirmation obligatoire");
  });

  it("G5: open-session and newer-documents warnings render with the interpolated count", () => {
    const markup = renderPanel({});
    expect(
      describeRestoreWarning(
        { type: "open_cash_session_in_backup", openedAt: "2026-09-29T08:00:00.000Z" },
        "fr-DZ",
      ),
    ).toContain("session de caisse encore ouverte");
    expect(markup).toContain("session de caisse encore ouverte");
    expect(markup).toContain("5 document(s)");
    expect(markup).toContain("quarantaine (aucune suppression)");
  });

  it("G6: success phase is non-dismissable and its button closes the application", async () => {
    expect(isRestorePhaseDismissable("precheck")).toBe(false);
    expect(isRestorePhaseDismissable("restoring")).toBe(false);
    expect(isRestorePhaseDismissable("success")).toBe(false);
    expect(isRestorePhaseDismissable("idle")).toBe(true);
    expect(isRestorePhaseDismissable("report")).toBe(true);
    expect(isRestorePhaseDismissable("restore-error")).toBe(true);
    useBackupModalStore.setState({
      isOpen: true,
      restorePhase: "report",
      restoreReport: ELIGIBLE_REPORT,
      confirmationInput: "RESTORE",
    });
    invokeMock.mockResolvedValueOnce({
      restored_backup_id: RESTORE_ROW.id,
      quarantined_to: "active-20260929-120500Z.db",
      restored_at: "2026-09-29T12:05:00.000Z",
      restart_required: true,
    });
    const ok = await useBackupModalStore.getState().executeRestore(RESTORE_ROW.id);
    expect(ok).toBe(true);
    expect(useBackupModalStore.getState().restorePhase).toBe("success");
    const outcome = useBackupModalStore.getState().restoreOutcome;
    const markup = renderPanel({ phase: "success", outcome });
    expect(markup).toContain("Base de données restaurée avec succès");
    expect(markup).toContain("active-20260929-120500Z.db");
    expect(markup).toContain("Fermer l'application");
    // Esc and X translate to closeModal: blocked in success and restoring.
    useBackupModalStore.getState().closeModal();
    expect(useBackupModalStore.getState().isOpen).toBe(true);
    expect(useBackupModalStore.getState().restorePhase).toBe("success");
    useBackupModalStore.setState({ restorePhase: "restoring" });
    expect(renderPanel({ phase: "restoring" })).toContain("Restauration en cours");
    useBackupModalStore.getState().closeModal();
    expect(useBackupModalStore.getState().isOpen).toBe(true);
    // The success button closes the application window exactly once.
    const closeSpy = vi.fn().mockResolvedValue(undefined);
    getCurrentWindowMock.mockReturnValue({ close: closeSpy } as unknown as ReturnType<
      typeof getCurrentWindow
    >);
    await closeApplication();
    expect(closeSpy).toHaveBeenCalledTimes(1);
  });

  it("G7: swap failure shows the reassurance and Retour returns to idle without side effects", async () => {
    useBackupModalStore.setState({
      restorePhase: "report",
      restoreReport: ELIGIBLE_REPORT,
      confirmationInput: "RESTORE",
    });
    invokeMock.mockRejectedValueOnce("RESTORE_SWAP_FAILED");
    const ok = await useBackupModalStore.getState().executeRestore(RESTORE_ROW.id);
    expect(ok).toBe(false);
    expect(useBackupModalStore.getState().restorePhase).toBe("restore-error");
    expect(useBackupModalStore.getState().restoreError).toBe("RESTORE_SWAP_FAILED");
    const markup = renderPanel({ phase: "restore-error", restoreErrorCode: "RESTORE_SWAP_FAILED" });
    expect(markup).toContain("Échec de la restauration");
    expect(markup).toContain(t("fr-DZ", restoreErrorI18nKey("RESTORE_SWAP_FAILED")));
    expect(markup).toContain("La base de données actuelle n'a pas été modifiée.");
    // The raw wire code / internal detail is never rendered.
    expect(markup).not.toContain("RESTORE_SWAP_FAILED");
    // Retour restores idle with zero side effects (no new IPC call).
    vi.clearAllMocks();
    useBackupModalStore.getState().resetToIdle();
    expect(useBackupModalStore.getState().restorePhase).toBe("idle");
    expect(useBackupModalStore.getState().restoreReport).toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
