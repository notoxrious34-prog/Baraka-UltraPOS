/**
 * Product creation modal with direct initial stock provisioning (TASK-DSN-041).
 *
 * Creation flow: `catalog_create_product` first, then — when an initial
 * quantity was entered — one `stock.adjust` INCREASE on the principal depot
 * (resolved by code `DEP-01` to its UUID; the wire contract only accepts
 * UUIDs). Optional post-creation XP-410B label trigger via `onCreated`.
 *
 * All money/quantity conversions use the canonical string/BigInt converters
 * from @/lib/money — parseFloat/Number are never used on these values.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import type { Category, Locale } from "@/types/pos";
import type { CatalogProduct } from "@/types/pos";
import { usePosUiStore } from "@/stores/pos-ui-store";
import {
  adjustStock,
  ipcCreateProduct,
  ipcListCategories,
  listWarehouses,
  type CreateProductWire,
  type NewProductUnitWire,
  type StockAdjustPayload,
  type WarehouseItem,
} from "@/lib/ipc";
import { formatMoney, parseMoneyToCentimes, parseQtyToMicro } from "@/lib/money";
import {
  applyUnitPreset,
  BASE_FACTOR_MICRO,
  emptyUnit,
  type UnitPreset,
} from "@/features/catalog/ProductCreateForm";
import { computeMargin, formatMarginPct } from "@/features/catalog/CatalogModule";

export interface ProductCreateModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreated: (product: CatalogProduct, printLabel: boolean) => void;
  token: string;
}

/** Principal depot code: initial stock is credited on this warehouse. */
export const PRINCIPAL_DEPOT_CODE = "DEP-01";

/** Human reason recorded on the automatic initial-stock movement. */
export const PROVISION_REASON = "Stock initial à la création";

/** Upper bound for the initial-stock input (base units, whole or decimal). */
const MAX_NAME_CHARS = 100;

export interface ModalFormData {
  sku: string;
  barcode: string;
  nameAr: string;
  nameFr: string;
  preset: UnitPreset;
  customAr: string;
  customFr: string;
  salePrice: string;
  costPrice: string;
  initialStock: string;
  printAfter: boolean;
}

export function emptyModalForm(): ModalFormData {
  return {
    sku: "",
    barcode: "",
    nameAr: "",
    nameFr: "",
    preset: "PCS",
    customAr: "",
    customFr: "",
    salePrice: "",
    costPrice: "",
    initialStock: "",
    printAfter: true,
  };
}

