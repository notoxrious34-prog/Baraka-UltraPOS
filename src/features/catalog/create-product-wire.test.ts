/**
 * Contract tests for the catalog creation wire builder.
 * These pin the exact backend key names (CreateProductDto field-for-field),
 * so a drift between this form and the Rust DTO fails here — not in front
 * of the store owner.
 */

import { describe, expect, it } from "vitest";
import {
  applyUnitPreset,
  BASE_FACTOR_MICRO,
  buildCreateProductWire,
  emptyProductForm,
  emptyUnit,
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
      preset: "CUSTOM",
      nameAr: "قطعة",
      nameFr: "Pièce",
      factor: "1",
      salePrice: "150.50",
      costPrice: "120",
      barcode: "6111250000013",
    },
    extras: [
      {
        preset: "CUSTOM",
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
          wholesale_price_centimes: null,
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
          wholesale_price_centimes: null,
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
          preset: "CUSTOM",
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

  it("accepts a blank Arabic product name and falls back to French on the wire", () => {
    const form: ProductFormData = { ...validForm(), nameAr: "   " };
    expect(validateProductForm(form).nameAr).toBeUndefined();
    const wire = buildCreateProductWire(form);
    expect(wire.name_ar).toBe("Vis");
    expect(wire.name_fr).toBe("Vis");
  });

  it("defaults new unit rows to the PCS preset with canonical names", () => {
    const unit = emptyUnit();
    expect(unit.preset).toBe("PCS");
    expect(unit.nameFr).toBe("PCS");
    expect(unit.nameAr).toBe("قطعة");
    expect(Object.keys(validateProductForm(emptyProductForm())).length).toBeGreaterThan(0);
    const form: ProductFormData = {
      sku: "VIS-8X40",
      nameAr: "",
      nameFr: "Vis",
      barcode: "",
      base: { ...emptyUnit(), salePrice: "", costPrice: "", barcode: "" },
      extras: [],
    };
    expect(Object.keys(validateProductForm(form)).length).toBe(0);
    const wire = buildCreateProductWire(form);
    expect(wire.units[0]?.unit_name_fr).toBe("PCS");
    expect(wire.units[0]?.unit_name_ar).toBe("قطعة");
  });

  it("applyUnitPreset maps presets to bilingual names and keeps CUSTOM values", () => {
    const base = emptyUnit();
    expect(applyUnitPreset(base, "SERIE")).toMatchObject({
      preset: "SERIE",
      nameFr: "SERIE",
      nameAr: "سيريا",
    });
    expect(applyUnitPreset(base, "CTN")).toMatchObject({
      preset: "CTN",
      nameFr: "CTN",
      nameAr: "كرتون",
    });
    expect(applyUnitPreset(base, "PCS")).toMatchObject({
      preset: "PCS",
      nameFr: "PCS",
      nameAr: "قطعة",
    });
    const custom = applyUnitPreset(base, "CUSTOM");
    expect(custom.preset).toBe("CUSTOM");
    expect(custom.nameFr).toBe("PCS");
    const blankCustom = applyUnitPreset(
      { ...emptyUnit(), preset: "CUSTOM", nameAr: "", nameFr: "" },
      "CUSTOM",
    );
    expect(validateProductForm({ ...validForm(), base: blankCustom })["base.nameFr"]).toBe(
      "cat.required",
    );
  });

  it("falls back blank CUSTOM unit Arabic names to French on the wire", () => {
    const form: ProductFormData = {
      ...validForm(),
      base: { ...validForm().base, nameAr: "" },
    };
    expect(validateProductForm(form)["base.nameAr"]).toBeUndefined();
    const wire = buildCreateProductWire(form);
    expect(wire.units[0]?.unit_name_ar).toBe("Pièce");
  });
});
