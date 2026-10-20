/**
 * Row-prefill regression (TASK-DSN-064).
 *
 * Simulates the StockModule table-row "Ajuster" click: the modal opens with a
 * stock row that carries NO pre-loaded units array and NO unit_id. Entering a
 * variation of 500 with reason INVENTAIRE must enable submission and dispatch
 * stock_adjust with the resolved default unit. FAILS on the pre-fix store
 * (unitId stays null, submitAdjustment refuses); passes after the fix.
 * Plus source contracts for the debounced picker (175ms) and the LTR-isolated
 * transition preview (<bdi dir="ltr">).
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { useStockAdjustModalStore } from "@/stores/stock-adjust-modal-store";
import { useSessionStore } from "@/stores/session-store";
import type { CatalogProduct, SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

declare const process: { cwd(): string };

const MODAL_SRC = readFileSync(
  join(process.cwd(), "src", "components", "pos", "modals", "StockAdjustModal.tsx"),
  "utf8",
);

function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function primeSession(): void {
  fakeTauriWindow();
  const session: SessionInfo = {
    token: "tok-row-7",
    userId: "77777777-7777-7777-7777-777777777777",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
  useStockAdjustModalStore.getState().reset();
}

const PRODUCT_WITH_UNITS: CatalogProduct = {
  product_id: "p-row-1",
  sku: "SKU-ROW-1",
  barcode: null,
  name_ar: "برغي",
  name_fr: "Vis 8x40",
  category_id: null,
  is_kit: false,
  is_active: true,
  units: [
    {
      unit_id: "u-base",
      product_id: "p-row-1",
      unit_name_ar: "قطعة",
      unit_name_fr: "Pièce",
      factor_micro: "10000",
      barcode: null,
      sale_price_centimes: "2500",
      purchase_price_centimes: "1500",
      wholesale_price_centimes: null,
      is_base_unit: true,
      is_default_selling: false,
    },
    {
      unit_id: "u-def",
      product_id: "p-row-1",
      unit_name_ar: "علبة",
      unit_name_fr: "Boîte",
      factor_micro: "100000",
      barcode: null,
      sale_price_centimes: "25000",
      purchase_price_centimes: "15000",
      wholesale_price_centimes: null,
      is_base_unit: false,
      is_default_selling: true,
    },
  ],
  kit_items: [],
};

function mockBridge(): void {
  invokeMock.mockImplementation((command: string) => {
    if (command === "warehouses_list") {
      return Promise.resolve({
        warehouses: [{ id: "w1", code: "DEP-01", name: "Dépôt", is_active: true }],
      });
    }
    if (command === "stock_get") {
      return Promise.resolve({
        product_id: "p-row-1",
        warehouse_id: "w1",
        base_unit_id: "u-base",
        quantity_base_micro: "-10000",
      });
    }
    if (command === "catalog_get_product") {
      return Promise.resolve(PRODUCT_WITH_UNITS);
    }
    if (command === "stock_adjust") {
      return Promise.resolve({
        movement_id: "m-row-9",
        new_quantity_base_micro: "4990000",
      });
    }
    return Promise.reject(`MOCK_UNMAPPED:${command}`);
  });
}

describe("stock row Ajuster prefill (TASK-DSN-064)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
    mockBridge();
  });

  it("resolves the default unit when opened from a unit-less stock row", async () => {
    // Exactly what StockModule.openAdjust(row) passes: no units, no unitId.
    useStockAdjustModalStore.getState().openModal({
      warehouseId: "w1",
      productId: "p-row-1",
      productName: "Vis 8x40",
      currentStockMicro: "-10000",
    });
    await vi.waitFor(() => {
      expect(useStockAdjustModalStore.getState().unitId).toBe("u-def");
    });
    expect(useStockAdjustModalStore.getState().unitName).toBe("Boîte");
  });

  it("enables submit with qty 500 + INVENTAIRE and dispatches unit_id", async () => {
    useStockAdjustModalStore.getState().openModal({
      warehouseId: "w1",
      productId: "p-row-1",
      productName: "Vis 8x40",
      currentStockMicro: "-10000",
    });
    await vi.waitFor(() => {
      expect(useStockAdjustModalStore.getState().unitId).toBe("u-def");
    });
    useStockAdjustModalStore.getState().setInputValue("500");
    useStockAdjustModalStore.getState().setReason("INVENTAIRE");
    const ok = await useStockAdjustModalStore.getState().submitAdjustment();
    expect(ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("stock_adjust", {
      request: {
        session_token: "tok-row-7",
        product_id: "p-row-1",
        warehouse_id: "w1",
        unit_id: "u-def",
        quantity_micro: "5000000",
        direction: "INCREASE",
        reason: "INVENTAIRE",
        authorize_negative_stock: false,
      },
      session_token: "tok-row-7",
    });
    expect(useStockAdjustModalStore.getState().successResult?.newMicro).toBe(4990000n);
  });
});

describe("stock adjust modal source contracts (TASK-DSN-064)", () => {
  it("searches the picker on a 175ms debounce without a manual trigger", () => {
    expect(MODAL_SRC).toContain("setTimeout");
    expect(MODAL_SRC).toContain("175");
    expect(MODAL_SRC).toContain("clearTimeout");
  });

  it("isolates the transition preview in <bdi dir=\"ltr\">", () => {
    expect(MODAL_SRC).toContain('<bdi dir="ltr" className="tabular-nums">');
  });
});
