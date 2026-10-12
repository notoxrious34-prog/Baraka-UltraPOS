/**
 * Offline CSV export engine for the Master Catalog Hub (TASK-DSN-043).
 *
 * Zero-dependency, UTF-8 BOM prefixed (`\uFEFF`) so Windows Excel detects
 * UTF-8 Arabic text; semicolon separator (French decimal commas never
 * collide); every text cell double-quoted with `""` escaping; money as
 * Latin-digit decimals; stock from base-unit micros. Pure except for the
 * download trigger, which is SSR/node guarded.
 */

import type { CatalogProduct } from "@/types/pos";
import { baseUnitOf, computeMargin } from "@/features/catalog/CatalogModule";

/** UTF-8 Byte Order Mark: forces Excel to decode the file as UTF-8. */
export const CSV_BOM = "\uFEFF";

export const CSV_SEPARATOR = ";";

const CSV_HEADERS =
  "Réf (SKU);Code-barres;Désignation FR;Désignation AR;Unité;Prix Achat HT (DZD);Prix Vente TTC (DZD);Marge (DZD);Marge (%);Stock (DEP-01);Statut";

/** Canonical centimes -> fixed 2-decimal Latin string (`"15050"` -> `"150.50"`). */
export function centimesToDecimalFixed(centimes: string): string {
  let value: bigint;
  try {
    value = BigInt(centimes === "" ? "0" : centimes);
  } catch {
    return "0.00";
  }
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const body = `${(abs / 100n).toString()}.${(abs % 100n).toString().padStart(2, "0")}`;
  return negative ? `-${body}` : body;
}

/** Micros -> trimmed base-unit quantity (`"150000"` -> `"15"`). Latin digits. */
export function microsToQuantity(micro: string): string {
  let value: bigint;
  try {
    value = BigInt(micro === "" ? "0" : micro);
  } catch {
    return "0";
  }
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const whole = (abs / 10_000n).toString();
  const frac = (abs % 10_000n).toString().padStart(4, "0").replace(/0+$/, "");
  const body = frac === "" ? whole : `${whole}.${frac}`;
  return negative ? `-${body}` : body;
}

/** Quote a text cell (`"` -> `""`, wrapped in `"..."`). */
export function csvText(cell: string): string {
  return `"${cell.replace(/"/g, '""')}"`;
}

/** Bare numeric cell when it matches a plain decimal, quoted text otherwise. */
export function csvNumber(cell: string): string {
  return /^-?\d+(\.\d+)?$/.test(cell) ? cell : csvText(cell);
}

function statusLabel(isActive: boolean, locale: string): string {
  if (locale.startsWith("ar")) {
    return isActive ? "نشط" : "غير نشط";
  }
  return isActive ? "Actif" : "Inactif";
}

function rowCells(
  product: CatalogProduct,
  stockLevels: Record<string, string>,
  locale: string,
): string[] {
  const base = baseUnitOf(product);
  const sale = base?.sale_price_centimes ?? "0";
  const purchase = base?.purchase_price_centimes ?? "0";
  const margin = computeMargin(sale, purchase);
  const micro = stockLevels[product.product_id] ?? "0";
  return [
    csvText(product.sku),
    csvText(product.barcode ?? ""),
    csvText(product.name_fr),
    csvText(product.name_ar),
    csvText(base === null ? "" : base.unit_name_fr),
    csvNumber(centimesToDecimalFixed(purchase)),
    csvNumber(centimesToDecimalFixed(sale)),
    csvNumber(centimesToDecimalFixed(margin.amountCentimes)),
    margin.pct === null ? csvText("—") : csvNumber(String(margin.pct)),
    csvNumber(microsToQuantity(micro)),
    csvText(statusLabel(product.is_active, locale)),
  ];
}

/**
 * Build the full CSV document (BOM + bilingual headers + one row per
 * product). The percent cell stays a bare Latin-digit number (the header
 * already carries `(%)`) so Excel can compute on it.
 */
export function buildCatalogCsvContent(
  products: CatalogProduct[],
  stockLevels: Record<string, string>,
  locale: string,
): string {
  const lines = [CSV_HEADERS];
  for (const product of products) {
    lines.push(rowCells(product, stockLevels, locale).join(CSV_SEPARATOR));
  }
  return `${CSV_BOM}${lines.join("\r\n")}\r\n`;
}

/**
 * Trigger a client-side download of CSV content. No-op outside a browser
 * (SSR / node test harness).
 */
export function triggerCsvDownload(content: string, filename: string): void {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return;
  }
  const blob = new Blob([content], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  try {
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  } finally {
    URL.revokeObjectURL(url);
  }
}
