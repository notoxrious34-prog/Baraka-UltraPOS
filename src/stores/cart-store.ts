/**
 * Active-sale cart store (preview only; backend commits truth).
 * All money stays in centime strings, quantities in micro strings.
 */

import { create } from "zustand";
import type {
  ActiveCustomer,
  CartLine,
  CartUnitSnapshot,
  CatalogProduct,
  DocType,
  Locale,
} from "@/types/pos";
import { addBig, maxZero, mulQtyPrice, subBig } from "@/lib/money";

const DEFAULT_QTY_MICRO = "10000";
const ZERO = "0";

let lineCounter = 0;

function toSnapshot(
  productId: string,
  unit: {
    unit_id: string;
    unit_name_ar: string;
    unit_name_fr: string;
    factor_micro: string;
    barcode: string | null;
    sale_price_centimes: string;
    is_base_unit: boolean;
    is_default_selling: boolean;
  },
): CartUnitSnapshot {
  return {
    unitId: unit.unit_id,
    productId,
    nameAr: unit.unit_name_ar,
    nameFr: unit.unit_name_fr,
    factorMicro: unit.factor_micro,
    barcode: unit.barcode,
    salePriceCentimes: unit.sale_price_centimes,
    isBaseUnit: unit.is_base_unit,
    isDefaultSelling: unit.is_default_selling,
  };
}

function defaultUnitIndex(product: CatalogProduct): number {
  const fallback = product.units.findIndex((unit) => unit.is_default_selling);
  if (fallback >= 0) {
    return fallback;
  }
  const base = product.units.findIndex((unit) => unit.is_base_unit);
  return base >= 0 ? base : 0;
}

interface CartState {
  lines: CartLine[];
  globalDiscountCentimes: string;
  customer: ActiveCustomer;
  docType: DocType;
  addProduct: (product: CatalogProduct, unitIndex?: number) => void;
  removeLine: (key: string) => void;
  setLineQty: (key: string, qtyMicro: string) => void;
  setLineUnit: (key: string, unitIndex: number) => void;
  cycleLineUnit: (key: string) => CycleLineUnitResult;
  setLineDiscount: (key: string, discountCentimes: string) => void;
  setGlobalDiscount: (value: string) => void;
  setCustomer: (customer: ActiveCustomer) => void;
  setDocType: (doc: DocType) => void;
  clearCart: () => void;
  /** Atomically replace cart lines (draft resume); resets discount, sets customer. */
  loadLines: (lines: CartLine[], customer?: ActiveCustomer) => void;
}

/** Structured F3/unit-switch feedback for toasts and badges. */
export interface CycleLineUnitResult {
  cycled: boolean;
  unitCount: number;
  unitNameFr: string;
  unitNameAr: string;
  salePriceCentimes: string;
}

function singleUnitResult(line: CartLine | undefined): CycleLineUnitResult {
  const unit = line?.units[line.unitIndex];
  return {
    cycled: false,
    unitCount: line?.units.length ?? 0,
    unitNameFr: unit?.nameFr ?? "",
    unitNameAr: unit?.nameAr ?? "",
    salePriceCentimes: unit?.salePriceCentimes ?? "0",
  };
}

