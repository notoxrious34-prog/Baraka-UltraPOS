/**
 * Product edition modal tests (TASK-DSN-042).
 *
 * Pure helpers (decimal conversion, pre-fill, validation, payload assembly,
 * update flow) are unit-tested directly; the presentational modal renders via
 * `renderToStaticMarkup` (node-only harness, no DOM library — same pattern
 * as the hub view tests); hub integration (edit action, status toggle, F1
 * focus) is pinned via source-text assertions, per repo convention.
 */

import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  buildUpdatePayload,
  centimesToDecimalInput,
  formFromProduct,
  ProductEditModalView,
  submitEditProduct,
  validateEditForm,
} from "@/features/catalog/ProductEditModal";
import { t } from "@/lib/i18n";
import type { CatalogProduct } from "@/types/pos";

declare const process: { cwd(): string };

function unit(overrides: Partial<CatalogProduct["units"][number]>) {
  return {
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
    category_id: "cat-9",
    is_kit: false,
    is_active: true,
    units: [unit({})],
    kit_items: [],
    ...overrides,
  };
}

describe("centimesToDecimalInput", () => {
  it("converts canonical centimes to decimal inputs with BigInt math", () => {
    expect(centimesToDecimalInput("15050")).toBe("150.5");
    expect(centimesToDecimalInput("12000")).toBe("120");
    expect(centimesToDecimalInput("0")).toBe("0");
    expect(centimesToDecimalInput("5")).toBe("0.05");
    expect(centimesToDecimalInput("-250")).toBe("-2.5");
    expect(centimesToDecimalInput("")).toBe("0");
  });

  it("falls back to zero on malformed input", () => {
    expect(centimesToDecimalInput("nope")).toBe("0");
  });
});

describe("formFromProduct", () => {
  it("pre-fills names, base-unit barcode, prices and active flag", () => {
    expect(formFromProduct(product({}))).toEqual({
      nameFr: "Vis 8x40",
      nameAr: "برغي",
      barcode: "6111250000013",
      salePrice: "150.5",
      costPrice: "120",
      isActive: true,
    });
  });

  it("tolerates missing barcodes and unit-less products", () => {
    const bare = formFromProduct(product({ units: [unit({ barcode: null })] }));
    expect(bare.barcode).toBe("");
    const unitless = formFromProduct(product({ units: [], is_active: false }));
    expect(unitless).toMatchObject({ salePrice: "0", costPrice: "0", isActive: false });
  });
});

describe("validateEditForm", () => {
  const valid = formFromProduct(product({}));

  it("accepts the pre-filled form", () => {
    expect(validateEditForm(valid)).toEqual({});
  });

  it("requires the French designation within 100 chars", () => {
    expect(validateEditForm({ ...valid, nameFr: "  " }).nameFr).toBe("cat.required");
    expect(validateEditForm({ ...valid, nameFr: "x".repeat(101) }).nameFr).toBe("cat.required");
  });

  it("keeps Arabic optional but length-guarded", () => {
    expect(validateEditForm({ ...valid, nameAr: "" })).toEqual({});
    expect(validateEditForm({ ...valid, nameAr: "x".repeat(101) }).nameAr).toBe("cat.required");
  });

  it("rejects negative and malformed prices", () => {
    expect(validateEditForm({ ...valid, salePrice: "-1" }).salePrice).toBe("cat.invalidPrice");
    expect(validateEditForm({ ...valid, costPrice: "abc" }).costPrice).toBe("cat.invalidPrice");
  });
});

