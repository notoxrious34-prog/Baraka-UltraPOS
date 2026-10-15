/**
 * Bulk CSV import modal for the Master Catalog Hub (TASK-DSN-045).
 *
 * Four-stage flow: file drop + template card -> client-side dry-run preview
 * (first 5 rows, policy selector) -> atomic live import -> structured
 * execution report. Mounted as an overlay inside `CatalogModule` (ADR-067:
 * `fixed` only for modal overlays). Pure sub-views (`ImportPreviewTable`,
 * `ImportReportCard`) are exported for SSR-safe tests; the container owns
 * FileReader + IPC side effects. No floats, no hardcoded colors, logical
 * properties for RTL.
 */

import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import {
  downloadSampleCsvTemplate,
  parseCatalogCsv,
  type CatalogCsvParseResult,
} from "@/features/catalog/catalog-import-parser";
import { t, translateBackendError } from "@/lib/i18n";
import { ipcImportProducts } from "@/lib/ipc";
import { usePosUiStore } from "@/stores/pos-ui-store";
import type {
  Category,
  ImportProductRow,
  ImportProductsReport,
  Locale,
} from "@/types/pos";

export interface ProductImportModalProps {
  isOpen: boolean;
  token: string;
  onClose: () => void;
  onSuccess: () => void;
  categories: Category[];
  /**
   * BRK-069 starter-pack preload: when present and the modal opens on the
   * drop stage, the CSV text is ingested straight to preview (zero file
   * hunting). `key` distinguishes successive preloads.
   */
  preset?: { key: number; filename: string; content: string } | null;
}

type ImportStage = "drop" | "preview" | "importing" | "report";

/** Build the code-or-id -> id lookup for category resolution. Pure. */
export function buildCategoryLookup(categories: Category[]): Map<string, string> {
  const lookup = new Map<string, string>();
  for (const category of categories) {
    lookup.set(category.id, category.id);
    lookup.set(category.code.trim().toUpperCase(), category.id);
  }
  return lookup;
}

/** First N valid rows for the dry-run preview table. Pure. */
export function previewRows(parsed: CatalogCsvParseResult, count = 5): ImportProductRow[] {
  return parsed.rows.slice(0, count);
}

/** Resolve a display name for a row category id. Pure. */
export function categoryLabelFor(
  categories: Category[],
  categoryId: string | null | undefined,
  locale: Locale,
): string {
  if (categoryId === null || categoryId === undefined || categoryId === "") {
    return "—";
  }
  const found = categories.find((category) => category.id === categoryId);
  if (found === undefined) {
    return "—";
  }
  return locale === "ar-DZ" ? found.name_ar : found.name_fr;
}

/** Format integer centimes as a fixed 2-decimal Latin string. Pure. */
export function formatCentimes(centimes: number | null | undefined): string {
  if (centimes === null || centimes === undefined) {
    return "—";
  }
  const negative = centimes < 0;
  const abs = Math.abs(centimes);
  const body = `${Math.floor(abs / 100)}.${(abs % 100).toString().padStart(2, "0")}`;
  return negative ? `-${body}` : body;
}

/** Format base-unit micros as a trimmed quantity string. Pure. */
export function formatMicros(micros: number | null | undefined): string {
  if (micros === null || micros === undefined) {
    return "—";
  }
  const whole = Math.trunc(micros / 10_000);
  const frac = Math.abs(micros % 10_000)
    .toString()
    .padStart(4, "0")
    .replace(/0+$/, "");
  return frac === "" ? `${whole}` : `${whole}.${frac}`;
}

const inputClass =
  "h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";

