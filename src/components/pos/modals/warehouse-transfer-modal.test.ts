/**
 * Unit tests for the warehouse transfer cockpit (TASK-BRK-051).
 *
 * Covers store initialization with endpoint defaults, same-warehouse
 * rejection, BigInt-micro line management, sequential canonical dispatch
 * with early abort, creation-form validation, the double-submit guard,
 * and role helpers — all without floats.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { join } from "node:path";
import {
  canSubmitTransfer,
  defaultEndpoints,
  parseTransferQtyInput,
  useWarehouseTransferStore,
} from "@/stores/warehouse-transfer-store";
import { translateBackendError } from "@/lib/i18n";
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

function primeSession(role: SessionInfo["role"] = "GERANT"): void {
  fakeTauriWindow();
  const session: SessionInfo = {
    token: "tok-wh-9",
    userId: "55555555-5555-5555-5555-555555555555",
    username: "gerant1",
    role,
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
  useWarehouseTransferStore.getState().reset();
}

// Minimal ambient typing for the single Node global used below.
// Type-only; zero runtime impact.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const STORE_SRC = readSrc("src", "stores", "warehouse-transfer-store.ts");
const MODAL_SRC = readSrc("src", "components", "pos", "modals", "WarehouseTransferModal.tsx");

const WAREHOUSES = [
  { id: "w1", code: "DEP-01", name: "Dépôt", is_active: true },
  { id: "w2", code: "MAG-01", name: "Magasin", is_active: true },
];

function line(productId = "p1") {
  return {
    productId,
    productName: "Vis 8x40",
    sku: "VIS-8X40",
    unitId: "u1",
    unitName: "Pièce",
  };
}

describe("parseTransferQtyInput", () => {
  it("parses selling units to micros without floats", () => {
    expect(parseTransferQtyInput("2")).toBe(20000n);
    expect(parseTransferQtyInput("2,5")).toBe(25000n);
    expect(parseTransferQtyInput("abc")).toBe(0n);
    expect(parseTransferQtyInput("")).toBe(0n);
  });
});

describe("canSubmitTransfer", () => {
  const lines = [{ ...line(), quantityMicro: 10000n }];
  it("opens the gate only for distinct endpoints with a positive line", () => {
    expect(
      canSubmitTransfer({
        sourceWarehouseId: "w1",
        destinationWarehouseId: "w2",
        lines,
        isSubmitting: false,
      }),
    ).toBe(true);
    expect(
      canSubmitTransfer({
        sourceWarehouseId: "w1",
        destinationWarehouseId: "w1",
        lines,
        isSubmitting: false,
      }),
    ).toBe(false);
    expect(
      canSubmitTransfer({
        sourceWarehouseId: "w1",
        destinationWarehouseId: "w2",
        lines: [{ ...line(), quantityMicro: 0n }],
        isSubmitting: false,
      }),
    ).toBe(false);
    expect(
      canSubmitTransfer({
        sourceWarehouseId: "w1",
        destinationWarehouseId: "w2",
        lines,
        isSubmitting: true,
      }),
    ).toBe(false);
    expect(
      canSubmitTransfer({
        sourceWarehouseId: null,
        destinationWarehouseId: "w2",
        lines,
        isSubmitting: false,
      }),
    ).toBe(false);
  });
});

describe("permission gates (TASK-DSN-080)", () => {
  it("exports no role-string helper from the transfer store", () => {
    expect(STORE_SRC).not.toContain("isAdminRole");
    expect(STORE_SRC).not.toContain("isManagerRole");
    expect(STORE_SRC).not.toContain('role === "ADMIN"');
    expect(STORE_SRC).not.toContain('role === "GERANT"');
  });

  it("gates transfer execution by TRANSFER_STOCK with guidance", () => {
    expect(MODAL_SRC).toContain('hasPermission("TRANSFER_STOCK")');
    expect(MODAL_SRC).toContain("disabled={");
    expect(MODAL_SRC).toContain("!canTransfer");
    expect(MODAL_SRC).toContain("stock.transferReservedGuidance");
    expect(MODAL_SRC).not.toContain("isManagerRole");
  });

  it("gates warehouse creation and depot management by MANAGE_WAREHOUSES", () => {
    expect(MODAL_SRC).toContain('hasPermission("MANAGE_WAREHOUSES")');
    expect(MODAL_SRC).toContain("!canManageWarehouses");
    expect(MODAL_SRC).toContain("stock.warehouseCreateReservedGuidance");
    expect(MODAL_SRC).not.toContain("isAdminRole");
    expect(MODAL_SRC).not.toContain("session.role");
  });
});

describe("warehouse transfer store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("initializes empty and loads warehouses with endpoint defaults", async () => {
    invokeMock.mockResolvedValueOnce({ warehouses: WAREHOUSES });
    await useWarehouseTransferStore.getState().openModal();
    const state = useWarehouseTransferStore.getState();
    expect(state.isOpen).toBe(true);
    expect(state.warehouses).toEqual(WAREHOUSES);
    expect(state.sourceWarehouseId).toBe("w1");
    expect(state.destinationWarehouseId).toBe("w2");
    expect(defaultEndpoints([])).toEqual({ source: null, destination: null });
  });

  it("manages transfer lines with BigInt quantities", () => {
    const store = useWarehouseTransferStore.getState();
    store.addTransferItem(line());
    store.addTransferItem(line());
    expect(useWarehouseTransferStore.getState().transferItems).toHaveLength(1);
    expect(useWarehouseTransferStore.getState().transferItems[0]?.quantityMicro).toBe(10000n);
    useWarehouseTransferStore.getState().updateTransferItemQty("p1", 25000n);
    expect(useWarehouseTransferStore.getState().transferItems[0]?.quantityMicro).toBe(25000n);
    useWarehouseTransferStore.getState().updateTransferItemQty("p1", -5n);
    expect(useWarehouseTransferStore.getState().transferItems[0]?.quantityMicro).toBe(0n);
    useWarehouseTransferStore.getState().removeTransferItem("p1");
    expect(useWarehouseTransferStore.getState().transferItems).toHaveLength(0);
  });

  it("dispatches one canonical call per line and stops at the first failure", async () => {
    invokeMock.mockResolvedValue({ warehouses: WAREHOUSES });
    await useWarehouseTransferStore.getState().openModal();
    useWarehouseTransferStore.getState().addTransferItem(line("p1"), 20000n);
    useWarehouseTransferStore.getState().addTransferItem(line("p2"), 10000n);
    vi.clearAllMocks();
    invokeMock.mockResolvedValueOnce(undefined);
    invokeMock.mockRejectedValueOnce("STOCK_INSUFFICIENT");
    const ok = await useWarehouseTransferStore.getState().submitTransfer();
    expect(ok).toBe(false);
    const transfers = invokeMock.mock.calls.filter((call) => call[0] === "stock_transfer");
    expect(transfers).toHaveLength(2);
    expect((transfers[0]?.[1] as { request: Record<string, unknown> }).request).toMatchObject({
      source_warehouse_id: "w1",
      destination_warehouse_id: "w2",
      product_id: "p1",
      quantity_micro: "20000",
    });
    expect(useWarehouseTransferStore.getState().errorMessage).toBe("STOCK_INSUFFICIENT");
  });

  it("closes on success and clears the manifest", async () => {
    invokeMock.mockResolvedValue({ warehouses: WAREHOUSES });
    await useWarehouseTransferStore.getState().openModal();
    useWarehouseTransferStore.getState().addTransferItem(line(), 10000n);
    vi.clearAllMocks();
    invokeMock.mockResolvedValueOnce(undefined);
    const ok = await useWarehouseTransferStore.getState().submitTransfer();
    expect(ok).toBe(true);
    expect(useWarehouseTransferStore.getState().successMessage).toBe("transferSuccess");
    expect(useWarehouseTransferStore.getState().transferItems).toHaveLength(0);
  });

  it("blocks concurrent double submission behind the lock", async () => {
    invokeMock.mockResolvedValue({ warehouses: WAREHOUSES });
    await useWarehouseTransferStore.getState().openModal();
    useWarehouseTransferStore.getState().addTransferItem(line(), 10000n);
    vi.clearAllMocks();
    invokeMock.mockResolvedValue(undefined);
    const first = useWarehouseTransferStore.getState().submitTransfer();
    const second = useWarehouseTransferStore.getState().submitTransfer();
    const [a, b] = await Promise.all([first, second]);
    expect(a).toBe(true);
    expect(b).toBe(false);
    expect(
      invokeMock.mock.calls.filter((call) => call[0] === "stock_transfer"),
    ).toHaveLength(1);
  });

  it("validates the creation form and uppercases codes", async () => {
    invokeMock.mockResolvedValue({ warehouses: WAREHOUSES });
    await useWarehouseTransferStore.getState().openModal();
    // Blank code/name: no dispatch.
    vi.clearAllMocks();
    expect(await useWarehouseTransferStore.getState().submitCreateWarehouse()).toBe(false);
    expect(invokeMock).not.toHaveBeenCalled();
    useWarehouseTransferStore.getState().setNewWarehouseCode("dep-02");
    expect(useWarehouseTransferStore.getState().newWarehouseCode).toBe("DEP-02");
    useWarehouseTransferStore.getState().setNewWarehouseName("Hangar");
    invokeMock.mockResolvedValueOnce({
      id: "w3",
      code: "DEP-02",
      name: "Hangar",
      is_active: true,
    });
    invokeMock.mockResolvedValueOnce({ warehouses: [...WAREHOUSES] });
    expect(await useWarehouseTransferStore.getState().submitCreateWarehouse()).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("warehouses_create", {
      request: { session_token: "tok-wh-9", code: "DEP-02", name: "Hangar" },
      session_token: "tok-wh-9",
    });
    expect(useWarehouseTransferStore.getState().successMessage).toBe(
      "createWarehouseSuccess",
    );
  });

  it("maps warehouse error codes bilingually", () => {
    for (const code of ["WAREHOUSE_CODE_EXISTS", "TRANSFER_SAME_WAREHOUSE", "UNAUTHORIZED"]) {
      expect(translateBackendError(code, "fr-DZ")).not.toBe("Erreur interne du noyau");
      expect(translateBackendError(code, "ar-DZ")).not.toBe("خطأ داخلي في النواة");
    }
  });
});
