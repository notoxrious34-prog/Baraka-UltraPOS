/**
 * Product barcode-label printing modal (TASK-DSN-039, XP-410B 203 DPI).
 *
 * Thermal presets (50×30 / 40×25 / 100×50mm), quantity stepper 1..500,
 * live vector preview via the zero-dependency Code 128 engine, and an
 * isolated iframe dispatch carrying `@page { size: Wmm Hmm; margin: 0 }`
 * so the Windows spooler feeds the label printer cleanly.
 */

import { useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import type { CatalogProduct } from "@/types/pos";
import { formatMoney } from "@/lib/money";
import { generateCode128Svg } from "@/lib/barcode-128";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useStoreIdentityStore } from "@/stores/store-identity-store";

/** Supported thermal label presets. */
export type LabelFormat = "50x30" | "40x25" | "100x50";

export const LABEL_FORMATS: Record<LabelFormat, { widthMm: number; heightMm: number }> = {
  "50x30": { widthMm: 50, heightMm: 30 },
  "40x25": { widthMm: 40, heightMm: 25 },
  "100x50": { widthMm: 100, heightMm: 50 },
};

export const LABEL_FORMAT_ORDER: LabelFormat[] = ["50x30", "40x25", "100x50"];

/** Clamp a raw quantity into the 1..500 print range (default 1). */
export function clampLabelQuantity(raw: number): number {
  if (!Number.isFinite(raw)) {
    return 1;
  }
  const floored = Math.floor(raw);
  if (floored < 1) {
    return 1;
  }
  if (floored > 500) {
    return 500;
  }
  return floored;
}

/** Barcode payload for a label: product barcode, SKU fallback. */
export function labelBarcodeSource(product: CatalogProduct): string {
  const barcode = product.barcode !== null ? product.barcode.trim() : "";
  if (barcode !== "") {
    return barcode;
  }
  return product.sku;
}

function formatDisplayName(product: CatalogProduct, locale: Locale): string {
  return locale === "ar-DZ" ? product.name_ar : product.name_fr;
}

/** Sale price + unit display name for the label (base unit preferred). */
export function labelPriceUnit(product: CatalogProduct): { priceCentimes: string; unitFr: string } {
  const unit =
    product.units.find((candidate) => candidate.is_base_unit) ?? product.units[0] ?? null;
  if (unit === null) {
    return { priceCentimes: "0", unitFr: "" };
  }
  return { priceCentimes: unit.sale_price_centimes, unitFr: unit.unit_name_fr };
}

export interface LabelPrintArgs {
  product: CatalogProduct;
  locale: Locale;
  storeName: string;
  format: LabelFormat;
  quantity: number;
}

/** Build the standalone print document (one sticker per copy). */
export function buildLabelPrintHtml(args: LabelPrintArgs): string {
  const { widthMm, heightMm } = LABEL_FORMATS[args.format];
  const copies = clampLabelQuantity(args.quantity);
  const source = labelBarcodeSource(args.product);
  const svg = generateCode128Svg(source, { moduleWidth: 2, height: 56 });
  const { priceCentimes, unitFr } = labelPriceUnit(args.product);
  const sticker = `<div class="sticker"><div class="shop">${escapePrint(args.storeName)}</div><div class="name">${escapePrint(formatDisplayName(args.product, args.locale))}</div><div class="price">${escapePrint(formatMoney(priceCentimes, args.locale))}${unitFr === "" ? "" : ` <span class="unit">/${escapePrint(unitFr)}</span>`}</div><div class="bars">${svg}</div><div class="code">${escapePrint(source)}</div></div>`;
  const pages = Array.from({ length: copies }, () => sticker).join("");
  return `<!DOCTYPE html><html><head><meta charset="utf-8"><title>labels</title><style>@page { size: ${widthMm}mm ${heightMm}mm; margin: 0; }html,body { margin: 0; padding: 0; }.sticker { width: ${widthMm}mm; height: ${heightMm}mm; page-break-after: always; display: flex; flex-direction: column; align-items: center; justify-content: center; font-family: Arial, sans-serif; overflow: hidden; }.shop { font-size: 8pt; font-weight: bold; }.name { font-size: 9pt; font-weight: bold; }.price { font-size: 12pt; font-weight: bold; }.unit { font-size: 8pt; }.code { font-size: 8pt; font-family: monospace; }</style></head><body>${pages}</body></html>`;
}

