import { useCallback, useState } from "react";
import type { AppHealth } from "@/types";
import { detectRuntime, fetchAppHealth, toDisplayError } from "@/lib/tauri";

export interface AppStatusState {
  status: "ready" | "checking" | "unreachable";
  health: AppHealth | null;
  lastError: string | null;
  runtime: "tauri" | "browser";
  refresh: () => Promise<void>;
}

/** Load the Rust core health snapshot; safe in browser and Tauri runtimes. */
export function useAppStatus(): AppStatusState {
  const [health, setHealth] = useState<AppHealth | null>(null);
  const [lastError, setLastError] = useState<string | null>(null);
  const [checking, setChecking] = useState<boolean>(false);
  const runtime = detectRuntime();

  const refresh = useCallback(async (): Promise<void> => {
    setChecking(true);
    try {
      const snapshot = await fetchAppHealth();
      setHealth(snapshot);
      setLastError(null);
    } catch (error: unknown) {
      setHealth(null);
      setLastError(toDisplayError(error));
    } finally {
      setChecking(false);
    }
  }, []);

  const status = checking ? "checking" : health !== null ? "ready" : "unreachable";

  return { status, health, lastError, runtime, refresh };
}
