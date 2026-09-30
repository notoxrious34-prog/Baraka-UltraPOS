/**
 * Stock adjustment modal store (TASK-BRK-049).
 *
 * Physical inventory reconciliation over the existing `stock_adjust`
 * command: DELTA mode (signed variation) or NEW_PHYSICAL_COUNT mode
 * (shelf count minus recorded stock). All quantities are canonical micro
 * strings parsed to BigInt (zero floats). The backend re-validates
 * everything (positive magnitude, reason length, negative-stock
 * authorization); client-side guards are UX-only.
 */

import { create } from "zustand";
import {
  adjustStock,
  ipcGetStock,
  ipcSearchCatalog,
  listWarehouses,
  type WarehouseItem,
} from "@/lib/ipc";
import type { CatalogProduct } from "@/types/pos";
import { parseQtyToMicro } from "@/lib/money";
import { useSessionStore } from "@/stores/session-store";

export type StockAdjustReason = "INVENTAIRE" | "CASSE" | "PERTE" | "SURPLUS" | "AUTRE";

export type StockAdjustInputMode = "DELTA" | "NEW_PHYSICAL_COUNT";

export const STOCK_ADJUST_REASONS: readonly StockAdjustReason[] = [
  "INVENTAIRE",
  "CASSE",
  "PERTE",
  "SURPLUS",
  "AUTRE",
];

/** Human decimal input → micros (invalid → 0n, never throws). */
export function parseDecimalToMicro(text: string): bigint {
  const normalized = text.trim().startsWith("+") ? text.trim().slice(1) : text;
  try {
    return BigInt(parseQtyToMicro(normalized));
  } catch {
    return 0n;
  }
}

function parseMicroStrict(value: string): bigint {
  const trimmed = value.trim();
  if (!/^-?\d+$/.test(trimmed)) {
    return 0n;
  }
  try {
    return BigInt(trimmed);
  } catch {
    return 0n;
  }
}

export interface StockAdjustSuccess {
  movementId: string;
  movementType: string;
  previousMicro: bigint;
  newMicro: bigint;
}

interface StockAdjustModalStoreState {
  isOpen: boolean;
  selectedWarehouseId: string | null;
  warehouses: WarehouseItem[];
  isLoadingWarehouses: boolean;
  productId: string | null;
  productName: string | null;
  unitId: string | null;
  unitName: string | null;
  currentStockMicro: string;
  isLoadingStock: boolean;
  inputMode: StockAdjustInputMode;
  rawInputValue: string;
  computedDeltaMicro: string;
  computedNewStockMicro: string;
  reason: StockAdjustReason;
  notes: string;
  isSubmitting: boolean;
  errorMessage: string | null;
  successResult: StockAdjustSuccess | null;
  openModal: (prefill?: {
    warehouseId?: string;
    productId?: string;
    productName?: string;
    unitId?: string;
    unitName?: string;
    currentStockMicro?: string;
  }) => void;
  closeModal: () => void;
  setWarehouseId: (id: string) => void;
  setInputMode: (mode: StockAdjustInputMode) => void;
  setInputValue: (value: string) => void;
  setReason: (reason: StockAdjustReason) => void;
  setNotes: (notes: string) => void;
  submitAdjustment: () => Promise<boolean>;
  reset: () => void;
}

const initialState = {
  isOpen: false,
  selectedWarehouseId: null as string | null,
  warehouses: [] as WarehouseItem[],
  isLoadingWarehouses: false,
  productId: null as string | null,
  productName: null as string | null,
  unitId: null as string | null,
  unitName: null as string | null,
  currentStockMicro: "0",
  isLoadingStock: false,
  inputMode: "DELTA" as StockAdjustInputMode,
  rawInputValue: "",
  computedDeltaMicro: "0",
  computedNewStockMicro: "0",
  reason: "INVENTAIRE" as StockAdjustReason,
  notes: "",
  isSubmitting: false,
  errorMessage: null as string | null,
  successResult: null as StockAdjustSuccess | null,
};

function recompute(
  currentStockMicro: string,
  inputMode: StockAdjustInputMode,
  rawInputValue: string,
): { computedDeltaMicro: string; computedNewStockMicro: string } {
  const current = parseMicroStrict(currentStockMicro);
  if (inputMode === "DELTA") {
    const delta = parseDecimalToMicro(rawInputValue);
    return {
      computedDeltaMicro: delta.toString(),
      computedNewStockMicro: (current + delta).toString(),
    };
  }
  const target = parseDecimalToMicro(rawInputValue);
  return {
    computedDeltaMicro: (target - current).toString(),
    computedNewStockMicro: target.toString(),
  };
}

