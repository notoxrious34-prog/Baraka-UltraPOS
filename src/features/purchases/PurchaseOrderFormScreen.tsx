/**
 * BC creation screen (TASK-BRK-011): supplier (selected or pasted),
 * warehouse, and lines fed by the existing catalog search bridge
 * (ipcSearchCatalog — the POS discovery pattern reused, not reimplemented).
 * Submitting allocates a gapless DRAFT purchase order with zero side effects.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcCreatePurchaseOrder, ipcSearchCatalog } from "@/lib/ipc";
import { formatMoney, newCommandId } from "@/lib/money";
import type { CatalogProduct } from "@/types/pos";
import type { PurchaseDocumentView } from "@/types/supplier";
import {
  linesTotalCentimes,
  rowTotalCentimes,
  toLineInputs,
  type PurchaseRow,
} from "@/lib/purchase-lines";

/** Shared editable purchase-lines editor (BC/BR/FF forms reuse it). */
export function PurchaseLinesEditor({
  token,
  rows,
  onChange,
}: {
  token: string;
  rows: PurchaseRow[];
  onChange: (rows: PurchaseRow[]) => void;
}): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CatalogProduct[]>([]);

  const runProductSearch = async (): Promise<void> => {
    try {
      setResults(await ipcSearchCatalog(token, query, 12));
    } catch {
      setResults([]);
    }
  };

  const addRow = (product: CatalogProduct): void => {
    const unit =
      product.units.find((candidate) => candidate.is_default_selling) ?? product.units[0];
    if (unit === undefined) {
      return;
    }
    onChange([
      ...rows,
      {
        productId: product.product_id,
        unitId: unit.unit_id,
        label: `${product.sku} — ${locale === "ar-DZ" ? product.name_ar : product.name_fr}`,
        qtyText: "1",
        costText: "",
      },
    ]);
  };

  const patchRow = (index: number, patch: Partial<PurchaseRow>): void => {
    onChange(rows.map((row, i) => (i === index ? { ...row, ...patch } : row)));
  };

  const total = linesTotalCentimes(rows);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2">
        <input
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              void runProductSearch();
            }
          }}
          placeholder={t(locale, "pur.productSearch")}
          className="h-9 flex-1 rounded-lg border border-slate-300 bg-white px-2 text-xs font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
        />
        <button
          type="button"
          onClick={() => {
            void runProductSearch();
          }}
          className="h-9 rounded-lg bg-primary-600 px-3 text-xs font-black text-white"
        >
          {t(locale, "sup.search")}
        </button>
      </div>
      {results.length > 0 && (
        <div className="flex max-h-40 flex-col gap-1 overflow-y-auto rounded-lg border border-slate-200 p-1 dark:border-slate-700">
          {results.map((product) => (
            <button
              key={product.product_id}
              type="button"
              onClick={() => {
                addRow(product);
              }}
              className="rounded-md px-2 py-1 text-start text-xs font-bold hover:bg-slate-100 dark:hover:bg-slate-800"
            >
              {product.sku} — {locale === "ar-DZ" ? product.name_ar : product.name_fr}
            </button>
          ))}
        </div>
      )}
      {rows.map((row, index) => {
        const lineTotal = rowTotalCentimes(row);
        return (
          <div key={`${row.productId}-${row.unitId}-${index}`} className="flex items-center gap-2">
            <span className="w-52 truncate text-xs font-bold">{row.label}</span>
            <input
              value={row.qtyText}
              onChange={(event) => {
                patchRow(index, { qtyText: event.target.value });
              }}
              inputMode="decimal"
              className="h-8 w-20 rounded-md border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
              title={t(locale, "pur.qty")}
            />
            <input
              value={row.costText}
              onChange={(event) => {
                patchRow(index, { costText: event.target.value });
              }}
              inputMode="decimal"
              className="h-8 w-24 rounded-md border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
              title={t(locale, "pur.cost")}
            />
            <span className="w-28 text-end font-mono text-xs font-black tabular-nums">
              {lineTotal === null ? "—" : formatMoney(lineTotal, locale)}
            </span>
            <button
              type="button"
              onClick={() => {
                onChange(rows.filter((_, i) => i !== index));
              }}
              className="rounded-md bg-slate-100 px-2 py-0.5 text-xs font-bold text-red-700 dark:bg-slate-800 dark:text-red-300"
            >
              ✕
            </button>
          </div>
        );
      })}
      {rows.length > 0 && total !== null && (
        <p className="text-end font-mono text-sm font-black tabular-nums">
          {t(locale, "pur.total")}: {formatMoney(total, locale)}
        </p>
      )}
    </div>
  );
}

interface Props {
  token: string;
  supplierId: string | null;
}

export function PurchaseOrderFormScreen({ token, supplierId }: Props): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const warehouses = useSessionStore((state) => state.warehouses);
  const defaultWarehouse = useSessionStore((state) => state.warehouse);
  const loadWarehouses = useSessionStore((state) => state.loadWarehouses);
  const [rows, setRows] = useState<PurchaseRow[]>([]);
  const [pasted, setPasted] = useState("");
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
  const activeWarehouse =
    warehouseId !== "" ? warehouseId : (defaultWarehouse?.id ?? "");

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
      const view = await ipcCreatePurchaseOrder(
        token,
        newCommandId(),
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
      <h4 className="text-sm font-black">{t(locale, "pur.order")}</h4>
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
