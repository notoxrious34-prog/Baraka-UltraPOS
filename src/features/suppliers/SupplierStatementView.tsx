/**
 * Supplier statement view: append-only ledger rows with the server-side
 * running balance, plus the cached balance for reconciliation display.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { formatMoney } from "@/lib/money";
import { ipcGetSupplierStatement } from "@/lib/ipc";
import type { SupplierLedgerEntry } from "@/types/supplier";

interface Props {
  token: string;
  supplierId: string;
  onClose: () => void;
}

export function SupplierStatementView({ token, supplierId, onClose }: Props): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [entries, setEntries] = useState<SupplierLedgerEntry[] | null>(null);
  const [cached, setCached] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    ipcGetSupplierStatement(token, supplierId)
      .then((statement) => {
        if (!cancelled) {
          setEntries(statement.entries);
          setCached(statement.currentBalanceCentimes);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, supplierId]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4">
      <div className="flex max-h-full w-full max-w-2xl flex-col gap-3 overflow-hidden rounded-xl bg-white p-4 dark:bg-slate-900">
        <div className="flex items-center justify-between">
          <h3 className="text-base font-black">{t(locale, "sup.statement")}</h3>
          <button
            type="button"
            onClick={onClose}
            className="rounded-md bg-slate-100 px-2 py-0.5 text-xs font-bold dark:bg-slate-800 dark:text-slate-200"
          >
            {t(locale, "sup.close")}
          </button>
        </div>
        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {translateBackendError(error, locale)}
          </p>
        )}
        {entries === null && error === null && (
          <p className="text-sm font-bold">{t(locale, "cash.loading")}</p>
        )}
        {entries !== null && (
          <div className="min-h-0 flex-1 overflow-y-auto">
            <table className="w-full text-xs font-bold">
              <thead>
                <tr>
                  <th className="py-1 text-start">{t(locale, "sup.statement")}</th>
                  <th className="py-1 text-start">{t(locale, "pur.amount")}</th>
                  <th className="py-1 text-start">{t(locale, "sup.balance")}</th>
                </tr>
              </thead>
              <tbody>
                {entries.map((entry) => (
                  <tr key={entry.id} className="border-t border-slate-200 dark:border-slate-700">
                    <td className="py-1.5 text-start">
                      <span className="font-mono text-[10px]">{entry.createdAt}</span>
                      <br />
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-black ${
                          entry.entryType === "DEBIT"
                            ? "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-200"
                            : "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200"
                        }`}
                      >
                        {entry.entryType} · {entry.referenceType}
                      </span>
                      {entry.description !== null && (
                        <span className="ms-2 text-slate-500">{entry.description}</span>
                      )}
                    </td>
                    <td className="py-1.5 text-start font-mono tabular-nums">
                      {formatMoney(entry.amountCentimes, locale)}
                    </td>
                    <td className="py-1.5 text-start font-mono tabular-nums">
                      {formatMoney(entry.balanceAfterCentimes, locale)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {cached !== null && (
              <p className="mt-2 text-sm font-black">
                {t(locale, "sup.balance")}: {formatMoney(cached, locale)}
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
