/**
 * Customer ledger statement table (TASK-DSN-027): Relevé de compte with
 * localized DEBIT (sale) / CREDIT (سند قبض) rows and running balances.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { ipcGetStatement } from "@/lib/ipc";
import { formatMoney } from "@/lib/money";
import type { Locale, StatementEntry } from "@/types/pos";

interface CustomerStatementTableProps {
  locale: Locale;
  token: string;
  customerId: string;
}

function fmtAmount(centimes: string, locale: Locale): string {
  try {
    return formatMoney(centimes, locale);
  } catch {
    return "…";
  }
}

function fmtDate(iso: string, locale: Locale): string {
  const time = Date.parse(iso);
  if (Number.isNaN(time)) {
    return iso;
  }
  try {
    return new Date(time).toLocaleString(locale === "ar-DZ" ? "ar-DZ" : "fr-DZ", {
      dateStyle: "short",
      timeStyle: "short",
    });
  } catch {
    return iso;
  }
}

export function CustomerStatementTable(props: CustomerStatementTableProps): ReactElement {
  const { locale, token, customerId } = props;
  const [entries, setEntries] = useState<StatementEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    ipcGetStatement(token, customerId, null, null)
      .then((rows) => {
        if (!cancelled) {
          setEntries(rows);
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, customerId]);

  return (
    <section className="flex min-h-0 flex-col">
      <h3 className="shrink-0 text-sm font-black">{t(locale, "customers.statement")}</h3>
      {loading && (
        <p className="p-4 text-center text-xs font-bold text-slate-500">
          {t(locale, "customers.searching")}
        </p>
      )}
      {!loading && error !== null && (
        <p className="p-4 text-center text-xs font-bold text-red-600">
          {translateBackendError(error, locale)}
        </p>
      )}
      {!loading && error === null && entries.length === 0 && (
        <p className="p-4 text-center text-xs font-bold text-slate-500">
          {t(locale, "customers.emptyStatement")}
        </p>
      )}
      {!loading && error === null && entries.length > 0 && (
        <div className="min-h-0 overflow-x-auto">
          <table className="w-full border-collapse text-xs">
            <thead>
              <tr className="border-b border-slate-200 text-start text-slate-500 dark:border-slate-700">
                <th className="px-2 py-1.5 text-start font-bold">{t(locale, "customers.colDate")}</th>
                <th className="px-2 py-1.5 text-start font-bold">{t(locale, "customers.colOp")}</th>
                <th className="px-2 py-1.5 text-start font-bold">{t(locale, "customers.colRef")}</th>
                <th className="px-2 py-1.5 text-end font-bold">{t(locale, "customers.colDebit")}</th>
                <th className="px-2 py-1.5 text-end font-bold">{t(locale, "customers.colCredit")}</th>
                <th className="px-2 py-1.5 text-end font-bold">{t(locale, "customers.colBalance")}</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((entry, index) => {
                const isDebit = entry.entry_type === "DEBIT";
                return (
                  <tr
                    key={`${entry.created_at}:${index}`}
                    className="border-b border-slate-100 dark:border-slate-800"
                  >
                    <td className="whitespace-nowrap px-2 py-1.5 text-slate-500">
                      {fmtDate(entry.created_at, locale)}
                    </td>
                    <td className="px-2 py-1.5">
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-black ${
                          isDebit
                            ? "bg-sky-100 text-sky-800 dark:bg-sky-900/40 dark:text-sky-200"
                            : "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200"
                        }`}
                      >
                        {isDebit ? t(locale, "customers.opSale") : t(locale, "customers.opPayment")}
                      </span>
                    </td>
                    <td className="max-w-40 truncate px-2 py-1.5 text-slate-500">
                      {entry.reference_type ?? entry.reference_id ?? "—"}
                    </td>
                    <td className="px-2 py-1.5 text-end">
                      <bdi dir="ltr" className="font-mono tabular-nums">
                        {isDebit ? fmtAmount(entry.amount_centimes, locale) : "—"}
                      </bdi>
                    </td>
                    <td className="px-2 py-1.5 text-end">
                      <bdi dir="ltr" className="font-mono tabular-nums">
                        {isDebit ? "—" : fmtAmount(entry.amount_centimes, locale)}
                      </bdi>
                    </td>
                    <td className="px-2 py-1.5 text-end">
                      <bdi dir="ltr" className="font-mono font-bold tabular-nums">
                        {fmtAmount(entry.balance_after_centimes, locale)}
                      </bdi>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
