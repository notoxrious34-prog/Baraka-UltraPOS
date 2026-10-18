/**
 * Supplier module store: directory search results and selection.
 * All money stays centime strings; backend code on error strings.
 */

import { create } from "zustand";
import type {
  SupplierFormInput,
  SupplierUpdateInput,
  SupplierView,
} from "@/types/supplier";
import { ipcCreateSupplier, ipcListSuppliers, ipcSearchSuppliers, ipcUpdateSupplier } from "@/lib/ipc";

interface SupplierState {
  suppliers: SupplierView[];
  selected: SupplierView | null;
  loading: boolean;
  error: string | null;
  search: (sessionToken: string, query: string) => Promise<void>;
  create: (sessionToken: string, input: SupplierFormInput) => Promise<SupplierView>;
  update: (
    sessionToken: string,
    supplierId: string,
    input: SupplierUpdateInput,
  ) => Promise<SupplierView>;
  select: (supplier: SupplierView | null) => void;
  clear: () => void;
}

export const useSupplierStore = create<SupplierState>()((set, get) => ({
  suppliers: [],
  selected: null,
  loading: false,
  error: null,
  search: async (sessionToken, query) => {
    set({ loading: true, error: null });
    try {
      // Empty query lists the directory (TASK-BRK-057); non-empty searches.
      const suppliers =
        query.trim() === ""
          ? await ipcListSuppliers(sessionToken)
          : await ipcSearchSuppliers(sessionToken, query, 50);
      set({ suppliers, loading: false });
    } catch (error: unknown) {
      const code = typeof error === "string" ? error : "INTERNAL_ERROR";
      set({ loading: false, error: code });
    }
  },
  create: async (sessionToken, input) => {
    const view = await ipcCreateSupplier(sessionToken, input);
    set({ error: null });
    await get().search(sessionToken, view.name);
    return view;
  },
  update: async (sessionToken, supplierId, input) => {
    const view = await ipcUpdateSupplier(sessionToken, supplierId, input);
    set({ error: null });
    const { suppliers } = get();
    set({
      suppliers: suppliers.map((candidate) => (candidate.id === view.id ? view : candidate)),
      selected: get().selected?.id === view.id ? view : get().selected,
    });
    return view;
  },
  select: (supplier) => set({ selected: supplier }),
  clear: () => set({ suppliers: [], selected: null, error: null }),
}));
