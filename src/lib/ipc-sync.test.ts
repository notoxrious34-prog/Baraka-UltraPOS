/**
 * Contract tests for the two-till sync IPC wrapper (TASK-BRK-124).
 *
 * Pins the exact backend command names (`sync_export_package`,
 * `sync_import_package`), the wire shape field-for-field (snake_case),
 * the snake_case-to-camelCase mapping, and verbatim error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ipcSyncExport, ipcSyncImport } from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

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
    username: "audit",
    role: "ADMIN",
    permissions: ["MANAGE_PRODUCTS", "POS_SELL"],
    expiresAt: "2026-01-01T00:00:00Z",
  };
  useSessionStore.setState({ session });
}

beforeEach(() => {
  fakeTauriWindow();
  invokeMock.mockReset();
});

afterEach(() => {
  clearTauriWindow();
  useSessionStore.setState({ session: null });
});

describe("ipcSyncExport", () => {
  it("calls sync_export_package with the cursor and limit", async () => {
    primeSession("tok-sync");
    invokeMock.mockResolvedValue({ package_text: "BARAKA-SYNC-1|T1|t|1|abc", exported_docs: 1 });
    const result = await ipcSyncExport("tok-sync", "2026-11-24T00:00:00Z", 500);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      { request: Record<string, unknown> },
    ];
    expect(command).toBe("sync_export_package");
    expect(args.request).toEqual({
      session_token: "tok-sync",
      since_created_at: "2026-11-24T00:00:00Z",
      limit: 500,
    });
    expect(result).toEqual({ packageText: "BARAKA-SYNC-1|T1|t|1|abc", exportedDocs: 1 });
  });

  it("propagates backend errors verbatim", async () => {
    primeSession("tok-sync");
    invokeMock.mockRejectedValue("INVALID_SYNC_CURSOR");
    await expect(ipcSyncExport("tok-sync", "", 10)).rejects.toBe("INVALID_SYNC_CURSOR");
  });
});

describe("ipcSyncImport", () => {
  it("maps the atomic outcome rows origin->local", async () => {
    primeSession("tok-sync");
    invokeMock.mockResolvedValue({
      imported_docs: 1,
      renumbered: [{ origin_number: "T-000001", new_number: "FACT-2026-000007" }],
      negative_stock_warnings: [{ sku: "BK1", new_number: "FACT-2026-000007" }],
    });
    const result = await ipcSyncImport("tok-sync", "BARAKA-SYNC-1|T1|t|1|abc", "WH-1");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      { request: Record<string, unknown> },
    ];
    expect(command).toBe("sync_import_package");
    expect(args.request).toEqual({
      session_token: "tok-sync",
      package_text: "BARAKA-SYNC-1|T1|t|1|abc",
      warehouse_id: "WH-1",
    });
    expect(result).toEqual({
      importedDocs: 1,
      renumbered: [{ originNumber: "T-000001", newNumber: "FACT-2026-000007" }],
      negativeStockWarnings: [{ sku: "BK1", newNumber: "FACT-2026-000007" }],
    });
  });

  it("propagates SYNC_ALREADY_IMPORTED verbatim", async () => {
    primeSession("tok-sync");
    invokeMock.mockRejectedValue("SYNC_ALREADY_IMPORTED");
    await expect(ipcSyncImport("tok-sync", "pkg", "WH-1")).rejects.toBe("SYNC_ALREADY_IMPORTED");
  });
});
