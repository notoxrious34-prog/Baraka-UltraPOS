/**
 * Reports home dashboard (TASK-BRK-012B): nine analytical tiles routing to
 * their dedicated screens through local view state (no router dependency,
 * consistent with the App.tsx NavRail pattern).
 */

import { useState, type ReactElement } from "react";
import { t, type I18nKey } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { DailySalesReport } from "@/features/reports/DailySalesReport";
import { SalesByProductReport } from "@/features/reports/SalesByProductReport";
import { SalesByCategoryReport } from "@/features/reports/SalesByCategoryReport";
import { GrossMarginReport } from "@/features/reports/GrossMarginReport";
import { CreditOutstandingReport } from "@/features/reports/CreditOutstandingReport";
import { LowStockReport } from "@/features/reports/LowStockReport";
import { StockMovementReport } from "@/features/reports/StockMovementReport";
import { CashSessionsHistoryReport } from "@/features/reports/CashSessionsHistoryReport";
import { UserActivityReport } from "@/features/reports/UserActivityReport";

type ReportKey =
  | "daily"
  | "byProduct"
  | "byCategory"
  | "margin"
  | "credit"
  | "lowStock"
  | "movements"
  | "sessions"
  | "activity";

const TILES: Array<{ key: ReportKey; label: I18nKey }> = [
  { key: "daily", label: "rep.daily" },
  { key: "byProduct", label: "rep.byProduct" },
  { key: "byCategory", label: "rep.byCategory" },
  { key: "margin", label: "rep.margin" },
  { key: "credit", label: "rep.credit" },
  { key: "lowStock", label: "rep.lowStock" },
  { key: "movements", label: "rep.movements" },
  { key: "sessions", label: "rep.sessions" },
  { key: "activity", label: "rep.activity" },
];

export function ReportsHomeScreen(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const [active, setActive] = useState<ReportKey | null>(null);

  if (active !== null) {
    return (
      <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
        <button
          type="button"
          onClick={() => {
            setActive(null);
          }}
          className="w-fit rounded-md bg-slate-200 px-3 py-1.5 text-xs font-black hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
        >
          ← {t(locale, "rep.back")}
        </button>
        {active === "daily" && <DailySalesReport />}
        {active === "byProduct" && <SalesByProductReport />}
        {active === "byCategory" && <SalesByCategoryReport />}
        {active === "margin" && <GrossMarginReport />}
        {active === "credit" && <CreditOutstandingReport />}
        {active === "lowStock" && <LowStockReport />}
        {active === "movements" && <StockMovementReport />}
        {active === "sessions" && <CashSessionsHistoryReport />}
        {active === "activity" && <UserActivityReport />}
      </section>
    );
  }

  return (
    <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
      <div>
        <h2 className="text-lg font-black">{t(locale, "rep.title")}</h2>
        <p className="text-xs font-bold text-slate-500">{t(locale, "rep.subtitle")}</p>
      </div>
      <div className="grid grid-cols-2 gap-3 md:grid-cols-3">
        {TILES.map((tile) => (
          <button
            key={tile.key}
            type="button"
            onClick={() => {
              setActive(tile.key);
            }}
            className="rounded-xl border border-slate-200 bg-white p-4 text-start shadow-sm hover:border-primary-600 hover:shadow dark:border-slate-700 dark:bg-slate-900"
          >
            <span className="text-sm font-black">{t(locale, tile.label)}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
