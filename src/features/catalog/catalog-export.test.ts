/**
 * Catalog CSV export tests (TASK-DSN-043).
 *
 * Pure formatting helpers plus end-to-end document assertions (BOM,
 * semicolons, quoting, Arabic preservation, Latin-digit numerics). The
 * download trigger is only pinned for its SSR/node no-op guard — DOM
 * clicking is not exercised in this node-only harness.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  buildCatalogCsvContent,
  centimesToDecimalFixed,
  CSV_BOM,
  CSV_SEPARATOR,
  csvNumber,
  csvText,
  microsToQuantity,
  triggerCsvDownload,
} from "@/features/catalog/catalog-export";
import type { CatalogProduct } from "@/types/pos";

declare const process: { cwd(): string };

function unit(overrides: Partial<CatalogProduct["units"][number]>) {
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

function product(overrides: Partial<CatalogProduct>): CatalogProduct {
  return {
    product_id: "p-1",
    sku: "VIS-8X40",
    barcode: "6111250000013",
    name_ar: "برغي",
    name_fr: "Vis 8x40",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [unit({})],
    kit_items: [],
    ...overrides,
  };
}

describe("CSV_BOM", () => {
  it("is exactly one U+FEFF escape (no raw-byte literal)", () => {
    expect(CSV_BOM.length).toBe(1);
    expect(CSV_BOM.charCodeAt(0)).toBe(0xfeff);
    expect(CSV_SEPARATOR).toBe(";");
  });
});

describe("centimesToDecimalFixed", () => {
  it("formats fixed 2-decimal Latin strings", () => {
    expect(centimesToDecimalFixed("15050")).toBe("150.50");
    expect(centimesToDecimalFixed("12000")).toBe("120.00");
    expect(centimesToDecimalFixed("5")).toBe("0.05");
    expect(centimesToDecimalFixed("-250")).toBe("-2.50");
    expect(centimesToDecimalFixed("")).toBe("0.00");
    expect(centimesToDecimalFixed("nope")).toBe("0.00");
  });
});

describe("microsToQuantity", () => {
  it("trims micros to base-unit quantities", () => {
    expect(microsToQuantity("150000")).toBe("15");
    expect(microsToQuantity("25000")).toBe("2.5");
    expect(microsToQuantity("0")).toBe("0");
    expect(microsToQuantity("")).toBe("0");
    expect(microsToQuantity("nope")).toBe("0");
  });
});

describe("csvText / csvNumber", () => {
  it("quotes text and doubles embedded quotes", () => {
    expect(csvText("Vis")).toBe('"Vis"');
    expect(csvText('Vis "Turbo"')).toBe('"Vis ""Turbo"""');
    expect(csvText("")).toBe('""');
  });

  it("leaves plain decimals bare and quotes the rest", () => {
    expect(csvNumber("150.50")).toBe("150.50");
    expect(csvNumber("-2.5")).toBe("-2.5");
    expect(csvNumber("—")).toBe('"—"');
    expect(csvNumber("15 PCS")).toBe('"15 PCS"');
  });
});

describe("buildCatalogCsvContent", () => {
  it("starts with the BOM and bilingual semicolon headers", () => {
    const content = buildCatalogCsvContent([], {}, "fr-DZ");
    expect(content.charCodeAt(0)).toBe(0xfeff);
    expect(content.startsWith(`${CSV_BOM}Réf (SKU);`)).toBe(true);
    const [header] = content.split("\r\n");
    expect(header?.split(";")).toHaveLength(11);
    expect(header).toContain("Marge (%);Stock (DEP-01);Statut");
  });

  it("renders one exact row with prices, margin, stock and French status", () => {
    const content = buildCatalogCsvContent([product({})], { "p-1": "150000" }, "fr-DZ");
    const lines = content.split("\r\n");
    expect(lines).toHaveLength(3);
    expect(lines[1]).toBe(
      '"VIS-8X40";"6111250000013";"Vis 8x40";"برغي";"PCS";120.00;150.50;30.50;25.41;15;"Actif"',
    );
  });

  it("localizes the status cell for Arabic locales", () => {
    const fr = buildCatalogCsvContent([product({})], {}, "fr-DZ");
    const ar = buildCatalogCsvContent([product({})], {}, "ar-DZ");
    expect(fr).toContain(';"Actif"');
    expect(ar).toContain(';"نشط"');
    const inactive = buildCatalogCsvContent([product({ is_active: false })], {}, "ar-DZ");
    expect(inactive).toContain(';"غير نشط"');
  });

  it("escapes quotes, keeps null barcodes empty, and dashes zero purchase", () => {
    const tricky = product({
      product_id: "p-9",
      sku: 'SCIE "PRO"',
      barcode: null,
      name_fr: 'Scie "Pro" 500',
      units: [unit({ unit_id: "u-9", product_id: "p-9", sale_price_centimes: "5000", purchase_price_centimes: "0" })],
    });
    const content = buildCatalogCsvContent([tricky], {}, "fr-DZ");
    expect(content).toContain('"SCIE ""PRO"""');
    expect(content).toContain('"Scie ""Pro"" 500"');
    expect(content).toContain('"";"Scie');
    expect(content).toContain(";50.00;50.00;");
  });
});

describe("triggerCsvDownload", () => {
  it("is a guarded no-op outside browsers", () => {
    expect(triggerCsvDownload("x", "f.csv")).toBeUndefined();
  });

  it("delegates to the hardened desktop download helper in source", () => {
    const root = process.cwd();
    const src = readFileSync(
      join(root, "src", "features", "catalog", "catalog-export.ts"),
      "utf8",
    ) as string;
    expect(src).toContain("downloadTextFile(content, filename)");
    expect(src).not.toContain("URL.revokeObjectURL");
  });
});
