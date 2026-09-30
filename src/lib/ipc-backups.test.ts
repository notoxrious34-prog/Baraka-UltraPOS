/**
 * Contract tests for the backup IPC wrappers (TASK-BRK-052).
 *
 * Pins the exact backend command names (`backups_create`, `backups_list`,
 * `backups_verify`), request wire shapes field-for-field, verbatim
 * response mapping, and code-preserving error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { createBackup, listBackups, precheckRestoreBackup, restoreBackup, verifyBackup } from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

/** The bridge probe requires a `window` carrying the Tauri marker. */
function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function clearTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  delete globals.window;
  delete globals.__TAURI_INTERNALS__;
}

function primeSession(token: string): void {
  const session: SessionInfo = {
    token,
    userId: "55555555-5555-5555-5555-555555555555",
    username: "admin1",
    role: "ADMIN",
    expiresAt: "2026-01-01T00:00:00Z",
  };
  useSessionStore.setState({ session });
}

const metadataWire = {
  id: "11111111-1111-4111-8111-111111111111",
  file_name: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
  file_size_bytes: 1048576,
  sha256_checksum: "ab".repeat(32),
  trigger_type: "MANUAL",
  status: "SUCCESS",
  schema_version: 13,
  error_message: null,
  created_at: "2026-09-29T12:00:00.000Z",
};

describe("createBackup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("invokes backups_create with the default MANUAL trigger", async () => {
    primeSession("tok-bkp-1");
    invokeMock.mockResolvedValueOnce(metadataWire);
    const result = await createBackup("tok-bkp-1");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("backups_create", {
      request: { session_token: "tok-bkp-1", trigger_type: "MANUAL" },
      session_token: "tok-bkp-1",
    });
    expect(result).toEqual({
      id: "11111111-1111-4111-8111-111111111111",
      fileName: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      fileSizeBytes: 1048576,
      sha256Checksum: "ab".repeat(32),
      triggerType: "MANUAL",
      status: "SUCCESS",
      schemaVersion: 13,
      errorMessage: null,
      createdAt: "2026-09-29T12:00:00.000Z",
    });
  });

  it("forwards an explicit SHIFT_CLOSE trigger", async () => {
    primeSession("tok-bkp-1");
    invokeMock.mockResolvedValueOnce({ ...metadataWire, trigger_type: "SHIFT_CLOSE" });
    const result = await createBackup("tok-bkp-1", "SHIFT_CLOSE");
    expect(invokeMock).toHaveBeenCalledWith("backups_create", {
      request: { session_token: "tok-bkp-1", trigger_type: "SHIFT_CLOSE" },
      session_token: "tok-bkp-1",
    });
    expect(result.triggerType).toBe("SHIFT_CLOSE");
  });

  it("propagates UNAUTHORIZED without alteration", async () => {
    primeSession("tok-bkp-1");
    invokeMock.mockRejectedValueOnce("UNAUTHORIZED");
    await expect(createBackup("tok-bkp-1")).rejects.toBe("UNAUTHORIZED");
  });
});

