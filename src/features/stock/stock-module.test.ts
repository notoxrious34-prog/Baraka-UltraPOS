/**
 * Stock module tests (TASK-DSN-029): symmetric i18n keys, navigation
 * wiring, IPC binding shape, and pure helper behavior. Node-only harness
 * (no jsdom): source-text assertions + pure logic.
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
const IPC_SRC = readSrc("src", "lib", "ipc.ts");
const APP_SRC = readSrc("src", "App.tsx");
const VIEW_SRC = readSrc("src", "stores", "app-view-store.ts");
const MODULE_SRC = readSrc("src", "features", "stock", "StockModule.tsx");
const OVERVIEW_SRC = readSrc("src", "features", "stock", "StockOverviewPane.tsx");
const HISTORY_SRC = readSrc("src", "features", "stock", "StockMovementHistoryPane.tsx");
const FORM_SRC = readSrc("src", "features", "stock", "WarehouseCreateModal.tsx");

describe("stock i18n keys", () => {
  it("declares nav.stock and the stock.* namespace in FR", () => {
    for (const entry of [
      '"nav.stock": "Stock & Dépôts"',
      '"stock.title": "Stock & Dépôts"',
      '"stock.tabAlerts": "Aperçu & Alertes Ruptures"',
      '"stock.tabHistory": "Historique des Mouvements"',
      '"stock.noRupture": "Aucune rupture de stock détectée"',
      '"stock.adjust": "Régularisation"',
      '"stock.transfer": "Transfert Inter-Dépôts"',
      '"stock.formCodeError": "Code requis : 1 à 20 caractères alphanumériques"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });

  it("declares nav.stock and the stock.* namespace in AR", () => {
    for (const entry of [
      '"nav.stock": "المخزون والمستودعات"',
      '"stock.tabAlerts": "نظرة عامة وتنبيهات النفاذ"',
      '"stock.tabHistory": "سجل حركات المخزون"',
      '"stock.noRupture": "لا توجد تنبيهات نفاذ مخزون"',
      '"stock.adjust": "تسوية مخزون"',
      '"stock.transfer": "تحويل بين المستودعات"',
      '"stock.formSave": "حفظ المستودع"',
      '"stock.allWarehouses": "جميع المستودعات"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});

describe("navigation and IPC wiring", () => {
  it("extends AppView with stock between pos and customers", () => {
    expect(VIEW_SRC).toContain('"pos" | "stock" | "customers"');
  });

  it("routes the stock view in App.tsx behind the manager gate", () => {
    expect(APP_SRC).toContain('entry("stock"');
    expect(APP_SRC).toContain('view === "stock" && manager');
    expect(APP_SRC).toContain("<StockModule />");
  });

  it("binds warehouses_create through the session-injecting bridge", () => {
    expect(IPC_SRC).toContain("export async function ipcCreateWarehouse");
    expect(IPC_SRC).toContain('call<WarehouseWire>("warehouses_create"');
    expect(IPC_SRC).toContain("code: code.trim().toUpperCase()");
  });
});

describe("module surface", () => {
  it("tabs switch alerts and history panes", () => {
    expect(MODULE_SRC).toContain('setTab("alerts")');
    expect(MODULE_SRC).toContain('setTab("history")');
    expect(MODULE_SRC).toContain("<StockOverviewPane");
    expect(MODULE_SRC).toContain("<StockMovementHistoryPane");
  });

  it("warehouse selector reloads low-stock and movement lists", () => {
    expect(MODULE_SRC).toContain("ipcListWarehouses()");
    expect(MODULE_SRC).toContain("ipcLowStock(token, warehouseId)");
    expect(MODULE_SRC).toContain("warehouseId,");
    expect(MODULE_SRC).toContain("stock.allWarehouses");
  });

  it("admin gate guards warehouse creation; modals reuse sovereign stores", () => {
    expect(MODULE_SRC).toContain('session.role === "ADMIN"');
    expect(MODULE_SRC).toContain("<WarehouseCreateModal");
    expect(MODULE_SRC).toContain('openPosModal("stock-adjust")');
    expect(MODULE_SRC).toContain('openPosModal("warehouse-transfer")');
    expect(MODULE_SRC).toContain("openAdjustStore(");
    expect(MODULE_SRC).toContain("currentStockMicro: row.quantityBaseMicro");
  });

  it("overview renders HUD cards, bdi-isolated quantities, and the empty state", () => {
    expect(OVERVIEW_SRC).toContain("formatQty(");
    expect(OVERVIEW_SRC).toContain('<bdi dir="ltr"');
    expect(OVERVIEW_SRC).toContain("tabular-nums");
    expect(OVERVIEW_SRC).toContain('t(locale, "stock.noRupture")');
    expect(OVERVIEW_SRC).toContain('t(locale, "stock.activeWarehouses")');
    expect(OVERVIEW_SRC).toContain("onAdjust(row)");
  });

  it("history groups client-side and isolates codes in bdi", () => {
    expect(HISTORY_SRC).toContain("inMovementGroup(");
    expect(HISTORY_SRC).toContain('<bdi dir="ltr"');
    expect(HISTORY_SRC).toContain("tabular-nums");
    expect(HISTORY_SRC).toContain('t(locale, "stock.noMovements")');
    expect(MODULE_SRC).toContain("movementType: null");
  });

  it("creation form validates and never touches floats", () => {
    expect(FORM_SRC).toContain("isValidWarehouseCode");
    expect(FORM_SRC).toContain("isValidWarehouseName");
    expect(FORM_SRC).toContain("ipcCreateWarehouse(props.token, code, name)");
    expect(FORM_SRC).not.toContain("parseFloat");
  });
});

describe("stock helpers", async () => {
  const helpers = await import("@/features/stock/stock-helpers");

  it("partitions the seven engine codes into history groups", () => {
    expect(helpers.inMovementGroup("PURCHASE_IN", "IN")).toBe(true);
    expect(helpers.inMovementGroup("TRANSFER_IN", "IN")).toBe(true);
    expect(helpers.inMovementGroup("SALE_OUT", "IN")).toBe(false);
    expect(helpers.inMovementGroup("SALE_OUT", "OUT")).toBe(true);
    expect(helpers.inMovementGroup("ADJUSTMENT_POSITIVE", "ADJUST")).toBe(true);
    expect(helpers.inMovementGroup("ADJUSTMENT_NEGATIVE", "ADJUST")).toBe(true);
    expect(helpers.inMovementGroup("TRANSFER_OUT", "TRANSFER")).toBe(true);
    expect(helpers.inMovementGroup("SALE_OUT", "TRANSFER")).toBe(false);
    expect(helpers.inMovementGroup("SALE_OUT", "ALL")).toBe(true);
  });

  it("assigns distinct badge classes to adjust, transfer, in, and sale-out", () => {
    const badges = new Set([
      helpers.movementBadge("ADJUSTMENT_POSITIVE"),
      helpers.movementBadge("TRANSFER_IN"),
      helpers.movementBadge("PURCHASE_IN"),
      helpers.movementBadge("SALE_OUT"),
    ]);
    expect(badges.size).toBe(4);
  });

  it("validates warehouse code and name lengths", () => {
    expect(helpers.isValidWarehouseCode("D1")).toBe(true);
    expect(helpers.isValidWarehouseCode("")).toBe(false);
    expect(helpers.isValidWarehouseCode("x".repeat(21))).toBe(false);
    expect(helpers.isValidWarehouseName("Dépôt principal")).toBe(true);
    expect(helpers.isValidWarehouseName("  ")).toBe(false);
  });
});
