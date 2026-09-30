import type { AppHealth, CommandErrorDto } from "@/types";

/** Detect whether the page runs inside the Tauri shell. */
export function detectRuntime(): "tauri" | "browser" {
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    return "tauri";
  }
  return "browser";
}

/**
 * Query the Rust core health command.
 * Uses dynamic import so browser dev mode (without the Tauri bridge)
 * degrades to a local baseline snapshot instead of throwing.
 */
export async function fetchAppHealth(): Promise<AppHealth> {
  const runtime = detectRuntime();
  if (runtime === "tauri") {
    const { invoke } = await import("@tauri-apps/api/core");
    interface HealthWire {
      version: string;
      captured_at: string;
      offline_ready: boolean;
    }
    const wire = await invoke<HealthWire>("app_health");
    return {
      version: wire.version,
      capturedAt: wire.captured_at,
      offlineReady: wire.offline_ready,
    };
  }
  return {
    version: "0.1.0",
    capturedAt: new Date().toISOString(),
    offlineReady: true,
  };
}

/** Narrow an unknown throw into a displayable French message. */
export function toDisplayError(error: unknown): string {
  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    const message: unknown = (error as { message: unknown }).message;
    if (typeof message === "string" && message.trim().length > 0) {
      return message;
    }
  }
  return "Erreur inconnue du noyau local";
}

/** Type guard for command error DTOs crossing the IPC bridge. */
export function isCommandErrorDto(value: unknown): value is CommandErrorDto {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  return typeof record["code"] === "string" && typeof record["message"] === "string";
}