describe("buildUpdatePayload", () => {
  it("emits the exact full-state payload preserving SKU-immutable fields", () => {
    const payload = buildUpdatePayload("tok-1", product({}), formFromProduct(product({})));
    expect(payload).toEqual({
      session_token: "tok-1",
      product_id: "p-1",
      name_ar: "برغي",
      name_fr: "Vis 8x40",
      category_id: "cat-9",
      is_active: true,
      units: [
        {
          unit_id: "u-1",
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
    });
    expect("sku" in payload).toBe(false);
  });

  it("applies edited names, barcode, prices and the active flag", () => {
    const payload = buildUpdatePayload("t", product({}), {
      nameFr: "Vis 8x42",
      nameAr: "",
      barcode: "999",
      salePrice: "200",
      costPrice: "150.25",
      isActive: false,
    });
    expect(payload.name_fr).toBe("Vis 8x42");
    expect(payload.name_ar).toBe("Vis 8x42");
    expect(payload.is_active).toBe(false);
    expect(payload.units[0]).toMatchObject({
      barcode: "999",
      sale_price_centimes: "20000",
      purchase_price_centimes: "15025",
      wholesale_price_centimes: null,
    });
  });

  it("clears the unit barcode and zeroes blank prices", () => {
    const payload = buildUpdatePayload("t", product({}), {
      ...formFromProduct(product({})),
      barcode: "   ",
      salePrice: "",
      costPrice: "",
    });
    expect(payload.units[0]).toMatchObject({
      barcode: null,
      sale_price_centimes: "0",
      purchase_price_centimes: "0",
      wholesale_price_centimes: null,
    });
  });

  it("preserves non-base units untouched", () => {
    const extra = unit({
      unit_id: "u-2",
      is_base_unit: false,
      is_default_selling: false,
      factor_micro: "120000",
      sale_price_centimes: "99999",
    });
    const payload = buildUpdatePayload("t", product({ units: [unit({}), extra] }), {
      ...formFromProduct(product({ units: [unit({}), extra] })),
      salePrice: "1",
    });
    expect(payload.units).toHaveLength(2);
    expect(payload.units[1]).toMatchObject({
      unit_id: "u-2",
      factor_micro: "120000",
      sale_price_centimes: "99999",
    });
    expect(payload.units[0]?.sale_price_centimes).toBe("100");
  });

  it("throws the first message key on invalid input", () => {
    const bad = { ...formFromProduct(product({})), nameFr: "" };
    expect(() => buildUpdatePayload("t", product({}), bad)).toThrow("cat.required");
  });
});

describe("submitEditProduct", () => {
  it("updates then re-reads the fresh detail view", async () => {
    const fresh = product({ name_fr: "Vis 8x42" });
    const update = vi.fn().mockResolvedValue(undefined);
    const get = vi.fn().mockResolvedValue(fresh);
    const result = await submitEditProduct("tok-9", product({}), formFromProduct(product({})), {
      update,
      get,
    });
    expect(result).toBe(fresh);
    expect(update).toHaveBeenCalledTimes(1);
    expect(update.mock.calls[0]?.[0]).toMatchObject({
      session_token: "tok-9",
      product_id: "p-1",
    });
    expect(get).toHaveBeenCalledWith("tok-9", "p-1");
  });

  it("propagates backend wire codes verbatim", async () => {
    const update = vi.fn().mockRejectedValue("UNAUTHORIZED");
    const get = vi.fn();
    await expect(
      submitEditProduct("t", product({}), formFromProduct(product({})), { update, get }),
    ).rejects.toBe("UNAUTHORIZED");
    expect(get).not.toHaveBeenCalled();
  });
});

describe("ProductEditModalView markup", () => {
  const noop = (): void => undefined;
  const noopPatch = (): void => undefined;
  const nullRef = { current: null };
  const view = (locale: "fr-DZ" | "ar-DZ", overrides: Partial<CatalogProduct> = {}) => (
    <ProductEditModalView
      product={product(overrides)}
      form={formFromProduct(product(overrides))}
      locale={locale}
      attempted={false}
      busy={false}
      submitError={null}
      cardRef={nullRef}
      onPatch={noopPatch}
      onClose={noop}
      onSubmit={noop}
    />
  );

  it("pre-fills the form with a read-only SKU and editable fields", () => {
    const html = renderToStaticMarkup(view("fr-DZ"));
    expect(html).toContain('role="dialog"');
    expect(html).toContain("VIS-8X40");
    expect(html).toContain("Modifier le Produit");
    expect(html).toContain("Le SKU est immuable");
    expect(html).toContain('value="Vis 8x40"');
    expect(html).toContain('value="150.5"');
    expect(html).toContain('value="120"');
    expect(html).toContain("checked");
    expect(html).toContain("+25.41%");
  });

  it("renders the Arabic strings and RTL designation input", () => {
    const html = renderToStaticMarkup(view("ar-DZ", { is_active: false }));
    expect(html).toContain("تعديل المنتج");
    expect(html).toContain("منتج نشط للبيع");
    expect(html).toContain("سيتم إخفاء هذا المنتج من شاشة البيع");
    expect(html).toContain('dir="rtl"');
  });

  it("surfaces submit errors and the attempted validation state", () => {
    const html = renderToStaticMarkup(
      <ProductEditModalView
        product={product({})}
        form={{ ...formFromProduct(product({})), nameFr: "" }}
        locale="fr-DZ"
        attempted
        busy={false}
        submitError="UNAUTHORIZED"
        cardRef={nullRef}
        onPatch={noopPatch}
        onClose={noop}
        onSubmit={noop}
      />,
    );
    expect(html).toContain("Champ obligatoire");
    expect(html).toContain("Non autorisé pour votre rôle");
  });
});

describe("edit i18n keys", () => {
  it("declares the edit strings in FR and AR", () => {
    expect(t("fr-DZ", "cat.editProduct")).toBe("Modifier le Produit");
    expect(t("ar-DZ", "cat.editProduct")).toBe("تعديل المنتج");
    expect(t("fr-DZ", "cat.saveChanges")).toBe("Enregistrer les modifications");
    expect(t("ar-DZ", "cat.saveChanges")).toBe("حفظ التعديلات");
    expect(t("fr-DZ", "cat.productActiveLabel")).toBe("Produit actif pour la vente");
    expect(t("ar-DZ", "cat.productActiveLabel")).toBe("منتج نشط للبيع");
    expect(t("fr-DZ", "cat.quickSearchHelp")).toBe("F1 pour rechercher / scanner");
    expect(t("ar-DZ", "cat.quickSearchHelp")).toBe("F1 للبحث أو المسح بالباركود");
  });
});

describe("hub edit integration (source pins)", () => {
  const root = process.cwd();
  const hubSrc = readFileSync(join(root, "src", "features", "catalog", "CatalogModule.tsx"), "utf8") as string;

  it("adds an edit action per row next to the label action", () => {
    expect(hubSrc).toContain("onEdit={setEditProduct}");
    expect(hubSrc).toContain("props.onEdit(product)");
    expect(hubSrc).toContain("✏️");
  });

  it("mounts the edit modal with the in-place update handoff", () => {
    expect(hubSrc).toContain("<ProductEditModal");
    expect(hubSrc).toContain("isOpen={editProduct !== null}");
    expect(hubSrc).toContain("onUpdated={handleUpdated}");
    expect(hubSrc).toContain("prev.map((row) => (row.product_id === updated.product_id ? updated : row))");
  });

  it("toggles the status badge through a full-state update", () => {
    expect(hubSrc).toContain("onToggleActive={toggleActive}");
    expect(hubSrc).toContain("isActive: !target.is_active");
    expect(hubSrc).toContain("setTogglingId(target.product_id)");
  });

  it("focuses the search input on F1 for wedge scanners", () => {
    expect(hubSrc).toContain('id="catalog-search-input"');
    expect(hubSrc).toContain('"F1"');
    expect(hubSrc).toContain('document.getElementById("catalog-search-input")');
  });
});
