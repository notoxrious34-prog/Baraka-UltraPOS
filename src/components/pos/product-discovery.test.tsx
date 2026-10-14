/**
 * Instant scan-to-cart resolution tests (TASK-DSN-040).
 *
 * `resolveScanToCart` is the pure exact-match engine behind the search-input
 * Enter handler; it is unit-tested directly (node-only harness, no DOM).
 * I18n wiring is pinned via source-text assertions, per repo convention.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { resolveScanToCart } from "@/components/pos/ProductDiscovery";
import type { CatalogProduct } from "@/types/pos";

declare const process: { cwd(): string };

function product(overrides: Partial<CatalogProduct>): CatalogProduct {
  return {
    product_id: "p-1",
    sku: "VIS-8X40",
    barcode: "6111250000013",
    name_ar: "برغي",
    name_fr: "Vis",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [
      {
        unit_id: "u-1",
        product_id: "p-1",
        unit_name_ar: "قطعة",
        unit_name_fr: "PCS",
        factor_micro: "10000",
        barcode: "6111250000013",
        sale_price_centimes: "15050",
        purchase_price_centimes: "12000",
        wholesale_price_centimes: null,
        is_base_unit: true,
        is_default_selling: true,
      },
    ],
    kit_items: [],
    ...overrides,
  };
}

const VIS = product({});
const OTHER = product({
  product_id: "p-2",
  sku: "CLOU",
  barcode: null,
  name_ar: "مسمار",
  name_fr: "Clou",
  units: [
    {
      unit_id: "u-2",
      product_id: "p-2",
      unit_name_ar: "قطعة",
      unit_name_fr: "PCS",
      factor_micro: "10000",
      barcode: null,
      sale_price_centimes: "500",
      purchase_price_centimes: "300",
      wholesale_price_centimes: null,
      is_base_unit: true,
      is_default_selling: true,
    },
  ],
});

describe("resolveScanToCart", () => {
  it("matches an exact product barcode and returns its unit index", () => {
    const hit = resolveScanToCart("6111250000013", [VIS, OTHER]);
    expect(hit?.product.product_id).toBe("p-1");
    expect(hit?.unitIndex).toBe(0);
  });

  it("matches an exact product SKU", () => {
    expect(resolveScanToCart("VIS-8X40", [VIS, OTHER])?.product.product_id).toBe("p-1");
  });

  it("matches a unit barcode that differs from the product barcode", () => {
    const boxed = product({
      barcode: null,
      units: [
        {
          unit_id: "u-3",
          product_id: "p-1",
          unit_name_ar: "علبة",
          unit_name_fr: "BL",
          factor_micro: "120000",
          barcode: "CASE-1200",
          sale_price_centimes: "150000",
          purchase_price_centimes: "144000",
          wholesale_price_centimes: null,
          is_base_unit: false,
          is_default_selling: false,
        },
      ],
    });
    const hit = resolveScanToCart("CASE-1200", [VIS, boxed]);
    expect(hit?.product.product_id).toBe("p-1");
    // The single-unit box product is the sole unit (index 0).
    expect(hit?.unitIndex).toBe(0);
  });

  it("adds the sole search result when there is no exact match", () => {
    expect(resolveScanToCart("anything", [OTHER])?.product.product_id).toBe("p-2");
  });

  it("returns null for a blank query or an ambiguous no-match", () => {
    expect(resolveScanToCart("   ", [VIS])).toBeNull();
    // Two results, neither matches, no exact hit → ambiguous, no auto-add.
    expect(resolveScanToCart("nope", [VIS, OTHER])).toBeNull();
  });

  it("falls back to the base unit when the SKU matched", () => {
    const hit = resolveScanToCart("CLOU", [VIS, OTHER]);
    expect(hit?.product.product_id).toBe("p-2");
    expect(hit?.unitIndex).toBe(0);
  });
});

describe("scan-to-cart i18n and source contracts", () => {
  const I18N_SRC = readFileSync(join(process.cwd(), "src", "lib", "i18n.ts"), "utf8");
  const PANEL_SRC = readFileSync(
    join(process.cwd(), "src", "components", "pos", "ProductDiscovery.tsx"),
    "utf8",
  );

  it("declares pos.scanPrompt and pos.itemScannedSuccess in FR and AR", () => {
    expect(I18N_SRC).toContain('"pos.scanPrompt": "Scanner un code-barres ou rechercher (F1)..."');
    expect(I18N_SRC).toContain('"pos.itemScannedSuccess": "ajouté au panier"');
    expect(I18N_SRC).toContain('"pos.scanPrompt": "امسح الباركود أو ابحث (F1)..."');
    expect(I18N_SRC).toContain('"pos.itemScannedSuccess": "تمت إضافته إلى السلة"');
  });

  it("wires the Enter handler and keeps the empty-state key references", () => {
    expect(PANEL_SRC).toContain("onKeyDown={handleSearchKeyDown}");
    expect(PANEL_SRC).toContain('t(locale, "pos.scanPrompt")');
    expect(PANEL_SRC).toContain('t(locale, "pos.itemScannedSuccess")');
    // pre-existing source-text contracts must survive
    expect(PANEL_SRC).toContain('"search.catalogEmpty"');
    expect(PANEL_SRC).toContain('"search.noResults"');
    expect(PANEL_SRC).toContain('"search.discoveryTitle"');
  });
});