/**
 * Stock overview & depletion alerts pane (TASK-DSN-029): HUD metric cards
 * plus the ADR-037 low-stock table. Quantities via formatQty (micro
 * strings, zero floats), wrapped in <bdi dir="ltr"> tabular figures.
 */

import { useMemo, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import { formatQty } from "@/lib/money";
import type { Locale } from "@/types/pos";
import type { LowStockRow } from "@/types/reports";

/**
 * Client-side instant filter over displayed stock rows (TASK-DSN-050).
 * Case-insensitive substring match on French designation or SKU; blank
 * queries return the rows untouched. Pure — unit-tested directly.
 */
export function filterStockRows(rows: LowStockRow[], query: string): LowStockRow[] {
  const q = query.trim().toLowerCase();
  if (q === "") {
    return rows;
  }
  return rows.filter(
    (row) => row.nameFr.toLowerCase().includes(q) || row.sku.toLowerCase().includes(q),
  );
}

interface StockOverviewPaneProps {
  locale: Locale;
  warehouseCount: number;
  rows: LowStockRow[];
  loading: boolean;
  onAdjust: (row: LowStockRow) => void;
}

function fmtQty(micro: string): string {
  try {
    return formatQty(micro);
  } catch {
    return "…";
  }
}

function HudCard(props: { label: string; value: string; alert: boolean }): ReactElement {
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1 rounded-lg border border-slate-200 bg-white p-3 dark:border-slate-700 dark:bg-slate-950">
      <span className="text-[11px] font-bold text-slate-500">{props.label}</span>
      <bdi
        dir="ltr"
        className={`font-mono text-xl font-black tabular-nums ${
          props.alert ? "text-red-600 dark:text-red-400" : ""
        }`}
      >
        {props.value}
      </bdi>
    </div>
  );
}

export function StockOverviewPane(props: StockOverviewPaneProps): ReactElement {
  const { locale } = props;
  // Instant client-side text filter over the displayed rows.
  const [stockQuery, setStockQuery] = useState("");
  const filteredRows = useMemo(
    () => filterStockRows(props.rows, stockQuery),
    [props.rows, stockQuery],
  );
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden p-3">
      <div className="flex shrink-0 flex-wrap gap-2">
        <HudCard
          label={t(locale, "stock.activeWarehouses")}
          value={String(props.warehouseCount)}
          alert={false}
        />
        <HudCard
          label={t(locale, "stock.ruptureItems")}
          value={String(props.rows.length)}
          alert={props.rows.length > 0}
        />
      </div>
      <input
        id="stock-search-input"
        value={stockQuery}
        onChange={(event) => {
          setStockQuery(event.target.value);
        }}
        placeholder={t(locale, "stock.searchPlaceholder")}
        className="h-9 w-full max-w-xs shrink-0 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-950"
      />
      <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-950">
        {props.loading && (
          <p className="p-6 text-center text-xs font-bold text-slate-500">
            {t(locale, "stock.searching")}
          </p>
        )}
        {!props.loading && props.rows.length === 0 && (
          <p className="p-6 text-center text-xs font-bold text-emerald-600 dark:text-emerald-400">
            {t(locale, "stock.noRupture")}
          </p>
        )}
        {!props.loading && props.rows.length > 0 && (
          <table className="w-full border-collapse text-xs">
            <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
              <tr className="text-start text-slate-500">
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colSku")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colProduct")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colWarehouse")}</th>
                <th className="px-2 py-2 text-end font-bold">{t(locale, "stock.colQty")}</th>
                <th className="px-2 py-2 text-end font-bold">{t(locale, "stock.colAdjust")}</th>
              </tr>
            </thead>
            <tbody>
              {filteredRows.map((row) => (
                <tr
                  key={`${row.warehouseId}:${row.productId}`}
                  className="border-b border-slate-100 dark:border-slate-800"
                >
                  <td className="whitespace-nowrap px-2 py-2 font-mono font-bold">
                    <bdi dir="ltr">{row.sku}</bdi>
                  </td>
                  <td className="max-w-44 truncate px-2 py-2">{row.nameFr}</td>
                  <td className="whitespace-nowrap px-2 py-2 text-slate-500">
                    <bdi dir="ltr">{row.warehouseCode}</bdi>
                  </td>
                  <td className="whitespace-nowrap px-2 py-2 text-end">
                    <bdi
                      dir="ltr"
                      className="rounded-full bg-red-100 px-2 py-0.5 font-mono font-black text-red-700 tabular-nums dark:bg-red-900/50 dark:text-red-200"
                    >
                      {fmtQty(row.quantityBaseMicro)}
                    </bdi>
                  </td>
                  <td className="whitespace-nowrap px-2 py-2 text-end">
                    <button
                      type="button"
                      onClick={() => {
                        props.onAdjust(row);
                      }}
                      className="rounded-md bg-primary-900 px-2.5 py-1 text-[11px] font-bold text-white hover:bg-primary-800 dark:bg-primary-700"
                    >
                      {t(locale, "stock.colAdjust")}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
