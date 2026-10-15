/**
 * BRK-069 starter-pack verification: the 300-item quincaillerie CSV parses
 * cleanly through the production import parser with zero rejections.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
import { parseCatalogCsv } from "@/features/catalog/catalog-import-parser";

const CSV_URL = new URL(
  "../../../assets/fixtures/quincaillerie_starter_300.csv",
  import.meta.url,
);

const CODES = new Map(
  ["CAT-PLOMB", "CAT-ELEC", "CAT-VISS", "CAT-OUTIL", "CAT-PEINT", "CAT-BATIM", "CAT-JARDIN"].map(
    (code) => [code, `id-${code}`] as [string, string],
  ),
);

/** EAN-13 checksum validity (Latin digits, pure integer math). */
function isValidEan13(code: string): boolean {
  if (!/^\d{13}$/.test(code)) {
    return false;
  }
  let sum = 0;
  for (let i = 0; i < 12; i += 1) {
    sum += Number(code[i]) * (i % 2 === 0 ? 1 : 3);
  }
  return String((10 - (sum % 10)) % 10) === code[12];
}

describe("quincaillerie starter pack (300)", () => {
  it("parses all 300 rows with zero errors", () => {
    const text = readFileSync(CSV_URL, "utf-8");
    expect(text.charCodeAt(0)).toBe(0xfeff);
    const { rows, errors } = parseCatalogCsv(text, CODES);
    expect(errors).toEqual([]);
    expect(rows).toHaveLength(300);
  });

  it("covers all 7 starter categories", () => {
    const text = readFileSync(CSV_URL, "utf-8");
    const { rows } = parseCatalogCsv(text, CODES);
    const seen = new Set(rows.map((row) => row.category_id));
    expect(seen.size).toBe(7);
  });

  it("carries dual FR/AR names, unique SKUs, and healthy margins", () => {
    const text = readFileSync(CSV_URL, "utf-8");
    const { rows } = parseCatalogCsv(text, CODES);
    const skus = new Set<string>();
    let barcoded = 0;
    for (const row of rows) {
      expect(row.name_fr.trim()).not.toBe("");
      expect((row.name_ar ?? "").trim()).not.toBe("");
      expect(row.sku).toBeDefined();
      expect(skus.has(row.sku ?? "")).toBe(false);
      skus.add(row.sku ?? "");
      const purchase = row.purchase_price_centimes ?? 0;
      const margin = (row.sale_price_centimes - purchase) / row.sale_price_centimes;
      expect(margin).toBeGreaterThanOrEqual(0.15);
      expect(margin).toBeLessThanOrEqual(0.4);
      if (row.barcode !== undefined && row.barcode !== null && row.barcode !== "") {
        barcoded += 1;
        expect(isValidEan13(row.barcode)).toBe(true);
      }
    }
    expect(barcoded / rows.length).toBeGreaterThanOrEqual(0.7);
  });
});