function isBlank(value: string): boolean {
  return value.trim() === "";
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
export function validateModalForm(data: ModalFormData): Partial<Record<string, I18nKey>> {
  const errors: Partial<Record<string, I18nKey>> = {};
  if (isBlank(data.sku)) {
    errors.sku = "cat.required";
  }
  if (!isBlank(data.nameAr) && data.nameAr.trim().length > MAX_NAME_CHARS) {
    errors.nameAr = "cat.required";
  }
  if (isBlank(data.nameFr) || data.nameFr.trim().length > MAX_NAME_CHARS) {
    errors.nameFr = "cat.required";
  }
  if (data.preset === "CUSTOM") {
    if (isBlank(data.customFr)) {
      errors.customFr = "cat.required";
    } else if (data.customFr.trim().length > MAX_NAME_CHARS) {
      errors.customFr = "cat.required";
    }
    if (!isBlank(data.customAr) && data.customAr.trim().length > MAX_NAME_CHARS) {
      errors.customAr = "cat.required";
    }
  }
  const sale = priceError(data.salePrice);
  if (sale !== null) {
    errors.salePrice = sale;
  }
  const cost = priceError(data.costPrice);
  if (cost !== null) {
    errors.costPrice = cost;
  }
  if (!isBlank(data.initialStock)) {
    try {
      const micro = parseQtyToMicro(data.initialStock);
      if (micro.startsWith("-")) {
        errors.initialStock = "cat.invalidPrice";
      }
    } catch {
      errors.initialStock = "cat.invalidPrice";
    }
  }
  return errors;
}

/**
 * Build the exact CreateProductDto wire object for the single base unit
 * (throws Error<messageKey> on invalid input). Unit preset names reuse the
 * DSN-038 preset logic from ProductCreateForm. The optional `categoryId`
 * carries the selected rayon (`null`/blank stays unassigned).
 */
export function buildModalWire(data: ModalFormData, categoryId?: string | null): CreateProductWire {
  const errors = validateModalForm(data);
  const first = Object.values(errors)[0];
  if (first !== undefined) {
    throw new Error(first);
  }
  const seeded = applyUnitPreset(
    {
      ...emptyUnit(),
      nameAr: data.customAr,
      nameFr: data.customFr,
      salePrice: data.salePrice,
      costPrice: data.costPrice,
      barcode: "",
    },
    data.preset,
  );
  const barcode = data.barcode.trim();
  const nameFr = data.nameFr.trim();
  const unit: NewProductUnitWire = {
    unit_name_ar: seeded.nameAr.trim() === "" ? seeded.nameFr.trim() : seeded.nameAr.trim(),
    unit_name_fr: seeded.nameFr.trim(),
    factor_micro: BASE_FACTOR_MICRO,
    barcode: null,
    sale_price_centimes:
      data.salePrice.trim() === "" ? "0" : parseMoneyToCentimes(data.salePrice),
    purchase_price_centimes:
      data.costPrice.trim() === "" ? "0" : parseMoneyToCentimes(data.costPrice),
    is_base_unit: true,
    is_default_selling: true,
  };
  return {
    sku: data.sku.trim(),
    barcode: barcode === "" ? null : barcode,
    name_ar: data.nameAr.trim() === "" ? nameFr : data.nameAr.trim(),
    name_fr: nameFr,
    category_id: categoryId !== undefined && categoryId !== null && categoryId !== "" ? categoryId : null,
    is_kit: false,
    units: [unit],
  };
}

/**
 * Parse the initial-stock input to base-unit micros (`"0"` when blank).
 * Throws on malformed or negative input. Pure.
 */
export function parseInitialStockToMicro(raw: string): string {
  if (raw.trim() === "") {
    return "0";
  }
  const micro = parseQtyToMicro(raw);
  if (micro.startsWith("-")) {
    throw new Error("NEGATIVE_INITIAL_STOCK");
  }
  return micro;
}

/** True when a canonical micro string is strictly positive. Pure. */
export function isPositiveMicro(micro: string): boolean {
  try {
    return BigInt(micro) > 0n;
  } catch {
    return false;
  }
}

/**
 * Resolve the principal depot id from a warehouse list: exact
 * `DEP-01` code match first, then the first active warehouse, then the
 * first row. `null` when the list is empty. Pure.
 */
export function pickDepotId(warehouses: WarehouseItem[]): string | null {
  const principal = warehouses.find((row) => row.code === PRINCIPAL_DEPOT_CODE);
  if (principal !== undefined) {
    return principal.id;
  }
  const active = warehouses.find((row) => row.is_active);
  if (active !== undefined) {
    return active.id;
  }
  const first = warehouses[0];
  return first === undefined ? null : first.id;
}

export interface ProvisionOutcome {
  product: CatalogProduct;
  /** Backend wire code when the product was created but stock failed. */
  provisionError: string | null;
}

interface ProvisionDeps {
  create: typeof ipcCreateProduct;
  adjust: typeof adjustStock;
}

const liveDeps: ProvisionDeps = { create: ipcCreateProduct, adjust: adjustStock };

/**
 * Create a product, then provision its initial stock with one INCREASE
 * movement on the resolved depot. The product promise resolves first; a
 * stock failure never rolls the product back — it is reported via
 * `provisionError` so the caller can keep the created product visible.
 */
export async function createProductWithProvisioning(
  token: string,
  wire: CreateProductWire,
  initialStockMicro: string,
  depotId: string | null,
  deps: ProvisionDeps = liveDeps,
): Promise<ProvisionOutcome> {
  const created = await deps.create(token, wire);
  if (!isPositiveMicro(initialStockMicro) || depotId === null) {
    return { product: created, provisionError: null };
  }
  const base = created.units.find((unit) => unit.is_base_unit) ?? created.units[0];
  if (base === undefined) {
    return { product: created, provisionError: "NO_BASE_UNIT" };
  }
  const payload: StockAdjustPayload = {
    session_token: token,
    product_id: created.product_id,
    warehouse_id: depotId,
    unit_id: base.unit_id,
    quantity_micro: initialStockMicro,
    direction: "INCREASE",
    reason: PROVISION_REASON,
    authorize_negative_stock: false,
  };
  try {
    await deps.adjust(payload);
  } catch (failure: unknown) {
    return {
      product: created,
      provisionError: typeof failure === "string" ? failure : "INTERNAL_ERROR",
    };
  }
  return { product: created, provisionError: null };
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

export function ProductCreateModal({
  isOpen,
  onClose,
  onCreated,
  token,
}: ProductCreateModalProps): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const [form, setForm] = useState<ModalFormData>(emptyModalForm);
  const [attempted, setAttempted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [depotId, setDepotId] = useState<string | null>(null);
  const [created, setCreated] = useState<CatalogProduct | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [categoryId, setCategoryId] = useState("");
  const cardRef = useRef<HTMLDivElement | null>(null);

  // Reset per opening and resolve the principal depot id (UUID) once.
  useEffect(() => {
    if (!isOpen) {
      return;
    }
    setForm(emptyModalForm());
    setAttempted(false);
    setBusy(false);
    setSubmitError(null);
    setCreated(null);
    setDepotId(null);
    setCategories([]);
    setCategoryId("");
    let cancelled = false;
    listWarehouses(token)
      .then((rows) => {
        if (!cancelled) {
          setDepotId(pickDepotId(rows));
        }
      })
      .catch(() => {
        if (!cancelled) {
          setDepotId(null);
        }
      });
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
  }, [isOpen, token]);

  // Escape closes (when idle); Tab cycles inside the card (focus trap).
  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        if (!busy && created === null) {
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
  }, [isOpen, busy, created, onClose]);

  if (!isOpen) {
    return null;
  }

  const errors = attempted ? validateModalForm(form) : {};
  const patch = (field: keyof ModalFormData, value: string | boolean): void => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

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

  const submit = (): void => {
    if (busy) {
      return;
    }
    // A product already created but not provisioned: the footer button now
    // just hands it to the hub (which refreshes and offers the label modal).
    if (created !== null) {
      onCreated(created, form.printAfter);
      return;
    }
    setAttempted(true);
    const validation = validateModalForm(form);
    if (Object.keys(validation).length > 0) {
      return;
    }
    let wire: CreateProductWire;
    let stockMicro: string;
    try {
      wire = buildModalWire(form, categoryId);
      stockMicro = parseInitialStockToMicro(form.initialStock);
    } catch (failure: unknown) {
      setSubmitError(failure instanceof Error ? failure.message : "INTERNAL_ERROR");
      return;
    }
    if (isPositiveMicro(stockMicro) && depotId === null) {
      setSubmitError("WAREHOUSE_NOT_FOUND");
      return;
    }
    setBusy(true);
    setSubmitError(null);
    createProductWithProvisioning(token, wire, stockMicro, depotId)
      .then((outcome) => {
        if (outcome.provisionError !== null) {
          setCreated(outcome.product);
          setSubmitError(outcome.provisionError);
          return;
        }
        onCreated(outcome.product, form.printAfter);
      })
      .catch((failure: unknown) => {
        setSubmitError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const presets: UnitPreset[] = ["PCS", "SERIE", "CTN", "CUSTOM"];

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={t(locale, "cat.newProduct")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy && created === null) {
          onClose();
        }
      }}
    >
      <div
        ref={cardRef}
        className="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl dark:border-slate-700 dark:bg-slate-900"
      >
        <div className="flex h-14 shrink-0 items-center justify-between bg-primary-900 px-4 text-white">
          <div className="flex flex-col">
            <h2 className="text-base font-extrabold">{t(locale, "cat.newProduct")}</h2>
            <p className="text-[11px] font-bold text-white/70">{t(locale, "cat.newSubtitle")}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
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
              {created !== null ? `${t(locale, "cat.provisionFailed")} — ` : ""}
              {translateBackendError(submitError, locale)}
            </p>
          )}
          {created !== null && (
            <p className="rounded-md bg-green-50 px-3 py-2 text-sm font-bold text-green-700 dark:bg-green-900/30 dark:text-green-200">
              {t(locale, "cat.created")} — {created.sku}
            </p>
          )}

          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.sku")}</span>
              <input
                value={form.sku}
                autoFocus
                dir="ltr"
                onChange={(event) => {
                  patch("sku", event.target.value.toUpperCase());
                }}
                className={`${inputClass} font-mono`}
              />
              {fieldError(errors, "sku", locale)}
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.barcode")}</span>
              <input
                value={form.barcode}
                dir="ltr"
                onChange={(event) => {
                  patch("barcode", event.target.value);
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
                onChange={(event) => {
                  patch("nameFr", event.target.value);
                }}
                className={inputClass}
              />
              {fieldError(errors, "nameFr", locale)}
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.nameArOptional")}</span>
              <input
                value={form.nameAr}
                dir="auto"
                onChange={(event) => {
                  patch("nameAr", event.target.value);
                }}
                className={inputClass}
              />
              {fieldError(errors, "nameAr", locale)}
            </label>
          </div>

          <label className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.categoryLabel")}</span>
            <select
              value={categoryId}
              onChange={(event) => {
                setCategoryId(event.target.value);
              }}
              className={inputClass}
            >
              <option value="">{t(locale, "cat.noCategoryOption")}</option>
              {categories.map((category) => (
                <option key={category.id} value={category.id}>
                  {locale === "ar-DZ" ? category.name_ar : category.name_fr}
                </option>
              ))}
            </select>
          </label>

          <div className="flex flex-col gap-1">
            <span className="text-xs font-bold">{t(locale, "cat.unitPreset")}</span>
            <div className="flex flex-wrap gap-1" role="group" aria-label={t(locale, "cat.unitPreset")}>
              {presets.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => {
                    patch("preset", preset);
                  }}
                  className={`rounded-lg px-3 py-2 text-xs font-black ${
                    form.preset === preset
                      ? "bg-primary-600 text-white"
                      : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                  }`}
                >
                  {preset}
                </button>
              ))}
            </div>
          </div>

          {form.preset === "CUSTOM" && (
            <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
              <label className="flex flex-col gap-1">
                <span className="text-xs font-bold">{t(locale, "cat.nameFr")}</span>
                <input
                  value={form.customFr}
                  onChange={(event) => {
                    patch("customFr", event.target.value);
                  }}
                  className={inputClass}
                />
                {fieldError(errors, "customFr", locale)}
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-xs font-bold">{t(locale, "cat.nameArOptional")}</span>
                <input
                  value={form.customAr}
                  dir="auto"
                  onChange={(event) => {
                    patch("customAr", event.target.value);
                  }}
                  className={inputClass}
                />
                {fieldError(errors, "customAr", locale)}
              </label>
            </div>
          )}

          <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.costPrice")}</span>
              <input
                value={form.costPrice}
                inputMode="decimal"
                dir="ltr"
                onChange={(event) => {
                  patch("costPrice", event.target.value);
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
                  patch("salePrice", event.target.value);
                }}
                className={`${inputClass} font-mono`}
              />
              {fieldError(errors, "salePrice", locale)}
            </label>
          </div>
          <div className="flex items-center justify-between gap-2 rounded-lg bg-slate-100 px-3 py-2 dark:bg-slate-800">
            <span className="text-xs font-bold">{t(locale, "cat.colMargin")}</span>
            {marginPreview}
          </div>

          <h3 className="text-xs font-black uppercase opacity-70">
            {t(locale, "cat.provisionSection")}
          </h3>
          <div className="flex flex-col gap-1">
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold">{t(locale, "cat.stockInitial")}</span>
              <input
                value={form.initialStock}
                inputMode="decimal"
                dir="ltr"
                placeholder="0"
                onChange={(event) => {
                  patch("initialStock", event.target.value);
                }}
                className={`${inputClass} font-mono`}
              />
              {fieldError(errors, "initialStock", locale)}
            </label>
            <p className="text-[11px] font-bold text-slate-500 dark:text-slate-400">
              {t(locale, "cat.depotNote")}
            </p>
          </div>

          <h3 className="text-xs font-black uppercase opacity-70">
            {t(locale, "cat.optionsSection")}
          </h3>
          <label className="flex cursor-pointer items-center gap-2 text-sm font-bold">
            <input
              type="checkbox"
              checked={form.printAfter}
              onChange={(event) => {
                patch("printAfter", event.target.checked);
              }}
              className="h-4 w-4 accent-amber-500"
            />
            <span>
              {"🏷️ "}
              {t(locale, "cat.printAfterCreate")}
            </span>
          </label>
        </div>

        <div className="flex shrink-0 gap-2 border-t border-slate-200 p-3 dark:border-slate-700">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-800 dark:hover:bg-slate-700"
          >
            {t(locale, "cat.cancel")}
          </button>
          <button
            type="button"
            onClick={submit}
            disabled={busy}
            className="h-10 flex-1 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
          >
            {busy ? t(locale, "cat.saving") : t(locale, "cat.saveProduct")}
          </button>
        </div>
      </div>
    </div>
  );
}
