import { useCallback, useEffect, useState } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import type { CatalogProduct, Locale } from "@/types/pos";
import { fitModuleWidth, generateCode128Svg } from "@/lib/barcode-128";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { ipcPrintTsplLabel, TSPL_COPIES_MIN, TSPL_COPIES_MAX, clampTsplCopies } from "@/lib/tspl-print";
import { ipcPreviewTsplLabel } from "@/lib/ipc";
import type { TsplLabelPreview, TsplPreviewLine } from "@/lib/ipc";
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

/** Printable-dot budget of the 40x20 TSPL label barcode band (engine parity:
 * 320 dots minus 20-dot quiet zones per side — payloads over 9 chars
 * reject loudly instead of shrinking). */
export const SILENT_LABEL_BARCODE_DOTS = 280;

/** Engine payload ceiling (BRK-098 module budget: 11·n + 35 ≤ 140). */
export const SILENT_LABEL_BARCODE_MAX_CHARS = 9;

function formatDisplayName(product: CatalogProduct, locale: Locale): string {
  return locale === "ar-DZ" ? product.name_ar : product.name_fr;
}

/** Label copy count only — never money. The `/^\d+$/` guard rejects signs,
 * decimals, and exponents before parsing, so no float path can reach a
 * financial value; prices travel separately as integer centimes
 * (`priceCentimes`). The stepper clamps to `[TSPL_COPIES_MIN,
 * TSPL_COPIES_MAX]`; the send path passes the parsed value through and
 * the domain rejects out-of-range copies loudly
 * (`TsplCopiesOutOfRange`, `tspl.rs:523`), so no silent misprint.
 * (TASK-DSN-101: AUD-049 Phase-1 documentation.) */
function toIntOrNull(raw: string): number | null {
  const trimmed = raw.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  return Number.parseInt(trimmed, 10);
}

/** Vector preview of the Code128 band (null when the payload is unencodable
 * or over the 9-char engine budget — the silent send would reject loudly,
 * so the modal guides instead of previewing a lie). */
export function silentLabelPreviewSvg(source: string): string | null {
  if (source.length > SILENT_LABEL_BARCODE_MAX_CHARS) {
    return null;
  }
  try {
    const moduleWidth = fitModuleWidth(source.length, SILENT_LABEL_BARCODE_DOTS, 2);
    return generateCode128Svg(source, { moduleWidth, height: 40, includeText: false });
  } catch {
    return null;
  }
}

/**
 * Engine-faithful 40x20 preview (DOC-03): renders the backend plan lines
 * verbatim, in spool order — brand, names, price (`350.00` engine
 * spelling, never locale `350,00`), currency, barcode band, human line.
 * The unit (`/ Pièce`) is deliberately absent: the engine never prints
 * it, so the preview must not show it either.
 */
export function SilentLabelPreviewView({
  preview,
  barcode,
}: {
  preview: TsplLabelPreview;
  barcode: string;
}) {
  const svg = silentLabelPreviewSvg(barcode);
  const humanAt = preview.lines.findIndex((line) => line.slot === "human");
  const head = humanAt === -1 ? preview.lines : preview.lines.slice(0, humanAt);
  const tail = humanAt === -1 ? [] : preview.lines.slice(humanAt);
  return (
    <>
      {head.map((line, index) => (
        <PreviewLine key={`${line.slot}-${index}`} line={line} />
      ))}
      {svg !== null && (
        <div
          className="w-full"
          style={{ width: `${Math.min(100, (preview.barcodeWidth / 320) * 100)}%` }}
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      )}
      {tail.map((line, index) => (
        <PreviewLine key={`${line.slot}-tail-${index}`} line={line} />
      ))}
    </>
  );
}

function PreviewLine({ line }: { line: TsplPreviewLine }) {
  switch (line.slot) {
    case "brand":
      return <p className="max-w-full truncate text-[10px] font-bold text-slate-500">{line.text}</p>;
    case "price":
      return (
        <p className="text-[15px] font-black leading-none text-slate-900" dir="ltr">
          {line.text}
        </p>
      );
    case "currency":
      return (
        <p className="self-end text-[10px] font-bold text-slate-700" dir="ltr">
          {line.text}
        </p>
      );
    case "human":
      return (
        <p className="max-w-full font-mono text-[10px] text-slate-900" dir="ltr" title={line.text}>
          {line.text}
        </p>
      );
    default:
      return <p className="max-w-full truncate text-[11px] font-extrabold text-slate-900">{line.text}</p>;
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
  const { priceCentimes } = labelPriceUnit(product);
  const source = labelBarcodeSource(product);
  const tooWide = source.length > SILENT_LABEL_BARCODE_MAX_CHARS;
  const [enginePreview, setEnginePreview] = useState<TsplLabelPreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);

  // DOC-03: the preview IS the backend plan — fetched per product/locale,
  // never assembled locally (local strings would drift from the bytes).
  useEffect(() => {
    if (!isOpen || tooWide) {
      setEnginePreview(null);
      setPreviewError(null);
      return;
    }
    let cancelled = false;
    setEnginePreview(null);
    setPreviewError(null);
    ipcPreviewTsplLabel(token, { name, priceCentimes, barcode: source }).then(
      (preview) => {
        if (!cancelled) {
          setEnginePreview(preview);
        }
      },
      (error: unknown) => {
        if (!cancelled) {
          setPreviewError(error instanceof Error ? error.message : String(error));
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [isOpen, product.product_id, locale, name, priceCentimes, source, tooWide, token]);

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
      const count = clampTsplCopies(toIntOrNull(copiesRaw));
      const bytes = await ipcPrintTsplLabel(token, {
        name,
        priceCentimes,
        barcode: source,
        copies: count,
      });
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
              {tooWide ? (
                <p
                  className="text-xs font-bold text-red-700 dark:text-red-300"
                  data-testid="silent-label-too-wide"
                  role="alert"
                >
                  {t(locale, "tspl.labelTooWide")}
                </p>
              ) : previewError !== null ? (
                <p
                  className="text-xs font-bold text-red-700 dark:text-red-300"
                  data-testid="silent-label-preview-error"
                  role="alert"
                >
                  {translateBackendError(previewError, locale)}
                </p>
              ) : enginePreview !== null ? (
                <SilentLabelPreviewView preview={enginePreview} barcode={source} />
              ) : null}
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
            disabled={busy || tooWide}
            className="h-11 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-60"
          >
            {busy ? t(locale, "tspl.sending") : t(locale, "tspl.sendLabels")}
          </button>
        </div>
      </div>
    </div>
  );
}
