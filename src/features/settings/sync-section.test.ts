/**
 * Two-till sync section pins (TASK-BRK-124): UI-shape pins via the
 * source-text pattern (no jsdom). The section carries opaque package text
 * only — every invariant lives in the engine.
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

const SECTION_SRC = readSrc("src", "features", "settings", "SyncSection.tsx");
const TAB_SRC = readSrc("src", "features", "settings", "BackupSettingsTab.tsx");
const IPC_SYNC_SRC = readSrc("src", "lib", "ipc-sync.ts");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

const SYNC_KEYS = [
  "backups.sync.title",
  "backups.sync.scope",
  "backups.sync.exportAction",
  "backups.sync.exportedDocs",
  "backups.sync.copyPackage",
  "backups.sync.copied",
  "backups.sync.importAction",
  "backups.sync.warehouse",
  "backups.sync.packageLabel",
  "backups.sync.importedDocs",
  "backups.sync.renumbered",
  "backups.sync.noRenumber",
  "backups.sync.warnings",
  "backups.sync.noWarnings",
  "backups.sync.loadWarehousesError",
];

describe("sync section mount + IPC bridge", () => {
  it("mounts inside the backups tab below the hardening section", () => {
    expect(TAB_SRC).toContain('import { SyncSection } from "./SyncSection"');
    expect(TAB_SRC).toContain("<SyncSection locale={locale} sessionToken={sessionToken} />");
  });

  it("calls the exact backend commands through the ipc-sync bridge", () => {
    expect(IPC_SYNC_SRC).toContain('"sync_export_package"');
    expect(IPC_SYNC_SRC).toContain('"sync_import_package"');
    expect(SECTION_SRC).toContain("ipcSyncExport(sessionToken");
    expect(SECTION_SRC).toContain("ipcSyncImport(sessionToken");
    expect(SECTION_SRC).toContain("listWarehouses(sessionToken)");
  });

  it("renders the v1 scope warning (returns stay on the origin till)", () => {
    expect(SECTION_SRC).toContain("backups.sync.scope");
    expect(SECTION_SRC).toContain("backups.sync.renumbered");
    expect(SECTION_SRC).toContain("backups.sync.warnings");
  });
});

describe("sync i18n FR/AR symmetry", () => {
  it("defines every sync key in both locales", () => {
    for (const key of SYNC_KEYS) {
      expect(I18N_SRC).toContain(`"${key}"`);
    }
  });
});

describe("sync UI carries text only (no financial math)", () => {
  it("never parses floats or computes totals in the section", () => {
    expect(SECTION_SRC).not.toContain("parseFloat");
    expect(SECTION_SRC).not.toContain("parseInt");
    expect(SECTION_SRC).not.toContain("toFixed");
  });
});
