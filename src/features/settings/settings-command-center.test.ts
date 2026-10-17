/**
 * Settings command-center tests (TASK-DSN-053): two-tier sidebar with
 * 4 groups / 8 tabs and the three relocated config surfaces.
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
const CATEGORY_TAB_SRC = readSrc("src", "features", "settings", "CategorySettingsTab.tsx");
const PACK_TAB_SRC = readSrc("src", "features", "settings", "StarterPackSettingsTab.tsx");
const EXCHANGE_TAB_SRC = readSrc("src", "features", "settings", "DataExchangeSettingsTab.tsx");
const HUB_SRC = readSrc("src", "features", "catalog", "CatalogModule.tsx");

describe("two-tier sidebar", () => {
  it("declares 4 groups and 8 tabs with icons and admin lock badges", () => {
    for (const tab of [
      '"identity"',
      '"categories"',
      '"pack"',
      '"exchange"',
      '"printer"',
      '"fiscal"',
      '"backups"',
      '"about"',
    ]) {
      expect(MODULE_SRC).toContain(tab);
    }
    for (const group of [
      '"settings.groupShop"',
      '"settings.groupData"',
      '"settings.groupSales"',
      '"settings.groupSystem"',
    ]) {
      expect(MODULE_SRC).toContain(group);
    }
    expect(MODULE_SRC).toContain('strokeWidth="1.75"');
    expect(MODULE_SRC).toContain("lockedForNonAdmin");
    expect(MODULE_SRC).toContain('aria-current={selected ? "page" : undefined}');
    expect(MODULE_SRC).toContain("border-e");
    expect(MODULE_SRC).toContain("w-56");
  });

  it("mounts the three relocated panels behind the session gate", () => {
    expect(MODULE_SRC).toContain("<CategorySettingsTab");
    expect(MODULE_SRC).toContain("<StarterPackSettingsTab");
    expect(MODULE_SRC).toContain("<DataExchangeSettingsTab");
    expect(MODULE_SRC).toContain("if (session === null)");
  });
});

describe("relocated panels", () => {
  it("embeds category management as an always-open panel", () => {
    expect(CATEGORY_TAB_SRC).toContain("<CategoryManagePanel");
  });

  it("loads depots and catalog rows for pack detection", () => {
    expect(PACK_TAB_SRC).toContain("listWarehouses(sessionToken)");
    expect(PACK_TAB_SRC).toContain("ipcSearchCatalog(sessionToken");
    expect(PACK_TAB_SRC).toContain("<StarterPackBanner");
  });

  it("hosts import, export, and template cards with the import modal", () => {
    expect(EXCHANGE_TAB_SRC).toContain("<ProductImportModal");
    expect(EXCHANGE_TAB_SRC).toContain("buildCatalogCsvContent");
    expect(EXCHANGE_TAB_SRC).toContain("downloadSampleCsvTemplate");
    expect(EXCHANGE_TAB_SRC).toContain('"settings.exchangeImport"');
    expect(EXCHANGE_TAB_SRC).toContain('"settings.exchangeExport"');
    expect(EXCHANGE_TAB_SRC).toContain('"settings.exchangeTemplate"');
  });
});

describe("purified catalog", () => {
  it("keeps search/filter/table flows with no config mounts", () => {
    expect(HUB_SRC).toContain("<CategoryPillStrip");
    expect(HUB_SRC).toContain("ipcDeleteProduct");
    expect(HUB_SRC).not.toContain("StarterPackBanner");
    expect(HUB_SRC).not.toContain("<DataExchangeMenu");
    expect(HUB_SRC).not.toContain("<CategoryManageModal");
    expect(HUB_SRC).not.toContain("<ProductImportModal");
    expect(HUB_SRC).not.toContain("onManageCategories");
  });
});

describe("settings bilingual copy (FR/AR)", () => {
  it("declares group headers and new tab labels in both locales", () => {
    for (const entry of [
      '"settings.tabCategories": "Rayons & Catégories"',
      '"settings.tabCategories": "الأقسام والتصنيفات"',
      '"settings.tabPack": "Pack Quincaillerie (300)"',
      '"settings.tabPack": "حزمة الخردوات (300)"',
      '"settings.tabExchange": "Échanges & Données"',
      '"settings.tabExchange": "التبادل والبيانات"',
      '"settings.groupShop": "Magasin"',
      '"settings.groupShop": "المتجر"',
      '"settings.groupData": "Données"',
      '"settings.groupData": "البيانات"',
      '"settings.groupSales": "Vente"',
      '"settings.groupSales": "البيع"',
      '"settings.groupSystem": "Système"',
      '"settings.groupSystem": "النظام"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});
