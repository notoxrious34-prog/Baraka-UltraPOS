/**
 * Non-POS view-switch store (TASK-BRK-011 navigation pattern).
 *
 * Investigation point 6 confirmed no rail existed: the authenticated root
 * was a LoginScreen/PosShell toggle. This store extends that exact pattern
 * with a third destination without adding a router dependency.
 */

import { create } from "zustand";

export type AppView = "pos" | "stock" | "customers" | "documents" | "suppliers" | "reports" | "catalog" | "users";

interface AppViewState {
  view: AppView;
  setView: (view: AppView) => void;
}

export const useAppViewStore = create<AppViewState>()((set) => ({
  view: "pos",
  setView: (view) => set({ view }),
}));
