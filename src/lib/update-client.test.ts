/**
 * UpdateFlow pins: probe/download/install ordering, progress math,
 * backup-before-install, handle release, error mapping. Zero network.
 */

import { describe, expect, it } from "vitest";
import {
  mapCheckError,
  readUpdateMode,
  UPDATE_SIGNING_FINGERPRINT,
  UpdateFlow,
  writeUpdateMode,
  type UpdateAvailable,
} from "./update-client";

// Minimal structural stand-in for the plugin `Update` resource.
interface FakeUpdate {
  version: string;
  currentVersion: string;
  body?: string;
  date?: string;
  downloadCalls: number;
  installCalls: number;
  closed: boolean;
  download: (cb: (event: { event: string; data: { contentLength?: number; chunkLength?: number } }) => void) => Promise<void>;
  install: () => Promise<void>;
  close: () => Promise<void>;
}

function fakeUpdate(version = "1.1.0"): FakeUpdate {
  const fake: FakeUpdate = {
    version,
    currentVersion: "1.0.0",
    body: "Arabic notes body",
    date: "2026-11-02T00:00:00Z",
    downloadCalls: 0,
    installCalls: 0,
    closed: false,
    download: async (cb) => {
      fake.downloadCalls += 1;
      cb({ event: "Started", data: { contentLength: 100 } });
      cb({ event: "Progress", data: { chunkLength: 40 } });
      cb({ event: "Progress", data: { chunkLength: 60 } });
      cb({ event: "Finished", data: {} });
    },
    install: async () => {
      fake.installCalls += 1;
    },
    close: async () => {
      fake.closed = true;
    },
  };
  return fake;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const asUpdate = (fake: FakeUpdate): any => fake;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const asBackup = (fileName: string): any => ({ fileName });

describe("update mode preference", () => {
  it("defaults to auto and round-trips manual", () => {
    const memory = new Map<string, string>();
    const storage = {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => {
        memory.set(key, value);
      },
    };
    expect(readUpdateMode(storage)).toBe("auto");
    writeUpdateMode("manual", storage);
    expect(readUpdateMode(storage)).toBe("manual");
    writeUpdateMode("auto", storage);
    expect(readUpdateMode(storage)).toBe("auto");
  });
});

describe("UpdateFlow", () => {
  it("probe maps the announced release and returns null when current", async () => {
    const fake = fakeUpdate();
    const flow = new UpdateFlow(async () => asUpdate(fake), async () => asBackup("b.db"));
    const info: UpdateAvailable | null = await flow.probe();
    expect(info?.version).toBe("1.1.0");
    expect(info?.currentVersion).toBe("1.0.0");
    expect(info?.notes).toBe("Arabic notes body");

    const quiet = new UpdateFlow(async () => null);
    expect(await quiet.probe()).toBeNull();
    await quiet.settle();
    await flow.settle();
    expect(fake.closed).toBe(true);
  });

  it("download accumulates progress and finishes", async () => {
    const fake = fakeUpdate();
    const seen: number[] = [];
    const flow = new UpdateFlow(async () => asUpdate(fake));
    await flow.probe();
    await flow.download((p) => {
      seen.push(p.downloadedBytes);
    });
    expect(seen).toEqual([0, 40, 100, 100]);
    expect(fake.downloadCalls).toBe(1);
    await flow.settle();
  });

  it("install runs backup BEFORE the installer and returns the backup", async () => {
    const order: string[] = [];
    const fake = fakeUpdate();
    fake.install = async () => {
      order.push("install");
      fake.installCalls += 1;
    };
    const flow = new UpdateFlow(async () => asUpdate(fake), async () => {
      order.push("backup");
      return asBackup("pre-update.db");
    });
    await flow.probe();
    const backup = await flow.installAfterBackup("token");
    expect(order).toEqual(["backup", "install"]);
    expect(backup.fileName).toBe("pre-update.db");
  });

  it("install never launches when the backup fails", async () => {
    const fake = fakeUpdate();
    const flow = new UpdateFlow(async () => asUpdate(fake), async () => {
      throw new Error("BACKUP_VERIFICATION_FAILED");
    });
    await flow.probe();
    await expect(flow.installAfterBackup("token")).rejects.toThrow("BACKUP_VERIFICATION_FAILED");
    expect(fake.installCalls).toBe(0);
  });

  it("actions without a probe reject with a stable code", async () => {
    const flow = new UpdateFlow(async () => null);
    let calls = 0;
    await expect(
      flow.download(() => {
        calls += 1;
      }),
    ).rejects.toThrow("UPDATE_NO_HANDLE:download");
    await expect(flow.installAfterBackup("t")).rejects.toThrow("UPDATE_NO_HANDLE:install");
    expect(calls).toBe(0);
  });
});

describe("mapCheckError", () => {
  it("maps network shapes to offline and keeps the rest verbatim", () => {
    expect(mapCheckError(new TypeError("Failed to fetch"))).toBe("UPDATE_OFFLINE");
    expect(mapCheckError(new Error("net::ERR_INTERNET_DISCONNECTED"))).toBe("UPDATE_OFFLINE");
    expect(mapCheckError(new Error("UPDATE_SIGNATURE_INVALID"))).toBe("UPDATE_SIGNATURE_INVALID");
    expect(mapCheckError("weird")).toBe("weird");
  });

  it("fingerprint is pinned to the bundle config", () => {
    expect(UPDATE_SIGNING_FINGERPRINT).toBe("dW50cnVzdGVkIGNvbW1lbnQ6");
  });
});
