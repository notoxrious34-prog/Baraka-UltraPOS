/**
 * Fiscal stamp toggle tests (TASK-DSN-051, ADR-092): settings tab
 * registration, IPC binding shape, and bilingual copy.
 * Node-only harness (no jsdom): source-text assertions.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
// Type-only; zero runtime impact.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const I18N_SRC = readSrc("src", "lib", "i18n.ts");
const MODULE_SRC = readSrc("src", "features", "settings", "SettingsModule.tsx");
const TAB_SRC = readSrc("src", "features", "settings", "FiscalSettingsTab.tsx");

describe("fiscal settings tab registration", () => {
  it("adds a fiscal tab behind the session gate", () => {
    expect(MODULE_SRC).toContain("FiscalSettingsTab");
    expect(MODULE_SRC).toContain('"fiscal"');
    expect(MODULE_SRC).toContain('"settings.tabFiscal"');
    expect(TAB_SRC).toContain("ipcGetFiscalSettings");
    expect(TAB_SRC).toContain("ipcUpdateFiscalSettings");
  });

  it("exposes an accessible switch gated to ADMIN writes", () => {
    expect(TAB_SRC).toContain('role="switch"');
    expect(TAB_SRC).toContain("aria-checked={enabled}");
    expect(TAB_SRC).toContain("disabled={saving || !isAdmin}");
  });
});

describe("fiscal bilingual copy (FR/AR)", () => {
  it("declares the fiscal.* namespace in both locales", () => {
    for (const entry of [
      '"settings.tabFiscal": "Fiscalité"',
      '"settings.tabFiscal": "الجبائية"',
      '"fiscal.timbreLabel": "Droit de Timbre Fiscal (1 %)"',
      '"fiscal.timbreLabel": "رسم الطابع الجبائي (1%)"',
      '"fiscal.updated": "Paramètre fiscal mis à jour"',
      '"fiscal.updated": "تم تحديث الإعداد الجبائي بنجاح"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});
