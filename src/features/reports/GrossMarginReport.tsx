/**
 * Gross margin report: HT revenue, tracked cost, margin and margin %
 * per product, with untracked-cost badges (zero unit cost lines).
 * Margins color-code green (positive) / red (negative), JetBrains Mono.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatMoney } from "@/lib/money";
import { ipcGrossMargin } from "@/lib/ipc";
import type { GrossMarginRow } from "@/types/reports";
import { formatBasisPoints } from "@/features/reports/components/report-format";
import {
  ReportDateRangePicker,
  todayIso,
} from "@/features/reports/components/ReportDateRangePicker";

export function GrossMarginReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [range, setRange] = useState(() => {
    const today = todayIso();
    return { dateFrom: today, dateTo: today };
  });
  const [rows, setRows] = useState<GrossMarginRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const token = session?.token ?? null;
  if (token === null) {
    return <div />;
  }

  const fetch = async (): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      setRows(await ipcGrossMargin(token, range));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  const marginClass = (marginCentimes: string): string =>
    BigInt(marginCentimes) < 0n
      ? "text-red-700 dark:text-red-300"
      : "text-emerald-700 dark:text-emerald-300";

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-base font-black">{t(locale, "rep.margin")}</h3>
      <ReportDateRangePicker value={range} onChange={setRange} onApply={() => void fetch()} applying={loading} />
      {loading && <p className="text-xs font-bold">{t(locale, "rep.loading")}</p>}
      {error !== null && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(error, locale)}
        </p>
      )}
      {rows !== null && !loading && error === null && (
        <>
          {rows.length === 0 ? (
            <p className="text-sm font-bold text-slate-500">{t(locale, "rep.empty")}</p>
          ) : (
            <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
              <table className="w-full text-xs font-bold">
                <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
                  <tr>
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.product")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.totalHt")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.cost")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.marginDzd")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.marginPct")}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr key={row.productId} className="border-t border-slate-200 dark:border-slate-700">
                      <td className="px-2 py-1.5 text-start">
                        {row.sku}
                        {row.hasUntrackedCost && (
                          <span className="ms-2 rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-black text-amber-800 dark:bg-amber-900/40 dark:text-amber-200">
                            {t(locale, "rep.untracked")}
                          </span>
                        )}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {formatMoney(row.revenueHtCentimes, locale)}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {formatMoney(row.costTotalCentimes, locale)}
                      </td>
                      <td className={`px-2 py-1.5 text-end font-mono tabular-nums ${marginClass(row.marginCentimes)}`}>
                        {formatMoney(row.marginCentimes, locale)}
                      </td>
                      <td className={`px-2 py-1.5 text-end font-mono tabular-nums ${marginClass(row.marginCentimes)}`}>
                        {formatBasisPoints(row.marginPercentBp)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </div>
  );
}
