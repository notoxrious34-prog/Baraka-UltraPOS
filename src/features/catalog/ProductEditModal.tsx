/**
 * Product edition modal (TASK-DSN-042): full-state update of an existing
 * catalog product over `catalog_update_product` (SKU immutable by backend
 * contract), with live BigInt margin preview and an active-state toggle.
 *
 * The update wire carries no product-level barcode, so the editable barcode
 * is the base unit's (`UpdateProductUnitDto.barcode`). All money/quantity
 * conversions use the canonical string/BigInt converters from @/lib/money —
 * parseFloat/Number are never used on these values.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import type { Category, Locale } from "@/types/pos";
import type { CatalogProduct } from "@/types/pos";
import { usePosUiStore } from "@/stores/pos-ui-store";
import {
  ipcGetProduct,
  ipcListCategories,
  updateProduct,
  type UpdateProductPayload,
  type UpdateProductUnitPayload,
} from "@/lib/ipc";
import { formatMoney, parseMoneyToCentimes } from "@/lib/money";
import { computeMargin, formatMarginPct } from "@/features/catalog/CatalogModule";

export interface ProductEditModalProps {
  isOpen: boolean;
  product: CatalogProduct | null;
  onClose: () => void;
  onUpdated: (updatedProduct: CatalogProduct) => void;
  token: string;
}

const MAX_NAME_CHARS = 100;

export interface EditFormData {
  nameFr: string;
  nameAr: string;
  /** Base-unit barcode input (blank clears the unit barcode). */
  barcode: string;
  /** Decimal DZD strings; blank means zero (creation-form semantics). */
  salePrice: string;
  costPrice: string;
  /** Artisan/wholesale DZD price (blank clears it; falls back to sale price). */
  wholesalePrice: string;
  isActive: boolean;
  /**
   * Selected rayon id (`""` clears the assignment; `undefined` keeps the
   * product's current `category_id` — existing callers omit it).
   */
  categoryId?: string | null;
}

/** Decimal DZD input from canonical centimes (`"15050"` -> `"150.5"`). Pure. */
export function centimesToDecimalInput(centimes: string): string {
  let value: bigint;
  try {
    value = BigInt(centimes === "" ? "0" : centimes);
  } catch {
    return "0";
  }
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const whole = (abs / 100n).toString();
  const frac = (abs % 100n).toString().padStart(2, "0").replace(/0+$/, "");
  const body = frac === "" ? whole : `${whole}.${frac}`;
  return negative ? `-${body}` : body;
}

/** Pre-fill the edition form from a live product (base unit preferred). Pure. */
export function formFromProduct(product: CatalogProduct): EditFormData {
  const base =
    product.units.find((unit) => unit.is_base_unit) ?? product.units[0] ?? null;
  return {
    nameFr: product.name_fr,
    nameAr: product.name_ar,
    barcode: base?.barcode ?? "",
    salePrice: base === null ? "0" : centimesToDecimalInput(base.sale_price_centimes),
    costPrice: base === null ? "0" : centimesToDecimalInput(base.purchase_price_centimes),
    wholesalePrice:
      base?.wholesale_price_centimes === null || base?.wholesale_price_centimes === undefined
        ? ""
        : centimesToDecimalInput(base.wholesale_price_centimes),
    isActive: product.is_active,
  };
}

function priceError(value: string): I18nKey | null {
  if (value.trim() === "") {
    return null;
  }
  try {
    const centimes = parseMoneyToCentimes(value);
    if (centimes.startsWith("-")) {
      return "cat.invalidPrice";
    }
    return null;
  } catch {
    return "cat.invalidPrice";
  }
}

/**
 * Full client-side validation. Returns field-id -> message-key pairs
 * (empty object = valid). Pure and side-effect free.
 */
export function validateEditForm(data: EditFormData): Partial<Record<string, I18nKey>> {
  const errors: Partial<Record<string, I18nKey>> = {};
  if (data.nameFr.trim() === "" || data.nameFr.trim().length > MAX_NAME_CHARS) {
    errors.nameFr = "cat.required";
  }
  if (data.nameAr.trim() !== "" && data.nameAr.trim().length > MAX_NAME_CHARS) {
    errors.nameAr = "cat.required";
  }
  const sale = priceError(data.salePrice);
  if (sale !== null) {
    errors.salePrice = sale;
  }
  const cost = priceError(data.costPrice);
  if (cost !== null) {
    errors.costPrice = cost;
  }
  const wholesale = priceError(data.wholesalePrice);
  if (wholesale !== null) {
    errors.wholesalePrice = wholesale;
  }
  return errors;
}

