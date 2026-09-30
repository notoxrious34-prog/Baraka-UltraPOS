/**
 * Supplier invoice (FF) screen: invoice against an optional BR or
 * standalone. ADR-029: a BR-sourced FF never touches stock (guarded by the
 * backend branch); quantities must match the BR exactly. A standalone FF
 * carries the receipt movements itself, atomically with its single DEBIT.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcRecordSupplierInvoice } from "@/lib/ipc";
import {
  formatMoney,
  newCommandId,
  parseMoneyToCentimes,
} from "@/lib/money";
import type { PurchaseDocumentView } from "@/types/supplier";
import { toLineInputs, type PurchaseRow } from "@/lib/purchase-lines";
import { PurchaseLinesEditor } from "@/features/purchases/PurchaseOrderFormScreen";

interface Props {
  token: string;
  supplierId: string | null;
}

export function SupplierInvoiceFormScreen({ token, supplierId }: Props): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const warehouses = useSessionStore((state) => state.warehouses);
  const defaultWarehouse = useSessionStore((state) => state.warehouse);
  const loadWarehouses = useSessionStore((state) => state.loadWarehouses);
  const [rows, setRows] = useState<PurchaseRow[]>([]);
  const [pasted, setPasted] = useState("");
  const [brId, setBrId] = useState("");
  const [warehouseId, setWarehouseId] = useState("");
  const [totalText, setTotalText] = useState("");
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
    let totalCentimes = "0";
    try {
      totalCentimes = parseMoneyToCentimes(totalText);
    } catch {
      setError("INVALID_PAYMENT_AMOUNT");
      return;
    }
    if (BigInt(totalCentimes) <= 0n) {
      setError("INVALID_PAYMENT_AMOUNT");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const view = await ipcRecordSupplierInvoice(
        token,
        newCommandId(),
        brId.trim() === "" ? null : brId.trim(),
        resolvedSupplier,
        activeWarehouse,
        lines,
        totalCentimes,
      );
      setDone(view);
      setRows([]);
      setTotalText("");
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <h4 className="text-sm font-black">{t(locale, "pur.invoice")}</h4>
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
        value={brId}
        onChange={(event) => {
          setBrId(event.target.value);
        }}
        placeholder={t(locale, "pur.brId")}
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
      <label className="flex flex-col gap-1">
        <span className="text-xs font-bold text-slate-500">{t(locale, "pur.invoiceTotal")}</span>
        <input
          value={totalText}
          onChange={(event) => {
            setTotalText(event.target.value);
          }}
          inputMode="decimal"
          className="h-9 rounded-lg border border-slate-300 bg-white px-2 font-mono text-sm font-black outline-none dark:border-slate-700 dark:bg-slate-800"
        />
      </label>
      {error !== null && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(error, locale)}
        </p>
      )}
      {done !== null && (
        <p className="rounded-md bg-emerald-50 px-3 py-2 text-sm font-bold text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
          {t(locale, "pur.created")}: <span className="font-mono">{done.number}</span> ·{" "}
          {formatMoney(done.totalCentimes, locale)}
        </p>
      )}
      <button
        type="button"
        onClick={() => {
          void submit();
        }}
        disabled={busy || rows.length === 0 || totalText.trim() === ""}
        className="h-10 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
      >
        {busy ? t(locale, "pur.submitting") : t(locale, "pur.submit")}
      </button>
    </div>
  );
}
