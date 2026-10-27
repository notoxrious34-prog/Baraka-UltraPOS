/**
 * Multi-unit cockpit store tests (TASK-DSN-046):
 * `setLineUnit`, `cycleLineUnit` structured feedback, and the pure
 * `calculateBaseQuantityEquivalent` BigInt helper (zero floats).
 */

import { beforeEach, describe, expect, it } from "vitest";
import type { CartLine, CartUnitSnapshot, CatalogProduct } from "@/types/pos";
import {
  calculateBaseQuantityEquivalent,
  previewDiscountedUnitPrice,
  previewLineTotal,
  useCartStore,
} from "@/stores/cart-store";

function unit(overrides: Partial<CartUnitSnapshot> & { unitId: string }): CartUnitSnapshot {
  return {
    productId: "p-vis",
    nameAr: "قطعة",
    nameFr: "Pièce",
    factorMicro: "10000",
    barcode: null,
    salePriceCentimes: "15020",
    wholesalePriceCentimes: null,
    isBaseUnit: true,
    isDefaultSelling: true,
    ...overrides,
  };
}

const PIECE = unit({ unitId: "u-piece" });
const BOX = unit({
  unitId: "u-box",
  nameAr: "علبة",
  nameFr: "Boîte",
  factorMicro: "1000000",
  barcode: "BOX-001",
  salePriceCentimes: "1205000",
  isBaseUnit: false,
  isDefaultSelling: false,
});

function product(): CatalogProduct {
  return {
    product_id: "p-vis",
    sku: "VIS-PLA-25",
    barcode: "PCS-001",
    name_ar: "برغي بلاكو",
    name_fr: "Vis Placo",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [
      {
        unit_id: "u-piece",
        product_id: "p-vis",
        unit_name_ar: "قطعة",
        unit_name_fr: "Pièce",
        factor_micro: "10000",
        barcode: "PCS-001",
        sale_price_centimes: "15020",
        purchase_price_centimes: "10010",
        wholesale_price_centimes: null,
        is_base_unit: true,
        is_default_selling: true,
      },
      {
        unit_id: "u-box",
        product_id: "p-vis",
        unit_name_ar: "علبة",
        unit_name_fr: "Boîte",
        factor_micro: "1000000",
        barcode: "BOX-001",
        sale_price_centimes: "1205000",
        purchase_price_centimes: "900000",
        wholesale_price_centimes: null,
        is_base_unit: false,
        is_default_selling: false,
      },
    ],
    kit_items: [],
  };
}

function line(overrides: Partial<CartLine> = {}): CartLine {
  return {
    key: "p-vis:u-piece",
    productId: "p-vis",
    sku: "VIS-PLA-25",
    nameAr: "برغي بلاكو",
    nameFr: "Vis Placo",
    units: [PIECE, BOX],
    unitIndex: 0,
    quantityMicro: "30000",
    lineDiscountCentimes: "0",
    ...overrides,
  };
}

beforeEach(() => {
  useCartStore.getState().clearCart();
});

describe("setLineUnit", () => {
  it("switches the unit and recomputes the line total", () => {
    useCartStore.getState().loadLines([line()]);
    useCartStore.getState().setLineUnit("p-vis:u-piece", 1);
    const state = useCartStore.getState();
    expect(state.lines[0]?.unitIndex).toBe(1);
    // 3 x 12050.00 DZD = 36150.00 DZD -> 3615000 centimes.
    expect(previewLineTotal(state.lines[0] as CartLine)).toBe("3615000");
  });

  it("ignores out-of-range unit indexes", () => {
    useCartStore.getState().loadLines([line()]);
    useCartStore.getState().setLineUnit("p-vis:u-piece", 7);
    useCartStore.getState().setLineUnit("p-vis:u-piece", -1);
    expect(useCartStore.getState().lines[0]?.unitIndex).toBe(0);
  });

  it("ignores unknown line keys", () => {
    useCartStore.getState().loadLines([line()]);
    useCartStore.getState().setLineUnit("nope", 1);
    expect(useCartStore.getState().lines[0]?.unitIndex).toBe(0);
  });
});

