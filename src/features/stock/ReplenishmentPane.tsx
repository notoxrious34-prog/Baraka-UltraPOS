import type { ReactElement } from "react";
import { Fragment, useState } from "react";

import { t } from "@/lib/i18n";
import { ipcUnitSalesMix } from "@/lib/ipc";
import { formatQty } from "@/lib/money";
import type { Locale } from "@/types/pos";
import type { ReplenishmentRow, UnitMixRow } from "@/types/reports";
import { formatBpPercent } from "@/features/stock/unit-mix-format";

interface ReplenishmentPaneProps {
  locale: Locale;
  token: string | null;
  rows: ReplenishmentRow[];
  loading: boolean;
  error: string | null;
}

function coverTone(coverDaysMicro: string): string {
  const value = BigInt(coverDaysMicro);
  if (value <= 0n) {
    return "bg-red-100 text-red-700 dark:bg-red-900/50 dark:text-red-200";
  }
  if (value < 70_000n) {
    return "bg-amber-100 text-amber-800 dark:bg-amber-900/50 dark:text-amber-200";
  }
  return "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/50 dark:text-emerald-200";
}

function UnitMixBlock(props: {
  locale: Locale;
  productId: string;
  mix: UnitMixRow[] | undefined;
  loading: boolean;
}): ReactElement {
  const { locale } = props;
  if (props.loading || props.mix === undefined) {
    return (
      <p className="py-1 text-xs font-bold text-slate-500">{t(locale, "stock.searching")}</p>
    );
  }
  if (props.mix.length === 0) {
    return <p className="py-1 text-xs font-bold text-slate-500">{t(locale, "mix.empty")}</p>;
  }
  const confidence = props.mix[0]?.confidenceBp ?? "0";
  return (
    <div className="flex flex-col gap-1">
      <p className="text-[11px] font-bold text-slate-500">
        {t(locale, "mix.title")} · {t(locale, "mix.confidence")}{" "}
        <bdi dir="ltr" className="font-mono tabular-nums">
          {formatBpPercent(confidence)}
        </bdi>
      </p>
      {props.mix.map((unit) => (
        <div key={unit.unitId} className="flex items-center gap-2">
          <span className="w-28 shrink-0 truncate text-xs font-bold">
            {locale === "ar-DZ" ? unit.unitNameAr : unit.unitNameFr}
          </span>
          <span className="flex h-3 min-w-0 flex-1 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700">
            <span
              className="h-full rounded-full bg-primary-600 dark:bg-primary-500"
              style={{ width: `${BigInt(unit.shareBp) / 100n}%` }}
            />
          </span>
          <bdi dir="ltr" className="w-16 shrink-0 text-end font-mono text-xs font-black tabular-nums">
            {formatBpPercent(unit.shareBp)}
          </bdi>
        </div>
      ))}
    </div>
  );
}

