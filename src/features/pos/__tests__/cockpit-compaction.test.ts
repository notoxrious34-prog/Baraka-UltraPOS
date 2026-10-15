/**
 * Cockpit compaction pins (TASK-DSN-048): Caisse header streamlining,
 * fast-sale tile cleanup, and empty-cart operator HUD.
 *
 * Source-text pins per repo convention (node-only harness, no DOM
 * library) plus i18n contract checks for the new HUD strings.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { t } from "@/lib/i18n";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const TOPBAR = readFileSync(join(ROOT, "src", "components", "pos", "Topbar.tsx"), "utf8");
const TILES = readFileSync(join(ROOT, "src", "components", "pos", "ProductDiscovery.tsx"), "utf8");
const CART = readFileSync(join(ROOT, "src", "components", "pos", "CartTable.tsx"), "utf8");
const I18N = readFileSync(join(ROOT, "src", "lib", "i18n.ts"), "utf8");

describe("compact Topbar (<=44px, no duplicate brand)", () => {
  it("uses the h-10 cockpit strip, not the h-14 brand bar", () => {
    expect(TOPBAR).toContain("flex h-10 ");
    expect(TOPBAR).not.toContain("flex h-14 ");
  });

  it("drops the logo/title block but keeps every operations widget", () => {
    expect(TOPBAR).not.toContain("barakaLogoIcon");
    expect(TOPBAR).not.toContain("app.subtitle");
    for (const widget of [
      "CashSessionIndicator",
      "OperationsMenu",
      "topbar.warehouse",
      "topbar.logout",
      "topbar.lang",
      "topbar.theme",
    ]) {
      expect(TOPBAR).toContain(widget);
    }
  });
});

describe("fast-sale tiles (whole-tile add, stock chip, no edit button)", () => {
  it("removes the Modifier affordance and its edit-store wiring", () => {
    expect(TILES).not.toContain("editProduct");
    expect(TILES).not.toContain("useProductEditStore");
    expect(TILES).not.toContain("canEditCatalog");
  });

  it("makes the whole tile the click target with pointer affordance", () => {
    expect(TILES).toContain("cursor-pointer");
    expect(TILES).toContain("addProduct(product)");
  });

  it("renders a per-product stock chip fed by stock_get levels", () => {
    expect(TILES).toContain("ipcGetStock");
    expect(TILES).toContain("stockById");
    expect(TILES).toContain("bg-emerald-50");
  });
});

describe("empty-cart operator HUD", () => {
  it("shows hint chips wired to locale strings, not bare text", () => {
    expect(CART).toContain('"cart.emptyHint"');
    expect(CART).toContain('"cart.hintSearch"');
    expect(CART).toContain('"cart.hintClient"');
    expect(CART).toContain('"cart.hintCheckout"');
    expect(CART).toContain("[F1]");
    expect(CART).toContain("[F2]");
    expect(CART).toContain("[F12]");
  });

  it("declares the HUD strings in both locales", () => {
    expect(t("fr-DZ", "cart.hintSearch")).toBe("Recherche");
    expect(t("fr-DZ", "cart.hintCheckout")).toBe("Encaisser");
    expect(t("ar-DZ", "cart.hintSearch")).toBe("بحث");
    expect(t("ar-DZ", "cart.hintCheckout")).toBe("الدفع");
    expect(I18N).toContain('"cart.emptyHint"');
  });
});
