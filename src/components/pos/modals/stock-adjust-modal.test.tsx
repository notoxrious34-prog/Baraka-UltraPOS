/**
 * Unit tests for the stock adjustment cockpit (TASK-BRK-049).
 *
 * Covers modal context rendering, DELTA negative math, physical-count
 * delta derivation, zero-delta submit refusal, canonical dispatch payload
 * with direction mapping, success state, and bilingual error mapping —
 * all without floats.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  parseDecimalToMicro,
  useStockAdjustModalStore,
} from "@/stores/stock-adjust-modal-store";
import { translateBackendError } from "@/lib/i18n";
import { resolvePickerPane } from "@/components/pos/modals/StockAdjustModal";
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

function primeSession(): void {
  fakeTauriWindow();
  const session: SessionInfo = {
    token: "tok-adj-9",
    userId: "55555555-5555-5555-5555-555555555555",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
  };
  useSessionStore.setState({ session });
  useStockAdjustModalStore.getState().reset();
}

const WAREHOUSES = [{ id: "w1", code: "DEP-01", name: "Dépôt", is_active: true }];

function openWithContext(): void {
  useStockAdjustModalStore.setState({
    isOpen: true,
    selectedWarehouseId: "w1",
    warehouses: WAREHOUSES,
    productId: "p1",
    productName: "Vis 8x40",
    unitId: "u1",
    unitName: "Pièce",
    currentStockMicro: "80000",
  });
}

describe("parseDecimalToMicro", () => {
  it("converts human decimals to micros without floats", () => {
    expect(parseDecimalToMicro("-2.5")).toBe(-25000n);
    expect(parseDecimalToMicro("+5.5")).toBe(55000n);
    expect(parseDecimalToMicro("10")).toBe(100000n);
    expect(parseDecimalToMicro("abc")).toBe(0n);
    expect(parseDecimalToMicro("")).toBe(0n);
  });
});

describe("stock adjust store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("renders context once opened with product and warehouse", () => {
    useStockAdjustModalStore.getState().openModal({
      warehouseId: "w1",
      productId: "p1",
      productName: "Vis 8x40",
      unitId: "u1",
      unitName: "Pièce",
      currentStockMicro: "80000",
    });
    const state = useStockAdjustModalStore.getState();
    expect(state.isOpen).toBe(true);
    expect(state.productName).toBe("Vis 8x40");
    expect(state.selectedWarehouseId).toBe("w1");
    expect(state.currentStockMicro).toBe("80000");
  });

  it("computes negative micro delta in DELTA mode with preview", () => {
    openWithContext();
    useStockAdjustModalStore.getState().setInputMode("DELTA");
    useStockAdjustModalStore.getState().setInputValue("-2.5");
    const state = useStockAdjustModalStore.getState();
    expect(state.computedDeltaMicro).toBe("-25000");
    expect(state.computedNewStockMicro).toBe("55000");
  });

  it("derives positive delta from a physical count above stock", () => {
    openWithContext();
    useStockAdjustModalStore.getState().setInputMode("NEW_PHYSICAL_COUNT");
    useStockAdjustModalStore.getState().setInputValue("10");
    const state = useStockAdjustModalStore.getState();
    expect(state.computedDeltaMicro).toBe("20000");
    expect(state.computedNewStockMicro).toBe("100000");
  });

  it("refuses to dispatch a zero delta", async () => {
    openWithContext();
    useStockAdjustModalStore.getState().setInputValue("0");
    vi.clearAllMocks();
    const ok = await useStockAdjustModalStore.getState().submitAdjustment();
    expect(ok).toBe(false);
    expect(invokeMock).not.toHaveBeenCalled();
    expect(useStockAdjustModalStore.getState().computedDeltaMicro).toBe("0");
  });

  it("dispatches the canonical payload with direction mapping", async () => {
    openWithContext();
    useStockAdjustModalStore.getState().setInputValue("-2.5");
    useStockAdjustModalStore.getState().setReason("CASSE");
    useStockAdjustModalStore.getState().setNotes("sac éventré");
    invokeMock.mockResolvedValueOnce({
      movement_id: "m-1",
      new_quantity_base_micro: "55000",
    });
    const ok = await useStockAdjustModalStore.getState().submitAdjustment();
    expect(ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("stock_adjust", {
      request: {
        session_token: "tok-adj-9",
        product_id: "p1",
        warehouse_id: "w1",
        unit_id: "u1",
        quantity_micro: "25000",
        direction: "DECREASE",
        reason: "CASSE — sac éventré",
        authorize_negative_stock: false,
      },
      session_token: "tok-adj-9",
    });
    const state = useStockAdjustModalStore.getState();
    expect(state.successResult?.movementId).toBe("m-1");
    expect(state.successResult?.movementType).toBe("ADJUSTMENT_NEGATIVE");
    expect(state.successResult?.previousMicro).toBe(80000n);
    expect(state.successResult?.newMicro).toBe(55000n);
    expect(state.isSubmitting).toBe(false);
  });

  it("blocks concurrent double submission behind the lock", async () => {
    openWithContext();
    useStockAdjustModalStore.getState().setInputValue("1");
    invokeMock.mockResolvedValue({
      movement_id: "m-1",
      new_quantity_base_micro: "90000",
    });
    const first = useStockAdjustModalStore.getState().submitAdjustment();
    const second = useStockAdjustModalStore.getState().submitAdjustment();
    const [a, b] = await Promise.all([first, second]);
    expect(a).toBe(true);
    expect(b).toBe(false);
    expect(
      invokeMock.mock.calls.filter((call) => call[0] === "stock_adjust"),
    ).toHaveLength(1);
  });

  it("maps adjustment error codes bilingually without swallowing", async () => {
    openWithContext();
    useStockAdjustModalStore.getState().setInputValue("1");
    invokeMock.mockRejectedValueOnce("STOCK_NEGATIVE_NOT_AUTHORIZED");
    const ok = await useStockAdjustModalStore.getState().submitAdjustment();
    expect(ok).toBe(false);
    expect(useStockAdjustModalStore.getState().errorMessage).toBe(
      "STOCK_NEGATIVE_NOT_AUTHORIZED",
    );
    expect(useStockAdjustModalStore.getState().successResult).toBeNull();
    for (const code of [
      "STOCK_NEGATIVE_NOT_AUTHORIZED",
      "ADJUSTMENT_REASON_REQUIRED",
      "UNAUTHORIZED",
    ]) {
      expect(translateBackendError(code, "fr-DZ")).not.toBe("Erreur interne du noyau");
      expect(translateBackendError(code, "ar-DZ")).not.toBe("خطأ داخلي في النواة");
    }
  });
});

describe("resolvePickerPane", () => {
  it("selects exactly one pane per state (selected handled by the caller)", () => {
    expect(resolvePickerPane({ busy: true, resultCount: 3, hasQuery: true })).toBe("loading");
    expect(resolvePickerPane({ busy: false, resultCount: 2, hasQuery: true })).toBe("results");
    expect(resolvePickerPane({ busy: false, resultCount: 0, hasQuery: true })).toBe("empty");
    expect(resolvePickerPane({ busy: false, resultCount: 0, hasQuery: false })).toBe("guidance");
  });

  it("never shows results and guidance together", () => {
    const busy = resolvePickerPane({ busy: false, resultCount: 1, hasQuery: false });
    expect(busy).not.toBe("guidance");
    expect(busy).not.toBe("empty");
    expect(busy).not.toBe("loading");
  });
});