function blankToZero(value: string, parse: (raw: string) => string): string {
  return value.trim() === "" ? "0" : parse(value);
}

function blankToNull(value: string, parse: (raw: string) => string): string | null {
  return value.trim() === "" ? null : parse(value);
}

/**
 * Build the exact full-state `UpdateProductPayload`: names, preserved
 * category, toggled active flag, and ALL existing units — the base unit
 * (or first row fallback) carrying the edited barcode and prices with its
 * factor and names preserved. Throws Error<messageKey> on invalid input.
 */
export function buildUpdatePayload(
  token: string,
  product: CatalogProduct,
  data: EditFormData,
): UpdateProductPayload {
  const errors = validateEditForm(data);
  const first = Object.values(errors)[0];
  if (first !== undefined) {
    throw new Error(first);
  }
  const base = product.units.find((unit) => unit.is_base_unit) ?? product.units[0] ?? null;
  const nameFr = data.nameFr.trim();
  const barcode = data.barcode.trim();
  const units: UpdateProductUnitPayload[] = product.units.map((unit) => {
    const edited = base !== null && unit.unit_id === base.unit_id;
    return {
      unit_id: unit.unit_id,
      unit_name_ar: unit.unit_name_ar,
      unit_name_fr: unit.unit_name_fr,
      factor_micro: unit.factor_micro,
      barcode: edited ? (barcode === "" ? null : barcode) : unit.barcode,
      sale_price_centimes: edited
        ? blankToZero(data.salePrice, parseMoneyToCentimes)
        : unit.sale_price_centimes,
      purchase_price_centimes: edited
        ? blankToZero(data.costPrice, parseMoneyToCentimes)
        : unit.purchase_price_centimes,
      wholesale_price_centimes: edited
        ? blankToNull(data.wholesalePrice, parseMoneyToCentimes)
        : unit.wholesale_price_centimes,
      is_base_unit: unit.is_base_unit,
      is_default_selling: unit.is_default_selling,
    };
  });
  return {
    session_token: token,
    product_id: product.product_id,
    name_ar: data.nameAr.trim() === "" ? nameFr : data.nameAr.trim(),
    name_fr: nameFr,
    category_id:
      data.categoryId === undefined
        ? product.category_id
        : data.categoryId === "" || data.categoryId === null
          ? null
          : data.categoryId,
    is_active: data.isActive,
    units,
  };
}

interface EditDeps {
  update: typeof updateProduct;
  get: typeof ipcGetProduct;
}

const liveDeps: EditDeps = { update: updateProduct, get: ipcGetProduct };

/**
 * Full-state update followed by a fresh read (the `updateProduct` wrapper
 * resolves void, so the detail view is re-fetched for the hub row).
 */
export async function submitEditProduct(
  token: string,
  product: CatalogProduct,
  data: EditFormData,
  deps: EditDeps = liveDeps,
): Promise<CatalogProduct> {
  const payload = buildUpdatePayload(token, product, data);
  await deps.update(payload);
  return deps.get(token, product.product_id);
}

function fieldError(
  errors: Partial<Record<string, I18nKey>>,
  field: string,
  locale: Locale,
): ReactElement | null {
  const key = errors[field];
  if (key === undefined) {
    return null;
  }
  return <p className="text-xs font-bold text-red-600 dark:text-red-300">{t(locale, key)}</p>;
}

const inputClass =
  "h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";

export interface ProductEditModalViewProps {
  product: CatalogProduct;
  form: EditFormData;
  locale: Locale;
  attempted: boolean;
  busy: boolean;
  submitError: string | null;
  cardRef: { current: HTMLDivElement | null };
  onPatch: (field: keyof EditFormData, value: string | boolean) => void;
  onClose: () => void;
  onSubmit: () => void;
  /** Active taxonomy rows for the rayon selector (absent = selector hidden). */
  categories?: Category[];
  categoryId?: string;
  onCategory?: (id: string) => void;
}

