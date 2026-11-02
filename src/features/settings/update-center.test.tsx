/**
 * Update Center wiring pins (node-only source assertions, same pattern as
 * tspl-probe-selector.test.ts): tab registration, gates, i18n keys, and the
 * backup-before-install chain. Real behavior is pinned in
 * update-client.test.ts; rendering is captured via screenshot harness.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const MODULE_SRC = readSrc("src", "features", "settings", "SettingsModule.tsx");
const TAB_SRC = readSrc("src", "features", "settings", "UpdateCenterTab.tsx");
const STORE_SRC = readSrc("src", "stores", "update-center-store.ts");
const CLIENT_SRC = readSrc("src", "lib", "update-client.ts");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");
const APP_SRC = readSrc("src", "App.tsx");

const TAB_KEYS = [
  "settings.tabUpdate",
  "update.statusTitle",
  "update.currentVersion",
  "update.checkNow",
  "update.upToDate",
  "update.available",
  "update.download",
  "update.ready",
  "update.backupNote",
  "update.installAtShiftClose",
  "update.installBlockedShift",
  "update.installBlockedPermission",
  "update.signatureTitle",
  "update.restoreNote",
  "update.offline",
  "update.noRelease",
  "update.dismiss",
  "update.backupDone",
] as const;

describe("Update Center registration", () => {
  it("registers the update tab under system with admin writes", () => {
    expect(MODULE_SRC).toContain('{ id: "update"');
    expect(MODULE_SRC).toContain('"settings.tabUpdate"');
    expect(MODULE_SRC).toContain('managePermissions: ["SYSTEM_SETTINGS_MANAGE"]');
    expect(MODULE_SRC).toContain("<UpdateCenterTab locale={locale} sessionToken={token} />");
  });

  it("startup probe is silent and auto-gated", () => {
    expect(APP_SRC).toContain("checkSilently");
    expect(STORE_SRC).toContain('mode !== "auto"');
    expect(STORE_SRC).toContain('phase: "idle", errorCode: null');
  });
});

describe("Update Center gates", () => {
  it("install requires closed shift and admin permission", () => {
    expect(TAB_SRC).toContain("isCheckoutPermitted");
    expect(TAB_SRC).toContain("shiftOpen");
    expect(TAB_SRC).toContain('"SYSTEM_SETTINGS_MANAGE"');
    expect(TAB_SRC).toContain("installBlockedShift");
  });

  it("backup runs before the installer in the client", () => {
    const backupPos = CLIENT_SRC.indexOf("installAfterBackup");
    expect(backupPos).toBeGreaterThan(-1);
    const body = CLIENT_SRC.slice(backupPos);
    expect(body.indexOf("backupFn")).toBeLessThan(body.indexOf(".install()"));
  });

  it("offline maps to a stable code, never a raw exception", () => {
    expect(CLIENT_SRC).toContain("UPDATE_OFFLINE");
    expect(TAB_SRC).toContain("update.offline");
  });
});

describe("Update Center i18n", () => {
  it("every tab key exists in the dictionary source", () => {
    for (const key of TAB_KEYS) {
      expect(I18N_SRC).toContain(`"${key}"`);
    }
  });
});
