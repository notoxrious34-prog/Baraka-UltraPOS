/**
 * Users bridge: operator directory management (Admin only server-side).
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import { call } from "./ipc-kernel";

/** Managed operator view (mirrors UserSummaryDto; never carries hashes). */
export interface UserInfo {
  id: string;
  username: string;
  role: "VENDEUR" | "CAISSIER" | "GERANT" | "ADMIN";
  is_active: boolean;
  has_pin: boolean;
  created_at: string;
  updated_at: string;
}

/** List every managed operator (Admin only). */
export async function ipcListUsers(sessionToken: string): Promise<UserInfo[]> {
  return call<UserInfo[]>("users_list", { session_token: sessionToken });
}

/** Create a validated operator; resolves the stored view (Admin only). */
export async function ipcCreateUser(
  sessionToken: string,
  payload: { username: string; password: string; pin?: string; role: string },
): Promise<UserInfo> {
  return call<UserInfo>("users_create", {
    request: {
      session_token: sessionToken,
      username: payload.username,
      password: payload.password,
      pin: payload.pin ?? null,
      role: payload.role,
    },
  });
}

/** Patch an operator's role/active flag; resolves the fresh view (Admin only). */
export async function ipcUpdateUser(
  sessionToken: string,
  payload: { user_id: string; role?: string; is_active?: boolean },
): Promise<UserInfo> {
  return call<UserInfo>("users_update", {
    request: {
      session_token: sessionToken,
      user_id: payload.user_id,
      role: payload.role ?? null,
      is_active: payload.is_active ?? null,
    },
  });
}

/** Replace an operator's password and/or PIN (Admin only). */
export async function ipcResetUserCredentials(
  sessionToken: string,
  payload: { user_id: string; password?: string; pin?: string },
): Promise<void> {
  await call<unknown>("users_reset_credentials", {
    request: {
      session_token: sessionToken,
      user_id: payload.user_id,
      password: payload.password ?? null,
      pin: payload.pin ?? null,
    },
  });
}
