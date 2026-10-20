/**
 * Session store: authenticated operator + active warehouse.
 * On login the first active warehouse is selected automatically.
 */

import { create } from "zustand";
import type { SessionInfo, WarehouseInfo } from "@/types/pos";
import { ipcBootstrapFirstAdmin, ipcListWarehouses, ipcLogin, ipcLogout } from "@/lib/ipc";
import { newCommandId } from "@/lib/money";
import { useCashSessionStore } from "@/stores/cash-session-store";

interface SessionState {
  session: SessionInfo | null;
  /** Effective permission set of the active session (ADR-100). */
  permissions: string[];
  warehouses: WarehouseInfo[];
  warehouse: WarehouseInfo | null;
  authBusy: boolean;
  authError: string | null;
  login: (username: string, password: string) => Promise<void>;
  bootstrap: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  loadWarehouses: () => Promise<void>;
  selectWarehouse: (id: string) => void;
  /** Synchronously read the active session token, if any (IPC edge source). */
  getSessionToken: () => string | null;
  /** Granular capability check against the stored permission set. */
  hasPermission: (permission: string) => boolean;
}

export const useSessionStore = create<SessionState>()((set, get) => ({
  session: null,
  permissions: [],
  warehouses: [],
  warehouse: null,
  authBusy: false,
  authError: null,
  login: async (username, password) => {
    set({ authBusy: true, authError: null });
    try {
      const session = await ipcLogin(username.trim(), password);
      // The token lands in the store HERE, before any subsequent IPC call,
      // so the wrapper's automatic injection is already sourced correctly
      // for loadWarehouses() and the cash refresh below (ordering matters).
      // The effective permission set lands alongside the session (ADR-100).
      set({ session, permissions: session.permissions, authBusy: false });
      await get().loadWarehouses();
      await useCashSessionStore.getState().refresh();
    } catch (error: unknown) {
      const code = typeof error === "string" ? error : "INTERNAL_ERROR";
      // Any post-login failure (including IPC_NO_ACTIVE_SESSION) surfaces
      // visibly via authError and always releases the loading flag — no
      // indefinite loading hang is constructible on this path.
      set({ authBusy: false, authError: code, session: null, permissions: [] });
      throw error;
    }
  },
  bootstrap: async (username, password) => {
    set({ authBusy: true, authError: null });
    try {
      await ipcBootstrapFirstAdmin(username.trim(), password, newCommandId());
      // Enter the authenticated shell exactly as a successful login would:
      // the standard login flow mints the session and loads warehouses.
      await get().login(username, password);
    } catch (error: unknown) {
      const code = typeof error === "string" ? error : "INTERNAL_ERROR";
      set({ authBusy: false, authError: code, session: null, permissions: [] });
      throw error;
    }
  },
  logout: async () => {
    const { session } = get();
    if (session !== null) {
      try {
        // Token injected by the wrapper from the still-present session.
        await ipcLogout();
      } catch {
        // Revocation is idempotent server-side; always clear locally.
      }
    }
    useCashSessionStore.getState().clear();
    set({ session: null, permissions: [], warehouse: null, warehouses: [], authError: null });
  },
  loadWarehouses: async () => {
    const { session } = get();
    if (session === null) {
      return;
    }
    // Token injected by the wrapper; no manual passing (TASK-BRK-021).
    const warehouses = await ipcListWarehouses();
    const active = warehouses.filter((warehouse) => warehouse.isActive);
    const current = get().warehouse;
    const stillValid = current !== null && active.some((w) => w.id === current.id);
    set({
      warehouses,
      warehouse: stillValid ? current : (active[0] ?? null),
    });
  },
  selectWarehouse: (id) =>
    set((state) => ({
      warehouse: state.warehouses.find((warehouse) => warehouse.id === id) ?? state.warehouse,
    })),
  getSessionToken: () => get().session?.token ?? null,
  hasPermission: (permission) => get().permissions.includes(permission),
}));
