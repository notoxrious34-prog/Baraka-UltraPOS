/**
 * Reusable report date-range filter (TASK-BRK-012B).
 *
 * ISO YYYY-MM-DD inputs with quick presets (Today / Last 7 Days / This
 * Month). `dateFrom <= dateTo` is enforced locally before any IPC leaves
 * the screen; violations render the bilingual inline banner instead.
 */

import { useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import type { ReportDateRange } from "@/types/reports";

function toIsoDay(date: Date): string {
  const pad = (value: number): string => value.toString().padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function shiftDays(base: Date, delta: number): Date {
  const next = new Date(base);
  next.setDate(next.getDate() + delta);
  return next;
}

/** Today as an ISO day string (local calendar). */
export function todayIso(): string {
  return toIsoDay(new Date());
}

interface Props {
  value: ReportDateRange;
  onChange: (range: ReportDateRange) => void;
  onApply: () => void;
  applying: boolean;
}

export function ReportDateRangePicker({ value, onChange, onApply, applying }: Props): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [localError, setLocalError] = useState<string | null>(null);

  const apply = (): void => {
    if (value.dateFrom > value.dateTo) {
      setLocalError(t(locale, "rep.invalidRange"));
      return;
    }
    setLocalError(null);
    onApply();
  };

  const preset = (from: string, to: string): void => {
    setLocalError(null);
    onChange({ dateFrom: from, dateTo: to });
  };

  const today = todayIso();
  const inputClass =
    "h-9 rounded-lg border border-slate-300 bg-white px-2 font-mono text-xs font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-1 text-xs font-bold">
          {t(locale, "rep.from")}
          <input
            type="date"
            value={value.dateFrom}
            max={value.dateTo}
            onChange={(event) => {
              onChange({ ...value, dateFrom: event.target.value });
            }}
            className={inputClass}
          />
        </label>
        <label className="flex items-center gap-1 text-xs font-bold">
          {t(locale, "rep.to")}
          <input
            type="date"
            value={value.dateTo}
            min={value.dateFrom}
            onChange={(event) => {
              onChange({ ...value, dateTo: event.target.value });
            }}
            className={inputClass}
          />
        </label>
        <button
          type="button"
          onClick={() => {
            const day = todayIso();
            preset(day, day);
          }}
          className="rounded-md bg-slate-200 px-2 py-1.5 text-xs font-bold hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
        >
          {t(locale, "rep.today")}
        </button>
        <button
          type="button"
          onClick={() => {
            preset(toIsoDay(shiftDays(new Date(), -6)), today);
          }}
          className="rounded-md bg-slate-200 px-2 py-1.5 text-xs font-bold hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
        >
          {t(locale, "rep.last7")}
        </button>
        <button
          type="button"
          onClick={() => {
            const now = new Date();
            preset(toIsoDay(new Date(now.getFullYear(), now.getMonth(), 1)), today);
          }}
          className="rounded-md bg-slate-200 px-2 py-1.5 text-xs font-bold hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
        >
          {t(locale, "rep.thisMonth")}
        </button>
        <button
          type="button"
          onClick={apply}
          disabled={applying}
          className="h-9 rounded-lg bg-primary-600 px-4 text-xs font-black text-white disabled:opacity-40"
        >
          {t(locale, "rep.apply")}
        </button>
      </div>
      {localError !== null && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {localError}
        </p>
      )}
    </div>
  );
}