export const useCartStore = create<CartState>()((set, get) => ({
  lines: [],
  globalDiscountCentimes: ZERO,
  customer: { kind: "guest" },
  docType: "TICKET",
  addProduct: (product, unitIndex) =>
    set((state) => {
      const resolved =
        unitIndex !== undefined &&
        unitIndex >= 0 &&
        unitIndex < product.units.length
          ? unitIndex
          : defaultUnitIndex(product);
      const unit = product.units[resolved];
      if (unit === undefined) {
        return state;
      }
      const existing = state.lines.find(
        (line) => line.productId === product.product_id && line.units[line.unitIndex]?.unitId === unit.unit_id,
      );
      if (existing !== undefined) {
        return {
          lines: state.lines.map((line) =>
            line.key === existing.key
              ? { ...line, quantityMicro: addBig(line.quantityMicro, DEFAULT_QTY_MICRO) }
              : line,
          ),
        };
      }
      lineCounter += 1;
      const line: CartLine = {
        key: `${product.product_id}:${unit.unit_id}:${lineCounter}`,
        productId: product.product_id,
        sku: product.sku,
        nameAr: product.name_ar,
        nameFr: product.name_fr,
        units: product.units.map((candidate) => toSnapshot(product.product_id, candidate)),
        unitIndex: resolved,
        quantityMicro: DEFAULT_QTY_MICRO,
        lineDiscountCentimes: ZERO,
      };
      return { lines: [...state.lines, line] };
    }),
  removeLine: (key) =>
    set((state) => ({ lines: state.lines.filter((line) => line.key !== key) })),
  setLineQty: (key, qtyMicro) =>
    set((state) => ({
      lines: state.lines.map((line) =>
        line.key === key ? { ...line, quantityMicro: qtyMicro } : line,
      ),
    })),
  cycleLineUnit: (key) => {
    const line = get().lines.find((candidate) => candidate.key === key);
    if (line === undefined || line.units.length < 2) {
      return singleUnitResult(line);
    }
    const unitIndex = (line.unitIndex + 1) % line.units.length;
    set((state) => ({
      lines: state.lines.map((candidate) =>
        candidate.key === key ? { ...candidate, unitIndex } : candidate,
      ),
    }));
    const unit = get().lines.find((candidate) => candidate.key === key)?.units[unitIndex];
    return {
      cycled: true,
      unitCount: line.units.length,
      unitNameFr: unit?.nameFr ?? "",
      unitNameAr: unit?.nameAr ?? "",
      salePriceCentimes: unit?.salePriceCentimes ?? "0",
    };
  },
  setLineUnit: (key, unitIndex) =>
    set((state) => ({
      lines: state.lines.map((line) =>
        line.key === key && unitIndex >= 0 && unitIndex < line.units.length
          ? { ...line, unitIndex }
          : line,
      ),
    })),
  setLineDiscount: (key, discountCentimes) =>
    set((state) => ({
      lines: state.lines.map((line) =>
        line.key === key ? { ...line, lineDiscountCentimes: discountCentimes } : line,
      ),
    })),
  setGlobalDiscount: (value) => set({ globalDiscountCentimes: value }),
  setCustomer: (customer) => set({ customer }),
  setDocType: (doc) => set({ docType: doc }),
  clearCart: () =>
    set({
      lines: [],
      globalDiscountCentimes: ZERO,
      customer: { kind: "guest" },
    }),
  loadLines: (lines, customer) =>
    set({
      lines,
      globalDiscountCentimes: ZERO,
      customer: customer ?? { kind: "guest" },
    }),
}));

/** Preview total of one line: max(0, qty*price - discount). */
export function previewLineTotal(line: CartLine): string {
  const unit = line.units[line.unitIndex];
  if (unit === undefined) {
    return "0";
  }
  return maxZero(subBig(mulQtyPrice(line.quantityMicro, unit.salePriceCentimes), line.lineDiscountCentimes));
}

/**
 * Base-unit equivalence of a line quantity (`3 Boîtes x100` -> `"(= 300 Pièces)"`).
 * Pure BigInt micro arithmetic — zero floats. Returns `""` for the base unit
 * itself (no equivalence needed) and for unparseable inputs.
 */
export function calculateBaseQuantityEquivalent(
  quantityMicro: string,
  factorMicro: string,
  baseUnitNameFr: string,
  baseUnitNameAr: string,
  locale: Locale,
): string {
  if (factorMicro === "10000") {
    return "";
  }
  let baseQtyMicros: bigint;
  try {
    baseQtyMicros = (BigInt(quantityMicro) * BigInt(factorMicro)) / 10000n;
  } catch {
    return "";
  }
  const negative = baseQtyMicros < 0n;
  const abs = negative ? -baseQtyMicros : baseQtyMicros;
  const whole = (abs / 10000n).toString();
  const frac = (abs % 10000n).toString().padStart(4, "0").replace(/0+$/, "");
  const amount = `${negative ? "-" : ""}${frac === "" ? whole : `${whole}.${frac}`}`;
  const baseName = locale === "ar-DZ" ? baseUnitNameAr : baseUnitNameFr;
  return `(= ${amount} ${baseName})`;
}

export interface PreviewTotals {
  gross: string;
  globalDiscount: string;
  net: string;
  lineCount: number;
}

/** Cart preview totals (display only; backend commits truth). */
export function previewTotals(lines: CartLine[], globalDiscountCentimes: string): PreviewTotals {
  let gross = "0";
  for (const line of lines) {
    gross = addBig(gross, previewLineTotal(line));
  }
  const discount = maxZero(globalDiscountCentimes);
  return {
    gross,
    globalDiscount: discount,
    net: maxZero(subBig(gross, discount)),
    lineCount: lines.length,
  };
}