describe("cycleLineUnit", () => {
  it("rotates through units and returns structured metadata", () => {
    useCartStore.getState().loadLines([line()]);
    const first = useCartStore.getState().cycleLineUnit("p-vis:u-piece");
    expect(first).toEqual({
      cycled: true,
      unitCount: 2,
      unitNameFr: "Boîte",
      unitNameAr: "علبة",
      salePriceCentimes: "1205000",
    });
    expect(useCartStore.getState().lines[0]?.unitIndex).toBe(1);
    const second = useCartStore.getState().cycleLineUnit("p-vis:u-piece");
    expect(second.cycled).toBe(true);
    expect(second.unitNameFr).toBe("Pièce");
    expect(useCartStore.getState().lines[0]?.unitIndex).toBe(0);
  });

  it("returns cycled:false with unit metadata for single-unit lines", () => {
    useCartStore.getState().loadLines([line({ units: [PIECE] })]);
    const result = useCartStore.getState().cycleLineUnit("p-vis:u-piece");
    expect(result).toEqual({
      cycled: false,
      unitCount: 1,
      unitNameFr: "Pièce",
      unitNameAr: "قطعة",
      salePriceCentimes: "15020",
    });
    expect(useCartStore.getState().lines[0]?.unitIndex).toBe(0);
  });

  it("returns safe defaults for unknown keys", () => {
    const result = useCartStore.getState().cycleLineUnit("missing");
    expect(result.cycled).toBe(false);
    expect(result.salePriceCentimes).toBe("0");
  });
});

describe("addProduct with unit fixtures", () => {
  it("adds the product with the exact unit index (barcode resolution)", () => {
    useCartStore.getState().addProduct(product(), 1);
    const state = useCartStore.getState();
    expect(state.lines).toHaveLength(1);
    expect(state.lines[0]?.unitIndex).toBe(1);
    expect(state.lines[0]?.key).toBe("p-vis:u-box:1");
  });
});

describe("calculateBaseQuantityEquivalent", () => {
  it("converts 3 boxes x100 to (= 300 Pièces) without float drift", () => {
    expect(
      calculateBaseQuantityEquivalent("30000", "1000000", "Pièces", "قطعة", "fr-DZ"),
    ).toBe("(= 300 Pièces)");
  });

  it("uses the Arabic base name for ar-DZ", () => {
    expect(
      calculateBaseQuantityEquivalent("30000", "1000000", "Pièces", "قطعة", "ar-DZ"),
    ).toBe("(= 300 قطعة)");
  });

  it("formats fractional equivalents cleanly (0.5)", () => {
    expect(
      calculateBaseQuantityEquivalent("25000", "2000", "Mètre", "متر", "fr-DZ"),
    ).toBe("(= 0.5 Mètre)");
    expect(
      calculateBaseQuantityEquivalent("50000", "20000", "Mètre", "متر", "fr-DZ"),
    ).toBe("(= 10 Mètre)");
  });

  it("returns empty for the base unit itself", () => {
    expect(
      calculateBaseQuantityEquivalent("30000", "10000", "Pièces", "قطعة", "fr-DZ"),
    ).toBe("");
  });

  it("returns empty for unparseable inputs", () => {
    expect(
      calculateBaseQuantityEquivalent("abc", "1000000", "Pièces", "قطعة", "fr-DZ"),
    ).toBe("");
  });
});

describe("per-unit discount display (DSN-097)", () => {
  it("prices the F5 amount on EACH unit (owner example: 3 x (1000 - 100) = 2700)", () => {
    const owner = line({
      units: [unit({ unitId: "u-piece", salePriceCentimes: "1000" })],
      quantityMicro: "30000",
      lineDiscountCentimes: "100",
    });
    expect(previewDiscountedUnitPrice(owner)).toBe("900");
    expect(previewLineTotal(owner)).toBe("2700");
  });

  it("shows the raw price and gross-derived total when no discount is set", () => {
    const plain = line();
    expect(previewDiscountedUnitPrice(plain)).toBe("15020");
    // 3 x 150.20 = 450.60 -> 45060 centimes.
    expect(previewLineTotal(plain)).toBe("45060");
  });

  it("floors a corrupt over-price discount at zero instead of going negative", () => {
    const corrupt = line({ lineDiscountCentimes: "99999" });
    expect(previewDiscountedUnitPrice(corrupt)).toBe("0");
    expect(previewLineTotal(corrupt)).toBe("0");
  });
});
