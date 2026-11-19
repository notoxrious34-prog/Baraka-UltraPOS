import { useCallback, useEffect, useState } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import type { CatalogProduct, Locale } from "@/types/pos";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { TSPL_COPIES_MIN, TSPL_COPIES_MAX, clampTsplCopies } from "@/lib/tspl-print";
import { ipcPreviewTsplStudioLabel, ipcPrintTsplStudioLabel } from "@/lib/ipc";
import type { StudioTemplate, TsplLabelPreview, TsplPreviewLine } from "@/lib/ipc";
import {
  labelBarcodeSource,
  labelPriceUnit,
} from "@/features/catalog/ProductLabelModal";
import { silentLabelPreviewSvg, toIntOrNull } from "@/features/catalog/SilentLabelModal";
import { baseUnitOf } from "@/features/catalog/CatalogModule";

export interface StudioLabelModalProps {
  product: CatalogProduct;
  isOpen: boolean;
  onClose: () => void;
  token: string;
}

function displayName(product: CatalogProduct, locale: Locale): string {
  return locale === "ar-DZ" ? product.name_ar : product.name_fr;
}

/** Template prefill: the seller edits text, never money (price rides silently). */
function templatePrefill(
  template: StudioTemplate,
  product: CatalogProduct,
  locale: Locale,
): [string, string, string, string] {
  const name = displayName(product, locale);
  if (template === "hardware") {
    const unit = baseUnitOf(product);
    const unitName =
      unit === null ? "" : locale === "ar-DZ" ? unit.unit_name_ar : unit.unit_name_fr;
    return [name, unitName, "", ""];
  }
  return [name, "", "", ""];
}

/**
 * Engine-faithful studio preview (DOC-03): the backend plan lines in
 * spool order, with the Code128 band inserted where the engine puts it —
 * lines fully above the band first (textile: bars y126, band last),
 * lines fully below it after (hardware: bars y16, band first). The plan
 * is disjoint from the band by construction, so the split is exact.
 */
export function StudioLabelPreviewView({
  preview,
  barcode,
}: {
  preview: TsplLabelPreview;
  barcode: string;
}) {
  const svg = silentLabelPreviewSvg(barcode);
  const above = preview.lines.filter((line) => line.y + line.cellH <= preview.barcodeY);
  const below = preview.lines.filter((line) => line.y + line.cellH > preview.barcodeY);
  return (
    <>
      {above.map((line, index) => (
        <StudioPreviewLine key={`${line.slot}-${index}`} line={line} />
      ))}
      {svg !== null && (
        <div
          className="w-full"
          style={{ width: `${Math.min(100, (preview.barcodeWidth / 320) * 100)}%` }}
          dangerouslySetInnerHTML={{ __html: svg }}
        />
      )}
      {below.map((line, index) => (
        <StudioPreviewLine key={`${line.slot}-tail-${index}`} line={line} />
      ))}
    </>
  );
}

function StudioPreviewLine({ line }: { line: TsplPreviewLine }) {
  switch (line.slot) {
    case "studio_model":
      return <p className="max-w-full truncate text-center text-[11px] font-extrabold text-slate-900">{line.text}</p>;
    case "studio_size":
      return (
        <p className="text-center text-[26px] font-black leading-none text-slate-900" dir="ltr">
          {line.text}
        </p>
      );
    case "studio_color":
    case "studio_pack":
      return (
        <p className="w-full truncate text-right text-[10px] font-bold text-slate-600" dir="auto">
          {line.text}
        </p>
      );
    case "studio_price":
      return (
        <p className="text-center text-[14px] font-black leading-none text-slate-900" dir="ltr">
          {line.text}
        </p>
      );
    case "studio_hw_name":
      return <p className="max-w-full truncate text-center text-[11px] font-extrabold text-slate-900">{line.text}</p>;
    case "studio_hw_dim":
      return (
        <p className="max-w-full truncate text-center font-mono text-[10px] text-slate-900" dir="ltr" title={line.text}>
          {line.text}
        </p>
      );
    case "studio_hw_price":
      return (
        <p className="text-center text-[12px] font-black leading-none text-slate-900" dir="ltr">
          {line.text}
        </p>
      );
    case "human":
      return (
        <p className="max-w-full text-center font-mono text-[10px] text-slate-900" dir="ltr" title={line.text}>
          {line.text}
        </p>
      );
    default:
      return <p className="max-w-full truncate text-center text-[11px] font-extrabold text-slate-900">{line.text}</p>;
  }
}

