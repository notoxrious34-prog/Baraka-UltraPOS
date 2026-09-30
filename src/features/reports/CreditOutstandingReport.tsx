/**
 * Credit outstanding report: non-disabled customers holding a positive
 * balance, balance descending. Row drill-down reuses the TASK-BRK-009
 * customer-statement bridge (ipcGetStatement) in an inline panel.
 */

import { Fragment, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatMoney } from "@/lib/money";
import { ipcCreditOutstanding, ipcGetStatement } from "@/lib/ipc";
import type { CreditOutstandingRow } from "@/types/reports";
import type { StatementEntry } from "@/types/pos";
import { formatBasisPoints } from "@/features/reports/components/report-format";

export function CreditOutstandingReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [rows, setRows] = useState<CreditOutstandingRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [entries, setEntries] = useState<StatementEntry[] | null>(null);
  const [entriesLoading, setEntriesLoading] = useState(false);
  const [entriesError, setEntriesError] = useState<string | null>(null);

  const token = session?.token ?? null;
  if (token === null) {
    return <div />;
  }

  const fetch = async (): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      setRows(await ipcCreditOutstanding(token));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  const toggle = async (customerId: string): Promise<void> => {
    if (expanded === customerId) {
      setExpanded(null);
      setEntries(null);
      return;
    }
    setExpanded(customerId);
    setEntries(null);
    setEntriesError(null);
    setEntriesLoading(true);
    try {
      setEntries(await ipcGetStatement(token, customerId, null, null));
    } catch (failure: unknown) {
      setEntriesError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setEntriesLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <h3 className="text-base font-black">{t(locale, "rep.credit")}</h3>
        <button
          type="button"
          onClick={() => void fetch()}
          disabled={loading}
          className="rounded-md bg-primary-600 px-3 py-1.5 text-xs font-black text-white disabled:opacity-40"
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
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.customer")}</th>
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.phone")}</th>
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.policy")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.limit")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.balance")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.limitUsed")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.statement")}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <Fragment key={row.customerId}>
                      <tr className="border-t border-slate-200 dark:border-slate-700">
                        <td className="px-2 py-1.5 text-start">{row.name}</td>
                        <td className="px-2 py-1.5 text-start font-mono">{row.phone ?? "—"}</td>
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">{row.creditPolicy}</td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {row.creditLimitCentimes === null
                            ? t(locale, "rep.unlimited")
                            : formatMoney(row.creditLimitCentimes, locale)}
                        </td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {formatMoney(row.currentBalanceCentimes, locale)}
                        </td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {row.limitUsedBp === null ? "—" : formatBasisPoints(row.limitUsedBp)}
                        </td>
                        <td className="px-2 py-1.5 text-end">
                          <button
                            type="button"
                            onClick={() => void toggle(row.customerId)}
                            className="rounded-md bg-slate-200 px-2 py-0.5 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
                          >
                            {t(locale, "rep.statement")}
                          </button>
                        </td>
                      </tr>
                      {expanded === row.customerId && (
                        <tr key={`${row.customerId}-statement`} className="bg-slate-50 dark:bg-slate-800/50">
                          <td colSpan={7} className="px-2 py-2">
                            {entriesLoading && <p className="text-xs font-bold">{t(locale, "rep.loading")}</p>}
                            {entriesError !== null && (
                              <p className="text-xs font-bold text-red-700 dark:text-red-300">
                                {translateBackendError(entriesError, locale)}
                              </p>
                            )}
                            {entries !== null && (
                              <table className="w-full text-[11px] font-bold">
                                <tbody>
                                  {entries.map((entry, index) => (
                                    <tr key={`${entry.created_at}-${index}`} className="border-t border-slate-200 dark:border-slate-700">
                                      <td className="py-1 font-mono text-[10px]">{entry.created_at}</td>
                                      <td className="py-1">
                                        <span
                                          className={`rounded-full px-2 py-0.5 text-[10px] font-black ${
                                            entry.entry_type === "DEBIT"
                                              ? "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-200"
                                              : "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200"
                                          }`}
                                        >
                                          {entry.entry_type}
                                        </span>
                                      </td>
                                      <td className="py-1 text-end font-mono tabular-nums">
                                        {formatMoney(entry.amount_centimes, locale)}
                                      </td>
                                      <td className="py-1 text-end font-mono tabular-nums">
                                        {formatMoney(entry.balance_after_centimes, locale)}
                                      </td>
                                    </tr>
                                  ))}
                                </tbody>
                              </table>
                            )}
                          </td>
                        </tr>
                      )}
                    </Fragment>
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
