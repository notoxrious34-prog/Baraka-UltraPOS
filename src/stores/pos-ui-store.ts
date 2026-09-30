/**
 * POS UI state: locale, modals, search, selection, banners.
 */

import { create } from "zustand";
import type { ReactNode } from "react";
import type { ActiveModal, Locale, SaleReceipt } from "@/types/pos";

interface PosUiState {
  locale: Locale;
  activeModal: ActiveModal;
  searchQuery: string;
  selectedLineIndex: number;
  lastReceipt: SaleReceipt | null;
  banner: string | null;
  setLocale: (locale: Locale) => void;
  toggleLocale: () => void;
  openModal: (modal: Exclude<ActiveModal, "none">) => void;
  closeModal: () => void;
  setSearchQuery: (query: string) => void;
  setSelectedLineIndex: (index: number) => void;
  moveSelection: (delta: number, lineCount: number) => void;
  setLastReceipt: (receipt: SaleReceipt | null) => void;
  setBanner: (banner: string | null) => void;
}

export const usePosUiStore = create<PosUiState>()((set) => ({
  locale: "ar-DZ",
  activeModal: "none",
  searchQuery: "",
  selectedLineIndex: 0,
  lastReceipt: null,
  banner: null,
  setLocale: (locale) => set({ locale }),
  toggleLocale: () =>
    set((state) => ({ locale: state.locale === "ar-DZ" ? "fr-DZ" : "ar-DZ" })),
  openModal: (modal) => set({ activeModal: modal, banner: null }),
  closeModal: () => set({ activeModal: "none" }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setSelectedLineIndex: (index) => set({ selectedLineIndex: Math.max(0, index) }),
  moveSelection: (delta, lineCount) =>
    set((state) => {
      if (lineCount <= 0) {
        return { selectedLineIndex: 0 };
      }
      const next = state.selectedLineIndex + delta;
      return { selectedLineIndex: Math.min(Math.max(next, 0), lineCount - 1) };
    }),
  setLastReceipt: (receipt) => set({ lastReceipt: receipt }),
  setBanner: (banner) => set({ banner }),
}));

/** Props shared by every modal shell (Escape handled globally). */
export interface ModalShellProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
}
