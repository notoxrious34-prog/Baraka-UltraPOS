/**
 * Catalog exchange round-trip tests (TASK-BRK-093).
 *
 * The owner-failing shape (display-header export) must parse; exported
 * files must re-import to identical covered fields; every formerly-silent
 * behavior must surface a loud pin.
 */

import { describe, expect, it } from "vitest";
import { buildCatalogCsvContent } from "@/features/catalog/catalog-export";
import { parseCatalogCsv } from "@/features/catalog/catalog-import-parser";
import type { CatalogProduct, ImportProductRow } from "@/types/pos";

function unit(overrides: Record<string, unknown> = {}) {
  return {
    unit_id: "u-1",
    product_id: "p-1",
    unit_name_ar: "قطعة",
    unit_name_fr: "PCS",
    factor_micro: "10000",
    barcode: null,
    sale_price_centimes: "15050",
    purchase_price_centimes: "12000",
    wholesale_price_centimes: null,
    is_base_unit: true,
    is_default_selling: true,
    ...overrides,
  };
}

function product(overrides: Record<string, unknown> = {}): CatalogProduct {
  return {
    product_id: "p-1",
    sku: "VIS-8X40",
    barcode: "6111250000013",
    name_ar: "برغي 8x40",
    name_fr: "Vis 8x40",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [unit({})],
    kit_items: [],
    ...overrides,
  } as unknown as CatalogProduct;
}

const OWNER_FAILING_HEADER =
  "Réf (SKU);Code-barres;Désignation FR;Désignation AR;Unité;Prix Achat HT (DZD);Prix Vente TTC (DZD);Marge (DZD);Marge (%);Stock (DEP-01);Statut";

describe("dialect pin (owner-failing export shape)", () => {
  it("parses the exact display-header export row with zero errors", () => {
    const content = [
      OWNER_FAILING_HEADER,
      '"VIS-PLA-25";"6111250090011";"Vis Placo 3.5x25";"برغي بلاكو";"PCS";120.50;150.00;29.50;24.49;"50";"Actif"',
    ].join("\r\n");
    const parsed = parseCatalogCsv(`\uFEFF${content}\r\n`);
    expect(parsed.errors).toEqual([]);
    expect(parsed.rows).toHaveLength(1);
    const row = parsed.rows[0] as ImportProductRow;
    expect(row.sku).toBe("VIS-PLA-25");
    expect(row.barcode).toBe("6111250090011");
    expect(row.name_fr).toBe("Vis Placo 3.5x25");
    expect(row.name_ar).toBe("برغي بلاكو");
    expect(row.base_unit_name_fr).toBe("PCS");
    expect(row.purchase_price_centimes).toBe(12050);
    expect(row.sale_price_centimes).toBe(15000);
    expect(row.initial_quantity_micro).toBe(500_000);
  });

  it("matches a depot-interpolated stock header by pattern", () => {
    const header = OWNER_FAILING_HEADER.replace("Stock (DEP-01)", "Stock (MAG-02)");
    const content = [
      header,
      '"VIS-PLA-25";"6111250090011";"Vis Placo";"برغي";"PCS";120.50;150.00;29.50;24.49;"7";"Actif"',
    ].join("\r\n");
    const parsed = parseCatalogCsv(content);
    expect(parsed.errors).toEqual([]);
    expect(parsed.rows[0]?.initial_quantity_micro).toBe(70_000);
  });
});

