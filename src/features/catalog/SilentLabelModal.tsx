import { useCallback, useEffect, useState } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import type { CatalogProduct, Locale } from "@/types/pos";
import { formatMoney } from "@/lib/money";
import { fitModuleWidth, generateCode128Svg } from "@/lib/barcode-128";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { ipcPrintTsplLabel, TSPL_COPIES_MAX, TSPL_COPIES_MIN } from "@/lib/tspl-print";
import {
  labelBarcodeSource,
  labelPriceUnit,
} from "@/features/catalog/ProductLabelModal";

export interface SilentLabelModalProps {
  product: CatalogProduct;
  isOpen: boolean;
  onClose: () => void;
  token: string;
}

/** Printable-dot budget of the 40x20 TSPL label barcode band (engine parity). */
export const SILENT_LABEL_BARCODE_DOTS = 304;

function formatDisplayName(product: CatalogProduct, locale: Locale): string {
  return locale === "ar-DZ" ? product.name_ar : product.name_fr;
}

function toIntOrNull(raw: string): number | null {
  const trimmed = raw.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  return Number.parseInt(trimmed, 10);
}

/** Vector preview of the Code128 band (null when the payload is unencodable —
 * the silent send then surfaces the verbatim engine code). */
export function silentLabelPreviewSvg(source: string): string | null {
  try {
    const moduleWidth = fitModuleWidth(source.length, SILENT_LABEL_BARCODE_DOTS, 2);
    return generateCode128Svg(source, { moduleWidth, height: 40, includeText: false });
  } catch {
    return null;
  }
}

/**
 * Silent 40x20 mm shelf-label modal (TASK-DSN-098): spooler-only TSPL,
 * button-triggered, never a dialog. Non-spooler transports keep the
 * frozen ProductLabelModal path (the caller branches before mounting).
 */
export function SilentLabelModal({ product, isOpen, onClose, token }: SilentLabelModalProps) {
  const locale = usePosUiStore((state) => state.locale);
  const [copiesRaw, setCopiesRaw] = useState("1");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const name = formatDisplayName(product, locale);
  const { priceCentimes, unitFr } = labelPriceUnit(product);
  const source = labelBarcodeSource(product);
  const previewSvg = silentLabelPreviewSvg(source);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    setCopiesRaw("1");
    setNotice(null);
    setFailure(null);
    setBusy(false);
  }, [isOpen, product.product_id]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, onClose]);

  const step = useCallback(
    (delta: number) => {
      setCopiesRaw((prev) => {
        const current = toIntOrNull(prev) ?? TSPL_COPIES_MIN;
        const next = Math.min(TSPL_COPIES_MAX, Math.max(TSPL_COPIES_MIN, current + delta));
        return String(next);
      });
    },
    [],
  );

  const send = useCallback(async () => {
    if (busy) {
      return;
    }
    setBusy(true);
    setNotice(null);
    setFailure(null);
    try {
      const bytes = await ipcPrintTsplLabel(token, {
        name,
        priceCentimes,
        barcode: source,
        copies: toIntOrNull(copiesRaw) ?? Number.NaN,
      });
      const count = toIntOrNull(copiesRaw) ?? TSPL_COPIES_MIN;
      setNotice(
        t(locale, "tspl.labelsOk")
          .replace("{n}", String(count))
          .replace("{bytes}", String(bytes)),
      );
    } catch (error) {
      const code = error instanceof Error ? error.message : String(error);
      setFailure(translateBackendError(code, locale));
    } finally {
      setBusy(false);
    }
  }, [busy, copiesRaw, locale, name, priceCentimes, source, token]);

  if (!isOpen) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4">
      <div
        className="flex max-h-full w-full max-w-lg flex-col overflow-hidden rounded-xl bg-white shadow-2xl dark:bg-slate-900"
        role="dialog"
        aria-modal="true"
        aria-label={t(locale, "tspl.labelTitle")}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex h-14 shrink-0 items-center justify-between bg-primary-900 px-4 text-white">
          <h2 className="text-base font-extrabold">{t(locale, "tspl.labelTitle")}</h2>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="rounded-md px-3 py-1 text-sm font-bold text-white/80 hover:bg-white/10 hover:text-white"
          >
            ✕
          </button>
        </div>
        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          <p className="text-xs font-bold text-slate-500">{t(locale, "tspl.labelFormat")}</p>
          <div className="flex justify-center rounded-lg border border-dashed border-slate-300 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800/50">
            <div
              className="flex max-w-full flex-col items-center justify-center gap-0.5 overflow-hidden border border-slate-400 bg-white px-2 py-1 text-slate-900"
              style={{ aspectRatio: "40 / 20" }}
              data-testid="silent-label-preview"
            >
              <p className="max-w-full truncate text-[11px] font-extrabold">{name}</p>
              <p className="text-[13px] font-black" dir="ltr">
                {formatMoney(priceCentimes, locale)}
                {unitFr === "" ? "" : ` / ${unitFr}`}
              </p>
              {previewSvg !== null && (
                <div className="w-full" dangerouslySetInnerHTML={{ __html: previewSvg }} />
              )}
              <p className="max-w-full font-mono text-[10px]" dir="ltr" title={source}>
                {source}
              </p>
            </div>
          </div>
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs font-bold text-slate-500">{t(locale, "tspl.copies")}</span>
            <span className="flex items-center gap-2">
              <button
                type="button"
                aria-label="−"
                data-testid="silent-label-copies-minus"
                onClick={() => step(-1)}
                disabled={busy}
                className="h-9 w-9 rounded-lg bg-slate-200 text-base font-black hover:bg-slate-300 dark:bg-slate-700"
              >
                −
              </button>
              <input
                className="h-9 w-16 rounded-lg border border-slate-300 text-center font-mono text-sm font-black dark:border-slate-600"
                data-testid="silent-label-copies"
                value={copiesRaw}
                dir="ltr"
                inputMode="numeric"
                disabled={busy}
                onChange={(event) => setCopiesRaw(event.target.value)}
              />
              <button
                type="button"
                aria-label="+"
                data-testid="silent-label-copies-plus"
                onClick={() => step(1)}
                disabled={busy}
                className="h-9 w-9 rounded-lg bg-slate-200 text-base font-black hover:bg-slate-300 dark:bg-slate-700"
              >
                +
              </button>
            </span>
          </div>
          {notice !== null && (
            <p className="text-sm font-bold text-emerald-700" data-testid="silent-label-notice" role="status">
              {notice}
            </p>
          )}
          {failure !== null && (
            <p className="text-sm font-bold text-red-700" data-testid="silent-label-error" role="alert">
              {failure}
            </p>
          )}
          <button
            type="button"
            data-testid="silent-label-send"
            onClick={() => void send()}
            disabled={busy}
            className="h-11 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-60"
          >
            {busy ? t(locale, "tspl.sending") : t(locale, "tspl.sendLabels")}
          </button>
        </div>
      </div>
    </div>
  );
}
