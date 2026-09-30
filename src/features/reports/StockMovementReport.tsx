/**
 * Stock-movement history: filtered, paginated audit trail of every stock
 * movement (newest first). Page size 50/200 honours the backend maximum;
 * oversized requests surface REPORT_PAGINATION_LIMIT_EXCEEDED verbatim.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatQty } from "@/lib/money";
import { ipcStockMovementHistory } from "@/lib/ipc";
import type { StockMovementRow } from "@/types/reports";
import {
  ReportDateRangePicker,
  todayIso,
} from "@/features/reports/components/ReportDateRangePicker";

const PAGE_SIZE = "50";

export function StockMovementReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const warehouses = useSessionStore((state) => state.warehouses);
  const [range, setRange] = useState(() => {
    const today = todayIso();
    return { dateFrom: today, dateTo: today };
  });
  const [warehouseId, setWarehouseId] = useState("");
  const [movementType, setMovementType] = useState("");
  const [page, setPage] = useState(0);
  const [rows, setRows] = useState<StockMovementRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const token = session?.token ?? null;
  if (token === null) {
    return <div />;
  }

  const fetch = async (offsetPages: number): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      const pageRows = await ipcStockMovementHistory(token, {
        ...range,
        warehouseId: warehouseId === "" ? null : warehouseId,
        productId: null,
        movementType: movementType === "" ? null : movementType,
        limit: PAGE_SIZE,
        offset: String(offsetPages * Number(PAGE_SIZE)),
      });
      setRows(pageRows);
      setPage(offsetPages);
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  const signedClass = (qty: string): string =>
    qty.startsWith("-")
      ? "text-red-700 dark:text-red-300"
      : "text-emerald-700 dark:text-emerald-300";

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-base font-black">{t(locale, "rep.movements")}</h3>
      <ReportDateRangePicker
        value={range}
        onChange={setRange}
        onApply={() => void fetch(0)}
        applying={loading}
      />
      <div className="flex flex-wrap items-center gap-2">
        <select
          value={warehouseId}
          onChange={(event) => {
            setWarehouseId(event.target.value);
          }}
          className="h-9 rounded-lg border border-slate-300 bg-white px-2 text-xs font-bold dark:border-slate-700 dark:bg-slate-800"
        >
          <option value="">{t(locale, "rep.allWarehouses")}</option>
          {warehouses.map((warehouse) => (
            <option key={warehouse.id} value={warehouse.id}>
              {warehouse.code} — {warehouse.name}
            </option>
          ))}
        </select>
        <select
          value={movementType}
          onChange={(event) => {
            setMovementType(event.target.value);
          }}
          className="h-9 rounded-lg border border-slate-300 bg-white px-2 font-mono text-xs font-bold dark:border-slate-700 dark:bg-slate-800"
        >
          <option value="">{t(locale, "rep.allTypes")}</option>
          {["SALE_OUT", "PURCHASE_IN", "RETURN_IN", "ADJUSTMENT_POSITIVE", "ADJUSTMENT_NEGATIVE", "TRANSFER_OUT", "TRANSFER_IN"].map(
            (type) => (
              <option key={type} value={type}>
                {type}
              </option>
            ),
          )}
        </select>
      </div>
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
              <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
                <table className="w-full text-xs font-bold">
                  <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
                    <tr>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.date")}</th>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.type")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.qty")}</th>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.reference")}</th>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.operator")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((row) => (
                      <tr key={row.id} className="border-t border-slate-200 dark:border-slate-700">
                        <td className="px-2 py-1.5 text-start font-mono text-[10px] tabular-nums">
                          {row.createdAt}
                        </td>
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">{row.movementType}</td>
                        <td className={`px-2 py-1.5 text-end font-mono tabular-nums ${signedClass(row.quantityBaseMicro)}`}>
                          {formatQty(row.quantityBaseMicro)}
                        </td>
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">
                          {row.referenceType ?? "—"}
                        </td>
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">{row.createdBy}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => void fetch(Math.max(0, page - 1))}
                  disabled={loading || page === 0}
                  className="rounded-md bg-slate-200 px-3 py-1.5 text-xs font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-800 dark:text-slate-200"
                >
                  {t(locale, "rep.prev")}
                </button>
                <span className="font-mono text-xs font-bold tabular-nums">
                  {t(locale, "rep.page")} {page + 1}
                </span>
                <button
                  type="button"
                  onClick={() => void fetch(page + 1)}
                  disabled={loading || rows.length < Number(PAGE_SIZE)}
                  className="rounded-md bg-slate-200 px-3 py-1.5 text-xs font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-800 dark:text-slate-200"
                >
                  {t(locale, "rep.next")}
                </button>
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
}
