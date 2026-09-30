/**
 * Product creation form (TASK-BRK-029, creation flow only).
 *
 * All money/quantity conversions use the canonical string/BigInt converters
 * from @/lib/money — parseFloat/Number are never used on these values.
 * The request shape mirrors CreateProductDto field-for-field; category is
 * omitted (nullable server-side) and is_kit is fixed false (kit UI deferred).
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { ipcCreateProduct, type CreateProductWire, type NewProductUnitWire } from "@/lib/ipc";
import { parseMoneyToCentimes, parseQtyToMicro } from "@/lib/money";

/** Canonical factor for the base unit: 1.0000 in micros (backend convention). */
export const BASE_FACTOR_MICRO = "10000";

const MAX_NAME_CHARS = 100;

export interface UnitFormData {
  nameAr: string;
  nameFr: string;
  /** Decimal factor string (extras only; base unit uses BASE_FACTOR_MICRO). */
  factor: string;
  /** Decimal DZD string, blank means zero. */
  salePrice: string;
  /** Decimal DZD string, blank means zero. */
  costPrice: string;
  barcode: string;
}

export interface ProductFormData {
  sku: string;
  nameAr: string;
  nameFr: string;
  barcode: string;
  base: UnitFormData;
  extras: UnitFormData[];
}

export function emptyUnit(): UnitFormData {
  return { nameAr: "", nameFr: "", factor: "", salePrice: "", costPrice: "", barcode: "" };
}

export function emptyProductForm(): ProductFormData {
  return { sku: "", nameAr: "", nameFr: "", barcode: "", base: emptyUnit(), extras: [] };
}

function isBlank(value: string): boolean {
  return value.trim() === "";
}

function isTooLong(value: string): boolean {
  return value.trim().length > MAX_NAME_CHARS;
}

