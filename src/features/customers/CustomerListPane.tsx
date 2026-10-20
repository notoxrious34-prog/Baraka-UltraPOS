/**
 * Customer directory master pane (TASK-DSN-027): search input, creation
 * entry point, Tous / Débiteurs / Inactifs pills, and the scrollable
 * customer list with balance-aware rows.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import type { Locale } from "@/types/pos";
import { useSessionStore } from "@/stores/session-store";
import { isDebtor, type CustomerFilter } from "@/features/customers/customer-helpers";

export interface DirectoryRow {
  id: string;
  name: string;
  phone: string | null;
  creditPolicy: "DISABLED" | "LIMITED" | "UNLIMITED";
  balanceCentimes: string | null;
  isActive: boolean;
}

interface CustomerListPaneProps {
  locale: Locale;
  query: string;
  onQuery: (query: string) => void;
  filter: CustomerFilter;
  onFilter: (filter: CustomerFilter) => void;
  rows: DirectoryRow[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
  loading: boolean;
}

function policyClass(policy: DirectoryRow["creditPolicy"]): string {
  if (policy === "UNLIMITED") {
    return "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200";
  }
  if (policy === "LIMITED") {
    return "bg-sky-100 text-sky-800 dark:bg-sky-900/40 dark:text-sky-200";
  }
  return "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200";
}

function fmtAmount(centimes: string, locale: Locale): string {
  try {
    return formatMoney(centimes, locale);
  } catch {
    return "…";
  }
}

export function CustomerListPane(props: CustomerListPaneProps): ReactElement {
  const { locale } = props;
  // Disabled-with-guidance (DSN-073): the entry point stays visible for
  // every role; sessions lacking CREATE_CUSTOMER get a disabled button
  // naming the missing habilitation instead of a silent hide.
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canCreateCustomer = hasPermission("CREATE_CUSTOMER");
  const pills: Array<{ id: CustomerFilter; label: string }> = [
    { id: "all", label: t(locale, "customers.filterAll") },
    { id: "debtors", label: t(locale, "customers.filterDebtors") },
    { id: "inactive", label: t(locale, "customers.filterInactive") },
  ];
  return (
    <section className="flex h-full w-[400px] min-h-0 flex-none flex-col border-e border-slate-200 dark:border-slate-800">
      <div className="flex shrink-0 flex-col gap-2 p-3">
        <input
          value={props.query}
          onChange={(event) => {
            props.onQuery(event.target.value);
          }}
          placeholder={t(locale, "customers.searchPlaceholder")}
          autoComplete="off"
          className="h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
        />
        <button
          type="button"
          onClick={props.onNew}
          disabled={!canCreateCustomer}
          title={canCreateCustomer ? undefined : t(locale, "customers.createCustomerReservedGuidance")}
          className="h-10 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-accent-400 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {t(locale, "customers.newCustomer")}
        </button>
        <div className="flex gap-1">
          {pills.map((pill) => (
            <button
              key={pill.id}
              type="button"
              onClick={() => {
                props.onFilter(pill.id);
              }}
              className={`rounded-full px-3 py-1 text-xs font-bold ${
                props.filter === pill.id
                  ? "bg-primary-900 text-white dark:bg-primary-700"
                  : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200"
              }`}
            >
              {pill.label}
            </button>
          ))}
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-3 pt-0">
        {props.loading && (
          <p className="p-4 text-center text-xs font-bold text-slate-500">
            {t(locale, "customers.searching")}
          </p>
        )}
        {!props.loading && props.rows.length === 0 && (
          <p className="p-4 text-center text-xs font-bold text-slate-500">
            {t(locale, "customers.noResults")}
          </p>
        )}
        <ul className="flex flex-col gap-1.5">
          {props.rows.map((row) => {
            const debtor = row.balanceCentimes !== null && isDebtor(row.balanceCentimes);
            const selected = row.id === props.selectedId;
            return (
              <li key={row.id}>
                <button
                  type="button"
                  onClick={() => {
                    props.onSelect(row.id);
                  }}
                  className={`flex w-full items-center gap-2 rounded-lg border p-2.5 text-start ${
                    selected
                      ? "border-accent-500 bg-accent-500/10"
                      : "border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800/60"
                  }`}
                >
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-extrabold">
                      {row.name}
                      {!row.isActive && <span className="text-slate-400"> · ✕</span>}
                    </p>
                    <p className="text-xs text-slate-500" dir="ltr">
                      <bdi>{row.phone ?? "-"}</bdi>
                    </p>
                  </div>
                  <div className="flex shrink-0 flex-col items-end gap-1">
                    <span
                      className={`rounded-full px-2 py-0.5 text-[10px] font-black ${policyClass(row.creditPolicy)}`}
                    >
                      {row.creditPolicy}
                    </span>
                    {row.balanceCentimes !== null && (
                      <bdi
                        dir="ltr"
                        className={`font-mono text-xs font-bold tabular-nums ${
                          debtor ? "text-amber-600 dark:text-amber-400" : "text-slate-500"
                        }`}
                      >
                        {fmtAmount(row.balanceCentimes, locale)}
                      </bdi>
                    )}
                  </div>
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    </section>
  );
}
