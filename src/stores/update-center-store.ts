/**
 * Update Center store: single-flight probe → download → install state.
 *
 * One module-level `UpdateFlow` at a time; dismissing settles the handle and
 * starts fresh. Auto-check failures stay silent (offline-first: the shop
 * never sees a popup for a missing network); manual checks surface codes.
 */

import { create } from "zustand";
import { getVersion } from "@tauri-apps/api/app";
import {
  mapCheckError,
  readUpdateMode,
  UpdateFlow,
  writeUpdateMode,
  type UpdateAvailable,
  type UpdateDownloadProgress,
  type UpdateMode,
} from "@/lib/update-client";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "uptodate"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
  | "error";

interface UpdateCenterState {
  phase: UpdatePhase;
  info: UpdateAvailable | null;
  progress: UpdateDownloadProgress;
  errorCode: string | null;
  currentVersion: string | null;
  lastCheckedAt: string | null;
  mode: UpdateMode;
  setMode: (mode: UpdateMode) => void;
  /** Loud check (user pressed the button): errors surface. */
  checkNow: () => Promise<void>;
  /** Startup check (auto mode only): every failure stays silent. */
  checkSilently: () => Promise<void>;
  download: () => Promise<void>;
  /** Backup-then-install; resolves with the backup file name. */
  install: (sessionToken: string) => Promise<string>;
  dismiss: () => Promise<void>;
}

let flow = new UpdateFlow();

function freshFlow(): UpdateFlow {
  flow = new UpdateFlow();
  return flow;
}

async function currentAppVersion(): Promise<string | null> {
  try {
    return await getVersion();
  } catch {
    return null;
  }
}

function stamp(): string {
  return new Date().toISOString();
}

export const useUpdateCenterStore = create<UpdateCenterState>()((set, get) => ({
  phase: "idle",
  info: null,
  progress: { downloadedBytes: 0, totalBytes: null },
  errorCode: null,
  currentVersion: null,
  lastCheckedAt: null,
  mode: readUpdateMode(),

  setMode: (mode) => {
    writeUpdateMode(mode);
    set({ mode });
  },

  checkNow: async () => {
    const active = freshFlow();
    set({ phase: "checking", errorCode: null, info: null });
    try {
      const info = await active.probe();
      set({
        phase: info === null ? "uptodate" : "available",
        info,
        currentVersion: (await currentAppVersion()) ?? info?.currentVersion ?? get().currentVersion,
        lastCheckedAt: stamp(),
      });
    } catch (failure) {
      await active.settle().catch(() => {});
      freshFlow();
      set({ phase: "error", errorCode: mapCheckError(failure) });
    }
  },

  checkSilently: async () => {
    if (get().mode !== "auto") {
      return;
    }
    const active = freshFlow();
    set({ phase: "checking", errorCode: null });
    try {
      const info = await active.probe();
      // Silent stay-available: keep the handle for a later download, but do
      // not interrupt the operator. A null probe settles back to idle.
      if (info === null) {
        await active.settle().catch(() => {});
        freshFlow();
        set({ phase: "idle", info: null, lastCheckedAt: stamp() });
        return;
      }
      set({
        phase: "available",
        info,
        currentVersion: (await currentAppVersion()) ?? info.currentVersion,
        lastCheckedAt: stamp(),
      });
    } catch {
      await active.settle().catch(() => {});
      freshFlow();
      set({ phase: "idle", errorCode: null });
    }
  },

  download: async () => {
    set({ phase: "downloading", errorCode: null, progress: { downloadedBytes: 0, totalBytes: null } });
    try {
      await flow.download((progress) => {
        set({ progress });
      });
      set({ phase: "ready" });
    } catch (failure) {
      await flow.settle().catch(() => {});
      freshFlow();
      set({ phase: "error", errorCode: mapCheckError(failure), info: null });
    }
  },

  install: async (sessionToken: string) => {
    set({ phase: "installing", errorCode: null });
    try {
      const backup = await flow.installAfterBackup(sessionToken);
      return backup.fileName;
    } catch (failure) {
      await flow.settle().catch(() => {});
      freshFlow();
      const code = failure instanceof Error ? failure.message : String(failure);
      set({ phase: "error", errorCode: code || "UPDATE_INSTALL_FAILED" });
      throw failure;
    }
  },

  dismiss: async () => {
    await flow.settle().catch(() => {});
    freshFlow();
    set({ phase: "idle", info: null, errorCode: null, progress: { downloadedBytes: 0, totalBytes: null } });
  },
}));
