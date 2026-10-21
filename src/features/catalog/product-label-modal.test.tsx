/**
 * Unit tests for the product label modal (TASK-DSN-039, XP-410B).
 *
 * Pure helpers plus `renderToStaticMarkup` on the presentational view
 * with explicit props (node-only harness, no DOM library — same pattern
 * as the OperationsMenu view tests).
 */

import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { createHash } from "node:crypto";
import {
  buildLabelPrintHtml,
  clampLabelQuantity,
  labelBarcodeSource,
  labelPriceUnit,
  ProductLabelModalView,
  resolveLabelBarcode,
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
        wholesale_price_centimes: null,
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
    const formats: LabelFormat[] = ["39x20", "50x30", "40x25", "100x50"];
    const sizes: Record<LabelFormat, string> = {
      "39x20": "39mm 20mm",
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
      expect(html.match(/class="sticker("| )/g)?.length).toBe(2);
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

describe("legacy preset golden identity (DSN-083)", () => {
  const sha = (text: string): string =>
    createHash("sha256").update(text).digest("hex").slice(0, 16);

  it("emits byte-identical documents for every pre-39x20 preset", () => {
    const goldens: Record<Exclude<LabelFormat, "39x20">, string> = {
      "50x30": "ec3b5b6684062da3",
      "40x25": "2044553e7a6e1987",
      "100x50": "9d3cf3867e14c09a",
    };
    for (const format of ["50x30", "40x25", "100x50"] as const) {
      const html = buildLabelPrintHtml({
        product: fixture(),
        locale: "fr-DZ",
        storeName: "Baraka",
        format,
        quantity: 1,
      });
      expect(sha(html)).toBe(goldens[format]);
    }
  });
});

describe("39x20 compact preset (DSN-083)", () => {
  const args39 = {
    product: fixture(),
    locale: "fr-DZ" as const,
    storeName: "Baraka",
    format: "39x20" as const,
    quantity: 1,
  };

  it("sizes the page 39x20 and drops the shop line", () => {
    const html = buildLabelPrintHtml(args39);
    expect(html).toContain("@page { size: 39mm 20mm; margin: 0; }");
    expect(html).not.toContain('class="shop"');
    expect(html).toContain('class="sticker compact"');
  });

  it("fits a 13-char barcode inside the 132px budget (no clip)", () => {
    const html = buildLabelPrintHtml(args39);
    const width = Number(/<svg[^>]*width="([\d.]+)"/.exec(html)?.[1]);
    expect(width).toBeGreaterThan(0);
    expect(width).toBeLessThanOrEqual(132);
  });

  it("ellipsizes the human-readable code with a full-text title tooltip", () => {
    const html = buildLabelPrintHtml(args39);
    expect(html).toContain("text-overflow: ellipsis");
    expect(html).toContain('title="6111250000013"');
  });

  it("resolves moduleWidth 2 on legacy presets and a fitted width on 39x20", () => {
    expect(resolveLabelBarcode("6111250000013", "50x30").moduleWidth).toBe(2);
    const compact = resolveLabelBarcode("6111250000013", "39x20");
    expect(compact.moduleWidth).toBeLessThanOrEqual(1);
    expect(compact.moduleWidth).toBeGreaterThan(0);
    const short = resolveLabelBarcode("P-001", "39x20");
    expect(short.moduleWidth).toBe(1);
  });
});
