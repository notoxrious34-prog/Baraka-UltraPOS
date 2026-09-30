/**
 * Contract tests for the catalog creation wire builder.
 * These pin the exact backend key names (CreateProductDto field-for-field),
 * so a drift between this form and the Rust DTO fails here — not in front
 * of the store owner.
 */

import { describe, expect, it } from "vitest";
import {
  BASE_FACTOR_MICRO,
  buildCreateProductWire,
  emptyProductForm,
  validateProductForm,
  type ProductFormData,
} from "@/features/catalog/ProductCreateForm";

function validForm(): ProductFormData {
  return {
    sku: "VIS-8X40",
    nameAr: "برغي",
    nameFr: "Vis",
    barcode: "",
    base: {
      nameAr: "قطعة",
      nameFr: "Pièce",
      factor: "1",
      salePrice: "150.50",
      costPrice: "120",
      barcode: "6111250000013",
    },
    extras: [
      {
        nameAr: "علبة",
        nameFr: "Boîte",
        factor: "12",
        salePrice: "",
        costPrice: "1400",
        barcode: "",
      },
    ],
  };
}

describe("buildCreateProductWire", () => {
  it("emits exact backend key names and canonical value strings", () => {
    const wire = buildCreateProductWire(validForm());
    expect(wire).toEqual({
      sku: "VIS-8X40",
      barcode: null,
      name_ar: "برغي",
      name_fr: "Vis",
      category_id: null,
      is_kit: false,
      units: [
        {
          unit_name_ar: "قطعة",
          unit_name_fr: "Pièce",
          factor_micro: BASE_FACTOR_MICRO,
          barcode: "6111250000013",
          sale_price_centimes: "15050",
          purchase_price_centimes: "12000",
          is_base_unit: true,
          is_default_selling: true,
        },
        {
          unit_name_ar: "علبة",
          unit_name_fr: "Boîte",
          factor_micro: "120000",
          barcode: null,
          sale_price_centimes: "0",
          purchase_price_centimes: "140000",
          is_base_unit: false,
          is_default_selling: false,
        },
      ],
    });
    expect(BASE_FACTOR_MICRO).toBe("10000");
  });

  it("throws a message key on invalid input", () => {
    const bad = { ...emptyProductForm() };
    expect(() => buildCreateProductWire(bad)).toThrowError("cat.required");
    const badFactor: ProductFormData = {
      ...validForm(),
      extras: [
        {
          nameAr: "علبة",
          nameFr: "Boîte",
          factor: "0",
          salePrice: "",
          costPrice: "1400",
          barcode: "",
        },
      ],
    };
    expect(() => buildCreateProductWire(badFactor)).toThrowError("cat.invalidFactor");
  });

  it("validateProductForm flags blanks, overlong names, and bad amounts", () => {
    expect(Object.keys(validateProductForm(validForm())).length).toBe(0);
    const errors = validateProductForm({
      ...validForm(),
      nameFr: "x".repeat(101),
      base: { ...validForm().base, salePrice: "nope" },
    });
    expect(errors.nameFr).toBe("cat.required");
    expect(errors["base.salePrice"]).toBe("cat.invalidPrice");
  });
});
