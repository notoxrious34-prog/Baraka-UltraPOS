/**
 * Data-exchange settings tab (TASK-DSN-053): three executive action cards
 * (CSV import, CSV export, file template) plus the import modal mount.
 * Settings → Données → Échanges & Données.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Category, CatalogProduct, Locale } from "@/types/pos";
import {
  ipcGetStock,
  ipcListCategories,
  ipcSearchCatalog,
  listWarehouses,
} from "@/lib/ipc";
import { pickDepotId } from "@/features/catalog/ProductCreateModal";
import { buildCatalogCsvContent } from "@/features/catalog/catalog-export";
import { downloadSampleCsvTemplate } from "@/features/catalog/catalog-import-parser";
import { downloadTextFile } from "@/features/catalog/desktop-download";
import { ProductImportModal } from "@/features/catalog/ProductImportModal";

interface DataExchangeSettingsTabProps {
  locale: Locale;
  sessionToken: string;
}

const EXPORT_LIMIT = 1000;

export function DataExchangeSettingsTab({
  locale,
  sessionToken,
}: DataExchangeSettingsTabProps): ReactElement {
  const [categories, setCategories] = useState<Category[]>([]);
  const [importOpen, setImportOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void ipcListCategories(sessionToken, true)
      .then((rows) => {
        if (!cancelled) {
          setCategories(rows);
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
  }, [sessionToken]);

  const flashNote = (text: string | null): void => {
    setNote(text);
    if (text !== null) {
      window.setTimeout(() => {
        setNote(null);
      }, 8000);
    }
  };

  const runExport = (): void => {
    if (busy) {
      return;
    }
    setBusy(true);
    void (async () => {
      try {
        const rows: CatalogProduct[] = await ipcSearchCatalog(sessionToken, "", EXPORT_LIMIT);
        const depots = await listWarehouses(sessionToken);
        const depot = pickDepotId(depots);
        const levels: Record<string, string> = {};
        if (depot !== null) {
          const entries = await Promise.all(
            rows.map(async (product) => {
              try {
                const level = await ipcGetStock(sessionToken, product.product_id, depot);
                return [product.product_id, level] as const;
              } catch {
                return null;
              }
            }),
          );
          for (const entry of entries) {
            if (entry !== null) {
              levels[entry[0]] = entry[1];
            }
          }
        }
        const stamp = new Date().toISOString().slice(0, 10);
        const outcome = downloadTextFile(
          buildCatalogCsvContent(rows, levels, locale),
          `${t(locale, "cat.csvExportFilename")}-${stamp}.csv`,
        );
        flashNote(outcome.ok ? outcome.filename : null);
      } catch {
        flashNote(null);
      } finally {
        setBusy(false);
      }
    })();
  };

  const runTemplate = (): void => {
    const outcome = downloadSampleCsvTemplate();
    flashNote(outcome.ok ? outcome.filename : null);
  };

  const cards: Array<{
    titleKey: "settings.exchangeImport" | "settings.exchangeExport" | "settings.exchangeTemplate";
    copyKey:
      | "settings.exchangeImportCopy"
      | "settings.exchangeExportCopy"
      | "settings.exchangeTemplateCopy";
    action: () => void;
    busyLabel?: boolean;
  }> = [
    {
      titleKey: "settings.exchangeImport",
      copyKey: "settings.exchangeImportCopy",
      action: () => {
        setImportOpen(true);
      },
    },
    {
      titleKey: "settings.exchangeExport",
      copyKey: "settings.exchangeExportCopy",
      action: runExport,
      busyLabel: true,
    },
    {
      titleKey: "settings.exchangeTemplate",
      copyKey: "settings.exchangeTemplateCopy",
      action: runTemplate,
    },
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
      <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
        {cards.map((card) => (
          <div
            key={card.titleKey}
            className="flex flex-col gap-2 rounded-xl border border-slate-200 bg-white p-4 shadow-[0_1px_3px_rgba(0,0,0,0.04)] dark:border-slate-700 dark:bg-slate-900"
          >
            <h2 className="text-sm font-black">{t(locale, card.titleKey)}</h2>
            <p className="min-h-8 flex-1 text-xs leading-relaxed text-slate-500">
              {t(locale, card.copyKey)}
            </p>
            <button
              type="button"
              disabled={busy}
              onClick={card.action}
              className="h-10 rounded-lg bg-primary-600 text-sm font-black text-white hover:bg-primary-900 disabled:opacity-50"
            >
              {card.busyLabel === true && busy
                ? t(locale, "settings.saving")
                : t(locale, "pack.start")}
            </button>
          </div>
        ))}
      </div>
      {note !== null && (
        <p
          role="status"
          className="inline-flex w-fit items-center rounded-lg bg-emerald-100 px-3 py-2 font-mono text-xs font-black text-emerald-800 dark:bg-emerald-900 dark:text-emerald-200"
        >
          {"✓ "}
          {note}
        </p>
      )}
      <ProductImportModal
        isOpen={importOpen}
        token={sessionToken}
        categories={categories}
        onClose={() => {
          setImportOpen(false);
        }}
        onSuccess={() => {
          setImportOpen(false);
        }}
      />
    </div>
  );
}
