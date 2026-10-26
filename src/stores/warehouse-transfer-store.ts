/**
 * Warehouse transfer cockpit store (TASK-BRK-051).
 *
 * Inter-warehouse transfers (one atomic OUT/IN pair per line server-side;
 * multi-line submissions dispatch sequentially and stop at the first
 * failure) plus ADMIN-only warehouse creation. Quantities travel
 * exclusively as canonical micro strings parsed to BigInt (zero floats).
 * The backend re-validates everything (distinct active endpoints,
 * positive quantities, source coverage); client-side guards are UX-only.
 */

import { create } from "zustand";
import {
  createWarehouse,
  ipcDeleteWarehouse,
  ipcSearchCatalog,
  ipcUpdateWarehouse,
  ipcWarehouseStockPresence,
  listWarehouses,
  transferStock,
  type WarehouseItem,
} from "@/lib/ipc";
import type { CatalogProduct } from "@/types/pos";
import { parseQtyToMicro } from "@/lib/money";
import { useSessionStore } from "@/stores/session-store";

export type WarehouseTransferTab = "transfer" | "create_warehouse" | "manage";

export interface TransferLineItem {
  productId: string;
  productName: string;
  sku: string;
  unitId: string;
  unitName: string;
  /** Transfer magnitude in selling-unit micros (> 0n to dispatch). */
  quantityMicro: bigint;
}

/** Human selling-unit input → micros (invalid → 0n, never throws). */
export function parseTransferQtyInput(text: string): bigint {
  try {
    return BigInt(parseQtyToMicro(text));
  } catch {
    return 0n;
  }
}

/** Submit gate: distinct endpoints plus at least one positive line. */
export function canSubmitTransfer(args: {
  sourceWarehouseId: string | null;
  destinationWarehouseId: string | null;
  lines: TransferLineItem[];
  isSubmitting: boolean;
}): boolean {
  return (
    !args.isSubmitting &&
    args.sourceWarehouseId !== null &&
    args.destinationWarehouseId !== null &&
    args.sourceWarehouseId !== args.destinationWarehouseId &&
    args.lines.some((line) => line.quantityMicro > 0n)
  );
}

interface WarehouseTransferStoreState {
  isOpen: boolean;
  activeTab: WarehouseTransferTab;
  warehouses: WarehouseItem[];
  isLoadingWarehouses: boolean;
  /** Non-zero level counts by depot id (BRK-095 presence wire). */
  stockPresence: Record<string, number>;
  isLoadingPresence: boolean;
  sourceWarehouseId: string | null;
  destinationWarehouseId: string | null;
  transferItems: TransferLineItem[];
  transferNotes: string;
  isSubmitting: boolean;
  errorMessage: string | null;
  successMessage: string | null;
  newWarehouseCode: string;
  newWarehouseName: string;
  openModal: () => Promise<void>;
  closeModal: () => void;
  setActiveTab: (tab: WarehouseTransferTab) => void;
  loadWarehouses: () => Promise<void>;
  /** Read-only presence snapshot for the delete pre-check (zero writes). */
  loadStockPresence: () => Promise<void>;
  setSourceWarehouse: (id: string) => void;
  setDestinationWarehouse: (id: string) => void;
  addTransferItem: (
    item: Omit<TransferLineItem, "quantityMicro">,
    initialQtyMicro?: bigint,
  ) => void;
  updateTransferItemQty: (productId: string, quantityMicro: bigint) => void;
  removeTransferItem: (productId: string) => void;
  clearTransferItems: () => void;
  setTransferNotes: (notes: string) => void;
  submitTransfer: () => Promise<boolean>;
  setNewWarehouseCode: (code: string) => void;
  setNewWarehouseName: (name: string) => void;
  submitCreateWarehouse: () => Promise<boolean>;
  /** Sparse rename/(de)activate; refreshes the depot list on success. */
  submitUpdateWarehouse: (
    id: string,
    input: { name?: string; isActive?: boolean },
  ) => Promise<boolean>;
  /** Dual-track delete; resolves the backend mode (null = rejected). */
  submitDeleteWarehouse: (id: string, reason: string) => Promise<"hard_deleted" | "deactivated" | null>;
  reset: () => void;
}