describe("round-trip identity (export → import → export)", () => {
  const fixtures = [
    product({}),
    product({
      product_id: "p-2",
      sku: "COU-PVC-32",
      barcode: null,
      name_fr: "Coude PVC 32",
      name_ar: "كوع بلاستيك 32",
      units: [unit({ unit_id: "u-2", product_id: "p-2", sale_price_centimes: "11000", purchase_price_centimes: "8500" })],
    }),
  ];
  const levels: Record<string, string> = { "p-1": "150000", "p-2": "25000" };

  function toProduct(row: ImportProductRow, productId: string): CatalogProduct {
    const purchase = String(row.purchase_price_centimes ?? 0);
    const sale = String(row.sale_price_centimes);
    return product({
      product_id: productId,
      sku: row.sku ?? "",
      barcode: row.barcode ?? null,
      name_fr: row.name_fr,
      name_ar: row.name_ar ?? row.name_fr,
      units: [
        unit({
          unit_id: `u-${productId}`,
          product_id: productId,
          unit_name_fr: row.base_unit_name_fr ?? "PCS",
          purchase_price_centimes: purchase,
          sale_price_centimes: sale,
        }),
      ],
    });
  }

  it("covered fields survive export → parse with an empty field-diff", () => {
    const exported = buildCatalogCsvContent(fixtures, levels, "fr-DZ", "DEP-01");
    const parsed = parseCatalogCsv(exported);
    expect(parsed.errors).toEqual([]);
    expect(parsed.rows).toHaveLength(2);
    const first = parsed.rows[0] as ImportProductRow;
    const second = parsed.rows[1] as ImportProductRow;
    expect(first.sku).toBe("VIS-8X40");
    expect(first.name_ar).toBe("برغي 8x40");
    expect(first.purchase_price_centimes).toBe(12000);
    expect(first.sale_price_centimes).toBe(15050);
    expect(first.initial_quantity_micro).toBe(150_000);
    expect(first.base_unit_name_fr).toBe("PCS");
    expect(second.barcode).toBeUndefined();
    expect(second.initial_quantity_micro).toBe(25_000);
    // Category is an explicit exclusion: the export carries no category
    // column, so re-imported rows are uncategorized.
    expect(first.category_id).toBeUndefined();
    expect(second.category_id).toBeUndefined();
  });

  it("re-export of the re-imported catalog is byte-identical", () => {
    const exported = buildCatalogCsvContent(fixtures, levels, "fr-DZ", "DEP-01");
    const parsed = parseCatalogCsv(exported);
    const rebuilt = parsed.rows.map((row, index) =>
      toProduct(row as ImportProductRow, fixtures[index]?.product_id ?? `p-${index + 1}`),
    );
    const rebuiltLevels: Record<string, string> = {};
    for (const [index, row] of (parsed.rows as ImportProductRow[]).entries()) {
      rebuiltLevels[fixtures[index]?.product_id ?? ""] = String(
        row.initial_quantity_micro ?? 0,
      );
    }
    expect(buildCatalogCsvContent(rebuilt, rebuiltLevels, "fr-DZ", "DEP-01")).toBe(exported);
  });
});

describe("honesty pins (never silent)", () => {
  const header = "sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;category_code";

  it("rejects malformed purchase and stock cells", () => {
    const content = [
      header,
      "A-1;;Vis;برغي;not-a-price;150.00;50;",
      "A-2;;Coude;كوع;120.00;110.00;lots;",
      "A-3;;Té;تي;120.00;110.00;-5;",
    ].join("\n");
    const parsed = parseCatalogCsv(content);
    expect(parsed.rows).toHaveLength(0);
    expect(parsed.errors.map((entry) => entry.message)).toEqual([
      "invalid purchase_price",
      "invalid initial_stock",
      "invalid initial_stock",
    ]);
  });

  it("rejects unknown category codes when a lookup is provided", () => {
    const lookup = new Map([["VISS", "cat-1"]]);
    const content = [header, "A-1;;Vis;برغي;120.00;150.00;50;NOPE"].join("\n");
    const parsed = parseCatalogCsv(content, lookup);
    expect(parsed.rows).toHaveLength(0);
    expect(parsed.errors).toEqual([
      { line: 2, message: 'unknown category "NOPE"' },
    ]);
  });

  it("resolves known category codes through the lookup", () => {
    const lookup = new Map([["VISS", "cat-1"]]);
    const content = [header, "A-1;;Vis;برغي;120.00;150.00;50;VISS"].join("\n");
    const parsed = parseCatalogCsv(content, lookup);
    expect(parsed.errors).toEqual([]);
    expect(parsed.rows[0]?.category_id).toBe("cat-1");
  });
});
