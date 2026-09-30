/**
 * Cash-session history: every session in range with frozen/actual/variance
 * figures. Row drill-down reuses the TASK-BRK-010 Z-report bridge
 * (ipcCashGetSessionReport) in a snapshot modal.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatMoney } from "@/lib/money";
import { ipcCashGetSessionReport, ipcCashSessionsHistory } from "@/lib/ipc";
import type { CashSessionHistoryRow } from "@/types/reports";
import type { CashSessionReport } from "@/types/cash";
import {
  ReportDateRangePicker,
  todayIso,
} from "@/features/reports/components/ReportDateRangePicker";

function nullableMoney(value: string | null, locale: "ar-DZ" | "fr-DZ"): string {
  if (value === null) {
    return "—";
  }
  return formatMoney(value, locale);
}

export function CashSessionsHistoryReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [range, setRange] = useState(() => {
    const today = todayIso();
    return { dateFrom: today, dateTo: today };
  });
  const [terminalId, setTerminalId] = useState("");
  const [rows, setRows] = useState<CashSessionHistoryRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [detail, setDetail] = useState<CashSessionReport | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);

  const token = session?.token ?? null;
  if (token === null) {
    return <div />;
  }

  const fetch = async (): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      setRows(await ipcCashSessionsHistory(token, range, terminalId === "" ? null : terminalId));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  const openDetail = async (sessionId: string): Promise<void> => {
    setDetail(null);
    setDetailError(null);
    setDetailLoading(true);
    try {
      setDetail(await ipcCashGetSessionReport(token, sessionId));
    } catch (failure: unknown) {
      setDetailError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setDetailLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-base font-black">{t(locale, "rep.sessions")}</h3>
      <ReportDateRangePicker value={range} onChange={setRange} onApply={() => void fetch()} applying={loading} />
      <div className="flex flex-wrap items-center gap-2">
        <input
          value={terminalId}
          onChange={(event) => {
            setTerminalId(event.target.value);
          }}
          placeholder={t(locale, "rep.terminal")}
          className="h-9 rounded-lg border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
        />
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
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.openedAt")}</th>
                    <th className="px-2 py-1.5 text-start">{t(locale, "rep.status")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.openingFloat")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.expected")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.counted")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.variance")}</th>
                    <th className="px-2 py-1.5 text-end">{t(locale, "rep.details")}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr key={row.id} className="border-t border-slate-200 dark:border-slate-700">
                      <td className="px-2 py-1.5 text-start font-mono text-[10px] tabular-nums">
                        {row.openedAt}
                      </td>
                      <td className="px-2 py-1.5 text-start font-mono text-[10px]">{row.status}</td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {formatMoney(row.openingFloatCentimes, locale)}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {nullableMoney(row.systemExpectedCentimes, locale)}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {nullableMoney(row.actualCountedCentimes, locale)}
                      </td>
                      <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                        {nullableMoney(row.varianceCentimes, locale)}
                      </td>
                      <td className="px-2 py-1.5 text-end">
                        <button
                          type="button"
                          onClick={() => void openDetail(row.id)}
                          className="rounded-md bg-slate-200 px-2 py-0.5 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
                        >
                          {t(locale, "rep.details")}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
      {(detailLoading || detailError !== null || detail !== null) && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4">
          <div className="flex max-h-full w-full max-w-2xl flex-col gap-3 overflow-hidden rounded-xl bg-white p-4 dark:bg-slate-900">
            <div className="flex items-center justify-between">
              <h3 className="text-base font-black">{t(locale, "rep.details")}</h3>
              <button
                type="button"
                onClick={() => {
                  setDetail(null);
                  setDetailError(null);
                }}
                className="rounded-md bg-slate-100 px-2 py-0.5 text-xs font-bold dark:bg-slate-800 dark:text-slate-200"
              >
                {t(locale, "sup.close")}
              </button>
            </div>
            {detailLoading && <p className="text-sm font-bold">{t(locale, "rep.loading")}</p>}
            {detailError !== null && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {translateBackendError(detailError, locale)}
              </p>
            )}
            {detail !== null && (
              <div className="min-h-0 flex-1 overflow-y-auto">
                <div className="grid grid-cols-2 gap-2 text-xs font-bold">
                  <p>
                    {t(locale, "rep.status")}:{" "}
                    <span className="font-mono">{detail.session.status}</span>
                  </p>
                  <p>
                    {t(locale, "rep.terminal")}:{" "}
                    <span className="font-mono">{detail.session.terminalId}</span>
                  </p>
                  <p>
                    {t(locale, "rep.openingFloat")}:{" "}
                    <span className="font-mono tabular-nums">
                      {formatMoney(detail.session.openingFloatCentimes, locale)}
                    </span>
                  </p>
                  <p>
                    {t(locale, "rep.expected")}:{" "}
                    <span className="font-mono tabular-nums">
                      {nullableMoney(detail.session.systemExpectedCentimes, locale)}
                    </span>
                  </p>
                  <p>
                    {t(locale, "rep.counted")}:{" "}
                    <span className="font-mono tabular-nums">
                      {nullableMoney(detail.session.actualCountedCentimes, locale)}
                    </span>
                  </p>
                  <p>
                    {t(locale, "rep.variance")}:{" "}
                    <span className="font-mono tabular-nums">
                      {nullableMoney(detail.session.varianceCentimes, locale)}
                    </span>
                  </p>
                </div>
                <table className="mt-3 w-full text-xs font-bold">
                  <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
                    <tr>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.type")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.totalTtc")}</th>
                      <th className="px-2 py-1.5 text-end">{t(locale, "rep.lines")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {detail.totalsByType.map((total) => (
                      <tr
                        key={total.movementType}
                        className="border-t border-slate-200 dark:border-slate-700"
                      >
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">
                          {total.movementType}
                        </td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">
                          {formatMoney(total.totalCentimes, locale)}
                        </td>
                        <td className="px-2 py-1.5 text-end font-mono tabular-nums">{total.count}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