/** Presentational edition dialog (pure props; SSR-safe for tests). */
export function ProductEditModalView(props: ProductEditModalViewProps): ReactElement {
  const { product, form, locale, busy, submitError } = props;
  const errors = props.attempted ? validateEditForm(form) : {};

  // Live margin preview from the canonical centime strings (BigInt only).
  let marginPreview: ReactElement | null = null;
  try {
    const sale = form.salePrice.trim() === "" ? "0" : parseMoneyToCentimes(form.salePrice);
    const cost = form.costPrice.trim() === "" ? "0" : parseMoneyToCentimes(form.costPrice);
    const margin = computeMargin(sale, cost);
    marginPreview =
      margin.pct === null ? (
        <span className="font-mono text-xs font-black text-slate-400">—</span>
      ) : (
        <span className="flex items-center gap-2 text-xs font-black">
          <span>{formatMoney(margin.amountCentimes, locale)}</span>
          <span
            className={
              margin.pct > 0
                ? "rounded-md bg-green-100 px-2 py-0.5 text-green-700 dark:bg-green-900/40 dark:text-green-200"
                : margin.pct < 0
                  ? "rounded-md bg-red-100 px-2 py-0.5 text-red-700 dark:bg-red-900/40 dark:text-red-200"
                  : "rounded-md bg-slate-200 px-2 py-0.5 text-slate-600 dark:bg-slate-700 dark:text-slate-300"
            }
          >
            {formatMarginPct(margin.pct)}
          </span>
        </span>
      );
  } catch {
    marginPreview = <span className="font-mono text-xs font-black text-slate-400">—</span>;
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={t(locale, "cat.editProduct")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) {
          props.onClose();
        }
      }}
    >
      <div
        ref={props.cardRef}
        className="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl dark:border-slate-700 dark:bg-slate-900"
      >
        <div className="flex h-14 shrink-0 items-center justify-between bg-primary-900 px-4 text-white">
          <div className="flex items-center gap-2">
            <div className="flex flex-col">
              <h2 className="text-base font-extrabold">{t(locale, "cat.editProduct")}</h2>
              <p className="text-[11px] font-bold text-white/70">{t(locale, "cat.editSubtitle")}</p>
            </div>
            <span className="rounded-md bg-white/15 px-2 py-0.5 font-mono text-[11px] font-black">
              {product.sku}
            </span>
          </div>
          <button
            type="button"
            onClick={props.onClose}
            disabled={busy}
            aria-label="Close"
            className="rounded-md px-3 py-1 text-sm font-bold text-white/80 hover:bg-white/10 hover:text-white disabled:opacity-50"
          >
            ✕
          </button>
        </div>

        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          {submitError !== null && (
            <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
              {translateBackendError(submitError, locale)}
            </p>
          )}

          <h3 className="text-xs font-black uppercase opacity-70">
            {t(locale, "cat.identSection")}
          </h3>
          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.sku")} 🔒</span>
              <input value={product.sku} readOnly disabled dir="ltr" className={`${inputClass} font-mono opacity-60`} />
              <span className="text-[11px] font-bold text-slate-500 dark:text-slate-400">
                {t(locale, "cat.skuReadOnlyNote")}
              </span>
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.barcode")}</span>
              <input
                value={form.barcode}
                dir="ltr"
                onChange={(event) => {
                  props.onPatch("barcode", event.target.value);
                }}
                className={`${inputClass} font-mono`}
              />
            </label>
          </div>
          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.nameFr")}</span>
              <input
                value={form.nameFr}
                autoFocus
                onChange={(event) => {
                  props.onPatch("nameFr", event.target.value);
                }}
                className={inputClass}
              />
              {fieldError(errors, "nameFr", locale)}
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.nameArOptional")}</span>
              <input
                value={form.nameAr}
                dir="rtl"
                onChange={(event) => {
                  props.onPatch("nameAr", event.target.value);
                }}
                className={inputClass}
              />
              {fieldError(errors, "nameAr", locale)}
            </label>
          </div>
          {props.categories !== undefined && props.onCategory !== undefined && (
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.categoryLabel")}</span>
              <select
                value={props.categoryId ?? ""}
                onChange={(event) => {
                  props.onCategory?.(event.target.value);
                }}
                className={inputClass}
              >
                <option value="">{t(locale, "cat.noCategoryOption")}</option>
                {props.categories.map((category) => (
                  <option key={category.id} value={category.id}>
                    {locale === "ar-DZ" ? category.name_ar : category.name_fr}
                  </option>
                ))}
              </select>
            </label>
          )}

          <h3 className="text-xs font-black uppercase opacity-70">
            {t(locale, "cat.pricingSection")}
          </h3>
          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.costPrice")}</span>
              <input
                value={form.costPrice}
                inputMode="decimal"
                dir="ltr"
                onChange={(event) => {
                  props.onPatch("costPrice", event.target.value);
                }}
                className={`${inputClass} font-mono`}
              />
              {fieldError(errors, "costPrice", locale)}
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.salePrice")}</span>
              <input
                value={form.salePrice}
                inputMode="decimal"
                dir="ltr"
                onChange={(event) => {
                  props.onPatch("salePrice", event.target.value);
                }}
                className={`${inputClass} font-mono`}
              />
              {fieldError(errors, "salePrice", locale)}
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">
                {t(locale, "wholesale_price")} (DZD)
              </span>
              <input
                value={form.wholesalePrice}
                inputMode="decimal"
                dir="ltr"
                onChange={(event) => {
                  props.onPatch("wholesalePrice", event.target.value);
                }}
                className={`${inputClass} font-mono`}
              />
              {fieldError(errors, "wholesalePrice", locale)}
            </label>
          </div>
          <div className="flex items-center justify-between gap-2 rounded-lg bg-slate-100 px-3 py-2 dark:bg-slate-800">
            <span className="text-xs font-bold">{t(locale, "cat.colMargin")}</span>
            {marginPreview}
          </div>

          <h3 className="text-xs font-black uppercase opacity-70">
            {t(locale, "cat.statusSection")}
          </h3>
          <label className="flex cursor-pointer items-center gap-2 text-sm font-bold">
            <input
              type="checkbox"
              checked={form.isActive}
              onChange={(event) => {
                props.onPatch("isActive", event.target.checked);
              }}
              className="h-4 w-4 accent-emerald-600"
            />
            <span>{t(locale, "cat.productActiveLabel")}</span>
          </label>
          {!form.isActive && (
            <p className="text-[11px] font-bold text-slate-500 dark:text-slate-400">
              {t(locale, "cat.productInactiveNotice")}
            </p>
          )}
        </div>

        <div className="flex shrink-0 gap-2 border-t border-slate-200 p-3 dark:border-slate-700">
          <button
            type="button"
            onClick={props.onClose}
            disabled={busy}
            className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-800 dark:hover:bg-slate-700"
          >
            {t(locale, "cat.cancel")}
          </button>
          <button
            type="button"
            onClick={props.onSubmit}
            disabled={busy}
            className="h-10 flex-1 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
          >
            {busy ? t(locale, "cat.saving") : t(locale, "cat.saveChanges")}
          </button>
        </div>
      </div>
    </div>
  );
}

