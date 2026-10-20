/**
 * Store-level coverage seal for the backup restore state machine
 * (TASK-BRK-054, Directive J2). No rendering: the ipc module is mocked
 * and transitions assert directly against the store.
 *
 * Covers: idle → precheck → report (startPrecheck); report → restoring
 * → success with the mid-flight phase observed; report → restoring →
 * restore-error; restore-error → idle and report → idle via resetToIdle
 * with zero IPC calls; the RESTORE confirmation gate.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { precheckRestoreBackup, restoreBackup } from "@/lib/ipc";
import { useBackupModalStore } from "@/stores/backup-modal-store";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";
import type { PreRestoreReport, RestoreOutcome } from "@/lib/ipc";

vi.mock("@/lib/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/ipc")>();
  return { ...actual, precheckRestoreBackup: vi.fn(), restoreBackup: vi.fn() };
});

const precheckMock = vi.mocked(precheckRestoreBackup);
const restoreMock = vi.mocked(restoreBackup);

const REPORT: PreRestoreReport = {
  backupId: "b.db",
  fileName: "b.db",
  fileSizeBytes: 1024,
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
  contentSummary: { products: 1, customers: 0, documentsTotal: 0, lastDocumentAt: null },
  warnings: [],
  eligible: true,
  ineligibleReasons: [],
};

const OUTCOME: RestoreOutcome = {
  restoredBackupId: "b.db",
  quarantinedTo: "active-20260929-120500Z.db",
  restoredAt: "2026-09-29T12:05:00.000Z",
  restartRequired: true,
};

function primeSession(): void {
  const session: SessionInfo = {
    token: "tok-seal",
    userId: "55555555-5555-5555-5555-555555555555",
    username: "admin1",
    role: "ADMIN",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
  useBackupModalStore.getState().reset();
}

describe("restore state machine (store-level seal)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("J2a: startPrecheck populates the report, then RESTORE drives restoring → success", async () => {
    precheckMock.mockResolvedValueOnce(REPORT);
    const eligible = await useBackupModalStore.getState().startPrecheck("b.db");
    expect(eligible).toBe(true);
    expect(precheckMock).toHaveBeenCalledWith("tok-seal", "b.db");
    expect(useBackupModalStore.getState().restorePhase).toBe("report");
    expect(useBackupModalStore.getState().restoreReport?.backupId).toBe("b.db");

    useBackupModalStore.getState().setConfirmationInput("RESTORE");
    let release!: (value: RestoreOutcome) => void;
    restoreMock.mockImplementationOnce(
      () =>
        new Promise<RestoreOutcome>((resolve) => {
          release = resolve;
        }),
    );
    const pending = useBackupModalStore.getState().executeRestore("b.db");
    // No await precedes the lock: the mid-flight phase is already visible.
    expect(useBackupModalStore.getState().restorePhase).toBe("restoring");
    release(OUTCOME);
    const ok = await pending;
    expect(ok).toBe(true);
    expect(restoreMock).toHaveBeenCalledWith("tok-seal", "b.db");
    expect(useBackupModalStore.getState().restorePhase).toBe("success");
    expect(useBackupModalStore.getState().restoreOutcome?.quarantinedTo).toBe(
      "active-20260929-120500Z.db",
    );
  });

  it("J2a-gate: executeRestore without exact RESTORE performs zero IPC", async () => {
    precheckMock.mockResolvedValueOnce(REPORT);
    await useBackupModalStore.getState().startPrecheck("b.db");
    useBackupModalStore.getState().setConfirmationInput("restore");
    const ok = await useBackupModalStore.getState().executeRestore("b.db");
    expect(ok).toBe(false);
    expect(restoreMock).not.toHaveBeenCalled();
    expect(useBackupModalStore.getState().restorePhase).toBe("report");
  });

  it("J2b: rejection lands in restore-error with the mapped code", async () => {
    precheckMock.mockResolvedValueOnce(REPORT);
    await useBackupModalStore.getState().startPrecheck("b.db");
    useBackupModalStore.getState().setConfirmationInput("RESTORE");
    restoreMock.mockRejectedValueOnce("RESTORE_SWAP_FAILED");
    const ok = await useBackupModalStore.getState().executeRestore("b.db");
    expect(ok).toBe(false);
    expect(useBackupModalStore.getState().restorePhase).toBe("restore-error");
    expect(useBackupModalStore.getState().restoreError).toBe("RESTORE_SWAP_FAILED");
  });

  it("J2b-precheck: precheck rejection lands in restore-error", async () => {
    precheckMock.mockRejectedValueOnce("BACKUP_NOT_FOUND");
    const eligible = await useBackupModalStore.getState().startPrecheck("missing.db");
    expect(eligible).toBe(false);
    expect(useBackupModalStore.getState().restorePhase).toBe("restore-error");
    expect(useBackupModalStore.getState().restoreError).toBe("BACKUP_NOT_FOUND");
  });

  it("J2c: resetToIdle clears everything with zero IPC calls", async () => {
    precheckMock.mockResolvedValueOnce(REPORT);
    await useBackupModalStore.getState().startPrecheck("b.db");
    useBackupModalStore.getState().setConfirmationInput("RESTORE");
    vi.clearAllMocks();
    // Retour from report: zero side effects.
    useBackupModalStore.getState().resetToIdle();
    const state = useBackupModalStore.getState();
    expect(state.restorePhase).toBe("idle");
    expect(state.restoreReport).toBeNull();
    expect(state.confirmationInput).toBe("");
    expect(state.restoreOutcome).toBeNull();
    expect(state.restoreError).toBeNull();
    expect(precheckMock).not.toHaveBeenCalled();
    expect(restoreMock).not.toHaveBeenCalled();
  });
});
