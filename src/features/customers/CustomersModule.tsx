/**
 * Customers & Credit module root (TASK-DSN-027, realigned TASK-DSN-077):
 * desktop master-detail layout binding the directory pane, dossier pane,
 * and create/edit modal. View access follows the VIEW_CUSTOMERS permission
 * (all authenticated store roles); inner mutating actions stay guarded by
 * CREATE_CUSTOMER, RECORD_CREDIT_PAYMENT, and MANAGE_CUSTOMER_CREDIT
 * (TASK-DSN-073). Balances come from per-customer detail enrichment
 * (list rows carry no balance); debtor/inactive pills filter on the
 * enriched details.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import type { Locale } from "@/types/pos";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcGetCustomer, ipcListCustomers, ipcSearchCustomers } from "@/lib/ipc";
import { ipcDeleteCustomer, type DeleteEntityResult } from "@/lib/ipc";
import { SafeDeleteModal } from "@/components/common/SafeDeleteModal";
import type { CustomerDetail, CustomerSummary } from "@/types/pos";
import { CustomerListPane, type DirectoryRow } from "@/features/customers/CustomerListPane";
import { CustomerDossierPane } from "@/features/customers/CustomerDossierPane";
import { CustomerFormModal, type CustomerFormMode } from "@/features/customers/CustomerFormModal";
import { filterCustomers, type CustomerFilter } from "@/features/customers/customer-helpers";

/** Non-zero canonical balance (either sign) blocks deletion (ADR-092). Pure. */
export function hasOpenBalance(balanceCentimes: string): boolean {
  try {
    return BigInt(balanceCentimes) !== 0n;
  } catch {
    return true;
  }
}

/** Best-effort balance copy for the blocked-deletion card (never throws). */
export function blockedBalanceReason(balanceCentimes: string, locale: Locale): string {
  let amount = balanceCentimes;
  try {
    amount = formatMoney(balanceCentimes, locale);
  } catch {
    // Fall through with the raw canonical string.
  }
  return `${t(locale, "del.blockedBalance")} (${amount})`;
}

export function CustomersModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<CustomerFilter>("all");
  const [details, setDetails] = useState<CustomerDetail[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [formMode, setFormMode] = useState<CustomerFormMode | null>(null);
  const [reloadTick, setReloadTick] = useState(0);
  const [deleteTarget, setDeleteTarget] = useState<CustomerDetail | null>(null);
  /** Inline confirmation after a safe deletion (mode-aware copy). */
  const [deleteNote, setDeleteNote] = useState<string | null>(null);

  // Permission gate (TASK-DSN-077): view access follows VIEW_CUSTOMERS so
  // Caissier and Vendeur reach the directory; mutating actions stay
  // permission-guarded inside the panes (TASK-DSN-073). Rust use cases
  // remain the final authority.
  const authorized = session !== null && hasPermission("VIEW_CUSTOMERS");
  const token = authorized && session !== null ? session.token : null;

  useEffect(() => {
    if (token === null) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    setListError(null);
    const trimmed = query.trim();
    const listed =
      trimmed === ""
        ? ipcListCustomers(token)
        : ipcSearchCustomers(token, trimmed, 50).then((rows: CustomerSummary[]) => rows);
    listed
      .then(async (rows) => {
        const capped = rows.slice(0, 50);
        const enriched = await Promise.all(
          capped.map((row) => ipcGetCustomer(token, row.id)),
        );
        if (!cancelled) {
          setDetails(enriched);
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setListError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
          setDetails([]);
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, query, reloadTick]);

  if (!authorized || token === null) {
    return (
      <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError("UNAUTHORIZED", locale)}
        </p>
      </section>
    );
  }

  const visible = filterCustomers(details, filter);
  const rows: DirectoryRow[] = visible.map((detail) => ({
    id: detail.id,
    name: detail.name,
    phone: detail.phone,
    creditPolicy: detail.credit_policy,
    balanceCentimes: detail.current_balance_centimes,
    isActive: detail.is_active,
  }));
  const selected = details.find((detail) => detail.id === selectedId) ?? null;

  const handleDeleted = (result: DeleteEntityResult): void => {
    setDeleteTarget(null);
    setSelectedId(null);
    setDeleteNote(t(locale, result.mode === "hard_deleted" ? "del.deleted" : "del.archived"));
    window.setTimeout(() => {
      setDeleteNote(null);
    }, 8000);
    setReloadTick((tick) => tick + 1);
  };

  const blocked = deleteTarget !== null && hasOpenBalance(deleteTarget.current_balance_centimes);

  // Single create-customer entry point (TASK-DSN-054): the directory "new"
  // button and the empty-selection teaching panel share this handler.
  const openCreate = (): void => {
    setFormMode({ kind: "create" });
  };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="flex h-10 shrink-0 items-center px-4">
        <h1 className="text-sm font-black">{t(locale, "customers.title")}</h1>
      </div>
      <div className="flex min-h-0 flex-1 overflow-hidden">
      <CustomerListPane
        locale={locale}
        query={query}
        onQuery={setQuery}
        filter={filter}
        onFilter={setFilter}
        rows={rows}
        selectedId={selectedId}
        onSelect={setSelectedId}
        onNew={openCreate}
        loading={loading}
      />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {listError !== null && (
          <p className="shrink-0 px-4 pt-3 text-xs font-bold text-red-600">
            {translateBackendError(listError, locale)}
          </p>
        )}
        <CustomerDossierPane
          locale={locale}
          token={token}
          detail={selected}
          onEdit={() => {
            if (selected !== null) {
              setFormMode({ kind: "edit", detail: selected });
            }
          }}
          onDelete={() => {
            if (selected !== null) {
              setDeleteTarget(selected);
            }
          }}
          onNew={openCreate}
        />
        {deleteNote !== null && (
          <p
            role="status"
            className="shrink-0 px-4 pt-2 text-xs font-black text-emerald-700 dark:text-emerald-300"
          >
            {deleteNote}
          </p>
        )}
      </div>
      {formMode !== null && (
        <CustomerFormModal
          locale={locale}
          token={token}
          mode={formMode}
          onClose={() => {
            setFormMode(null);
          }}
          onSaved={(detail) => {
            setFormMode(null);
            setSelectedId(detail.id);
            setReloadTick((tick) => tick + 1);
          }}
        />
      )}
      {deleteTarget !== null && (
        <SafeDeleteModal
          isOpen
          titleKey="del.customerTitle"
          entityName={deleteTarget.name}
          entityCode={deleteTarget.phone ?? undefined}
          isBlocked={blocked}
          blockedReason={
            blocked ? blockedBalanceReason(deleteTarget.current_balance_centimes, locale) : null
          }
          onClose={() => {
            setDeleteTarget(null);
          }}
          onConfirm={(reason: string) => ipcDeleteCustomer(token, deleteTarget.id, reason)}
          onSuccess={handleDeleted}
        />
      )}
      </div>
    </div>
  );
}
