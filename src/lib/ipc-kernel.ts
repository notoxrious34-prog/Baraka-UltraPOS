/**
 * Typed Tauri IPC bridge (ADR-004 wire contract).
 *
 * Backend command mapping (sovereign contract):
 * - auth_login / auth_logout / check_first_run_status /
 *   bootstrap_first_admin / sales_checkout / sales_preview_totals /
 *   sales_create_draft / warehouses_list / stock_get — exact backend names.
 * - catalog_search — the backend search command (the task text calls it
 *   `products_search`; the sovereign backend exposes `catalog_search` and
 *   Rust code is frozen, so this bridge targets the real name).
 */

import { invoke } from "@tauri-apps/api/core";
import { useSessionStore } from "@/stores/session-store";

function bridgeError(): Error {
  const error = new Error("BRIDGE_UNAVAILABLE");
  return error;
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw bridgeError();
  }
  return invoke<T>(command, withSessionToken(command, args));
}

/**
 * Commands legitimately callable before any session exists. Every other
 * command receives the active session token automatically (single source of
 * truth: the session store — ADR-045 edge-centralization precedent).
 */
const PRE_AUTH_COMMANDS: readonly string[] = [
  "app_health",
  "auth_login",
  "check_first_run_status",
  "bootstrap_first_admin",
];

/**
 * Merge the store-owned session token into invoke args, overwriting any
 * caller-supplied value (single source of truth). Exactly ONE rule applies
 * project-wide: the key is always `session_token`, at the top level (bare
 * scalar parameters) and inside nested `request` DTO objects alike. This
 * holds because every #[tauri::command] in commands/src carries
 * rename_all = "snake_case" (uniform wire policy), while DTO struct fields
 * deserialize via serde with exact Rust field names (likewise snake_case).
 * Throws `IPC_NO_ACTIVE_SESSION` instead of invoking tokenless, so a missing
 * session fails fast and legibly at this edge rather than as a backend
 * arg-validation rejection downstream.
 */
function withSessionToken(
  command: string,
  args: Record<string, unknown>,
): Record<string, unknown> {
  if (PRE_AUTH_COMMANDS.includes(command)) {
    return args;
  }
  const token = useSessionStore.getState().getSessionToken();
  if (token === null) {
    throw new Error("IPC_NO_ACTIVE_SESSION");
  }
  const merged: Record<string, unknown> = { ...args, session_token: token };
  const nested = merged.request;
  if (typeof nested === "object" && nested !== null && !Array.isArray(nested)) {
    merged.request = { ...(nested as Record<string, unknown>), session_token: token };
  }
  return merged;
}

/** Dual-track deletion outcome (ADR-092): purge or soft-deactivation. */
export interface DeleteEntityResult {
  id: string;
  mode: "hard_deleted" | "deactivated";
  message: string;
}
