/**
 * Contract tests for the warehouse IPC wrappers (TASK-BRK-051).
 *
 * Pins the exact backend command names (`warehouses_list`,
 * `warehouses_create`, `stock_transfer`), request wire shapes
 * field-for-field (canonical micro strings, no is_default on the wire),
 * session-token injection, verbatim response passthrough, and
 * code-preserving error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  createWarehouse,
  listWarehouses,
  transferStock,
} from "@/lib/ipc";
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
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

describe("listWarehouses", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls warehouses_list with the session token", async () => {
    primeSession("tok-wh-1");
    invokeMock.mockResolvedValueOnce({
      warehouses: [{ id: "w1", code: "DEP-01", name: "Dépôt", is_active: true }],
    });
    const result = await listWarehouses("tok-wh-1");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("warehouses_list", {
      session_token: "tok-wh-1",
    });
    expect(result).toEqual([{ id: "w1", code: "DEP-01", name: "Dépôt", is_active: true }]);
  });
});

describe("createWarehouse", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("formats the payload and invokes warehouses_create", async () => {
    primeSession("tok-wh-2");
    invokeMock.mockResolvedValueOnce({
      id: "w2",
      code: "DEP-02",
      name: "Hangar",
      is_active: true,
    });
    const result = await createWarehouse({
      session_token: "tok-wh-2",
      code: "DEP-02",
      name: "Hangar",
    });
    expect(invokeMock).toHaveBeenCalledWith("warehouses_create", {
      request: { session_token: "tok-wh-2", code: "DEP-02", name: "Hangar" },
      session_token: "tok-wh-2",
    });
    expect(result.code).toBe("DEP-02");
  });

  it("propagates WAREHOUSE_CODE_EXISTS without alteration", async () => {
    primeSession("tok-wh-2");
    invokeMock.mockRejectedValueOnce("WAREHOUSE_CODE_EXISTS");
    await expect(
      createWarehouse({ session_token: "tok-wh-2", code: "DEP-01", name: "X" }),
    ).rejects.toBe("WAREHOUSE_CODE_EXISTS");
  });
});

describe("transferStock", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("formats micro quantities and invokes stock_transfer", async () => {
    primeSession("tok-wh-3");
    invokeMock.mockResolvedValueOnce(undefined);
    await transferStock({
      session_token: "tok-wh-3",
      source_warehouse_id: "w1",
      destination_warehouse_id: "w2",
      product_id: "p1",
      unit_id: "u1",
      quantity_micro: "20000",
      reason: "Réassort",
    });
    expect(invokeMock).toHaveBeenCalledWith("stock_transfer", {
      request: {
        session_token: "tok-wh-3",
        source_warehouse_id: "w1",
        destination_warehouse_id: "w2",
        product_id: "p1",
        unit_id: "u1",
        quantity_micro: "20000",
        reason: "Réassort",
      },
      session_token: "tok-wh-3",
    });
  });

  it("propagates transfer error codes without alteration", async () => {
    primeSession("tok-wh-3");
    for (const code of [
      "TRANSFER_SAME_WAREHOUSE",
      "WAREHOUSE_INACTIVE",
      "STOCK_INSUFFICIENT",
      "UNAUTHORIZED",
    ]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(
        transferStock({
          session_token: "tok-wh-3",
          source_warehouse_id: "w1",
          destination_warehouse_id: "w2",
          product_id: "p1",
          unit_id: "u1",
          quantity_micro: "10000",
          reason: null,
        }),
      ).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(4);
  });
});
