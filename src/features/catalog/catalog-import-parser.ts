/**
 * Offline CSV bulk-import parser for the Master Catalog Hub (TASK-DSN-045).
 *
 * Zero-dependency, UTF-8 BOM tolerant, semicolon-first (Algerian/French
 * Excel standard) with comma fallback, quoted-field aware. Money and
 * quantities are parsed with pure string arithmetic into integer centimes
 * and base-unit micros — never floats, never `Number()` on decimals.
 * Pure except for the template download trigger, which is SSR/node guarded.
 */

import type { ImportProductRow } from "@/types/pos";

/** UTF-8 Byte Order Mark stripped from pasted/Excel-saved files. */
export const IMPORT_CSV_BOM = "﻿";

/** Download name of the standard import template. */
export const IMPORT_TEMPLATE_FILENAME = "modele_import_baraka_pos.csv";

/** Canonical template header (semicolon = Algerian/French Excel standard). */
export const IMPORT_TEMPLATE_HEADER =
  "sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;category_code";

/** One client-side row rejection (1-based file line). */
export interface CsvParseError {
  line: number;
  message: string;
}

/** Parser output: backend-ready rows plus per-line rejections. */
export interface CatalogCsvParseResult {
  rows: ImportProductRow[];
  errors: CsvParseError[];
}

/** Normalize a header cell for alias matching (`Code-Barre` -> `codebarre`). */
export function normalizeHeaderCell(cell: string): string {
  return cell
    .trim()
    .toLowerCase()
    .replace(/[\s_-]+/g, "");
}

const SKU_ALIASES = new Set(["sku", "code", "reference", "ref"]);
const BARCODE_ALIASES = new Set(["barcode", "codebarre", "cb", "ean"]);
const NAME_FR_ALIASES = new Set([
  "namefr",
  "designation",
  "nom",
  "article",
  "libelle",
  "libellé",
]);
const NAME_AR_ALIASES = new Set([
  "namear",
  "nomar",
  "designationar",
  "الاسم",
  "التعيين",
]);
const PURCHASE_ALIASES = new Set(["purchaseprice", "prixachat", "pa", "cout", "coût"]);
const SALE_ALIASES = new Set(["saleprice", "prixvente", "pv", "prix"]);
const STOCK_ALIASES = new Set(["initialstock", "stock", "quantite", "quantité", "qty"]);
const CATEGORY_ALIASES = new Set(["category", "categorycode", "rayon", "categorie", "catégorie"]);

/**
 * Split raw text into logical CSV lines, honouring quoted newlines.
 * Handles `\r\n`, `\n`, and `\r` endings.
 */
export function splitCsvLines(text: string): string[] {
  const lines: string[] = [];
  let current = "";
  let quoted = false;
  for (let i = 0; i < text.length; i += 1) {
    const char = text[i];
    if (char === '"') {
      if (quoted && text[i + 1] === '"') {
        current += '""';
        i += 1;
        continue;
      }
      quoted = !quoted;
      current += char;
      continue;
    }
    if (!quoted && (char === "\n" || char === "\r")) {
      if (char === "\r" && text[i + 1] === "\n") {
        i += 1;
      }
      lines.push(current);
      current = "";
      continue;
    }
    current += char;
  }
  lines.push(current);
  return lines;
}

/**
 * Split one logical line into fields at `delimiter`, honouring `"..."`
 * quoting and `""` escapes. Returns unescaped cell text.
 */
export function splitCsvFields(line: string, delimiter: string): string[] {
  const fields: string[] = [];
  let current = "";
  let quoted = false;
  for (let i = 0; i < line.length; i += 1) {
    const char = line[i];
    if (quoted) {
      if (char === '"') {
        if (line[i + 1] === '"') {
          current += '"';
          i += 1;
        } else {
          quoted = false;
        }
      } else {
        current += char;
      }
      continue;
    }
    if (char === '"') {
      quoted = true;
      continue;
    }
    if (char === delimiter) {
      fields.push(current);
      current = "";
      continue;
    }
    current += char;
  }
  fields.push(current);
  return fields;
}

