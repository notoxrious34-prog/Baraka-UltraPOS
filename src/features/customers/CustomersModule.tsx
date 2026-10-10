/**
 * Customers & Credit module root (TASK-DSN-027): desktop master-detail
 * layout binding the directory pane, dossier pane, and create/edit modal.
 * Manager-only with defense-in-depth role gating (same rule as App.tsx).
 * Balances come from per-customer detail enrichment (list rows carry no
 * balance); debtor/inactive pills filter on the enriched details.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcGetCustomer, ipcListCustomers, ipcSearchCustomers } from "@/lib/ipc";
import type { CustomerDetail, CustomerSummary } from "@/types/pos";
import { CustomerListPane, type DirectoryRow } from "@/features/customers/CustomerListPane";
import { CustomerDossierPane } from "@/features/customers/CustomerDossierPane";
import { CustomerFormModal, type CustomerFormMode } from "@/features/customers/CustomerFormModal";
import { filterCustomers, type CustomerFilter } from "@/features/customers/customer-helpers";

/** Same manager rule as App.tsx isManagerRole (ADMIN or GERANT). */
function isManagerRole(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

export function CustomersModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<CustomerFilter>("all");
  const [details, setDetails] = useState<CustomerDetail[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [formMode, setFormMode] = useState<CustomerFormMode | null>(null);
  const [reloadTick, setReloadTick] = useState(0);

  // Defense in depth: the screen verifies the role itself, independent of
  // NavRail visibility.
  const authorized = session !== null && isManagerRole(session.role);
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
        onNew={() => {
          setFormMode({ kind: "create" });
        }}
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
        />
      </div>
      {formMode !== null && (
        <CustomerFormModal
          locale={locale}
          token={token}
          role={session.role}
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
      </div>
    </div>
  );
}