/**
 * Label Studio modal (TASK-DSN-107): template-indexed silent prints to
 * the LABEL role — spooler-only, button-triggered, never a dialog. Text
 * fields prefill from the product (the seller edits words, never money);
 * the wholesale price-hide exists on textile only (hardware rejects it
 * loudly). The preview IS the backend plan per DOC-03.
 */
export function StudioLabelModal({ product, isOpen, onClose, token }: StudioLabelModalProps) {
  const locale = usePosUiStore((state) => state.locale);
  const [template, setTemplate] = useState<StudioTemplate>("textile");
  const [lines, setLines] = useState<[string, string, string, string]>(() =>
    templatePrefill("textile", product, locale),
  );
  const [hidePrice, setHidePrice] = useState(false);
  const [copiesRaw, setCopiesRaw] = useState("1");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const { priceCentimes } = labelPriceUnit(product);
  const source = labelBarcodeSource(product);
  const tooWide = source.length > 9;
  const [enginePreview, setEnginePreview] = useState<TsplLabelPreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);

  const switchTemplate = useCallback(
    (next: StudioTemplate) => {
      setTemplate(next);
      setLines(templatePrefill(next, product, locale));
      setHidePrice(false);
      setNotice(null);
      setFailure(null);
    },
    [product, locale],
  );

  // DOC-03: the preview IS the backend plan — fetched per field edit,
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
    void ipcPreviewTsplStudioLabel(token, {
      template,
      line1: lines[0],
      line2: lines[1],
      line3: lines[2],
      line4: lines[3],
      hidePrice: template === "textile" && hidePrice,
      priceCentimes: template === "textile" && hidePrice ? null : priceCentimes,
      barcode: source,
    }).then(
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
  }, [isOpen, template, lines, hidePrice, priceCentimes, source, tooWide, token]);

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
      const bytes = await ipcPrintTsplStudioLabel(token, {
        template,
        line1: lines[0],
        line2: lines[1],
        line3: lines[2],
        line4: lines[3],
        hidePrice: template === "textile" && hidePrice,
        priceCentimes: template === "textile" && hidePrice ? null : priceCentimes,
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
  }, [busy, copiesRaw, locale, template, lines, hidePrice, priceCentimes, source, token]);

  if (!isOpen) {
    return null;
  }

  const fieldKeys =
    template === "textile"
      ? (["tspl.studioModel", "tspl.studioSize", "tspl.studioColor", "tspl.studioPack"] as const)
      : (["tspl.studioDesignation", "tspl.studioUnit", "tspl.studioDimension"] as const);
  const placeholders = template === "textile" ? ["", "XL", "NOIR", "PACK 6 PCS"] : ["", "", "O8X32MM"];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4">
      <div
        className="flex max-h-full w-full max-w-lg flex-col overflow-hidden rounded-xl bg-white shadow-2xl dark:bg-slate-900"
        role="dialog"
        aria-modal="true"
        aria-label={t(locale, "tspl.studioTitle")}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex h-14 shrink-0 items-center justify-between bg-primary-900 px-4 text-white">
          <h2 className="text-base font-extrabold">{t(locale, "tspl.studioTitle")}</h2>
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
          <div className="flex gap-2" role="tablist" aria-label={t(locale, "tspl.studioLayout")}>
            {(["textile", "hardware"] as const).map((value) => (
              <button
                key={value}
                type="button"
                role="tab"
                aria-selected={template === value}
                data-testid={`studio-template-${value}`}
                onClick={() => switchTemplate(value)}
                disabled={busy}
                className={`h-9 flex-1 rounded-lg text-sm font-black ${
                  template === value
                    ? "bg-primary-900 text-white"
                    : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                }`}
              >
                {t(locale, value === "textile" ? "tspl.studioTextile" : "tspl.studioHardware")}
              </button>
            ))}
          </div>
          <div className="flex justify-center rounded-lg border border-dashed border-slate-300 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800/50">
            <div
              className="flex max-w-full flex-col items-center justify-center gap-0.5 overflow-hidden border border-slate-400 bg-white px-2 py-1 text-slate-900"
              style={{ aspectRatio: "40 / 20" }}
              data-testid="studio-label-preview"
            >
              {tooWide ? (
                <p
                  className="text-xs font-bold text-red-700 dark:text-red-300"
                  data-testid="studio-label-too-wide"
                  role="alert"
                >
                  {t(locale, "tspl.labelTooWide")}
                </p>
              ) : previewError !== null ? (
                <p
                  className="text-xs font-bold text-red-700 dark:text-red-300"
                  data-testid="studio-label-preview-error"
                  role="alert"
                >
                  {translateBackendError(previewError, locale)}
                </p>
              ) : enginePreview !== null ? (
                <StudioLabelPreviewView preview={enginePreview} barcode={source} />
              ) : null}
            </div>
          </div>
          {fieldKeys.map((key, index) => (
            <label key={key} className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">{t(locale, key)}</span>
              <input
                className="h-10 rounded-lg border border-slate-300 px-3 text-sm font-bold dark:border-slate-600 dark:bg-slate-800"
                data-testid={`studio-field-${index}`}
                value={lines[index]}
                dir="auto"
                disabled={busy}
                placeholder={placeholders[index]}
                onChange={(event) =>
                  setLines((prev) => {
                    const next: [string, string, string, string] = [...prev];
                    next[index] = event.target.value;
                    return next;
                  })
                }
              />
            </label>
          ))}
          {template === "textile" && (
            <label className="flex items-center gap-2 text-sm font-bold text-slate-700 dark:text-slate-200">
              <input
                type="checkbox"
                data-testid="studio-hide-price"
                checked={hidePrice}
                disabled={busy}
                onChange={(event) => setHidePrice(event.target.checked)}
                className="h-4 w-4"
              />
              {t(locale, "tspl.studioHidePrice")}
            </label>
          )}
          {template === "textile" && hidePrice && (
            <p className="text-xs font-bold text-slate-500" data-testid="studio-wholesale-hint">
              {t(locale, "tspl.studioWholesaleHint")}
            </p>
          )}
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs font-bold text-slate-500">{t(locale, "tspl.copies")}</span>
            <span className="flex items-center gap-2">
              <button
                type="button"
                aria-label="−"
                data-testid="studio-label-copies-minus"
                onClick={() => step(-1)}
                disabled={busy}
                className="h-9 w-9 rounded-lg bg-slate-200 text-base font-black hover:bg-slate-300 dark:bg-slate-700"
              >
                −
              </button>
              <input
                className="h-9 w-16 rounded-lg border border-slate-300 text-center font-mono text-sm font-black dark:border-slate-600"
                data-testid="studio-label-copies"
                value={copiesRaw}
                dir="ltr"
                inputMode="numeric"
                disabled={busy}
                onChange={(event) => setCopiesRaw(event.target.value)}
              />
              <button
                type="button"
                aria-label="+"
                data-testid="studio-label-copies-plus"
                onClick={() => step(1)}
                disabled={busy}
                className="h-9 w-9 rounded-lg bg-slate-200 text-base font-black hover:bg-slate-300 dark:bg-slate-700"
              >
                +
              </button>
            </span>
          </div>
          {notice !== null && (
            <p className="text-sm font-bold text-emerald-700" data-testid="studio-label-notice" role="status">
              {notice}
            </p>
          )}
          {failure !== null && (
            <p className="text-sm font-bold text-red-700" data-testid="studio-label-error" role="alert">
              {failure}
            </p>
          )}
          <button
            type="button"
            data-testid="studio-label-send"
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