/**
 * Parse a decimal amount string into integer minor units with `places`
 * fractional digits — pure string arithmetic, no floats. Accepts `.` or
 * `,` decimals and strips thousands separators and spaces. Returns `null`
 * when the text is not a number, or `undefined` when blank (absent).
 */
export function parseDecimalToMinorUnits(
  raw: string,
  places: 2 | 4,
): number | null | undefined {
  const compact = raw
    .replace(/\s+/g, "")
    .split("")
    .filter((ch) => {
      const code = ch.charCodeAt(0);
      return (
        code !== 0x200b && code !== 0x200c && code !== 0x200d && code !== 0xfeff
      );
    })
    .join("");
  if (compact === "") {
    return undefined;
  }
  let text = compact;
  const lastDot = text.lastIndexOf(".");
  const lastComma = text.lastIndexOf(",");
  if (lastDot >= 0 && lastComma >= 0) {
    const decimalAt = Math.max(lastDot, lastComma);
    const kept = text[decimalAt];
    text = `${text.slice(0, decimalAt).replace(/[.,]/g, "")}${kept === "," ? "." : kept}${text.slice(decimalAt + 1)}`;
  } else if (lastComma >= 0) {
    text = text.replace(",", ".");
  }
  const match = /^(-)?(\d+)(?:\.(\d*))?$/.exec(text);
  if (match === null) {
    return null;
  }
  const [, sign, whole, fracRaw = ""] = match;
  const padded = (fracRaw + "0".repeat(places + 1)).slice(0, places + 1);
  let minor =
    BigInt(whole ?? "0") * 10n ** BigInt(places) + BigInt(padded.slice(0, places));
  const roundDigit = padded[places] ?? "0";
  if (roundDigit >= "5") {
    minor += 1n;
  }
  if (minor > BigInt(Number.MAX_SAFE_INTEGER)) {
    return null;
  }
  const value = Number(minor);
  return sign === "-" ? -value : value;
}

/** Price cell -> integer centimes (`"150,50"` -> `15050`). Pure. */
export function parsePriceToCentimes(raw: string): number | null | undefined {
  return parseDecimalToMinorUnits(raw, 2);
}

/** Stock cell -> base-unit micros (`"2.5"` -> `25000`). Pure. */
export function parseQuantityToMicros(raw: string): number | null | undefined {
  return parseDecimalToMinorUnits(raw, 4);
}

/** Trim + uppercase SKU/barcode (`" plomb "` -> `"PLOMB"`). Pure. */
export function normalizeImportCode(raw: string): string {
  return raw.trim().toUpperCase();
}

/**
 * Parse a catalog CSV into backend-ready import rows. Never throws on
 * content: every rejection lands in `errors` with its 1-based line number.
 */
