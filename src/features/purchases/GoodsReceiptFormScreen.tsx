/**
 * Goods receipt (BR) screen: receive against an optional BC or standalone.
 * ADR-029: the BR is the physical fact — committed immediately with its
 * PURCHASE_IN stock movements. Over-receipt needs the manager override.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcReceiveGoods } from "@/lib/ipc";
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
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<PurchaseDocumentView | null>(null);

  useEffect(() => {
    if (warehouses.length === 0) {
      void loadWarehouses();
    }
  }, [warehouses.length, loadWarehouses]);

  const resolvedSupplier = supplierId ?? pasted.trim();
  const activeWarehouse = warehouseId !== "" ? warehouseId : (defaultWarehouse?.id ?? "");

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