export function ProductEditModal({
  isOpen,
  product,
  onClose,
  onUpdated,
  token,
}: ProductEditModalProps): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const [form, setForm] = useState<EditFormData | null>(() =>
    isOpen && product !== null ? formFromProduct(product) : null,
  );
  const [attempted, setAttempted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [categoryId, setCategoryId] = useState("");
  const cardRef = useRef<HTMLDivElement | null>(null);

  // Re-seed the form every time a (new) product is opened.
  useEffect(() => {
    if (!isOpen) {
      return;
    }
    setForm(product === null ? null : formFromProduct(product));
    setAttempted(false);
    setBusy(false);
    setSubmitError(null);
    setCategoryId(product?.category_id ?? "");
    setCategories([]);
    let cancelled = false;
    ipcListCategories(token, false)
      .then((rows) => {
        if (!cancelled) {
          setCategories(rows.filter((row) => row.is_active));
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCategories([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [isOpen, product, token]);

  // Escape closes (when idle); Tab cycles inside the card (focus trap).
  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        if (!busy) {
          onClose();
        }
        return;
      }
      if (event.key !== "Tab" || cardRef.current === null) {
        return;
      }
      const focusables = cardRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
      );
      if (focusables.length === 0) {
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (first === undefined || last === undefined) {
        return;
      }
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
    };
  }, [isOpen, busy, onClose]);

  if (!isOpen || product === null || form === null) {
    return null;
  }

  const patch = (field: keyof EditFormData, value: string | boolean): void => {
    setForm((prev) => (prev === null ? prev : { ...prev, [field]: value }));
  };

  const submit = (): void => {
    if (busy) {
      return;
    }
    setAttempted(true);
    if (Object.keys(validateEditForm(form)).length > 0) {
      return;
    }
    setBusy(true);
    setSubmitError(null);
    submitEditProduct(token, product, { ...form, categoryId })
      .then((updated) => {
        onUpdated(updated);
      })
      .catch((failure: unknown) => {
        setSubmitError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <ProductEditModalView
      product={product}
      form={form}
      locale={locale}
      attempted={attempted}
      busy={busy}
      submitError={submitError}
      cardRef={cardRef}
      onPatch={patch}
      onClose={onClose}
      onSubmit={submit}
      categories={categories}
      categoryId={categoryId}
      onCategory={setCategoryId}
    />
  );
}