function escapePrint(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** Dispatch the print document through an isolated hidden iframe. */
export function printLabelDocument(docHtml: string): void {
  if (typeof document === "undefined") {
    return;
  }
  const frame = document.createElement("iframe");
  frame.setAttribute("aria-hidden", "true");
  frame.style.position = "fixed";
  frame.style.right = "0";
  frame.style.bottom = "0";
  frame.style.width = "0";
  frame.style.height = "0";
  frame.style.border = "0";
  document.body.appendChild(frame);
  const cleanup = (): void => {
    if (frame.parentNode !== null) {
      frame.parentNode.removeChild(frame);
    }
  };
  try {
    const doc = frame.contentDocument;
    const win = frame.contentWindow;
    if (doc === null || win === null) {
      cleanup();
      return;
    }
    doc.open();
    doc.write(docHtml);
    doc.close();
    win.focus();
    win.print();
  } finally {
    window.setTimeout(cleanup, 1000);
  }
}

function presetLabel(format: LabelFormat, locale: Locale): string {
  switch (format) {
    case "50x30":
      return locale === "ar-DZ" ? "50 × 30 مم (قياسي)" : "50 × 30 mm (Standard Rayon)";
    case "40x25":
      return locale === "ar-DZ" ? "40 × 25 مم (صغير)" : "40 × 25 mm (Petit Format)";
    case "100x50":
      return locale === "ar-DZ" ? "100 × 50 مم (كبير)" : "100 × 50 mm (Grand Format / Carton)";
  }
}

export interface ProductLabelModalViewProps {
  product: CatalogProduct;
  locale: Locale;
  storeName: string;
  format: LabelFormat;
  quantity: number;
  onFormat: (format: LabelFormat) => void;
  onQuantity: (quantity: number) => void;
  onPrint: () => void;
  onClose: () => void;
}

/** Presentational label designer (pure props; SSR-safe for tests). */
export function ProductLabelModalView(props: ProductLabelModalViewProps): ReactElement {
  const { product, locale, storeName, format, quantity } = props;
  const source = labelBarcodeSource(product);
  const svg = generateCode128Svg(source, { moduleWidth: 2, height: 56 });
  const { priceCentimes, unitFr } = labelPriceUnit(product);
  const dims = LABEL_FORMATS[format];
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4">
      <div className="flex max-h-full w-full max-w-lg flex-col overflow-hidden rounded-xl bg-white shadow-2xl dark:bg-slate-900">
        <div className="flex h-14 shrink-0 items-center justify-between bg-primary-900 px-4 text-white">
          <h2 className="text-base font-extrabold">{t(locale, "cat.labelModalTitle")}</h2>
          <button
            type="button"
            onClick={props.onClose}
            aria-label="Close"
            className="rounded-md px-3 py-1 text-sm font-bold text-white/80 hover:bg-white/10 hover:text-white"
          >
            ✕
          </button>
        </div>
        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          <div className="flex flex-col gap-1">
            <span className="text-xs font-bold text-slate-500">{t(locale, "cat.labelFormat")}</span>
            <div className="flex flex-wrap gap-1">
              {LABEL_FORMAT_ORDER.map((option) => (
                <button
                  key={option}
                  type="button"
                  onClick={() => {
                    props.onFormat(option);
                  }}
                  className={`rounded-lg px-3 py-2 text-xs font-black ${
                    format === option
                      ? "bg-primary-600 text-white"
                      : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                  }`}
                >
                  {presetLabel(option, locale)}
                </button>
              ))}
            </div>
          </div>
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs font-bold text-slate-500">
              {t(locale, "cat.labelQuantity")}
            </span>
            <span className="flex items-center gap-2">
              <button
                type="button"
                aria-label="-"
                onClick={() => {
                  props.onQuantity(clampLabelQuantity(quantity - 1));
                }}
                className="h-9 w-9 rounded-lg bg-slate-200 text-base font-black hover:bg-slate-300 dark:bg-slate-700"
              >
                −
              </button>
              <span className="min-w-10 text-center font-mono text-sm font-black">{quantity}</span>
              <button
                type="button"
                aria-label="+"
                onClick={() => {
                  props.onQuantity(clampLabelQuantity(quantity + 1));
                }}
                className="h-9 w-9 rounded-lg bg-slate-200 text-base font-black hover:bg-slate-300 dark:bg-slate-700"
              >
                +
              </button>
            </span>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs font-bold text-slate-500">{t(locale, "cat.labelPreview")}</span>
            <div className="flex justify-center rounded-lg border border-dashed border-slate-300 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800/50">
              <div
                className="flex flex-col items-center justify-center gap-0.5 overflow-hidden border border-slate-400 bg-white px-2 py-1 text-slate-900"
                style={{ aspectRatio: `${dims.widthMm} / ${dims.heightMm}`, maxWidth: "100%" }}
              >
                <p className="text-[10px] font-bold">{storeName}</p>
                <p className="max-w-full truncate text-xs font-extrabold">
                  {formatDisplayName(product, locale)}
                </p>
                <p className="text-sm font-black">
                  {formatMoney(priceCentimes, locale)}
                  {unitFr === "" ? "" : ` / ${unitFr}`}
                </p>
                <div
                  className="w-full"
                  dangerouslySetInnerHTML={{ __html: svg }}
                />
                <p className="font-mono text-[10px]">{source}</p>
              </div>
            </div>
          </div>
          <button
            type="button"
            onClick={props.onPrint}
            className="h-11 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
          >
            {t(locale, "cat.printAction")} × {quantity}
          </button>
        </div>
      </div>
    </div>
  );
}

export function ProductLabelModal({
  product,
  isOpen,
  onClose,
}: {
  product: CatalogProduct;
  isOpen: boolean;
  onClose: () => void;
}): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const storeName = useStoreIdentityStore((state) => state.saved.storeName);
  const [format, setFormat] = useState<LabelFormat>("50x30");
  const [quantity, setQuantity] = useState(1);
  if (!isOpen) {
    return null;
  }
  return (
    <ProductLabelModalView
      product={product}
      locale={locale}
      storeName={storeName}
      format={format}
      quantity={quantity}
      onFormat={setFormat}
      onQuantity={setQuantity}
      onPrint={() => {
        printLabelDocument(
          buildLabelPrintHtml({ product, locale, storeName, format, quantity }),
        );
      }}
      onClose={onClose}
    />
  );
}
