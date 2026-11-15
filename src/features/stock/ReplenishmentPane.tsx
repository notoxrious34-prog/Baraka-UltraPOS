import type { ReactElement } from "react";

import { t } from "@/lib/i18n";
import { formatQty } from "@/lib/money";
import type { Locale } from "@/types/pos";
import type { ReplenishmentRow } from "@/types/reports";

interface ReplenishmentPaneProps {
  locale: Locale;
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

export function ReplenishmentPane(props: ReplenishmentPaneProps): ReactElement {
  const { locale } = props;
  const ruptures = props.rows.filter((row) => BigInt(row.coverDaysMicro) <= 0n).length;
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
                <tr
                  key={row.productId}
                  className="border-b border-slate-100 dark:border-slate-800"
                >
                  <td className="whitespace-nowrap px-2 py-2 font-mono font-bold">
                    <bdi dir="ltr">{row.sku}</bdi>
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
