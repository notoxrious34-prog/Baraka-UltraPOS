/**
 * Auth bridge: login, logout, first-run probe, first-admin bootstrap.
 * Split from `ipc.ts` (Wave 3 frontend twin of the BRK-106/107/112 splits);
 * byte-identical motion, hub re-exports preserve the `@/lib/ipc` path.
 */

import type { SessionInfo } from "@/types/pos";
import { call } from "./ipc-kernel";

interface LoginWire {
  session_token: string;
  user_id: string;
  username: string;
  role: string;
  permissions: string[];
  expires_at: string;
}

/** Authenticate; resolves a typed session or rejects with a wire code. */
export async function ipcLogin(username: string, password: string): Promise<SessionInfo> {
  const wire = await call<LoginWire>("auth_login", {
    request: { username, password },
  });
  return {
    token: wire.session_token,
    userId: wire.user_id,
    username: wire.username,
    role: wire.role,
    permissions: wire.permissions,
    expiresAt: wire.expires_at,
  };
}

/** Revoke a session (idempotent server-side; token injected by the wrapper). */
export async function ipcLogout(): Promise<void> {
  await call<void>("auth_logout", { request: {} });
}

/** First-run probe: true while the users table is empty (no session needed). */
export async function ipcCheckFirstRunStatus(): Promise<boolean> {
  const wire = await call<{ requires_setup: boolean }>("check_first_run_status", {});
  return wire.requires_setup;
}

/**
 * One-time first Admin creation; resolves the stored id and name.
 * The caller then logs in through the standard login flow (no session
 * token is minted here).
 */
export async function ipcBootstrapFirstAdmin(
  username: string,
  password: string,
  commandId: string,
): Promise<{ userId: string; username: string }> {
  const wire = await call<{ user_id: string; username: string }>("bootstrap_first_admin", {
    request: { command_id: commandId, username, password },
  });
  return { userId: wire.user_id, username: wire.username };
}