export function parseCatalogCsv(
  rawText: string,
  categoriesByCodeOrId?: Map<string, string>,
): CatalogCsvParseResult {
  const rows: ImportProductRow[] = [];
  const errors: CsvParseError[] = [];
  const withoutBom = rawText.startsWith(IMPORT_CSV_BOM)
    ? rawText.slice(IMPORT_CSV_BOM.length)
    : rawText;
  const lines = splitCsvLines(withoutBom).filter(
    (line, index) => index === 0 || line.trim() !== "",
  );
  if (lines.length === 0) {
    return { rows, errors };
  }
  const header = lines[0] ?? "";
  const delimiter = header.includes(";") ? ";" : ",";
  const headers = splitCsvFields(header, delimiter).map(normalizeHeaderCell);
  const findColumn = (aliases: Set<string>): number =>
    headers.findIndex((cell) => aliases.has(cell));
  const skuIdx = findColumn(SKU_ALIASES);
  const barcodeIdx = findColumn(BARCODE_ALIASES);
  const nameFrIdx = findColumn(NAME_FR_ALIASES);
  const nameArIdx = findColumn(NAME_AR_ALIASES);
  const purchaseIdx = findColumn(PURCHASE_ALIASES);
  const saleIdx = findColumn(SALE_ALIASES);
  const stockIdx = findColumn(STOCK_ALIASES);
  const categoryIdx = findColumn(CATEGORY_ALIASES);
  if (nameFrIdx < 0) {
    errors.push({ line: 1, message: "missing name_fr column" });
    return { rows, errors };
  }
  if (saleIdx < 0) {
    errors.push({ line: 1, message: "missing sale_price column" });
    return { rows, errors };
  }
  const resolveCategory = (raw: string): string | undefined => {
    if (categoriesByCodeOrId === undefined) {
      const trimmed = raw.trim();
      return trimmed === "" ? undefined : trimmed;
    }
    const trimmed = raw.trim();
    if (trimmed === "") {
      return undefined;
    }
    return (
      categoriesByCodeOrId.get(trimmed) ??
      categoriesByCodeOrId.get(trimmed.toUpperCase())
    );
  };
  for (let lineNo = 2; lineNo <= lines.length; lineNo += 1) {
    const fields = splitCsvFields(lines[lineNo - 1] ?? "", delimiter);
    const cell = (index: number): string => {
      const value = index >= 0 && index < fields.length ? fields[index] : undefined;
      return value === undefined ? "" : value.trim();
    };
    const nameFr = cell(nameFrIdx);
    if (nameFr === "") {
      errors.push({ line: lineNo, message: "missing name_fr" });
      continue;
    }
    const saleRaw = cell(saleIdx);
    if (saleRaw === "") {
      errors.push({ line: lineNo, message: "missing sale_price" });
      continue;
    }
    const sale = parsePriceToCentimes(saleRaw);
    if (sale === null || sale === undefined || sale < 0) {
      errors.push({ line: lineNo, message: "invalid sale_price" });
      continue;
    }
    const purchaseRaw = purchaseIdx >= 0 ? cell(purchaseIdx) : "";
    const purchase =
      purchaseRaw === "" ? undefined : parsePriceToCentimes(purchaseRaw);
    const stockRaw = stockIdx >= 0 ? cell(stockIdx) : "";
    const stock = stockRaw === "" ? undefined : parseQuantityToMicros(stockRaw);
    const skuRaw = skuIdx >= 0 ? cell(skuIdx) : "";
    const barcodeRaw = barcodeIdx >= 0 ? cell(barcodeIdx) : "";
    const nameArRaw = nameArIdx >= 0 ? cell(nameArIdx) : "";
    const categoryRaw = categoryIdx >= 0 ? cell(categoryIdx) : "";
    rows.push({
      sku: skuRaw === "" ? undefined : normalizeImportCode(skuRaw),
      barcode: barcodeRaw === "" ? undefined : normalizeImportCode(barcodeRaw),
      name_fr: nameFr,
      name_ar: nameArRaw === "" ? undefined : nameArRaw,
      category_id: categoryRaw === "" ? undefined : resolveCategory(categoryRaw),
      purchase_price_centimes:
        purchase === null || purchase === undefined ? undefined : purchase,
      sale_price_centimes: sale,
      initial_quantity_micro:
        stock === null || stock === undefined || stock < 0 ? undefined : stock,
    });
  }
  return { rows, errors };
}

/** Standard import template: BOM + semicolon + 3 quincaillerie samples. */
export function generateSampleCsvTemplate(): string {
  const body = [
    IMPORT_TEMPLATE_HEADER,
    "VIS-PLA-25;6111250090011;Vis Placo 3.5x25;برغي بلاكو;120.50;150.00;50;VISS",
    "COU-PVC-32;;Coude PVC 32;كوع بلاستيك 32;85;110.00;100;PLOMB",
    "CAB-2.5;;Câble électrique 2.5mm;كابل كهربائي;210.75;260.00;25;ELEC",
  ].join("\n");
  return `${IMPORT_CSV_BOM}${body}\n`;
}

/** Browser download of the standard template. SSR/node guarded no-op. */
export function downloadSampleCsvTemplate(): void {
  if (typeof document === "undefined") {
    return;
  }
  const blob = new Blob([generateSampleCsvTemplate()], {
    type: "text/csv;charset=utf-8;",
  });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = IMPORT_TEMPLATE_FILENAME;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  URL.revokeObjectURL(url);
}
