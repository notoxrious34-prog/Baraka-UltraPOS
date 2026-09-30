/**
 * Cash session store: active OPEN/CLOSING session for this terminal.
 * Reactive source for the Topbar indicator and F12 checkout gating.
 * All money stays as centime strings; no number carries money.
 */

import { create } from "zustand";
import type { CashSession } from "@/types/cash";
import { ipcCashGetActiveSession } from "@/lib/ipc";

interface CashSessionState {
  session: CashSession | null;
  loading: boolean;
  error: string | null;
  lastLoadedAt: string | null;
  refresh: () => Promise<void>;
  setSession: (session: CashSession | null) => void;
  clear: () => void;
}

export const useCashSessionStore = create<CashSessionState>()((set) => ({
  session: null,
  loading: false,
  error: null,
  lastLoadedAt: null,
  refresh: async () => {
    set({ loading: true, error: null });
    try {
      // Token injected by the wrapper; no manual passing (TASK-BRK-021).
      const session = await ipcCashGetActiveSession();
      set({
        session,
        loading: false,
        lastLoadedAt: new Date().toISOString(),
      });
    } catch (error: unknown) {
      const code = typeof error === "string" ? error : "INTERNAL_ERROR";
      set({ loading: false, error: code });
    }
  },
  setSession: (session) =>
    set({ session, lastLoadedAt: new Date().toISOString(), error: null }),
  clear: () => set({ session: null, error: null, lastLoadedAt: null }),
}));

/** True when checkout is permitted (active session is OPEN). */
export function isCheckoutPermitted(session: CashSession | null): boolean {
  return session !== null && session.status === "OPEN";
}
