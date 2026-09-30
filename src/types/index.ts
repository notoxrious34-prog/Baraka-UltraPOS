/** Shared frontend types. All money is integer centimes (1 DZD = 100). */

export type RuntimeKind = "tauri" | "browser";

export interface AppHealth {
  version: string;
  capturedAt: string;
  offlineReady: boolean;
}

export interface CommandErrorDto {
  code: string;
  message: string;
}

export type AppStatus = "ready" | "checking" | "unreachable";

export interface CockpitSnapshot {
  status: AppStatus;
  runtime: RuntimeKind;
  health: AppHealth | null;
  lastError: string | null;
}
