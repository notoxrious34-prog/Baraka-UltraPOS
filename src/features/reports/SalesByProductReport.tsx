/**
 * Sales-by-product report: volume and revenue per product, revenue
 * descending (backend order preserved, never re-sorted client-side).
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatMoney, formatQty } from "@/lib/money";
import { ipcSalesByProduct } from "@/lib/ipc";
import type { SalesByProductRow } from "@/types/reports";
import {
  ReportDateRangePicker,
  todayIso,
} from "@/features/reports/components/ReportDateRangePicker";

export function SalesByProductReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [range, setRange] = useState(() => {
    const today = todayIso();
    return { dateFrom: today, dateTo: today };
  });
  const [rows, setRows] = useState<SalesByProductRow[] | null>(null);
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
      setRows(await ipcSalesByProduct(token, range));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-base font-black">{t(locale, "rep.byProduct")}</h3>
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
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.sku")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.qty")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.revenue")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.lines")}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr key={row.productId} className="border-t border-slate-200 dark:border-slate-700">
                      <td className="px-2 py-1.5 text-start">{row.nameFr}</td>
                      <td className="px-2 py-1.5 text-start font-mono">{row.sku}</td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {formatQty(row.quantitySoldBaseMicro)}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {formatMoney(row.revenueTtcCentimes, locale)}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">{row.lineCount}</td>
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