function priceError(value: string): I18nKey | null {
  if (isBlank(value)) {
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
export function validateProductForm(data: ProductFormData): Partial<Record<string, I18nKey>> {
  const errors: Partial<Record<string, I18nKey>> = {};
  if (isBlank(data.sku)) {
    errors.sku = "cat.required";
  }
  if (isBlank(data.nameAr) || isTooLong(data.nameAr)) {
    errors.nameAr = "cat.required";
  }
  if (isBlank(data.nameFr) || isTooLong(data.nameFr)) {
    errors.nameFr = "cat.required";
  }
  if (isBlank(data.base.nameAr) || isTooLong(data.base.nameAr)) {
    errors["base.nameAr"] = "cat.required";
  }
  if (isBlank(data.base.nameFr) || isTooLong(data.base.nameFr)) {
    errors["base.nameFr"] = "cat.required";
  }
  const baseSale = priceError(data.base.salePrice);
  if (baseSale !== null) {
    errors["base.salePrice"] = baseSale;
  }
  const baseCost = priceError(data.base.costPrice);
  if (baseCost !== null) {
    errors["base.costPrice"] = baseCost;
  }
  data.extras.forEach((extra, index) => {
    const prefix = `extras.${index}`;
    if (isBlank(extra.nameAr) || isTooLong(extra.nameAr)) {
      errors[`${prefix}.nameAr`] = "cat.required";
    }
    if (isBlank(extra.nameFr) || isTooLong(extra.nameFr)) {
      errors[`${prefix}.nameFr`] = "cat.required";
    }
    if (isBlank(extra.factor)) {
      errors[`${prefix}.factor`] = "cat.required";
    } else {
      try {
        const micros = parseQtyToMicro(extra.factor);
        if (micros === "0" || micros.startsWith("-")) {
          errors[`${prefix}.factor`] = "cat.invalidFactor";
        }
      } catch {
        errors[`${prefix}.factor`] = "cat.invalidFactor";
      }
    }
    const sale = priceError(extra.salePrice);
    if (sale !== null) {
      errors[`${prefix}.salePrice`] = sale;
    }
    const cost = priceError(extra.costPrice);
    if (cost !== null) {
      errors[`${prefix}.costPrice`] = cost;
    }
  });
  return errors;
}

function toWireUnit(unit: UnitFormData, factorMicro: string, isBase: boolean): NewProductUnitWire {
  const barcode = unit.barcode.trim();
  return {
    unit_name_ar: unit.nameAr.trim(),
    unit_name_fr: unit.nameFr.trim(),
    factor_micro: factorMicro,
    barcode: barcode === "" ? null : barcode,
    sale_price_centimes:
      unit.salePrice.trim() === "" ? "0" : parseMoneyToCentimes(unit.salePrice),
    purchase_price_centimes:
      unit.costPrice.trim() === "" ? "0" : parseMoneyToCentimes(unit.costPrice),
    is_base_unit: isBase,
    is_default_selling: isBase,
  };
}

/**
 * Build the exact CreateProductDto wire object (throws Error<messageKey> on
 * invalid input; validated first so backend receives only clean payloads).
 */
export function buildCreateProductWire(data: ProductFormData): CreateProductWire {
  const errors = validateProductForm(data);
  const first = Object.values(errors)[0];
  if (first !== undefined) {
    throw new Error(first);
  }
  const sku = data.sku.trim();
  const barcode = data.barcode.trim();
  return {
    sku,
    barcode: barcode === "" ? null : barcode,
    name_ar: data.nameAr.trim(),
    name_fr: data.nameFr.trim(),
    category_id: null,
    is_kit: false,
    units: [
      toWireUnit(data.base, BASE_FACTOR_MICRO, true),
      ...data.extras.map((extra) => toWireUnit(extra, parseQtyToMicro(extra.factor), false)),
    ],
  };
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

export function ProductCreateForm({
  token,
  onCreated,
}: {
  token: string;
  onCreated: (sku: string) => void;
}): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [data, setData] = useState<ProductFormData>(emptyProductForm);
  const [attempted, setAttempted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [createdSku, setCreatedSku] = useState<string | null>(null);
  // Errors render only after a submit attempt; the submit button stays live
  // so corrected input can always be resubmitted (no validation deadlock).
  const errors = attempted ? validateProductForm(data) : {};

  const patch = (field: "sku" | "nameAr" | "nameFr" | "barcode", value: string): void => {
    setData((prev) => ({ ...prev, [field]: value }));
  };

  const patchBase = (field: keyof UnitFormData, value: string): void => {
    setData((prev) => ({ ...prev, base: { ...prev.base, [field]: value } }));
  };

  const patchExtra = (index: number, field: keyof UnitFormData, value: string): void => {
    setData((prev) => ({
      ...prev,
      extras: prev.extras.map((extra, i) => (i === index ? { ...extra, [field]: value } : extra)),
    }));
  };

  const submit = (): void => {
    if (busy) {
      return;
    }
    setAttempted(true);
    const validation = validateProductForm(data);
    setSubmitError(null);
    setCreatedSku(null);
    if (Object.keys(validation).length > 0) {
      return;
    }
    setBusy(true);
    ipcCreateProduct(token, buildCreateProductWire(data))
      .then((product) => {
        setCreatedSku(product.sku);
        setData(emptyProductForm());
        setAttempted(false);
        onCreated(product.sku);
      })
      .catch((failure: unknown) => {
        const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
        setSubmitError(code);
      })
      .finally(() => {
        setBusy(false);
      });
  };

  // Enter inside dynamic unit rows must not accidentally submit the form.
  const swallowEnter = (event: React.KeyboardEvent): void => {
    if (event.key === "Enter") {
      event.preventDefault();
    }
  };

  return (
    <form
      className="flex flex-col gap-3 rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <h3 className="text-sm font-black">{t(locale, "cat.newTitle")}</h3>
      {createdSku !== null && (
        <p className="rounded-md bg-green-50 px-3 py-2 text-sm font-bold text-green-700 dark:bg-green-900/30 dark:text-green-200">
          {t(locale, "cat.created")} — {createdSku}
        </p>
      )}
      {submitError !== null && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(submitError, locale)}
        </p>
      )}
      <div className="grid grid-cols-1 gap-2 md:grid-cols-3">
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold">{t(locale, "cat.sku")}</span>
          <input value={data.sku} onChange={(e) => patch("sku", e.target.value)} className={inputClass} />
          {fieldError(errors, "sku", locale)}
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold">{t(locale, "cat.nameAr")}</span>
          <input
            value={data.nameAr}
            dir="auto"
            onChange={(e) => patch("nameAr", e.target.value)}
            className={inputClass}
          />
          {fieldError(errors, "nameAr", locale)}
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold">{t(locale, "cat.nameFr")}</span>
          <input value={data.nameFr} onChange={(e) => patch("nameFr", e.target.value)} className={inputClass} />
          {fieldError(errors, "nameFr", locale)}
        </label>
      </div>
      <label className="flex flex-col gap-1">
        <span className="text-xs font-bold">{t(locale, "cat.barcode")}</span>
        <input value={data.barcode} onChange={(e) => patch("barcode", e.target.value)} className={inputClass} />
      </label>

      <h4 className="text-xs font-black uppercase opacity-70">{t(locale, "cat.baseUnit")}</h4>
      <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold">{t(locale, "cat.nameAr")}</span>
          <input
            value={data.base.nameAr}
            dir="auto"
            onChange={(e) => patchBase("nameAr", e.target.value)}
            className={inputClass}
          />
          {fieldError(errors, "base.nameAr", locale)}
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold">{t(locale, "cat.nameFr")}</span>
          <input value={data.base.nameFr} onChange={(e) => patchBase("nameFr", e.target.value)} className={inputClass} />
          {fieldError(errors, "base.nameFr", locale)}
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold">{t(locale, "cat.salePrice")}</span>
          <input
            value={data.base.salePrice}
            inputMode="decimal"
            onChange={(e) => patchBase("salePrice", e.target.value)}
            className={inputClass}
          />
          {fieldError(errors, "base.salePrice", locale)}
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold">{t(locale, "cat.costPrice")}</span>
          <input
            value={data.base.costPrice}
            inputMode="decimal"
            onChange={(e) => patchBase("costPrice", e.target.value)}
            className={inputClass}
          />
          {fieldError(errors, "base.costPrice", locale)}
        </label>
        <label className="flex flex-col gap-1 md:col-span-2">
          <span className="text-xs font-bold">{t(locale, "cat.barcode")}</span>
          <input value={data.base.barcode} onChange={(e) => patchBase("barcode", e.target.value)} className={inputClass} />
        </label>
      </div>

      {data.extras.map((extra, index) => (
        <div
          key={index}
          onKeyDown={swallowEnter}
          className="grid grid-cols-1 gap-2 rounded-lg border border-slate-200 p-2 md:grid-cols-3 dark:border-slate-700"
        >
          <label className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.nameAr")}</span>
            <input
              value={extra.nameAr}
              dir="auto"
              onChange={(e) => patchExtra(index, "nameAr", e.target.value)}
              className={inputClass}
            />
            {fieldError(errors, `extras.${index}.nameAr`, locale)}
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.nameFr")}</span>
            <input
              value={extra.nameFr}
              onChange={(e) => patchExtra(index, "nameFr", e.target.value)}
              className={inputClass}
            />
            {fieldError(errors, `extras.${index}.nameFr`, locale)}
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.factor")}</span>
            <input
              value={extra.factor}
              inputMode="decimal"
              onChange={(e) => patchExtra(index, "factor", e.target.value)}
              className={inputClass}
            />
            {fieldError(errors, `extras.${index}.factor`, locale)}
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.salePrice")}</span>
            <input
              value={extra.salePrice}
              inputMode="decimal"
              onChange={(e) => patchExtra(index, "salePrice", e.target.value)}
              className={inputClass}
            />
            {fieldError(errors, `extras.${index}.salePrice`, locale)}
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.costPrice")}</span>
            <input
              value={extra.costPrice}
              inputMode="decimal"
              onChange={(e) => patchExtra(index, "costPrice", e.target.value)}
              className={inputClass}
            />
            {fieldError(errors, `extras.${index}.costPrice`, locale)}
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.barcode")}</span>
            <input
              value={extra.barcode}
              onChange={(e) => patchExtra(index, "barcode", e.target.value)}
              className={inputClass}
            />
          </label>
          <button
            type="button"
            onClick={() => {
              setData((prev) => ({ ...prev, extras: prev.extras.filter((_, i) => i !== index) }));
            }}
            className="h-10 rounded-lg bg-slate-200 text-sm font-bold hover:bg-slate-300 md:col-span-3 dark:bg-slate-800 dark:hover:bg-slate-700"
          >
            {t(locale, "cat.removeUnit")}
          </button>
        </div>
      ))}

      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => {
            setData((prev) => ({ ...prev, extras: [...prev.extras, emptyUnit()] }));
          }}
          className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-bold hover:bg-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700"
        >
          {t(locale, "cat.addUnit")}
        </button>
        <button
          type="submit"
          disabled={busy || Object.keys(errors).length > 0}
          className="h-10 flex-1 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
        >
          {busy ? t(locale, "cat.creating") : t(locale, "cat.submit")}
        </button>
      </div>
    </form>
  );
}
