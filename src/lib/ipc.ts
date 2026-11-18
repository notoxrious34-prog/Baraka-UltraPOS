/**
 * Typed Tauri IPC bridge (ADR-004 wire contract) — hub.
 *
 * Domain modules (Wave 3 frontend twin of the BRK-106/107/112 splits):
 * kernel (transport + session edge) / auth / catalog / inventory / users /
 * sales / customers / printing / cash / suppliers / reports / backups.
 * Every consumer keeps importing from `@/lib/ipc`; the flat path is frozen.
 */

import type { Locale } from "@/types/pos";

export * from "./ipc-kernel";
export * from "./ipc-auth";
export * from "./ipc-catalog";
export * from "./ipc-inventory";
export * from "./ipc-users";
export * from "./ipc-sales";
export * from "./ipc-customers";
export * from "./ipc-printing";
export * from "./ipc-cash";
export * from "./ipc-suppliers";
export * from "./ipc-reports";
export * from "./ipc-backups";

export type { Locale };
