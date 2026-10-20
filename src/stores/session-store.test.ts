/**
 * Session permission-set contract tests (TASK-BRK-070 / ADR-100).
 *
 * The login wire contract exposes the role's effective permission set as a
 * sorted string vector; the session store persists it and answers
 * `hasPermission` checks so UI gates read granular capabilities instead of
 * comparing role names.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useSessionStore } from "@/stores/session-store";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

/** The bridge probe requires a `window` carrying the Tauri marker. */
function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function clearTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  delete globals.window;
  delete globals.__TAURI_INTERNALS__;
}

function resetStore(): void {
  useSessionStore.setState({
    session: null,
    warehouses: [],
    warehouse: null,
    permissions: [],
    authBusy: false,
    authError: null,
  });
}

describe("session permission set (ADR-100)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
    resetStore();
  });

  afterEach(() => {
    clearTauriWindow();
  });

  it("starts with no permissions and denies every capability", () => {
    const state = useSessionStore.getState();
    expect(state.permissions).toEqual([]);
    expect(state.hasPermission("USER_MANAGE")).toBe(false);
    expect(state.hasPermission("POS_SELL")).toBe(false);
  });

  it("login stores the wire permission vector and grants only listed capabilities", async () => {
    const wirePermissions = ["SYSTEM_SETTINGS_MANAGE", "USER_MANAGE"];
    invokeMock.mockResolvedValueOnce({
      session_token: "tok-perm-1",
      user_id: "u-admin-1",
      username: "admin",
      role: "ADMIN",
      expires_at: "2026-01-02T00:00:00Z",
      permissions: wirePermissions,
    });
    invokeMock.mockResolvedValueOnce({ warehouses: [] });
    invokeMock.mockResolvedValueOnce(null);

    await useSessionStore.getState().login("admin", "secret");

    const state = useSessionStore.getState();
    expect(state.session?.permissions).toEqual(wirePermissions);
    expect(state.permissions).toEqual(wirePermissions);
    expect(state.hasPermission("USER_MANAGE")).toBe(true);
    expect(state.hasPermission("SYSTEM_SETTINGS_MANAGE")).toBe(true);
    expect(state.hasPermission("POS_SELL")).toBe(false);
  });

  it("logout clears the stored permissions and denies every capability", async () => {
    useSessionStore.setState({
      session: {
        token: "tok-perm-2",
        userId: "u-admin-2",
        username: "admin",
        role: "ADMIN",
        expiresAt: "2026-01-02T00:00:00Z",
        permissions: ["USER_MANAGE"],
      },
      permissions: ["USER_MANAGE"],
    });
    invokeMock.mockResolvedValueOnce(undefined);

    await useSessionStore.getState().logout();

    const state = useSessionStore.getState();
    expect(state.session).toBeNull();
    expect(state.permissions).toEqual([]);
    expect(state.hasPermission("USER_MANAGE")).toBe(false);
  });
});