const initialState = {
  isOpen: false,
  activeTab: "transfer" as WarehouseTransferTab,
  warehouses: [] as WarehouseItem[],
  isLoadingWarehouses: false,
  stockPresence: {} as Record<string, number>,
  isLoadingPresence: false,
  sourceWarehouseId: null as string | null,
  destinationWarehouseId: null as string | null,
  transferItems: [] as TransferLineItem[],
  transferNotes: "",
  isSubmitting: false,
  errorMessage: null as string | null,
  successMessage: null as string | null,
  newWarehouseCode: "",
  newWarehouseName: "",
};

/** Default endpoint pairing: first warehouse drives both selectors. */
export function defaultEndpoints(warehouses: WarehouseItem[]): {
  source: string | null;
  destination: string | null;
} {
  const first = warehouses[0]?.id ?? null;
  const second = warehouses[1]?.id ?? first;
  return { source: first, destination: second };
}

export const useWarehouseTransferStore = create<WarehouseTransferStoreState>()(
  (set, get) => ({
    ...initialState,

    openModal: async () => {
      set({ ...initialState, isOpen: true });
      await get().loadWarehouses();
    },

    closeModal: () => {
      if (get().isSubmitting) {
        return;
      }
      set({ ...initialState });
    },

    setActiveTab: (tab) => {
      set({ activeTab: tab, errorMessage: null, successMessage: null });
    },

    loadWarehouses: async () => {
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION", warehouses: [] });
        return;
      }
      set({ isLoadingWarehouses: true, errorMessage: null });
      try {
        const warehouses = await listWarehouses(token);
        const endpoints = defaultEndpoints(warehouses);
        set({
          warehouses,
          sourceWarehouseId: get().sourceWarehouseId ?? endpoints.source,
          destinationWarehouseId:
            get().destinationWarehouseId ?? endpoints.destination,
        });
      } catch (failure: unknown) {
        set({
          errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR",
          warehouses: [],
        });
      } finally {
        set({ isLoadingWarehouses: false });
      }
    },

    loadStockPresence: async () => {
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        return;
      }
      set({ isLoadingPresence: true });
      try {
        const rows = await ipcWarehouseStockPresence(token);
        const stockPresence: Record<string, number> = {};
        for (const row of rows) {
          stockPresence[row.warehouseId] = row.nonzeroLevels;
        }
        set({ stockPresence });
      } catch {
        // Fail-open to the pre-existing behavior: buttons stay enabled and
        // the engine rejection remains the backstop. Never surface a
        // read-path failure as a red banner on the manage tab.
        set({ stockPresence: {} });
      } finally {
        set({ isLoadingPresence: false });
      }
    },

    setSourceWarehouse: (id) => {
      set({ sourceWarehouseId: id });
    },

    setDestinationWarehouse: (id) => {
      set({ destinationWarehouseId: id });
    },

    addTransferItem: (item, initialQtyMicro = 10_000n) => {
      set((state) => {
        if (state.transferItems.some((line) => line.productId === item.productId)) {
          return {};
        }
        return {
          transferItems: [
            ...state.transferItems,
            { ...item, quantityMicro: initialQtyMicro },
          ],
        };
      });
    },

    updateTransferItemQty: (productId, quantityMicro) => {
      set((state) => ({
        transferItems: state.transferItems.map((line) =>
          line.productId === productId
            ? { ...line, quantityMicro: quantityMicro < 0n ? 0n : quantityMicro }
            : line,
        ),
      }));
    },

    removeTransferItem: (productId) => {
      set((state) => ({
        transferItems: state.transferItems.filter((line) => line.productId !== productId),
      }));
    },

    clearTransferItems: () => {
      set({ transferItems: [] });
    },

    setTransferNotes: (notes) => {
      set({ transferNotes: notes });
    },

    submitTransfer: async () => {
      const state = get();
      const session = useSessionStore.getState().session;
      if (session === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
        return false;
      }
      if (
        !canSubmitTransfer({
          sourceWarehouseId: state.sourceWarehouseId,
          destinationWarehouseId: state.destinationWarehouseId,
          lines: state.transferItems,
          isSubmitting: state.isSubmitting,
        })
      ) {
        return false;
      }
      // Synchronous lock before the first await (TASK-BRK-032 precedent).
      set({ isSubmitting: true, errorMessage: null, successMessage: null });
      try {
        const fresh = get();
        const sourceId = fresh.sourceWarehouseId;
        const destinationId = fresh.destinationWarehouseId;
        if (sourceId === null || destinationId === null) {
          return false;
        }
        const lines = fresh.transferItems.filter((line) => line.quantityMicro > 0n);
        const trimmedNotes = fresh.transferNotes.trim();
        // One atomic OUT/IN pair per line; stop at the first rejection so
        // a short source never leaves a half-applied manifest ambiguous.
        for (const line of lines) {
          await transferStock({
            session_token: session.token,
            source_warehouse_id: sourceId,
            destination_warehouse_id: destinationId,
            product_id: line.productId,
            unit_id: line.unitId,
            quantity_micro: line.quantityMicro.toString(),
            reason: trimmedNotes === "" ? null : trimmedNotes,
          });
        }
        set({ successMessage: "transferSuccess", transferItems: [] });
        return true;
      } catch (failure: unknown) {
        set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        return false;
      } finally {
        set({ isSubmitting: false });
      }
    },

    setNewWarehouseCode: (code) => {
      set({ newWarehouseCode: code.toUpperCase() });
    },

    setNewWarehouseName: (name) => {
      set({ newWarehouseName: name });
    },

    submitCreateWarehouse: async () => {
      const state = get();
      const session = useSessionStore.getState().session;
      if (session === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
        return false;
      }
      if (state.isSubmitting) {
        return false;
      }
      const code = state.newWarehouseCode.trim();
      const name = state.newWarehouseName.trim();
      if (code === "" || name === "") {
        return false;
      }
      set({ isSubmitting: true, errorMessage: null, successMessage: null });
      try {
        await createWarehouse({ session_token: session.token, code, name });
        await get().loadWarehouses();
        set({
          successMessage: "createWarehouseSuccess",
          newWarehouseCode: "",
          newWarehouseName: "",
        });
        return true;
      } catch (failure: unknown) {
        set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        return false;
      } finally {
        set({ isSubmitting: false });
      }
    },

    submitUpdateWarehouse: async (id, input) => {
      const session = useSessionStore.getState().session;
      if (session === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
        return false;
      }
      if (get().isSubmitting) {
        return false;
      }
      set({ isSubmitting: true, errorMessage: null, successMessage: null });
      try {
        await ipcUpdateWarehouse(session.token, id, input);
        await get().loadWarehouses();
        set({ successMessage: "settings.saved" });
        return true;
      } catch (failure: unknown) {
        set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        return false;
      } finally {
        set({ isSubmitting: false });
      }
    },

    submitDeleteWarehouse: async (id, reason) => {
      const session = useSessionStore.getState().session;
      if (session === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
        return null;
      }
      if (get().isSubmitting) {
        return null;
      }
      set({ isSubmitting: true, errorMessage: null, successMessage: null });
      try {
        const result = await ipcDeleteWarehouse(session.token, id, reason);
        await get().loadWarehouses();
        return result.mode;
      } catch (failure: unknown) {
        set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        return null;
      } finally {
        set({ isSubmitting: false });
      }
    },

    reset: () => {
      set({ ...initialState });
    },
  }),
);

/** Product search helper for the transfer picker (session-guarded). */
export async function searchTransferProducts(query: string): Promise<CatalogProduct[]> {
  const token = useSessionStore.getState().getSessionToken();
  if (token === null || query.trim() === "") {
    return [];
  }
  return ipcSearchCatalog(token, query.trim(), 12);
}
