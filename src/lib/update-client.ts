/**
 * Update Center client (GitHub Releases auto-update).
 *
 * Thin orchestration over `@tauri-apps/plugin-updater`: probe the signed
 * `latest.json` channel, download with progress, then install only after a
 * verified pre-update backup. The plugin enforces the signature (embedded
 * pubkey) and refuses downgrades; this module maps transport failures to
 * stable codes and keeps every byte-count in integers (bytes are counts,
 * never money — but the no-float discipline holds everywhere).
 *
 * All plugin entry points are injectable so the state machine pins without
 * touching the network.
 */

import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { createBackup, type BackupMetadata } from "./ipc";

/** Persisted check preference (this machine only — a UX preference, not business data). */
export const UPDATE_MODE_KEY = "baraka.update.mode";

export type UpdateMode = "auto" | "manual";

/** Storage backend for the mode preference (injectable for tests). */
export interface ModeStorage {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
}

function defaultModeStorage(): ModeStorage {
  try {
    if (typeof window !== "undefined" && window.localStorage) {
      return window.localStorage;
    }
  } catch {
    // Locked profile / non-DOM runtime: fall through to memory.
  }
  const memory = new Map<string, string>();
  return {
    getItem: (key) => memory.get(key) ?? null,
    setItem: (key, value) => {
      memory.set(key, value);
    },
  };
}

export function readUpdateMode(storage: ModeStorage = defaultModeStorage()): UpdateMode {
  try {
    return storage.getItem(UPDATE_MODE_KEY) === "manual" ? "manual" : "auto";
  } catch {
    return "auto";
  }
}

export function writeUpdateMode(mode: UpdateMode, storage: ModeStorage = defaultModeStorage()): void {
  try {
    storage.setItem(UPDATE_MODE_KEY, mode);
  } catch {
    // Preference storage is best-effort (private mode, locked profile).
  }
}

export interface UpdateAvailable {
  version: string;
  currentVersion: string;
  notes: string | null;
  date: string | null;
}

export interface UpdateDownloadProgress {
  downloadedBytes: number;
  totalBytes: number | null;
}

/**
 * First 24 chars of the minisign pubkey — must match
 * `plugins.updater.pubkey` in `src-tauri/tauri.conf.json` (fingerprint only,
 * the full key stays in the bundle config).
 */
export const UPDATE_SIGNING_FINGERPRINT = "dW50cnVzdGVkIGNvbW1lbnQ6";

export type UpdateCheckFn = () => Promise<Update | null>;
export type UpdateBackupFn = (sessionToken: string) => Promise<BackupMetadata>;

/** Network-shaped failures map to OFFLINE; an empty channel (HTTP 404 on the
 *  `latest.json` URL — no release published yet, or channel URL to verify)
 *  maps to NO_RELEASE; everything else surfaces verbatim.
 *
 * LIVE-2026-11-06: the plugin surfaces a missing channel as a bare reqwest
 * send error (`error sending request for url (...)`) with no status in the
 * text, so the channel URL shape itself is the NO_RELEASE signal — checked
 * AFTER the transport-failure rule so genuine offline stays OFFLINE. */
export function mapCheckError(failure: unknown): string {
  const text = failure instanceof Error ? failure.message : String(failure);
  if (/404|not found|introuvable|non trouv[ée]/i.test(text)) {
    return "UPDATE_NO_RELEASE";
  }
  if (/network|net::|internet|disconnected|dns|offline|fetch|ENOTFOUND|ECONNRESET|ETIMEDOUT|timed out|Load failed|trying to connect|connection refused|connection reset|failed to lookup/i.test(text)) {
    return "UPDATE_OFFLINE";
  }
  if (/releases\/latest\/download|latest\.json/i.test(text)) {
    return "UPDATE_NO_RELEASE";
  }
  return text || "UPDATE_CHECK_FAILED";
}

/**
 * One probe-download-install cycle. The plugin `Update` handle is held
 * between `probe()` and `settle()`; call `settle()` exactly once to release
 * the Rust-side resource when no install follows.
 */
export class UpdateFlow {
  private handle: Update | null = null;

  constructor(
    private readonly checkFn: UpdateCheckFn = check,
    private readonly backupFn: UpdateBackupFn = (token) => createBackup(token, "MANUAL"),
  ) {}

  /** Announced release info, or null when already current. */
  async probe(): Promise<UpdateAvailable | null> {
    const found = await this.checkFn();
    if (found === null) {
      return null;
    }
    this.handle = found;
    return {
      version: found.version,
      currentVersion: found.currentVersion,
      notes: found.body ?? null,
      date: found.date ?? null,
    };
  }

  /** Download the announced package; resolves when the Finished event lands. */
  async download(onProgress: (progress: UpdateDownloadProgress) => void): Promise<void> {
    const active = this.mustHaveHandle("download");
    let downloadedBytes = 0;
    let totalBytes: number | null = null;
    await active.download((event: DownloadEvent) => {
      if (event.event === "Started") {
        totalBytes = event.data.contentLength ?? null;
      } else if (event.event === "Progress") {
        downloadedBytes += event.data.chunkLength;
      }
      onProgress({ downloadedBytes, totalBytes });
    });
  }

  /**
   * Verified backup first, installer second. Resolves only when the
   * installer has taken over (Windows exits the app here); rejects when the
   * backup itself fails — the installer never launches on a failed backup.
   */
  async installAfterBackup(sessionToken: string): Promise<BackupMetadata> {
    const active = this.mustHaveHandle("install");
    const backup = await this.backupFn(sessionToken);
    await active.install();
    return backup;
  }

  /** Release the handle without installing (user dismissed the release). */
  async settle(): Promise<void> {
    if (this.handle !== null) {
      await this.handle.close();
      this.handle = null;
    }
  }

  private mustHaveHandle(action: string): Update {
    if (this.handle === null) {
      throw new Error(`UPDATE_NO_HANDLE:${action}`);
    }
    return this.handle;
  }
}
