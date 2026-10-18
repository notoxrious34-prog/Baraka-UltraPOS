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
import { ipcDeleteSupplier, type DeleteEntityResult } from "@/lib/ipc";
import { SafeDeleteModal } from "@/components/common/SafeDeleteModal";
import { SupplierStatementView } from "@/features/suppliers/SupplierStatementView";
import { PurchaseOrderFormScreen } from "@/features/purchases/PurchaseOrderFormScreen";
import { GoodsReceiptFormScreen } from "@/features/purchases/GoodsReceiptFormScreen";
import { SupplierInvoiceFormScreen } from "@/features/purchases/SupplierInvoiceFormScreen";
import { RecordSupplierPaymentModal } from "@/features/purchases/RecordSupplierPaymentModal";

type Panel = "none" | "order" | "receive" | "invoice";
type Overlay = "none" | "create" | "edit" | "statement" | "payment" | "delete";

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
  const [deleting, setDeleting] = useState<SupplierView | null>(null);
  /** Inline confirmation after a safe deletion (mode-aware copy). */
  const [deleteNote, setDeleteNote] = useState<string | null>(null);

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

  const handleDeleted = (result: DeleteEntityResult): void => {
    setDeleting(null);
    setOverlay("none");
    if (selected?.id === result.id) {
      select(null);
    }
    setDeleteNote(t(locale, result.mode === "hard_deleted" ? "del.deleted" : "del.archived"));
    window.setTimeout(() => {
      setDeleteNote(null);
    }, 8000);
    runSearch();
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
        <div className="flex flex-col items-center gap-2 rounded-md bg-red-50 px-3 py-6 text-center dark:bg-red-900/30">
          <svg
            width="28"
            height="28"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            className="text-red-400 dark:text-red-300"
          >
            <path d="M3 21h18" />
            <path d="M5 21V7l7-4 7 4v14" />
            <path d="M9 21v-4h6v4" />
          </svg>
          <p className="text-sm font-bold text-red-700 dark:text-red-200">
            {t(locale, "sup.loadError")}
          </p>
          <button
            type="button"
            onClick={runSearch}
            className="h-10 rounded-lg bg-red-700 px-4 text-sm font-bold text-white hover:bg-red-600 dark:bg-red-600 dark:hover:bg-red-500"
          >
            {t(locale, "sup.retry")}
          </button>
          <p className="font-mono text-[11px] text-red-500 dark:text-red-300">
            {translateBackendError(storeError, locale)}
          </p>
        </div>
      )}

      {storeError === null && suppliers.length === 0 && !loading && query.trim() !== "" && (
        <div className="flex flex-col items-center gap-2 px-3 py-6 text-center">
          <svg
            width="28"
            height="28"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            className="text-line-strong"
          >
            <path d="M3 21h18" />
            <path d="M5 21V7l7-4 7 4v14" />
            <path d="M9 21v-4h6v4" />
          </svg>
          <p className="text-sm font-bold text-ink-secondary">
            {t(locale, "sup.noMatchLead")}
            {locale === "ar-DZ" ? " «" : " « "}
            <bdi dir={/[A-Za-z0-9]/.test(query) ? "ltr" : undefined} className="tabular-nums">{query.trim()}</bdi>
            {locale === "ar-DZ" ? "»." : " »."}
          </p>
          <button
            type="button"
            onClick={() => {
              setQuery("");
            }}
            className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-bold text-slate-800 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100 dark:hover:bg-slate-600"
          >
            {t(locale, "sup.clearSearch")}
          </button>
        </div>
      )}
      {/* Empty query with zero rows: zero-database first-run teaching state
          (TASK-DSN-054-C2). The store lists the directory on empty query, so
          reaching here means the database holds no suppliers at all. */}
      {storeError === null && suppliers.length === 0 && !loading && query.trim() === "" && (
        <div className="flex flex-col items-center gap-2 px-3 py-6 text-center">
          <svg
            width="36"
            height="36"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            className="text-line-strong"
          >
            <path d="M3 21h18" />
            <path d="M5 21V7l7-4 7 4v14" />
            <path d="M9 21v-4h6v4" />
          </svg>
          <h2 className="text-base font-bold text-ink-primary">
            {t(locale, "sup.emptyTitle")}
          </h2>
          <p className="max-w-md text-sm font-bold text-ink-secondary">
            {t(locale, "sup.emptyBody")}
          </p>
          <button
            type="button"
            onClick={() => {
              setOverlay("create");
            }}
            className="h-10 rounded-lg bg-accent-500 px-4 text-sm font-bold text-slate-950 hover:bg-accent-400"
          >
            {t(locale, "sup.emptyCta")}
          </button>
        </div>
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
                  <bdi dir="ltr">{formatMoney(supplier.currentBalanceCentimes, locale)}</bdi>
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
                  <button
                    type="button"
                    onClick={() => {
                      setDeleting(supplier);
                      setOverlay("delete");
                    }}
                    aria-label={t(locale, "del.delete")}
                    title={t(locale, "del.delete")}
                    className="rounded-md bg-red-50 px-2 py-0.5 font-black text-red-700 hover:bg-red-100 dark:bg-red-900/30 dark:text-red-200 dark:hover:bg-red-900/50"
                  >
                    {t(locale, "del.delete")}
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
      {overlay === "delete" && deleting !== null && (
        <SafeDeleteModal
          isOpen
          titleKey="del.supplierTitle"
          entityName={deleting.name}
          entityCode={deleting.phone ?? undefined}
          onClose={() => {
            setDeleting(null);
            setOverlay("none");
          }}
          onConfirm={() => ipcDeleteSupplier(token, deleting.id)}
          onSuccess={handleDeleted}
        />
      )}
      {deleteNote !== null && (
        <p role="status" className="text-xs font-black text-emerald-700 dark:text-emerald-300">
          {deleteNote}
        </p>
      )}
    </section>
  );
}
