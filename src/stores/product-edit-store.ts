/**
 * Product edit store (TASK-BRK-050).
 *
 * Full-state product update over the existing `catalog_update_product`
 * command (SKU immutable, echoed unit set). Money travels exclusively as
 * canonical centime strings parsed by parseMoneyToCentimes (zero floats).
 * The backend re-validates everything (both names required, unit-set
 * invariants); client-side guards are UX-only.
 */

import { create } from "zustand";
import {
  updateProduct,
  type UpdateProductUnitPayload,
} from "@/lib/ipc";
import type { CatalogProduct } from "@/types/pos";
import { formatMoney, parseMoneyToCentimes } from "@/lib/money";
import { useSessionStore } from "@/stores/session-store";

export interface ProductEditFormState {
  productId: string;
  /** Read-only, never sent (backend ignores SKU on update). */
  sku: string;
  nameAr: string;
  nameFr: string;
  /** Read-only echo (no categories endpoint exists to offer a selector). */
  categoryId: string;
  /** Base-unit barcode under edit. */
  barcode: string;
  /** French base-unit name under edit. */
  baseUnitName: string;
  /** Human DZD input, e.g. "1250.00". */
  sellingPriceDzd: string;
  /** Human DZD input, e.g. "950.00". */
  costPriceDzd: string;
  isActive: boolean;
}

export interface ProductEditTarget {
  id: string;
  sku: string;
  name_ar: string;
  name_fr: string;
  category_id: string;
  barcode: string | null;
  base_unit_name: string;
  selling_price_cents: string | bigint;
  cost_price_cents: string | bigint;
  is_active: boolean;
}

/** Centime string/bigint → human DZD text (`"15050"` → `"150.50"`). */
export function centsToDzdText(cents: string | bigint): string {
  const raw = typeof cents === "bigint" ? cents.toString() : cents.trim();
  if (!/^-?\d+$/.test(raw)) {
    return "0";
  }
  const negative = raw.startsWith("-");
  const digits = negative ? raw.slice(1) : raw;
  const padded = digits.padStart(3, "0");
  const whole = padded.slice(0, -2).replace(/^0+(?=\d)/, "");
  const frac = padded.slice(-2);
  return `${negative ? "-" : ""}${whole}.${frac}`;
}

/** Human DZD input → canonical centime string (throws key on failure). */
export function parseDzdToCentimes(input: string): string {
  return parseMoneyToCentimes(input);
}

export type ProductEditValidation =
  | { readonly ok: true; readonly sellingCentimes: string; readonly costCentimes: string }
  | { readonly ok: false; readonly key: "nameRequired" | "invalidPrice" };

/**
 * Backend parity validation: BOTH names required
 * (`Product::validate_names` rejects either blank) and both prices
 * parseable non-negative centimes.
 */
export function validateProductEditForm(form: ProductEditFormState): ProductEditValidation {
  if (form.nameAr.trim() === "" || form.nameFr.trim() === "") {
    return { ok: false, key: "nameRequired" };
  }
  let selling = "0";
  let cost = "0";
  try {
    selling = parseMoneyToCentimes(form.sellingPriceDzd);
    cost = parseMoneyToCentimes(form.costPriceDzd);
  } catch {
    return { ok: false, key: "invalidPrice" };
  }
  if (BigInt(selling) < 0n || BigInt(cost) < 0n) {
    return { ok: false, key: "invalidPrice" };
  }
  return { ok: true, sellingCentimes: selling, costCentimes: cost };
}

/** Margin display: selling − cost as formatted DZD (BigInt only). */
export function formatMargin(
  sellingCentimes: string,
  costCentimes: string,
  locale: "ar-DZ" | "fr-DZ",
): string {
  try {
    return formatMoney((BigInt(sellingCentimes) - BigInt(costCentimes)).toString(), locale);
  } catch {
    return "…";
  }
}

interface ProductEditStoreState {
  isOpen: boolean;
  isSubmitting: boolean;
  errorMessage: string | null;
  successMessage: string | null;
  form: ProductEditFormState;
  /** Untouched unit snapshots echoed back (full-state contract). */
  unitsSnapshot: CatalogProduct["units"];
  baseUnitId: string | null;
  openForProduct: (product: CatalogProduct) => void;
  openForTarget: (target: ProductEditTarget) => void;
  closeModal: () => void;
  setFormField: <K extends keyof ProductEditFormState>(
    field: K,
    value: ProductEditFormState[K],
  ) => void;
  submitUpdate: () => Promise<boolean>;
  reset: () => void;
}

function emptyForm(): ProductEditFormState {
  return {
    productId: "",
    sku: "",
    nameAr: "",
    nameFr: "",
    categoryId: "",
    barcode: "",
    baseUnitName: "",
    sellingPriceDzd: "",
    costPriceDzd: "",
    isActive: true,
  };
}

