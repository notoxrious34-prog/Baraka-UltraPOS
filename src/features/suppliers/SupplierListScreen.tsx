/**
 * Suppliers module root (TASK-BRK-011): search directory, create/edit,
 * statement, and entry points into the purchase workflow (BC/BR/FF/payment).
 * Manager-only; mounted via the non-POS NavRail view switch in App.tsx.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useSupplierStore } from "@/stores/supplier-store";
import { formatMoney } from "@/lib/money";
import type { SupplierView } from "@/types/supplier";
import { SupplierFormModal } from "@/features/suppliers/SupplierFormModal";
import { SupplierStatementView } from "@/features/suppliers/SupplierStatementView";
import { PurchaseOrderFormScreen } from "@/features/purchases/PurchaseOrderFormScreen";
import { GoodsReceiptFormScreen } from "@/features/purchases/GoodsReceiptFormScreen";
import { SupplierInvoiceFormScreen } from "@/features/purchases/SupplierInvoiceFormScreen";
import { RecordSupplierPaymentModal } from "@/features/purchases/RecordSupplierPaymentModal";

type Panel = "none" | "order" | "receive" | "invoice";
type Overlay = "none" | "create" | "edit" | "statement" | "payment";

export function SuppliersModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const suppliers = useSupplierStore((state) => state.suppliers);
  const selected = useSupplierStore((state) => state.selected);
  const loading = useSupplierStore((state) => state.loading);
  const storeError = useSupplierStore((state) => state.error);
  const search = useSupplierStore((state) => state.search);
  const select = useSupplierStore((state) => state.select);
  const [query, setQuery] = useState("");
  const [panel, setPanel] = useState<Panel>("none");
  const [overlay, setOverlay] = useState<Overlay>("none");
  const [editing, setEditing] = useState<SupplierView | null>(null);

  const token = session?.token ?? null;

  // Instant debounced search: every keystroke refreshes results ~150 ms
  // after the user stops typing. Enter/button submit stays immediate.
  // Guarded on token so the pre-login render issues no IPC call.
  useEffect(() => {
    if (token === null) {
      return;
    }
    const timer = setTimeout(() => {
      void search(token, query);
    }, 150);
    return () => {
      clearTimeout(timer);
    };
  }, [query, search, token]);

  if (token === null) {
    return <div />;
  }

  const runSearch = (): void => {
    void search(token, query);
  };

  const toolButton = (
    key: I18nKey,
    onClick: () => void,
    enabled = true,
  ): ReactElement => (
    <button
      type="button"
      onClick={onClick}
      disabled={!enabled}
      className="rounded-md bg-slate-200 px-3 py-1.5 text-xs font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-800 dark:text-slate-200 dark:hover:bg-slate-700"
    >
      {t(locale, key)}
    </button>
  );

  return (
    <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
      <h2 className="text-lg font-black">{t(locale, "sup.title")}</h2>

      <div className="flex gap-2">
        <input
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              runSearch();
            }
          }}
          placeholder={t(locale, "sup.searchPlaceholder")}
          className="h-10 flex-1 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
        />
        <button
          type="button"
          onClick={runSearch}
          className="h-10 rounded-lg bg-primary-600 px-4 text-sm font-black text-white"
        >
          {t(locale, "sup.search")}
        </button>
        {toolButton("sup.new", () => {
          setOverlay("create");
        })}
      </div>

      <div className="flex flex-wrap gap-2">
        {toolButton("pur.order", () => {
          setPanel("order");
        })}
        {toolButton("pur.receive", () => {
          setPanel("receive");
        })}
        {toolButton("pur.invoice", () => {
          setPanel("invoice");
        })}
        {toolButton(
          "pur.payment",
          () => {
            setOverlay("payment");
          },
          selected !== null,
        )}
        {toolButton(
          "sup.statement",
          () => {
            setOverlay("statement");
          },
          selected !== null,
        )}
      </div>

      {loading && <p className="text-xs font-bold">{t(locale, "cash.loading")}</p>}
      {storeError !== null && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(storeError, locale)}
        </p>
      )}

      {suppliers.length === 0 && !loading && (
        <p className="text-sm font-bold text-slate-500">{t(locale, "sup.noResults")}</p>
      )}
      {suppliers.length > 0 && (
        <table className="w-full text-start text-xs font-bold">
          <thead>
            <tr className="text-start">
              <th className="py-1 text-start">{t(locale, "sup.name")}</th>
              <th className="py-1 text-start">{t(locale, "sup.phone")}</th>
              <th className="py-1 text-start">{t(locale, "sup.balance")}</th>
              <th className="py-1 text-start">{t(locale, "sup.active")}</th>
              <th className="py-1 text-end">—</th>
            </tr>
          </thead>
          <tbody>
            {suppliers.map((supplier) => (
              <tr
                key={supplier.id}
                className={`border-t border-slate-200 dark:border-slate-700 ${
                  selected?.id === supplier.id ? "bg-accent-500/10" : ""
                }`}
              >
                <td className="py-1.5 text-start">{supplier.name}</td>
                <td className="py-1.5 text-start font-mono">{supplier.phone ?? "—"}</td>
                <td className="py-1.5 text-start font-mono tabular-nums">
                  {formatMoney(supplier.currentBalanceCentimes, locale)}
                </td>
                <td className="py-1.5">
                  <span
                    className={`rounded-full px-2 py-0.5 text-[10px] font-black ${
                      supplier.isActive
                        ? "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200"
                        : "bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300"
                    }`}
                  >
                    {t(locale, supplier.isActive ? "sup.active" : "sup.inactive")}
                  </span>
                </td>
                <td className="flex justify-end gap-1 py-1.5">
                  <button
                    type="button"
                    onClick={() => {
                      select(supplier);
                    }}
                    className="rounded-md bg-slate-100 px-2 py-0.5 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-200"
                  >
                    {t(locale, "common.select")}
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setEditing(supplier);
                      setOverlay("edit");
                    }}
                    className="rounded-md bg-slate-100 px-2 py-0.5 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-200"
                  >
                    {t(locale, "sup.edit")}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {panel !== "none" && (
        <div className="rounded-lg border border-slate-200 bg-white p-3 dark:border-slate-700 dark:bg-slate-900">
          <div className="mb-2 flex items-center justify-between">
            <h3 className="text-sm font-black">{t(locale, "pur.title")}</h3>
            <button
              type="button"
              onClick={() => {
                setPanel("none");
              }}
              className="rounded-md bg-slate-100 px-2 py-0.5 text-xs font-bold dark:bg-slate-800 dark:text-slate-200"
            >
              {t(locale, "sup.close")}
            </button>
          </div>
          {panel === "order" && <PurchaseOrderFormScreen token={token} supplierId={selected?.id ?? null} />}
          {panel === "receive" && <GoodsReceiptFormScreen token={token} supplierId={selected?.id ?? null} />}
          {panel === "invoice" && <SupplierInvoiceFormScreen token={token} supplierId={selected?.id ?? null} />}
        </div>
      )}

      {(overlay === "create" || overlay === "edit") && (
        <SupplierFormModal
          token={token}
          initial={overlay === "edit" ? editing : null}
          onClose={() => {
            setOverlay("none");
            setEditing(null);
          }}
          onSaved={() => {
            setOverlay("none");
            setEditing(null);
          }}
        />
      )}

      {overlay === "statement" && selected !== null && (
        <SupplierStatementView
          token={token}
          supplierId={selected.id}
          onClose={() => {
            setOverlay("none");
          }}
        />
      )}

      {overlay === "payment" && (
        <RecordSupplierPaymentModal
          token={token}
          supplier={selected}
          onClose={() => {
            setOverlay("none");
          }}
        />
      )}
    </section>
  );
}
