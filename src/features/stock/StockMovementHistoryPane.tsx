/**
 * Stock movement history pane (TASK-DSN-029): group pills + ledger table
 * over ipcStockMovementHistory rows. Backend movement_type filter stays
 * null (exact-code semantics); the IN/OUT/ADJUST/TRANSFER grouping is a
 * client-side partition via inMovementGroup.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import { formatQty } from "@/lib/money";
import type { Locale } from "@/types/pos";
import type { StockMovementRow } from "@/types/reports";
import {
  inMovementGroup,
  movementBadge,
  movementGroupKey,
  type MovementGroup,
} from "@/features/stock/stock-helpers";

const GROUPS: readonly MovementGroup[] = ["ALL", "IN", "OUT", "ADJUST", "TRANSFER"];

interface StockMovementHistoryPaneProps {
  locale: Locale;
  rows: StockMovementRow[];
  loading: boolean;
  group: MovementGroup;
  onGroup: (group: MovementGroup) => void;
  warehouseCodeOf: (warehouseId: string) => string;
}

function fmtQty(micro: string): string {
  try {
    return formatQty(micro);
  } catch {
    return "…";
  }
}

function fmtDate(iso: string): string {
  const time = Date.parse(iso);
  if (Number.isNaN(time)) {
    return iso;
  }
  const date = new Date(time);
  const pad = (part: number): string => String(part).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function StockMovementHistoryPane(props: StockMovementHistoryPaneProps): ReactElement {
  const { locale } = props;
  const visible = props.rows.filter((row) => inMovementGroup(row.movementType, props.group));
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden p-3">
      <div className="flex shrink-0 flex-wrap gap-1">
        {GROUPS.map((group) => (
          <button
            key={group}
            type="button"
            onClick={() => {
              props.onGroup(group);
            }}
            className={`rounded-full px-3 py-1 text-xs font-bold ${
              props.group === group
                ? "bg-primary-900 text-white dark:bg-primary-700"
                : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200"
            }`}
          >
            {t(locale, movementGroupKey(group))}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-950">
        {props.loading && (
          <p className="p-6 text-center text-xs font-bold text-slate-500">
            {t(locale, "stock.searching")}
          </p>
        )}
        {!props.loading && visible.length === 0 && (
          <p className="p-6 text-center text-xs font-bold text-slate-500">
            {t(locale, "stock.noMovements")}
          </p>
        )}
        {!props.loading && visible.length > 0 && (
          <table className="w-full border-collapse text-xs">
            <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
              <tr className="text-start text-slate-500">
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colDate")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colProduct")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colWarehouse")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colType")}</th>
                <th className="px-2 py-2 text-end font-bold">{t(locale, "stock.colQty")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colReason")}</th>
                <th className="px-2 py-2 text-start font-bold">{t(locale, "stock.colAuthor")}</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((row) => (
                <tr
                  key={row.id}
                  className="border-b border-slate-100 dark:border-slate-800"
                >
                  <td className="whitespace-nowrap px-2 py-2 text-slate-500">
                    <bdi dir="ltr">{fmtDate(row.createdAt)}</bdi>
                  </td>
                  <td className="max-w-40 truncate px-2 py-2 font-mono" title={row.productId}>
                    <bdi dir="ltr">{row.productId.slice(0, 8)}</bdi>
                  </td>
                  <td className="whitespace-nowrap px-2 py-2 text-slate-500">
                    <bdi dir="ltr">{props.warehouseCodeOf(row.warehouseId)}</bdi>
                  </td>
                  <td className="whitespace-nowrap px-2 py-2">
                    <span
                      className={`rounded-full px-2 py-0.5 text-[10px] font-black ${movementBadge(row.movementType)}`}
                    >
                      <bdi dir="ltr">{row.movementType}</bdi>
                    </span>
                  </td>
                  <td className="whitespace-nowrap px-2 py-2 text-end">
                    <bdi dir="ltr" className="font-mono font-bold tabular-nums">
                      {fmtQty(row.quantityBaseMicro)}
                    </bdi>
                  </td>
                  <td className="max-w-32 truncate px-2 py-2 text-slate-500" title={row.referenceId ?? ""}>
                    {row.referenceType ?? "—"}
                  </td>
                  <td className="max-w-32 truncate px-2 py-2 text-slate-500">
                    {row.createdBy.slice(0, 8)}
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
