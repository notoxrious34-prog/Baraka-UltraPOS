/**
 * Scrollable cart table: keyboard navigation, inline unit cycling,
 * quantity stepping, line deletion. Selected row carries the ring.
 */

import { useEffect, useRef, type ReactElement } from "react";
import { previewLineTotal, useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import { addBig, formatMoney, formatQty, subBig } from "@/lib/money";

const STEP_MICRO = "10000";

export function CartTable(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const lines = useCartStore((state) => state.lines);
  const selectedLineIndex = usePosUiStore((state) => state.selectedLineIndex);
  const setSelectedLineIndex = usePosUiStore((state) => state.setSelectedLineIndex);
  const removeLine = useCartStore((state) => state.removeLine);
  const cycleLineUnit = useCartStore((state) => state.cycleLineUnit);
  const setLineQty = useCartStore((state) => state.setLineQty);
  const rowRefs = useRef<Array<HTMLTableRowElement | null>>([]);

  useEffect(() => {
    const row = rowRefs.current[selectedLineIndex];
    if (row !== undefined && row !== null) {
      row.scrollIntoView({ block: "nearest" });
    }
  }, [selectedLineIndex, lines.length]);

  if (lines.length === 0) {
    return (
      <p className="p-6 text-center text-sm font-semibold text-slate-400">
        {t(locale, "cart.empty")}
      </p>
    );
  }

  return (
    <table className="w-full border-collapse text-sm">
      <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
        <tr className="text-start text-[11px] uppercase tracking-wide text-slate-500">
          <th className="px-1 py-2 font-bold">{t(locale, "cart.colIndex")}</th>
          <th className="px-1 py-2 font-bold">{t(locale, "cart.colDesignation")}</th>
          <th className="px-1 py-2 font-bold">{t(locale, "cart.colUnit")}</th>
          <th className="px-1 py-2 text-end font-bold">{t(locale, "cart.colPrice")}</th>
          <th className="px-1 py-2 text-center font-bold">{t(locale, "cart.colQty")}</th>
          <th className="px-1 py-2 text-end font-bold">{t(locale, "cart.colTotal")}</th>
          <th className="px-1 py-2 font-bold">{t(locale, "cart.colAction")}</th>
        </tr>
      </thead>
      <tbody>
        {lines.map((line, index) => {
          const unit = line.units[line.unitIndex];
          const selected = index === selectedLineIndex;
          return (
            <tr
              key={line.key}
              ref={(element) => {
                rowRefs.current[index] = element;
              }}
              onClick={() => {
                setSelectedLineIndex(index);
              }}
              className={`cursor-pointer border-t border-slate-100 dark:border-slate-800 ${
                selected ? "bg-sky-50 ring-2 ring-inset ring-primary-600 dark:bg-sky-950/30" : ""
              }`}
            >
              <td className="px-1 py-2 font-mono text-xs text-slate-500">{index + 1}</td>
              <td className="max-w-40 px-1 py-2">
                <p className="truncate font-bold text-slate-900 dark:text-slate-100">
                  {locale === "ar-DZ" ? line.nameAr : line.nameFr}
                </p>
                <p className="font-mono text-[11px] text-slate-500">{line.sku}</p>
              </td>
              <td className="px-1 py-2">
                <button
                  type="button"
                  title="F3"
                  onClick={(event) => {
                    event.stopPropagation();
                    cycleLineUnit(line.key);
                  }}
                  className="rounded-md bg-slate-200 px-2 py-1 text-xs font-bold text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200"
                >
                  {unit !== undefined
                    ? locale === "ar-DZ"
                      ? unit.nameAr
                      : unit.nameFr
                    : "—"}
                </button>
              </td>
              <td className="px-1 py-2 text-end font-mono text-xs font-bold">
                {unit !== undefined ? formatMoney(unit.salePriceCentimes, locale) : "—"}
              </td>
              <td className="px-1 py-2">
                <span className="flex items-center justify-center gap-1">
                  <button
                    type="button"
                    aria-label="-"
                    onClick={(event) => {
                      event.stopPropagation();
                      try {
                        const next = subBig(line.quantityMicro, STEP_MICRO);
                        setLineQty(line.key, next.startsWith("-") ? "0" : next);
                      } catch {
                        setLineQty(line.key, "0");
                      }
                    }}
                    className="h-7 w-7 rounded-md bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700"
                  >
                    −
                  </button>
                  <span className="min-w-12 text-center font-mono text-xs font-black">
                    {formatQty(line.quantityMicro)}
                  </span>
                  <button
                    type="button"
                    aria-label="+"
                    onClick={(event) => {
                      event.stopPropagation();
                      try {
                        setLineQty(line.key, addBig(line.quantityMicro, STEP_MICRO));
                      } catch {
                        setLineQty(line.key, STEP_MICRO);
                      }
                    }}
                    className="h-7 w-7 rounded-md bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700"
                  >
                    +
                  </button>
                </span>
              </td>
              <td className="px-1 py-2 text-end font-mono text-xs font-black">
                {formatMoney(previewLineTotal(line), locale)}
              </td>
              <td className="px-1 py-2 text-center">
                <button
                  type="button"
                  aria-label={t(locale, "cart.removeLine")}
                  onClick={(event) => {
                    event.stopPropagation();
                    removeLine(line.key);
                  }}
                  className="rounded-md px-2 py-1 text-sm font-black text-red-600 hover:bg-red-100 dark:hover:bg-red-900/40"
                >
                  ✕
                </button>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
