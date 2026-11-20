/**
 * Goods receipt (BR) screen: receive against an optional BC or standalone.
 * ADR-029: the BR is the physical fact — committed immediately with its
 * PURCHASE_IN stock movements. Over-receipt needs the manager override.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcPreviewReceiveDeviations, ipcReceiveGoods, type ReceiveDeviation } from "@/lib/ipc";
import { formatMoney, newCommandId } from "@/lib/money";
import type { PurchaseDocumentView } from "@/types/supplier";
import { toLineInputs, type PurchaseRow } from "@/lib/purchase-lines";
import { PurchaseLinesEditor } from "@/features/purchases/PurchaseOrderFormScreen";

interface Props {
  token: string;
  supplierId: string | null;
}

export function GoodsReceiptFormScreen({ token, supplierId }: Props): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const warehouses = useSessionStore((state) => state.warehouses);
  const defaultWarehouse = useSessionStore((state) => state.warehouse);
  const loadWarehouses = useSessionStore((state) => state.loadWarehouses);
  const [rows, setRows] = useState<PurchaseRow[]>([]);
  const [pasted, setPasted] = useState("");
  const [bcId, setBcId] = useState("");
  const [warehouseId, setWarehouseId] = useState("");
  const [busy, setBusy] = useState(false);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deviations, setDeviations] = useState<ReceiveDeviation[] | null>(null);
  const [done, setDone] = useState<PurchaseDocumentView | null>(null);

  useEffect(() => {
    if (warehouses.length === 0) {
      void loadWarehouses();
    }
  }, [warehouses.length, loadWarehouses]);

  const resolvedSupplier = supplierId ?? pasted.trim();
  const activeWarehouse = warehouseId !== "" ? warehouseId : (defaultWarehouse?.id ?? "");

  const preview = async (): Promise<void> => {
    if (previewBusy) {
      return;
    }
    const lines = toLineInputs(rows);
    if (lines === null) {
      setError("PURCHASE_LINE_QUANTITY_INVALID");
      return;
    }
    setPreviewBusy(true);
    setError(null);
    try {
      // Advisory only: threshold 1000 bps = 10% over last + 90-day average.
      setDeviations(await ipcPreviewReceiveDeviations(token, lines, 1000));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setPreviewBusy(false);
    }
  };

  const flagged = deviations === null ? [] : deviations.filter((line) => line.flagged);

  const submit = async (): Promise<void> => {
    if (busy) {
      return;
    }
    if (resolvedSupplier === "") {
      setError("SUPPLIER_NOT_FOUND");
      return;
    }
    if (activeWarehouse === "") {
      setError("WAREHOUSE_NOT_FOUND");
      return;
    }
    const lines = toLineInputs(rows);
    if (lines === null) {
      setError("PURCHASE_LINE_QUANTITY_INVALID");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const view = await ipcReceiveGoods(
        token,
        newCommandId(),
        bcId.trim() === "" ? null : bcId.trim(),
        resolvedSupplier,
        activeWarehouse,
        lines,
      );
      setDone(view);
      setRows([]);
      setDeviations(null);
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <h4 className="text-sm font-black">{t(locale, "pur.receive")}</h4>
      {supplierId === null && (
        <input
          value={pasted}
          onChange={(event) => {
            setPasted(event.target.value);
          }}
          placeholder={`${t(locale, "sup.idLabel")} — ${t(locale, "pur.optionalId")}`}
          className="h-9 rounded-lg border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
        />
      )}
      <input
        value={bcId}
        onChange={(event) => {
          setBcId(event.target.value);
        }}
        placeholder={t(locale, "pur.bcId")}
        className="h-9 rounded-lg border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
      />
      <select
        value={activeWarehouse}
        onChange={(event) => {
          setWarehouseId(event.target.value);
        }}
        className="h-9 rounded-lg border border-slate-300 bg-white px-2 text-xs font-bold dark:border-slate-700 dark:bg-slate-800"
      >
        <option value="">{t(locale, "pur.warehouse")}</option>
        {warehouses.map((warehouse) => (
          <option key={warehouse.id} value={warehouse.id}>
            {warehouse.code} — {warehouse.name}
          </option>
        ))}
      </select>
      <PurchaseLinesEditor token={token} rows={rows} onChange={setRows} />
      {error !== null && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(error, locale)}
        </p>
      )}
      {done !== null && (
        <p className="rounded-md bg-emerald-50 px-3 py-2 text-sm font-bold text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
          {t(locale, "pur.created")}: <span className="font-mono">{done.number}</span> ·{" "}
          {done.status} · {formatMoney(done.totalCentimes, locale)}
        </p>
      )}
      <button
        type="button"
        onClick={() => {
          void preview();
        }}
        disabled={previewBusy || rows.length === 0}
        className="h-9 rounded-lg border border-amber-500 text-xs font-black text-amber-700 hover:bg-amber-50 disabled:opacity-40 dark:text-amber-300 dark:hover:bg-amber-900/20"
      >
        {previewBusy ? t(locale, "pur.submitting") : t(locale, "pur.checkCosts")}
      </button>
      {deviations !== null && flagged.length === 0 && (
        <p className="rounded-md bg-emerald-50 px-3 py-2 text-sm font-bold text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
          {t(locale, "pur.costClear")}
        </p>
      )}
      {flagged.length > 0 && (
        <div className="rounded-md bg-amber-50 px-3 py-2 dark:bg-amber-900/30">
          <p className="text-sm font-black text-amber-800 dark:text-amber-200">
            {t(locale, "pur.costFlag")} ({flagged.length})
          </p>
          {flagged.map((line) => (
            <p
              key={line.productId}
              className="font-mono text-xs font-bold text-amber-700 dark:text-amber-300"
            >
              {line.productId.slice(0, 8)} · +{line.deltaVsLastBps ?? 0} bps
              {line.lastCostCentimes !== null && ` · ${formatMoney(line.lastCostCentimes, locale)}`}
            </p>
          ))}
        </div>
      )}
      <button
        type="button"
        onClick={() => {
          void submit();
        }}
        disabled={busy || rows.length === 0}
        className="h-10 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
      >
        {busy ? t(locale, "pur.submitting") : t(locale, "pur.submit")}
      </button>
    </div>
  );
}