const initialState = {
  isOpen: false,
  isSubmitting: false,
  errorMessage: null as string | null,
  successMessage: null as string | null,
  form: emptyForm(),
  unitsSnapshot: [] as CatalogProduct["units"],
  baseUnitId: null as string | null,
};

export function targetFromProduct(product: CatalogProduct): ProductEditTarget {
  const base =
    product.units.find((unit) => unit.is_base_unit) ??
    product.units.find((unit) => unit.is_default_selling) ??
    product.units[0];
  return {
    id: product.product_id,
    sku: product.sku,
    name_ar: product.name_ar,
    name_fr: product.name_fr,
    category_id: product.category_id ?? "",
    barcode: base?.barcode ?? product.barcode ?? null,
    base_unit_name: base?.unit_name_fr ?? "",
    selling_price_cents: base?.sale_price_centimes ?? "0",
    cost_price_cents: base?.purchase_price_centimes ?? "0",
    is_active: product.is_active,
  };
}

export const useProductEditStore = create<ProductEditStoreState>()((set, get) => ({
  ...initialState,

  openForProduct: (product) => {
    const base =
      product.units.find((unit) => unit.is_base_unit) ??
      product.units.find((unit) => unit.is_default_selling) ??
      product.units[0];
    set({
      ...initialState,
      isOpen: true,
      form: {
        productId: product.product_id,
        sku: product.sku,
        nameAr: product.name_ar,
        nameFr: product.name_fr,
        categoryId: product.category_id ?? "",
        barcode: base?.barcode ?? product.barcode ?? "",
        baseUnitName: base?.unit_name_fr ?? "",
        sellingPriceDzd: centsToDzdText(base?.sale_price_centimes ?? "0"),
        costPriceDzd: centsToDzdText(base?.purchase_price_centimes ?? "0"),
        isActive: product.is_active,
      },
      unitsSnapshot: product.units,
      baseUnitId: base?.unit_id ?? null,
    });
  },

  openForTarget: (target) => {
    set({
      ...initialState,
      isOpen: true,
      form: {
        productId: target.id,
        sku: target.sku,
        nameAr: target.name_ar,
        nameFr: target.name_fr,
        categoryId: target.category_id,
        barcode: target.barcode ?? "",
        baseUnitName: target.base_unit_name,
        sellingPriceDzd: centsToDzdText(target.selling_price_cents),
        costPriceDzd: centsToDzdText(target.cost_price_cents),
        isActive: target.is_active,
      },
      unitsSnapshot: [],
      baseUnitId: null,
    });
  },

  closeModal: () => {
    if (get().isSubmitting) {
      return;
    }
    set({ ...initialState });
  },

  setFormField: (field, value) => {
    set((state) => ({ form: { ...state.form, [field]: value } }));
  },

  submitUpdate: async () => {
    const state = get();
    if (state.isSubmitting || !state.isOpen) {
      return false;
    }
    const session = useSessionStore.getState().session;
    if (session === null) {
      set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
      return false;
    }
    const checked = validateProductEditForm(state.form);
    if (!checked.ok) {
      set({ errorMessage: checked.key });
      return false;
    }
    // Synchronous lock before the first await (TASK-BRK-032 precedent).
    set({ isSubmitting: true, errorMessage: null, successMessage: null });
    try {
      const units: UpdateProductUnitPayload[] = state.unitsSnapshot.map((unit) =>
        unit.unit_id === state.baseUnitId
          ? {
              unit_id: unit.unit_id,
              unit_name_ar: unit.unit_name_ar,
              unit_name_fr:
                state.form.baseUnitName.trim() === ""
                  ? unit.unit_name_fr
                  : state.form.baseUnitName.trim(),
              factor_micro: unit.factor_micro,
              barcode:
                state.form.barcode.trim() === "" ? null : state.form.barcode.trim(),
              sale_price_centimes: checked.sellingCentimes,
              purchase_price_centimes: checked.costCentimes,
              is_base_unit: unit.is_base_unit,
              is_default_selling: unit.is_default_selling,
            }
          : {
              unit_id: unit.unit_id,
              unit_name_ar: unit.unit_name_ar,
              unit_name_fr: unit.unit_name_fr,
              factor_micro: unit.factor_micro,
              barcode: unit.barcode,
              sale_price_centimes: unit.sale_price_centimes,
              purchase_price_centimes: unit.purchase_price_centimes,
              is_base_unit: unit.is_base_unit,
              is_default_selling: unit.is_default_selling,
            },
      );
      await updateProduct({
        session_token: session.token,
        product_id: state.form.productId,
        name_ar: state.form.nameAr.trim(),
        name_fr: state.form.nameFr.trim(),
        category_id: state.form.categoryId === "" ? null : state.form.categoryId,
        is_active: state.form.isActive,
        units,
      });
      set({ successMessage: "updateSuccess" });
      return true;
    } catch (failure: unknown) {
      set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
      return false;
    } finally {
      set({ isSubmitting: false });
    }
  },

  reset: () => {
    set({ ...initialState, form: emptyForm(), unitsSnapshot: [] });
  },
}));
