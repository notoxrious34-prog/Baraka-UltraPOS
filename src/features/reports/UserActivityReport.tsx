/**
 * User-activity report: paginated audit-log viewer with optional operator
 * and action filters. Payloads render as sanitized, expandable JSON code
 * blocks — never decoded into business-specific UI per entity type.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcUserActivity } from "@/lib/ipc";
import type { UserActivityRow } from "@/types/reports";
import {
  ReportDateRangePicker,
  todayIso,
} from "@/features/reports/components/ReportDateRangePicker";

const PAGE_SIZE = "50";

function PayloadBlock({ label, json }: { label: string; json: string | null }): ReactElement {
  const [open, setOpen] = useState(false);
  if (json === null) {
    return <span className="font-mono text-[10px] text-slate-400">—</span>;
  }
  let pretty = json;
  try {
    pretty = JSON.stringify(JSON.parse(json) as unknown, null, 2);
  } catch {
    pretty = json;
  }
  return (
    <span>
      <button
        type="button"
        onClick={() => {
          setOpen((value) => !value);
        }}
        className="rounded-md bg-slate-100 px-2 py-0.5 font-mono text-[10px] font-bold dark:bg-slate-800 dark:text-slate-200"
      >
        {label} {open ? "▾" : "▸"}
      </button>
      {open && (
        <pre className="mt-1 max-h-40 overflow-auto rounded-md bg-slate-950 p-2 font-mono text-[10px] text-emerald-200">
          {pretty}
        </pre>
      )}
    </span>
  );
}

export function UserActivityReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [range, setRange] = useState(() => {
    const today = todayIso();
    return { dateFrom: today, dateTo: today };
  });
  const [userId, setUserId] = useState("");
  const [action, setAction] = useState("");
  const [page, setPage] = useState(0);
  const [rows, setRows] = useState<UserActivityRow[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const token = session?.token ?? null;
  if (token === null) {
    return <div />;
  }

  const fetch = async (offsetPages: number): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      const pageRows = await ipcUserActivity(token, {
        ...range,
        userId: userId === "" ? null : userId,
        action: action === "" ? null : action,
        limit: PAGE_SIZE,
        offset: String(offsetPages * Number(PAGE_SIZE)),
      });
      setRows(pageRows);
      setPage(offsetPages);
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-base font-black">{t(locale, "rep.activity")}</h3>
      <ReportDateRangePicker
        value={range}
        onChange={setRange}
        onApply={() => void fetch(0)}
        applying={loading}
      />
      <div className="flex flex-wrap items-center gap-2">
        <input
          value={userId}
          onChange={(event) => {
            setUserId(event.target.value);
          }}
          placeholder={t(locale, "rep.user")}
          className="h-9 rounded-lg border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
        />
        <input
          value={action}
          onChange={(event) => {
            setAction(event.target.value);
          }}
          placeholder={t(locale, "rep.action")}
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
            <>
              <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
                <table className="w-full text-xs font-bold">
                  <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
                    <tr>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.date")}</th>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.operator")}</th>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.action")}</th>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.entity")}</th>
                      <th className="px-2 py-1.5 text-start">{t(locale, "rep.payload")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((row) => (
                      <tr
                        key={row.sequenceNo}
                        className="border-t border-slate-200 align-top dark:border-slate-700"
                      >
                        <td className="px-2 py-1.5 text-start font-mono text-[10px] tabular-nums">
                          {row.timestamp}
                        </td>
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">{row.userId}</td>
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">{row.action}</td>
                        <td className="px-2 py-1.5 text-start font-mono text-[10px]">
                          {row.entityType}
                          <br />
                          <span className="text-slate-500">{row.entityId}</span>
                        </td>
                        <td className="px-2 py-1.5 text-start">
                          <PayloadBlock label="old" json={row.oldValueJson} />{" "}
                          <PayloadBlock label="new" json={row.newValueJson} />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => void fetch(Math.max(0, page - 1))}
                  disabled={loading || page === 0}
                  className="rounded-md bg-slate-200 px-3 py-1.5 text-xs font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-800 dark:text-slate-200"
                >
                  {t(locale, "rep.prev")}
                </button>
                <span className="font-mono text-xs font-bold tabular-nums">
                  {t(locale, "rep.page")} {page + 1}
                </span>
                <button
                  type="button"
                  onClick={() => void fetch(page + 1)}
                  disabled={loading || rows.length < Number(PAGE_SIZE)}
                  className="rounded-md bg-slate-200 px-3 py-1.5 text-xs font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-800 dark:text-slate-200"
                >
                  {t(locale, "rep.next")}
                </button>
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
}