export function ImportPreviewTable({
  rows,
  categories,
  locale,
}: {
  rows: ImportProductRow[];
  categories: Category[];
  locale: Locale;
}): ReactElement {
  return (
    <table className="w-full text-start text-xs">
      <thead>
        <tr className="border-b border-slate-200 text-slate-500 dark:border-slate-700">
          <th className="px-2 py-1 text-start font-black">SKU</th>
          <th className="px-2 py-1 text-start font-black">Barcode</th>
          <th className="px-2 py-1 text-start font-black">
            {locale === "ar-DZ" ? "التعيين" : "Désignation"}
          </th>
          <th className="px-2 py-1 text-end font-black">
            {locale === "ar-DZ" ? "شراء" : "Achat"}
          </th>
          <th className="px-2 py-1 text-end font-black">
            {locale === "ar-DZ" ? "بيع" : "Vente"}
          </th>
          <th className="px-2 py-1 text-end font-black">Stock</th>
          <th className="px-2 py-1 text-start font-black">
            {locale === "ar-DZ" ? "القسم" : "Rayon"}
          </th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row, index) => (
          <tr
            key={`${row.sku ?? ""}-${row.name_fr}-${index}`}
            className="border-b border-slate-100 dark:border-slate-800"
          >
            <td className="px-2 py-1 font-bold">{row.sku ?? "—"}</td>
            <td className="px-2 py-1">{row.barcode ?? "—"}</td>
            <td className="px-2 py-1 font-bold">{row.name_fr}</td>
            <td className="px-2 py-1 text-end">
              {formatCentimes(row.purchase_price_centimes)}
            </td>
            <td className="px-2 py-1 text-end font-black">
              {formatCentimes(row.sale_price_centimes)}
            </td>
            <td className="px-2 py-1 text-end">
              {formatMicros(row.initial_quantity_micro)}
            </td>
            <td className="px-2 py-1">
              {categoryLabelFor(categories, row.category_id, locale)}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function ImportReportCard({
  report,
  locale,
}: {
  report: ImportProductsReport;
  locale: Locale;
}): ReactElement {
  return (
    <div className="space-y-3">
      <p className="text-sm font-black text-emerald-700 dark:text-emerald-400">
        {t(locale, "imp.reportTitle")} {report.duration_ms} ms
      </p>
      <div className="flex gap-2">
        <span className="rounded-full bg-emerald-100 px-3 py-1 text-xs font-black text-emerald-800 dark:bg-emerald-900 dark:text-emerald-200">
          {report.imported_count} {t(locale, "imp.imported")}
        </span>
        <span className="rounded-full bg-amber-100 px-3 py-1 text-xs font-black text-amber-800 dark:bg-amber-900 dark:text-amber-200">
          {report.skipped_count} {t(locale, "imp.skipped")}
        </span>
        <span className="rounded-full bg-rose-100 px-3 py-1 text-xs font-black text-rose-800 dark:bg-rose-900 dark:text-rose-200">
          {report.errors.length} {t(locale, "imp.errored")}
        </span>
      </div>
      {report.errors.length > 0 && (
        <ul className="max-h-40 space-y-1 overflow-y-auto rounded-lg border border-slate-200 p-2 text-xs dark:border-slate-700">
          {report.errors.map((error) => (
            <li key={`${error.row_index}-${error.sku ?? ""}`} className="font-bold">
              #{error.row_index} {error.sku ?? "—"} {error.name_fr} — {error.error}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function ProductImportModal({
  isOpen,
  token,
  onClose,
  onSuccess,
  categories,
  preset = null,
}: ProductImportModalProps): ReactElement | null {
  const locale = usePosUiStore((state) => state.locale);
  const fileRef = useRef<HTMLInputElement>(null);
  const [stage, setStage] = useState<ImportStage>("drop");
  const [fileName, setFileName] = useState("");
  const [parsed, setParsed] = useState<CatalogCsvParseResult>({ rows: [], errors: [] });
  const [policy, setPolicy] = useState<"skip" | "error">("skip");
  const [report, setReport] = useState<ImportProductsReport | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  /** Last downloaded template filename (inline confirmation, TASK-REL-055). */
  const [templateNote, setTemplateNote] = useState<string | null>(null);

  const handleTemplateDownload = (): void => {
    const outcome = downloadSampleCsvTemplate();
    setTemplateNote(outcome.ok ? outcome.filename : null);
    if (outcome.ok) {
      window.setTimeout(() => {
        setTemplateNote(null);
      }, 8000);
    }
  };
  const lookup = useMemo(() => buildCategoryLookup(categories), [categories]);

  useEffect(() => {
    if (isOpen && preset !== null && preset.content !== "") {
      setFileName(preset.filename);
      setParsed(parseCatalogCsv(preset.content, lookup));
      setFailure(null);
      setReport(null);
      setStage("preview");
    }
  }, [isOpen, preset, lookup]);

  if (!isOpen) {
    return null;
  }

  const reset = (): void => {
    setStage("drop");
    setFileName("");
    setParsed({ rows: [], errors: [] });
    setReport(null);
    setFailure(null);
  };

  const ingestText = (text: string, name: string): void => {
    setFileName(name);
    setParsed(parseCatalogCsv(text, lookup));
    setFailure(null);
    setStage("preview");
  };

  const ingestFile = (file: File | undefined): void => {
    if (file === undefined) {
      return;
    }
    const reader = new FileReader();
    reader.onerror = () => {
      setFailure(t(locale, "imp.readError"));
    };
    reader.onload = () => {
      if (typeof reader.result === "string") {
        ingestText(reader.result, file.name);
      } else {
        setFailure(t(locale, "imp.readError"));
      }
    };
    reader.readAsText(file, "utf-8");
  };

  const launchImport = (): void => {
    setStage("importing");
    setFailure(null);
    ipcImportProducts(token, parsed.rows, undefined, policy).then(
      (result) => {
        setReport(result);
        setStage("report");
      },
      (error: unknown) => {
        setFailure(
          error instanceof Error ? error.message : translateBackendError(error, locale),
        );
        setStage("preview");
      },
    );
  };

  const busy = stage === "importing";
  const totalDetected = parsed.rows.length + parsed.errors.length;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={t(locale, "imp.title")}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) {
          reset();
          onClose();
        }
      }}
    >
      <div className="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl dark:border-slate-700 dark:bg-slate-900">
        <div className="flex h-14 shrink-0 items-center justify-between bg-primary-900 px-4 text-white">
          <h2 className="text-base font-extrabold">{t(locale, "imp.title")}</h2>
          <button
            type="button"
            disabled={busy}
            aria-label={t(locale, "imp.cancel")}
            onClick={() => {
              reset();
              onClose();
            }}
            className="rounded-lg px-2 py-1 text-lg font-black hover:bg-white/10 disabled:opacity-50"
          >
            ✕
          </button>
        </div>
        <div className="space-y-4 overflow-y-auto p-4">
          {failure !== null && (
            <p className="rounded-lg bg-rose-100 px-3 py-2 text-xs font-black text-rose-800 dark:bg-rose-900 dark:text-rose-200">
              {failure}
            </p>
          )}
          {stage === "drop" && (
            <div className="space-y-3">
              <div
                role="button"
                tabIndex={0}
                aria-label={t(locale, "imp.dropHint")}
                onClick={() => fileRef.current?.click()}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    fileRef.current?.click();
                  }
                }}
                onDragOver={(event) => {
                  event.preventDefault();
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  ingestFile(event.dataTransfer.files?.[0]);
                }}
                className="flex cursor-pointer flex-col items-center gap-2 rounded-xl border-2 border-dashed border-slate-300 px-4 py-10 text-center dark:border-slate-600"
              >
                <span className="text-4xl" aria-hidden="true">
                  📥
                </span>
                <p className="text-sm font-bold">
                  {t(locale, "imp.dropHint")} {t(locale, "imp.browse")}
                </p>
                <p className="text-xs opacity-60">.csv · .txt</p>
              </div>
              <input
                ref={fileRef}
                type="file"
                accept=".csv,.txt"
                className="hidden"
                aria-hidden="true"
                tabIndex={-1}
                onChange={(event) => {
                  ingestFile(event.target.files?.[0]);
                  event.target.value = "";
                }}
              />
              <button
                type="button"
                onClick={handleTemplateDownload}
                className="h-10 w-full rounded-lg bg-slate-200 px-4 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
              >
                {t(locale, "imp.downloadTemplate")}
              </button>
              {templateNote !== null && (
                <p
                  role="status"
                  className="rounded-lg bg-emerald-100 px-3 py-2 font-mono text-xs font-black text-emerald-800 dark:bg-emerald-900 dark:text-emerald-200"
                >
                  {"✓ "}
                  {templateNote}
                </p>
              )}
            </div>
          )}
          {stage === "preview" && (
            <div className="space-y-3">
              <div className="flex flex-wrap gap-2">
                <span className="rounded-full bg-slate-200 px-3 py-1 text-xs font-black dark:bg-slate-700">
                  {t(locale, "imp.totalDetected")}: {totalDetected}
                </span>
                <span className="rounded-full bg-emerald-100 px-3 py-1 text-xs font-black text-emerald-800 dark:bg-emerald-900 dark:text-emerald-200">
                  {t(locale, "imp.validRows")}: {parsed.rows.length}
                </span>
                {parsed.errors.length > 0 && (
                  <span className="rounded-full bg-amber-100 px-3 py-1 text-xs font-black text-amber-800 dark:bg-amber-900 dark:text-amber-200">
                    {t(locale, "imp.errorRows")}: {parsed.errors.length}
                  </span>
                )}
              </div>
              <p className="truncate text-xs font-bold opacity-70">{fileName}</p>
              <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
                <ImportPreviewTable
                  rows={previewRows(parsed)}
                  categories={categories}
                  locale={locale}
                />
              </div>
              {parsed.errors.length > 0 && (
                <ul className="max-h-28 space-y-1 overflow-y-auto rounded-lg border border-amber-300 p-2 text-xs dark:border-amber-700">
                  {parsed.errors.slice(0, 10).map((error) => (
                    <li key={error.line} className="font-bold">
                      #{error.line} — {error.message}
                    </li>
                  ))}
                </ul>
              )}
              <label className="block text-xs font-black">
                <select
                  value={policy}
                  onChange={(event) => {
                    setPolicy(event.target.value === "error" ? "error" : "skip");
                  }}
                  className={inputClass}
                >
                  <option value="skip">{t(locale, "imp.policySkip")}</option>
                  <option value="error">{t(locale, "imp.policyError")}</option>
                </select>
              </label>
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => {
                    reset();
                    onClose();
                  }}
                  className="h-10 flex-1 rounded-lg bg-slate-200 px-4 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
                >
                  {t(locale, "imp.cancel")}
                </button>
                <button
                  type="button"
                  disabled={parsed.rows.length === 0}
                  onClick={launchImport}
                  className="h-10 flex-1 rounded-lg bg-primary-600 px-4 text-sm font-black text-white hover:bg-primary-900 disabled:opacity-50"
                >
                  {t(locale, "imp.startImport")} ({parsed.rows.length}{" "}
                  {t(locale, "imp.articles")})
                </button>
              </div>
            </div>
          )}
          {stage === "importing" && (
            <div className="flex flex-col items-center gap-3 py-10">
              <span
                className="h-10 w-10 animate-spin rounded-full border-4 border-slate-300 border-t-primary-600"
                aria-hidden="true"
              />
              <p className="text-sm font-black">{t(locale, "imp.importing")}</p>
            </div>
          )}
          {stage === "report" && report !== null && (
            <div className="space-y-4">
              <ImportReportCard report={report} locale={locale} />
              <button
                type="button"
                onClick={() => {
                  onSuccess();
                  reset();
                  onClose();
                }}
                className="h-10 w-full rounded-lg bg-primary-600 px-4 text-sm font-black text-white hover:bg-primary-900"
              >
                {t(locale, "imp.finishRefresh")}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
