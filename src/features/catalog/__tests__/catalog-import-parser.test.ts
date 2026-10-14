/**
 * CSV import parser tests (TASK-DSN-045).
 *
 * The zero-dependency parser is fully unit-tested: semicolon/comma
 * dialects, quoted fields, float-free centimes/micros math, row
 * validation, BOM stripping, and template generation.
 */

import { describe, expect, it } from "vitest";
import {
  generateSampleCsvTemplate,
  IMPORT_TEMPLATE_HEADER,
  parseCatalogCsv,
  parsePriceToCentimes,
  parseQuantityToMicros,
  splitCsvFields,
} from "@/features/catalog/catalog-import-parser";

const SEMICOLON_CSV = [
  "sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;category_code",
  "VIS-PLA-25;6111250090011;Vis Placo 3.5x25;برغي بلاكو;120.50;150.00;50;VISS",
  "COU-PVC-32;;Coude PVC 32;كوع بلاستيك 32;85;110.00;100;PLOMB",
  "CAB-2.5;;Câble électrique 2.5mm;كابل كهربائي;210.75;260.00;25;ELEC",
].join("\n");

describe("parseCatalogCsv semicolon dialect", () => {
  it("parses French accented names and centimes prices", () => {
    const { rows, errors } = parseCatalogCsv(SEMICOLON_CSV);
    expect(errors).toEqual([]);
    expect(rows).toHaveLength(3);
    expect(rows[0]).toMatchObject({
      sku: "VIS-PLA-25",
      barcode: "6111250090011",
      name_fr: "Vis Placo 3.5x25",
      name_ar: "برغي بلاكو",
      purchase_price_centimes: 12050,
      sale_price_centimes: 15000,
      initial_quantity_micro: 500_000,
      category_id: "VISS",
    });
    expect(rows[2]?.name_fr).toBe("Câble électrique 2.5mm");
    expect(rows[2]?.purchase_price_centimes).toBe(21075);
  });
});

describe("parseCatalogCsv comma dialect", () => {
  it("parses quoted descriptions containing commas", () => {
    const csv = [
      "sku,barcode,designation,prix_achat,prix_vente,stock",
      'VIS-1,6111250000001,"Vis, Placo",100.10,150.20,10',
    ].join("\n");
    const { rows, errors } = parseCatalogCsv(csv);
    expect(errors).toEqual([]);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.name_fr).toBe("Vis, Placo");
    expect(rows[0]?.purchase_price_centimes).toBe(10010);
    expect(rows[0]?.sale_price_centimes).toBe(15020);
    expect(rows[0]?.initial_quantity_micro).toBe(100_000);
  });

  it("unescapes doubled quotes", () => {
    expect(splitCsvFields('"a ""quoted"" b",c', ",")).toEqual(['a "quoted" b', "c"]);
  });
});

describe("float-free money math", () => {
  it("computes centimes without float drift", () => {
    expect(parsePriceToCentimes("150.00")).toBe(15000);
    expect(parsePriceToCentimes("150,50")).toBe(15050);
    expect(parsePriceToCentimes("0.07")).toBe(7);
    expect(parsePriceToCentimes("1 500,50")).toBe(150050);
    expect(parsePriceToCentimes("abc")).toBeNull();
    expect(parsePriceToCentimes("")).toBeUndefined();
  });

  it("computes micros without float drift", () => {
    expect(parseQuantityToMicros("50")).toBe(500_000);
    expect(parseQuantityToMicros("2.5")).toBe(25_000);
    expect(parseQuantityToMicros("0,0001")).toBe(1);
    expect(parseQuantityToMicros("")).toBeUndefined();
  });
});

describe("row validation", () => {
  it("flags missing names and negative prices with line numbers", () => {
    const csv = [
      "sku;name_fr;sale_price",
      "A-1;Vis Placo;150.00",
      "A-2;;150.00",
      "A-3;Coude PVC;-5",
      "A-4;Robinet;",
    ].join("\n");
    const { rows, errors } = parseCatalogCsv(csv);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.sku).toBe("A-1");
    expect(errors).toHaveLength(3);
    expect(errors[0]).toMatchObject({ line: 3, message: "missing name_fr" });
    expect(errors[1]).toMatchObject({ line: 4, message: "invalid sale_price" });
    expect(errors[2]).toMatchObject({ line: 5, message: "missing sale_price" });
  });

  it("uppercases codes and resolves categories through the lookup", () => {
    const lookup = new Map([["VISS", "cat-uuid-1"]]);
    const csv = ["sku;name_fr;sale_price;rayon", "vis-1;Vis;10;viss"].join("\n");
    const { rows } = parseCatalogCsv(csv, lookup);
    expect(rows[0]?.sku).toBe("VIS-1");
    expect(rows[0]?.category_id).toBe("cat-uuid-1");
  });
});

describe("BOM handling", () => {
  it("strips a leading UTF-8 BOM cleanly", () => {
    const { rows, errors } = parseCatalogCsv("﻿" + SEMICOLON_CSV);
    expect(errors).toEqual([]);
    expect(rows).toHaveLength(3);
    expect(rows[0]?.sku).toBe("VIS-PLA-25");
  });
});

describe("generateSampleCsvTemplate", () => {
  it("returns BOM-prefixed CSV with the required headers", () => {
    const template = generateSampleCsvTemplate();
    expect(template.charCodeAt(0)).toBe(0xfeff);
    expect(template).toContain(IMPORT_TEMPLATE_HEADER);
    const lines = template
      .slice(1)
      .split("\n")
      .filter((line) => line.trim() !== "");
    expect(lines).toHaveLength(4);
  });
});
