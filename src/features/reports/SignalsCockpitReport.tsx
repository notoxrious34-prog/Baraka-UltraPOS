/**
 * Intelligence cockpit (TASK-DSN-110, Wave 5): five signals in one
 * read-only screen — broken size runs, dead stock, demand velocity,
 * model margins, credit aging. FR/AR through `rep.signals.*`, dark-ready
 * through theme tokens, no fixed positioning. The container bridges the
 * stores to the pure `SignalsCockpitView` (node-testable through
 * `renderToStaticMarkup`, mirroring the OperationsMenu pattern).
 */

import { useCallback, useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { formatMoney, formatQty } from "@/lib/money";
import { ipcSignals } from "@/lib/ipc";
import type {
  BrokenRun,
  DeadStockLine,
  ModelMarginLine,
  SignalAging,
  SignalsFeed,
  SignalVelocity,
} from "@/types/reports";

type WindowDays = 7 | 30 | 90;

const WINDOWS: WindowDays[] = [7, 30, 90];

/** Basis points (integer string) -> percent with two decimals, BigInt only. */
export function formatMarginBp(bps: string): string {
  const negative = bps.startsWith("-");
  const abs = BigInt(negative ? bps.slice(1) : bps);
  const whole = (abs / 100n).toString();
  const frac = (abs % 100n).toString().padStart(2, "0");
  return `${negative ? "-" : ""}${whole}.${frac} %`;
}

export interface SignalsCockpitViewProps {
  locale: "fr-DZ" | "ar-DZ";
  feed: SignalsFeed | null;
  loading: boolean;
  error: string | null;
  windowDays: WindowDays;
  onWindowChange: (window: WindowDays) => void;
  onRetry: () => void;
}

function errorBanner(locale: "fr-DZ" | "ar-DZ", error: string): ReactElement {
  return (
    <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
      {translateBackendError(error, locale)}
    </p>
  );
}

function sectionTitle(locale: "fr-DZ" | "ar-DZ", key: I18nKey, count: number): ReactElement {
  return (
    <h4 className="text-sm font-black">
      {t(locale, key)} <span className="text-ink-muted">({count})</span>
    </h4>
  );
}

function BrokenRunsSection(props: { locale: "fr-DZ" | "ar-DZ"; runs: BrokenRun[] }): ReactElement {
  const { locale, runs } = props;
  if (runs.length === 0) {
    return <p className="text-sm font-bold text-slate-500">{t(locale, "rep.signalsNone")}</p>;
  }
  return (
    <ul className="flex flex-col gap-2">
      {runs.map((run) => (
        <li
          key={run.modelId}
          className="rounded-lg border border-amber-300 bg-amber-50 px-3 py-2 dark:border-amber-700 dark:bg-amber-900/20"
        >
          <p className="text-xs font-black text-amber-800 dark:text-amber-200">
            {run.modelId} — {run.cells.length} {t(locale, "rep.signalsBrokenCells")}
          </p>
          <ul className="mt-1 flex flex-wrap gap-1">
            {run.cells.map((cell) => (
              <li
                key={`${cell.axis1Value}/${cell.axis2Value}`}
                className="rounded-md bg-amber-100 px-2 py-0.5 text-xs font-bold text-amber-900 dark:bg-amber-900/40 dark:text-amber-100"
              >
                {cell.axis1Value} / {cell.axis2Value} · +{cell.siblingUnitsSold}
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  );
}

function DeadStockSection(props: {
  locale: "fr-DZ" | "ar-DZ";
  lines: DeadStockLine[];
}): ReactElement {
  const { locale, lines } = props;
  if (lines.length === 0) {
    return <p className="text-sm font-bold text-slate-500">{t(locale, "rep.signalsNone")}</p>;
  }
  return (
    <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
      <table className="w-full text-xs font-bold">
        <thead className="bg-slate-100 dark:bg-slate-800">
          <tr>
            <th className="px-2 py-1.5 text-start">{t(locale, "rep.product")}</th>
            <th className="px-2 py-1.5 text-end">{t(locale, "rep.qtyOnHand")}</th>
            <th className="px-2 py-1.5 text-end">{t(locale, "rep.signalsIdleDays")}</th>
          </tr>
        </thead>
        <tbody>
          {lines.map((line) => (
            <tr key={line.productId} className="border-t border-slate-100 dark:border-slate-800">
              <td className="px-2 py-1.5">{line.productId}</td>
              <td className="px-2 py-1.5 text-end">{formatQty(line.qtyOnHandMicro)}</td>
              <td className="px-2 py-1.5 text-end">{line.daysSinceLastMovement}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function VelocitySection(props: {
  locale: "fr-DZ" | "ar-DZ";
  velocity: SignalVelocity | null;
}): ReactElement {
  const { locale, velocity } = props;
  if (velocity === null) {
    return <p className="text-sm font-bold text-slate-500">{t(locale, "rep.signalsNone")}</p>;
  }
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs font-bold">
      <span className="rounded-md bg-slate-100 px-2 py-1 dark:bg-slate-800">
        {velocity.unitsPerDay} {t(locale, "rep.signalsUnitsDay")}
      </span>
      {velocity.isSlow && (
        <span className="rounded-md bg-red-100 px-2 py-1 text-red-800 dark:bg-red-900/30 dark:text-red-200">
          {t(locale, "rep.signalsSlow")}
        </span>
      )}
      {velocity.trendBps !== null && (
        <span className="rounded-md bg-slate-100 px-2 py-1 dark:bg-slate-800">
          {t(locale, "rep.signalsTrend")}: {formatMarginBp(velocity.trendBps)}
        </span>
      )}
    </div>
  );
}

function MarginsSection(props: {
  locale: "fr-DZ" | "ar-DZ";
  margins: ModelMarginLine[];
}): ReactElement {
  const { locale, margins } = props;
  if (margins.length === 0) {
    return <p className="text-sm font-bold text-slate-500">{t(locale, "rep.signalsNone")}</p>;
  }
  return (
    <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
      <table className="w-full text-xs font-bold">
        <thead className="bg-slate-100 dark:bg-slate-800">
          <tr>
            <th className="px-2 py-1.5 text-start">{t(locale, "rep.signalsModel")}</th>
            <th className="px-2 py-1.5 text-end">{t(locale, "rep.totalHt")}</th>
            <th className="px-2 py-1.5 text-end">{t(locale, "rep.cost")}</th>
            <th className="px-2 py-1.5 text-end">{t(locale, "rep.signalsMargin")}</th>
          </tr>
        </thead>
        <tbody>
          {margins.map((margin) => (
            <tr key={margin.modelId} className="border-t border-slate-100 dark:border-slate-800">
              <td className="px-2 py-1.5">{margin.modelId}</td>
              <td className="px-2 py-1.5 text-end">
                {formatMoney(margin.revenueCentimes, locale)}
              </td>
              <td className="px-2 py-1.5 text-end">{formatMoney(margin.cogsCentimes, locale)}</td>
              <td className="px-2 py-1.5 text-end">
                {margin.marginBps === null ? "—" : formatMarginBp(margin.marginBps)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function AgingSection(props: { locale: "fr-DZ" | "ar-DZ"; aging: SignalAging }): ReactElement {
  const { locale, aging } = props;
  const buckets: Array<{ label: I18nKey; value: string }> = [
    { label: "rep.signalsAge030", value: aging.bucket030Centimes },
    { label: "rep.signalsAge3160", value: aging.bucket3160Centimes },
    { label: "rep.signalsAge6190", value: aging.bucket6190Centimes },
    { label: "rep.signalsAge90Plus", value: aging.bucket90PlusCentimes },
  ];
  return (
    <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
      {buckets.map((bucket) => (
        <div
          key={bucket.label}
          className="rounded-lg border border-slate-200 px-3 py-2 dark:border-slate-700"
        >
          <p className="text-[11px] font-bold text-ink-muted">{t(locale, bucket.label)}</p>
          <p className="text-sm font-black">{formatMoney(bucket.value, locale)}</p>
        </div>
      ))}
    </div>
  );
}

export function SignalsCockpitView(props: SignalsCockpitViewProps): ReactElement {
  const { locale, feed, loading, error, windowDays, onWindowChange, onRetry } = props;
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        <h3 className="text-base font-black">{t(locale, "rep.signals")}</h3>
        {feed !== null && (
          <span className="rounded-md bg-slate-100 px-2 py-0.5 text-[11px] font-bold text-ink-muted dark:bg-slate-800">
            {feed.profile}
          </span>
        )}
        <div className="ms-auto flex items-center gap-1" role="group" aria-label={t(locale, "rep.signalsWindow")}>
          {WINDOWS.map((days) => (
            <button
              key={days}
              type="button"
              onClick={() => {
                onWindowChange(days);
              }}
              disabled={loading}
              aria-pressed={windowDays === days}
              className={
                windowDays === days
                  ? "h-8 rounded-lg bg-primary-600 px-3 text-xs font-black text-white disabled:opacity-40"
                  : "h-8 rounded-lg bg-slate-200 px-3 text-xs font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-800 dark:text-slate-200"
              }
            >
              {days} {t(locale, "rep.signalsDays")}
            </button>
          ))}
        </div>
      </div>
      {loading && <p className="text-xs font-bold">{t(locale, "rep.loading")}</p>}
      {error !== null && !loading && errorBanner(locale, error)}
      {feed !== null && !loading && error === null && (
        <>
          <section className="flex flex-col gap-2">
            {sectionTitle(locale, "rep.signalsBrokenRuns", feed.brokenRuns.length)}
            <BrokenRunsSection locale={locale} runs={feed.brokenRuns} />
          </section>
          <section className="flex flex-col gap-2">
            {sectionTitle(locale, "rep.signalsDeadStock", feed.deadStock.length)}
            <DeadStockSection locale={locale} lines={feed.deadStock} />
          </section>
          <section className="flex flex-col gap-2">
            {sectionTitle(locale, "rep.signalsVelocity", 1)}
            <VelocitySection locale={locale} velocity={feed.velocity} />
          </section>
          <section className="flex flex-col gap-2">
            {sectionTitle(locale, "rep.signalsMargins", feed.modelMargins.length)}
            <MarginsSection locale={locale} margins={feed.modelMargins} />
          </section>
          <section className="flex flex-col gap-2">
            {sectionTitle(locale, "rep.signalsAging", 4)}
            <AgingSection locale={locale} aging={feed.aging} />
          </section>
        </>
      )}
      {feed === null && !loading && error !== null && (
        <button
          type="button"
          onClick={onRetry}
          className="w-fit rounded-md bg-slate-200 px-3 py-1.5 text-xs font-black hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
        >
          {t(locale, "sup.retry")}
        </button>
      )}
    </div>
  );
}

export function SignalsCockpitReport(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [windowDays, setWindowDays] = useState<WindowDays>(30);
  const [feed, setFeed] = useState<SignalsFeed | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const token = session?.token ?? null;

  const fetch = useCallback(async (): Promise<void> => {
    if (token === null) {
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setFeed(await ipcSignals(token, windowDays));
    } catch (failure: unknown) {
      setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
    } finally {
      setLoading(false);
    }
  }, [token, windowDays]);

  if (token === null) {
    return <div />;
  }

  return (
    <SignalsCockpitView
      locale={locale}
      feed={feed}
      loading={loading}
      error={error}
      windowDays={windowDays}
      onWindowChange={(days) => {
        setWindowDays(days);
        setFeed(null);
      }}
      onRetry={() => void fetch()}
    />
  );
}
