/**
 * Daily sales report: KPI summary cards plus a per-day breakdown table.
 * Source: reports.dailySales (committed TICKET/FACTURE/BL only).
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatMoney } from "@/lib/money";
import { ipcDailySales } from "@/lib/ipc";
import type { DailySalesRow } from "@/types/reports";
import {
  ReportDateRangePicker,
  todayIso,
} from "@/features/reports/components/ReportDateRangePicker";

function sumField(rows: DailySalesRow[], pick: (row: DailySalesRow) => string): string {
  let total = 0n;
  for (const row of rows) {
    total += BigInt(pick(row));
  }
  return total.toString();
}

export function DailySalesReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [range, setRange] = useState(() => {
    const today = todayIso();
    return { dateFrom: today, dateTo: today };
  });
  const [rows, setRows] = useState<DailySalesRow[] | null>(null);
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
      setRows(await ipcDailySales(token, range));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  const kpi = (
    label: string,
    value: string,
  ): ReactElement => (
    <div className="rounded-xl bg-primary-900 p-3 text-white">
      <p className="text-[11px] font-bold text-white/70">{label}</p>
      <p className="font-mono text-lg font-black tabular-nums">{value}</p>
    </div>
  );

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-base font-black">{t(locale, "rep.daily")}</h3>
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
            <>
              <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
                {kpi(t(locale, "rep.totalTtc"), formatMoney(sumField(rows, (r) => r.totalTtcCentimes), locale))}
                {kpi(t(locale, "rep.totalHt"), formatMoney(sumField(rows, (r) => r.totalHtCentimes), locale))}
                {kpi(t(locale, "rep.totalTva"), formatMoney(sumField(rows, (r) => r.totalTvaCentimes), locale))}
                {kpi(
                  t(locale, "rep.documents"),
                  rows.reduce((acc, row) => acc + BigInt(row.documentCount), 0n).toString(),
                )}
              </div>
              <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
                <table className="w-full text-xs font-bold">
                  <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
                    <tr>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.day")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.totalTtc")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.totalHt")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.totalTva")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.documents")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.basket")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((row) => (
                      <tr key={row.day} className="border-t border-slate-200 dark:border-slate-700">
                        <td className="px-2 py-1.5 text-start font-mono tabular-nums">{row.day}</td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {formatMoney(row.totalTtcCentimes, locale)}
                        </td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {formatMoney(row.totalHtCentimes, locale)}
                        </td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {formatMoney(row.totalTvaCentimes, locale)}
                        </td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">{row.documentCount}</td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {formatMoney(row.averageBasketCentimes, locale)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
}