describe("listBackups", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("invokes backups_list and maps the sorted array", async () => {
    primeSession("tok-bkp-2");
    invokeMock.mockResolvedValueOnce([metadataWire, { ...metadataWire, status: "PRUNED" }]);
    const result = await listBackups("tok-bkp-2");
    expect(invokeMock).toHaveBeenCalledWith("backups_list", {
      session_token: "tok-bkp-2",
    });
    expect(result).toHaveLength(2);
    expect(result[0]?.fileName).toBe("baraka_pos_backup_20260929T120000_a1b2c3d4.db");
    expect(result[1]?.status).toBe("PRUNED");
    expect(typeof result[0]?.fileSizeBytes).toBe("number");
    expect(typeof result[0]?.schemaVersion).toBe("number");
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(listBackups("ignored-token")).rejects.toThrow("IPC_NO_ACTIVE_SESSION");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("verifyBackup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("passes the backup id and maps the verification payload", async () => {
    primeSession("tok-bkp-3");
    invokeMock.mockResolvedValueOnce({
      backup_id: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      is_valid: true,
      sha256_checksum: "ab".repeat(32),
      status: "SUCCESS",
      message: null,
    });
    const result = await verifyBackup(
      "tok-bkp-3",
      "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
    );
    expect(invokeMock).toHaveBeenCalledWith("backups_verify", {
      request: {
        session_token: "tok-bkp-3",
        backup_id: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      },
      session_token: "tok-bkp-3",
    });
    expect(result).toEqual({
      backupId: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      isValid: true,
      sha256Checksum: "ab".repeat(32),
      status: "SUCCESS",
      message: null,
    });
  });

  it("propagates corruption codes without alteration", async () => {
    primeSession("tok-bkp-3");
    for (const code of ["BACKUP_CORRUPTED", "BACKUP_CHECKSUM_MISMATCH", "BACKUP_MISSING"]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(verifyBackup("tok-bkp-3", "any.db")).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(3);
  });
});

const precheckWire = {
  backup_id: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
  file_name: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
  file_size_bytes: 2097152,
  created_at: "2026-09-29T12:00:00.000Z",
  schema_version_backup: 12,
  schema_version_app: 13,
  checks: {
    file_exists: true,
    sha256_matches: true,
    integrity_ok: true,
    foreign_keys_ok: true,
    audit_chain_ok: true,
    customer_ledger_ok: true,
    supplier_ledger_ok: true,
    stock_levels_ok: true,
  },
  content_summary: {
    products: 4,
    customers: 2,
    documents_total: 9,
    last_document_at: "2026-09-29T11:59:00.000Z",
  },
  warnings: [
    { type: "older_schema_will_migrate", from_version: 12, to_version: 13 },
    { type: "newer_activity_will_be_quarantined", document_count: 3 },
  ],
  eligible: true,
  ineligible_reasons: [],
};

describe("precheckRestoreBackup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("invokes backups_restore_precheck and maps the typed report", async () => {
    primeSession("tok-rst-1");
    invokeMock.mockResolvedValueOnce(precheckWire);
    const result = await precheckRestoreBackup(
      "tok-rst-1",
      "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
    );
    expect(invokeMock).toHaveBeenCalledWith("backups_restore_precheck", {
      request: {
        session_token: "tok-rst-1",
        backup_id: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      },
      session_token: "tok-rst-1",
    });
    expect(result).toEqual({
      backupId: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      fileName: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      fileSizeBytes: 2097152,
      createdAt: "2026-09-29T12:00:00.000Z",
      schemaVersionBackup: 12,
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
        { type: "older_schema_will_migrate", fromVersion: 12, toVersion: 13 },
        { type: "newer_activity_will_be_quarantined", documentCount: 3 },
      ],
      eligible: true,
      ineligibleReasons: [],
    });
  });

  it("keeps counts numeric, timestamps textual, and maps reason payloads", async () => {
    primeSession("tok-rst-1");
    invokeMock.mockResolvedValueOnce({
      ...precheckWire,
      eligible: false,
      ineligible_reasons: [
        { type: "hash_mismatch" },
        { type: "customer_ledger_mismatch", mismatch_count: 2 },
        { type: "schema_newer_than_app", backup_version: 14, app_version: 13 },
      ],
    });
    const result = await precheckRestoreBackup("tok-rst-1", "any.db");
    expect(typeof result.fileSizeBytes).toBe("number");
    expect(typeof result.schemaVersionBackup).toBe("number");
    expect(typeof result.contentSummary.products).toBe("number");
    expect(typeof result.createdAt).toBe("string");
    expect(result.ineligibleReasons).toEqual([
      { type: "hash_mismatch" },
      { type: "customer_ledger_mismatch", mismatchCount: 2 },
      { type: "schema_newer_than_app", backupVersion: 14, appVersion: 13 },
    ]);
  });

  it("propagates precheck failures without alteration", async () => {
    primeSession("tok-rst-1");
    for (const code of ["UNAUTHORIZED", "BACKUP_NOT_FOUND", "BACKUP_MISSING"]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(precheckRestoreBackup("tok-rst-1", "any.db")).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(3);
  });
});

describe("restoreBackup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("passes the backup id and maps the outcome", async () => {
    primeSession("tok-rst-2");
    invokeMock.mockResolvedValueOnce({
      restored_backup_id: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      quarantined_to: "active-20260929-120500Z.db",
      restored_at: "2026-09-29T12:05:00.000Z",
      restart_required: true,
    });
    const result = await restoreBackup(
      "tok-rst-2",
      "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
    );
    expect(invokeMock).toHaveBeenCalledWith("backups_restore", {
      request: {
        session_token: "tok-rst-2",
        backup_id: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      },
      session_token: "tok-rst-2",
    });
    expect(result).toEqual({
      restoredBackupId: "baraka_pos_backup_20260929T120000_a1b2c3d4.db",
      quarantinedTo: "active-20260929-120500Z.db",
      restoredAt: "2026-09-29T12:05:00.000Z",
      restartRequired: true,
    });
  });

  it("propagates RestoreError wire codes verbatim", async () => {
    primeSession("tok-rst-2");
    for (const code of [
      "BACKUP_CHECKSUM_MISMATCH",
      "BACKUP_CORRUPTED",
      "BACKUP_FK_VIOLATION",
      "BACKUP_SCHEMA_NEWER",
      "BACKUP_AUDIT_CHAIN_BROKEN",
      "BACKUP_LEDGER_INCONSISTENT",
      "RESTORE_IN_PROGRESS",
      "RESTORE_SWAP_FAILED",
    ]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(restoreBackup("tok-rst-2", "any.db")).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(8);
  });
});
