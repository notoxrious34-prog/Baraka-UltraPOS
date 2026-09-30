/**
 * Low-stock report (ADR-037): stocked pairs at or below zero on hand,
 * most depleted first. Optional warehouse filter; no date range (a point-
 * in-time depletion signal, not a ranged aggregate).
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatQty } from "@/lib/money";
import { ipcLowStock } from "@/lib/ipc";
import type { LowStockRow } from "@/types/reports";

export function LowStockReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const warehouses = useSessionStore((state) => state.warehouses);
  const [warehouseId, setWarehouseId] = useState<string>("");
  const [rows, setRows] = useState<LowStockRow[] | null>(null);
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
      setRows(await ipcLowStock(token, warehouseId === "" ? null : warehouseId));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-base font-black">{t(locale, "rep.lowStock")}</h3>
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
        <button
          type="button"
          onClick={() => void fetch()}
          disabled={loading}
          className="h-9 rounded-lg bg-primary-600 px-4 text-xs font-black text-white disabled:opacity-40"
        >
          {t(locale, "rep.apply")}
        </button>
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
            <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
              <table className="w-full text-xs font-bold">
                <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
                  <tr>
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.product")}</th>
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.sku")}</th>
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.warehouse")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.qtyOnHand")}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr
                      key={`${row.warehouseId}-${row.productId}`}
                      className="border-t border-slate-200 dark:border-slate-700"
                    >
                      <td className="px-2 py-1.5 text-start">{row.nameFr}</td>
                      <td className="px-2 py-1.5 text-start font-mono">{row.sku}</td>
                      <td className="px-2 py-1.5 text-start font-mono text-[11px]">
                        {row.warehouseCode}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums text-red-700 dark:text-red-300">
                        {formatQty(row.quantityBaseMicro)}
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
