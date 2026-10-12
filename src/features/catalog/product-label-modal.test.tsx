/**
 * Unit tests for the product label modal (TASK-DSN-039, XP-410B).
 *
 * Pure helpers plus `renderToStaticMarkup` on the presentational view
 * with explicit props (node-only harness, no DOM library — same pattern
 * as the OperationsMenu view tests).
 */

import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  buildLabelPrintHtml,
  clampLabelQuantity,
  labelBarcodeSource,
  labelPriceUnit,
  ProductLabelModalView,
  type LabelFormat,
} from "@/features/catalog/ProductLabelModal";
import type { CatalogProduct } from "@/types/pos";

function fixture(overrides?: Partial<CatalogProduct>): CatalogProduct {
  return {
    product_id: "prod-1",
    sku: "VIS-8X40",
    barcode: "6111250000013",
    name_ar: "برغي",
    name_fr: "Vis",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [
      {
        unit_id: "unit-1",
        product_id: "prod-1",
        unit_name_ar: "قطعة",
        unit_name_fr: "PCS",
        factor_micro: "10000",
        barcode: "6111250000013",
        sale_price_centimes: "15050",
        purchase_price_centimes: "12000",
        is_base_unit: true,
        is_default_selling: true,
      },
    ],
    kit_items: [],
    ...overrides,
  };
}

describe("label helpers", () => {
  it("clamps quantities into 1..500", () => {
    expect(clampLabelQuantity(1)).toBe(1);
    expect(clampLabelQuantity(12)).toBe(12);
    expect(clampLabelQuantity(0)).toBe(1);
    expect(clampLabelQuantity(-4)).toBe(1);
    expect(clampLabelQuantity(501)).toBe(500);
    expect(clampLabelQuantity(Number.NaN)).toBe(1);
  });

  it("prefers the product barcode and falls back to the SKU", () => {
    expect(labelBarcodeSource(fixture())).toBe("6111250000013");
    expect(labelBarcodeSource(fixture({ barcode: null }))).toBe("VIS-8X40");
    expect(labelBarcodeSource(fixture({ barcode: "   " }))).toBe("VIS-8X40");
  });

  it("reads the base-unit price for the sticker", () => {
    expect(labelPriceUnit(fixture())).toEqual({ priceCentimes: "15050", unitFr: "PCS" });
    expect(labelPriceUnit(fixture({ units: [] }))).toEqual({ priceCentimes: "0", unitFr: "" });
  });
});

describe("buildLabelPrintHtml", () => {
  it("sizes the page per preset and repeats the sticker per copy", () => {
    const formats: LabelFormat[] = ["50x30", "40x25", "100x50"];
    const sizes: Record<LabelFormat, string> = {
      "50x30": "50mm 30mm",
      "40x25": "40mm 25mm",
      "100x50": "100mm 50mm",
    };
    for (const format of formats) {
      const html = buildLabelPrintHtml({
        product: fixture(),
        locale: "fr-DZ",
        storeName: "Baraka",
        format,
        quantity: 2,
      });
      expect(html).toContain(`@page { size: ${sizes[format]}; margin: 0; }`);
      expect(html).toContain("<svg");
      expect(html.match(/class="sticker"/g)?.length).toBe(2);
    }
  });
});

describe("ProductLabelModalView rendering", () => {
  function markup(): string {
    return renderToStaticMarkup(
      ProductLabelModalView({
        product: fixture(),
        locale: "fr-DZ",
        storeName: "Baraka",
        format: "50x30",
        quantity: 3,
        onFormat: vi.fn(),
        onQuantity: vi.fn(),
        onPrint: vi.fn(),
        onClose: vi.fn(),
      }),
    );
  }

  it("renders name, formatted price, barcode svg, and format selector", () => {
    const html = markup();
    expect(html).toContain("Vis");
    expect(html).toContain("150,50 DZD");
    expect(html).toContain("<svg");
    expect(html).toContain("6111250000013");
    expect(html).toContain("50 × 30 mm (Standard Rayon)");
    expect(html).toContain("40 × 25 mm (Petit Format)");
    expect(html).toContain("100 × 50 mm (Grand Format / Carton)");
  });

  it("shows the quantity stepper value and print action", () => {
    const html = markup();
    expect(html).toContain(">3<");
    expect(html).toContain("Imprimer les étiquettes");
  });
});