export const useStockAdjustModalStore = create<StockAdjustModalStoreState>()((set, get) => {
  async function refreshWarehouses(): Promise<void> {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      set({ errorMessage: "IPC_NO_ACTIVE_SESSION", warehouses: [] });
      return;
    }
    set({ isLoadingWarehouses: true });
    try {
      const warehouses = await listWarehouses(token);
      set((state) => ({
        warehouses,
        selectedWarehouseId:
          state.selectedWarehouseId ?? warehouses[0]?.id ?? null,
      }));
    } catch (failure: unknown) {
      set({
        errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR",
        warehouses: [],
      });
    } finally {
      set({ isLoadingWarehouses: false });
    }
  }

  async function refreshStock(): Promise<void> {
    const token = useSessionStore.getState().getSessionToken();
    const { productId, selectedWarehouseId } = get();
    if (token === null || productId === null || selectedWarehouseId === null) {
      return;
    }
    set({ isLoadingStock: true });
    try {
      const level = await ipcGetStock(token, productId, selectedWarehouseId);
      set((state) => ({
        currentStockMicro: level,
        ...recompute(level, state.inputMode, state.rawInputValue),
      }));
    } catch (failure: unknown) {
      set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
    } finally {
      set({ isLoadingStock: false });
    }
  }

  return {
    ...initialState,

    openModal: (prefill) => {
      set({
        ...initialState,
        isOpen: true,
        selectedWarehouseId: prefill?.warehouseId ?? null,
        productId: prefill?.productId ?? null,
        productName: prefill?.productName ?? null,
        unitId: prefill?.unitId ?? null,
        unitName: prefill?.unitName ?? null,
        currentStockMicro: prefill?.currentStockMicro ?? "0",
      });
      void refreshWarehouses().then(() => {
        const state = get();
        if (state.productId !== null && state.selectedWarehouseId !== null) {
          void refreshStock();
        }
      });
    },

    closeModal: () => {
      if (get().isSubmitting) {
        return;
      }
      set({ ...initialState });
    },

    setWarehouseId: (id) => {
      set({ selectedWarehouseId: id });
      void refreshStock();
    },

    setInputMode: (mode) => {
      set((state) => ({
        inputMode: mode,
        ...recompute(state.currentStockMicro, mode, state.rawInputValue),
      }));
    },

    setInputValue: (value) => {
      set((state) => ({
        rawInputValue: value,
        ...recompute(state.currentStockMicro, state.inputMode, value),
      }));
    },

    setReason: (reason) => {
      set({ reason });
    },

    setNotes: (notes) => {
      set({ notes });
    },

    submitAdjustment: async () => {
      const state = get();
      if (state.isSubmitting) {
        return false;
      }
      const session = useSessionStore.getState().session;
      if (session === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
        return false;
      }
      if (
        state.productId === null ||
        state.unitId === null ||
        state.selectedWarehouseId === null
      ) {
        return false;
      }
      const delta = parseMicroStrict(state.computedDeltaMicro);
      if (delta === 0n) {
        return false;
      }
      // Synchronous lock before the first await (TASK-BRK-032 precedent).
      set({ isSubmitting: true, errorMessage: null });
      try {
        const fresh = get();
        const magnitude = (delta < 0n ? -delta : delta).toString();
        const result = await adjustStock({
          session_token: session.token,
          product_id: fresh.productId ?? "",
          warehouse_id: fresh.selectedWarehouseId ?? "",
          unit_id: fresh.unitId ?? "",
          quantity_micro: magnitude,
          direction: delta < 0n ? "DECREASE" : "INCREASE",
          reason: `${fresh.reason}${fresh.notes.trim() === "" ? "" : ` — ${fresh.notes.trim()}`}`,
          authorize_negative_stock: false,
        });
        const previous = parseMicroStrict(fresh.currentStockMicro);
        set({
          currentStockMicro: result.new_quantity_base_micro,
          ...recompute(result.new_quantity_base_micro, fresh.inputMode, fresh.rawInputValue),
          successResult: {
            movementId: result.movement_id,
            movementType: delta < 0n ? "ADJUSTMENT_NEGATIVE" : "ADJUSTMENT_POSITIVE",
            previousMicro: previous,
            newMicro: parseMicroStrict(result.new_quantity_base_micro),
          },
        });
        return true;
      } catch (failure: unknown) {
        set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        return false;
      } finally {
        set({ isSubmitting: false });
      }
    },

    reset: () => {
      set({ ...initialState });
    },
  };
});

/** Product search helper for the adjustment picker (session-guarded). */
export async function searchAdjustProducts(query: string): Promise<CatalogProduct[]> {
  const token = useSessionStore.getState().getSessionToken();
  if (token === null || query.trim() === "") {
    return [];
  }
  return ipcSearchCatalog(token, query.trim(), 12);
}