export function ReplenishmentPane(props: ReplenishmentPaneProps): ReactElement {
  const { locale } = props;
  const ruptures = props.rows.filter((row) => BigInt(row.coverDaysMicro) <= 0n).length;
  // Lazy per-product unit-mix (S4): fetched once per expansion, cached.
  const [expanded, setExpanded] = useState<string | null>(null);
  const [mixes, setMixes] = useState<Record<string, UnitMixRow[]>>({});
  const [mixLoading, setMixLoading] = useState<string | null>(null);
  const toggle = (productId: string): void => {
    if (expanded === productId) {
      setExpanded(null);
      return;
    }
    setExpanded(productId);
    if (mixes[productId] !== undefined || props.token === null) {
      return;
    }
    const token = props.token;
    setMixLoading(productId);
    ipcUnitSalesMix(token, productId)
      .then((mix) => {
        setMixes((prev) => ({ ...prev, [productId]: mix }));
        setMixLoading(null);
      })
      .catch(() => {
        setMixes((prev) => ({ ...prev, [productId]: [] }));
        setMixLoading(null);
      });
  };
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden p-3">
      <div className="flex shrink-0 flex-wrap gap-2">
        <HudCard label={t(locale, "radar.movers")} value={String(props.rows.length)} alert={false} />
        <HudCard label={t(locale, "radar.ruptures")} value={String(ruptures)} alert={ruptures > 0} />
      </div>
      <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-950">
        {props.loading && (
          <p className="p-6 text-center text-xs font-bold text-slate-500">
            {t(locale, "stock.searching")}
          </p>
        )}
        {props.error !== null && !props.loading && (
          <p className="p-6 text-center text-xs font-bold text-red-600">{props.error}</p>
        )}
        {props.error === null && !props.loading && props.rows.length === 0 && (
          <p className="p-6 text-center text-xs font-bold text-emerald-600 dark:text-emerald-400">
            {t(locale, "radar.empty")}
          </p>
        )}
        {props.error === null && !props.loading && props.rows.length > 0 && (
          <table className="w-full border-collapse text-xs">
            <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
              <tr className="text-start text-slate-500">
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colSku")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colProduct")}</th>
                <th className="px-2 py-2 text-end font-bold">{t(locale, "radar.colOnHand")}</th>
                <th className="px-2 py-2 text-end font-bold">{t(locale, "radar.colSold30")}</th>
                <th className="px-2 py-2 text-end font-bold">{t(locale, "radar.colCover")}</th>
              </tr>
            </thead>
            <tbody>
              {props.rows.map((row) => (
                <Fragment key={row.productId}>
                  <tr
                    key={row.productId}
                    className="border-b border-slate-100 dark:border-slate-800"
                  >
                    <td className="whitespace-nowrap px-2 py-2">
                      <button
                        type="button"
                        onClick={() => {
                          toggle(row.productId);
                        }}
                        aria-expanded={expanded === row.productId}
                        className="me-1 inline-block h-5 w-5 rounded bg-slate-200 text-xs font-black text-slate-700 dark:bg-slate-700 dark:text-slate-200"
                      >
                        {expanded === row.productId ? "−" : "+"}
                      </button>
                      <bdi dir="ltr" className="font-mono font-bold">
                        {row.sku}
                      </bdi>
                    </td>
                    <td className="max-w-44 truncate px-2 py-2">
                      {locale === "ar-DZ" ? row.nameAr : row.nameFr}
                    </td>
                    <td className="whitespace-nowrap px-2 py-2 text-end">
                      <bdi dir="ltr" className="font-mono font-bold tabular-nums">
                        {formatQty(row.onHandBaseMicro)}
                      </bdi>
                    </td>
                    <td className="whitespace-nowrap px-2 py-2 text-end">
                      <bdi dir="ltr" className="font-mono font-bold tabular-nums">
                        {formatQty(row.sold30dBaseMicro)}
                      </bdi>
                    </td>
                    <td className="whitespace-nowrap px-2 py-2 text-end">
                      <bdi
                        dir="ltr"
                        className={`rounded-full px-2 py-0.5 font-mono font-black tabular-nums ${coverTone(row.coverDaysMicro)}`}
                      >
                        {formatQty(row.coverDaysMicro)} {t(locale, "radar.days")}
                      </bdi>
                    </td>
                  </tr>
                  {expanded === row.productId && (
                    <tr key={`${row.productId}:mix`}>
                      <td colSpan={5} className="bg-slate-50 px-4 py-2 dark:bg-slate-900">
                        <UnitMixBlock
                          locale={locale}
                          productId={row.productId}
                          mix={mixes[row.productId]}
                          loading={mixLoading === row.productId}
                        />
                      </td>
                    </tr>
                  )}
                </Fragment>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function HudCard(props: { label: string; value: string; alert: boolean }): ReactElement {
  return (
    <div className="flex min-w-28 flex-col rounded-lg border border-slate-200 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-950">
      <span className="text-[11px] font-bold text-slate-500">{props.label}</span>
      <span
        className={`font-mono text-lg font-black tabular-nums ${props.alert ? "text-red-600 dark:text-red-400" : ""}`}
      >
        <bdi dir="ltr">{props.value}</bdi>
      </span>
    </div>
  );
}
