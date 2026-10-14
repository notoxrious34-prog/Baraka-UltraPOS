/**
 * Unit tests for the product edit workstation (TASK-BRK-050).
 *
 * Covers form initialization from live catalog views, exact DZD↔centime
 * conversion without floats, both-names validation parity with the
 * backend, price validation, successful submission with full-state unit
 * echo, and the single-flight double-submit guard.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  centsToDzdText,
  formatMargin,
  useProductEditStore,
  validateProductEditForm,
  type ProductEditFormState,
} from "@/stores/product-edit-store";
import { useSessionStore } from "@/stores/session-store";
import type { CatalogProduct, SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

/** The bridge probe requires a `window` carrying the Tauri marker. */
function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function primeSession(): void {
  fakeTauriWindow();
  const session: SessionInfo = {
    token: "tok-edit-9",
    userId: "55555555-5555-5555-5555-555555555555",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
  };
  useSessionStore.setState({ session });
  useProductEditStore.getState().reset();
}

const product: CatalogProduct = {
  product_id: "11111111-1111-4111-8111-111111111111",
  sku: "VIS-8X40",
  barcode: null,
  name_ar: "برغي",
  name_fr: "Vis 8x40",
  category_id: null,
  is_kit: false,
  is_active: true,
  units: [
    {
      unit_id: "22222222-2222-4222-8222-222222222222",
      product_id: "11111111-1111-4111-8111-111111111111",
      unit_name_ar: "قطعة",
      unit_name_fr: "Pièce",
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
};

function blankForm(): ProductEditFormState {
  return {
    productId: "p",
    sku: "S",
    nameAr: "a",
    nameFr: "b",
    categoryId: "",
    barcode: "",
    baseUnitName: "u",
    sellingPriceDzd: "10",
    costPriceDzd: "5",
    isActive: true,
  };
}

describe("centsToDzdText", () => {
  it("renders exact decimal text without floats", () => {
    expect(centsToDzdText("15050")).toBe("150.50");
    expect(centsToDzdText("0")).toBe("0.00");
    expect(centsToDzdText(12000n)).toBe("120.00");
    expect(centsToDzdText("nope")).toBe("0");
  });
});

describe("validateProductEditForm", () => {
  it("requires both names (backend parity)", () => {
    expect(validateProductEditForm(blankForm()).ok).toBe(true);
    expect(validateProductEditForm({ ...blankForm(), nameAr: "  " })).toEqual({
      ok: false,
      key: "nameRequired",
    });
    expect(validateProductEditForm({ ...blankForm(), nameFr: "" })).toEqual({
      ok: false,
      key: "nameRequired",
    });
  });

  it("rejects negative and unparsable prices", () => {
    expect(
      validateProductEditForm({ ...blankForm(), sellingPriceDzd: "-5" }),
    ).toEqual({ ok: false, key: "invalidPrice" });
    expect(validateProductEditForm({ ...blankForm(), costPriceDzd: "abc" })).toEqual({
      ok: false,
      key: "invalidPrice",
    });
  });

  it("parses DZD text to exact centimes", () => {
    const checked = validateProductEditForm({
      ...blankForm(),
      sellingPriceDzd: "150,50",
      costPriceDzd: "120",
    });
    expect(checked).toEqual({ ok: true, sellingCentimes: "15050", costCentimes: "12000" });
  });
});

describe("formatMargin", () => {
  it("subtracts in integer cents", () => {
    expect(formatMargin("15050", "12000", "fr-DZ")).toContain("30,50");
  });
});

describe("product edit store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("initializes the form from live catalog values", () => {
    useProductEditStore.getState().openForProduct(product);
    const { form, baseUnitId } = useProductEditStore.getState();
    expect(form.productId).toBe(product.product_id);
    expect(form.sku).toBe("VIS-8X40");
    expect(form.nameAr).toBe("برغي");
    expect(form.sellingPriceDzd).toBe("150.50");
    expect(form.costPriceDzd).toBe("120.00");
    expect(form.barcode).toBe("6111250000013");
    expect(baseUnitId).toBe("22222222-2222-4222-8222-222222222222");
  });

  it("submits the full-state payload and flags success", async () => {
    useProductEditStore.getState().openForProduct(product);
    useProductEditStore.getState().setFormField("sellingPriceDzd", "160");
    invokeMock.mockResolvedValueOnce({ ...product, name_fr: "Vis 8x40" });
    const ok = await useProductEditStore.getState().submitUpdate();
    expect(ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("catalog_update_product", {
      request: {
        session_token: "tok-edit-9",
        product_id: product.product_id,
        name_ar: "برغي",
        name_fr: "Vis 8x40",
        category_id: null,
        is_active: true,
        units: [
          {
            unit_id: "22222222-2222-4222-8222-222222222222",
            unit_name_ar: "قطعة",
            unit_name_fr: "Pièce",
            factor_micro: "10000",
            barcode: "6111250000013",
            sale_price_centimes: "16000",
            purchase_price_centimes: "12000",
            wholesale_price_centimes: null,
            is_base_unit: true,
            is_default_selling: true,
          },
        ],
      },
      session_token: "tok-edit-9",
    });
    expect(useProductEditStore.getState().successMessage).toBe("updateSuccess");
    expect(useProductEditStore.getState().isSubmitting).toBe(false);
  });

  it("blocks concurrent double submission behind the lock", async () => {
    useProductEditStore.getState().openForProduct(product);
    invokeMock.mockResolvedValue({ ...product });
    const first = useProductEditStore.getState().submitUpdate();
    const second = useProductEditStore.getState().submitUpdate();
    const [a, b] = await Promise.all([first, second]);
    expect(a).toBe(true);
    expect(b).toBe(false);
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("surfaces backend errors without swallowing", async () => {
    useProductEditStore.getState().openForProduct(product);
    invokeMock.mockRejectedValueOnce("UNAUTHORIZED");
    const ok = await useProductEditStore.getState().submitUpdate();
    expect(ok).toBe(false);
    expect(useProductEditStore.getState().errorMessage).toBe("UNAUTHORIZED");
    expect(useProductEditStore.getState().successMessage).toBeNull();
  });
});
