/**
 * Scrollable cart table: keyboard navigation, inline unit cycling,
 * quantity stepping, line deletion. Selected row carries the ring.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import {
  calculateBaseQuantityEquivalent,
  previewLineTotal,
  useCartStore,
} from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import { addBig, formatMoney, formatQty, subBig } from "@/lib/money";
import { UnitSelectPopover, factorTag, isBaseUnit } from "@/features/pos/UnitSelectPopover";

import type { CartLine, Locale } from "@/types/pos";

const STEP_MICRO = "10000";

export interface CartTableViewProps {
  lines: CartLine[];
  selectedLineIndex: number;
  locale: Locale;
  popoverKey: string | null;
  onSelectLine: (index: number) => void;
  onTogglePopover: (key: string) => void;
  onClosePopover: () => void;
  onSelectUnit: (key: string, index: number) => void;
  onStepQty: (key: string, direction: 1 | -1) => void;
  onRemoveLine: (key: string) => void;
}

export function CartTableView({
  lines,
  selectedLineIndex,
  locale,
  popoverKey,
  onSelectLine,
  onTogglePopover,
  onClosePopover,
  onSelectUnit,
  onStepQty,
  onRemoveLine,
}: CartTableViewProps): ReactElement {
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
                onSelectLine(index);
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
              <td className="relative px-1 py-2">
                {unit !== undefined && line.units.length > 1 ? (
                  <button
                    type="button"
                    title={t(locale, "pos.unitF3")}
                    aria-haspopup="dialog"
                    aria-expanded={popoverKey === line.key}
                    onClick={(event) => {
                      event.stopPropagation();
                      onTogglePopover(line.key);
                    }}
                    className="inline-flex cursor-pointer items-center gap-1.5 rounded-md border border-primary-300 bg-primary-50 px-2 py-1 text-xs font-black hover:bg-primary-100 dark:border-primary-700 dark:bg-primary-950/60"
                  >
                    <span>{locale === "ar-DZ" ? unit.nameAr : unit.nameFr}</span>
                    {!isBaseUnit(unit.factorMicro) && (
                      <span className="rounded bg-primary-600 px-1 font-mono text-[10px] font-black text-white">
                        {factorTag(unit.factorMicro)}
                      </span>
                    )}
                    <kbd className="rounded bg-slate-200 px-1 font-mono text-[10px] font-black text-slate-600 dark:bg-slate-700 dark:text-slate-300">
                      F3
                    </kbd>
                    <span aria-hidden="true" className="text-[10px] text-slate-500">
                      ▾
                    </span>
                  </button>
                ) : (
                  <span
                    title={t(locale, "pos.singleUnit")}
                    className="inline-flex items-center rounded-md bg-slate-200 px-2 py-1 text-xs font-bold text-slate-500 dark:bg-slate-700 dark:text-slate-400"
                  >
                    {unit !== undefined
                      ? locale === "ar-DZ"
                        ? unit.nameAr
                        : unit.nameFr
                      : "—"}
                  </span>
                )}
                {unit !== undefined && !isBaseUnit(unit.factorMicro) && (
                  <p
                    title={t(locale, "pos.stockEquiv")}
                    className="mt-0.5 font-mono text-xs font-medium text-primary-600 dark:text-primary-400"
                  >
                    {calculateBaseQuantityEquivalent(
                      line.quantityMicro,
                      unit.factorMicro,
                      line.units.find((candidate) => candidate.isBaseUnit)?.nameFr ??
                        unit.nameFr,
                      line.units.find((candidate) => candidate.isBaseUnit)?.nameAr ??
                        unit.nameAr,
                      locale,
                    )}
                  </p>
                )}
                <UnitSelectPopover
                  isOpen={popoverKey === line.key}
                  onClose={onClosePopover}
                  units={line.units}
                  selectedUnitIndex={line.unitIndex}
                  onSelectUnit={(index) => onSelectUnit(line.key, index)}
                  locale={locale}
                />
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
                      onStepQty(line.key, -1);
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
                      onStepQty(line.key, 1);
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
                    onRemoveLine(line.key);
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

/** Thin store-to-props bridge: owns selection, popover state, and qty stepping. */
export function CartTable(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const lines = useCartStore((state) => state.lines);
  const selectedLineIndex = usePosUiStore((state) => state.selectedLineIndex);
  const setSelectedLineIndex = usePosUiStore((state) => state.setSelectedLineIndex);
  const removeLine = useCartStore((state) => state.removeLine);
  const setLineUnit = useCartStore((state) => state.setLineUnit);
  const setLineQty = useCartStore((state) => state.setLineQty);
  const [popoverKey, setPopoverKey] = useState<string | null>(null);

  const stepQty = (key: string, direction: 1 | -1): void => {
    const line = useCartStore.getState().lines.find((candidate) => candidate.key === key);
    if (line === undefined) {
      return;
    }
    try {
      if (direction === 1) {
        setLineQty(key, addBig(line.quantityMicro, STEP_MICRO));
      } else {
        const next = subBig(line.quantityMicro, STEP_MICRO);
        setLineQty(key, next.startsWith("-") ? "0" : next);
      }
    } catch {
      setLineQty(key, direction === 1 ? STEP_MICRO : "0");
    }
  };

  return (
    <CartTableView
      lines={lines}
      selectedLineIndex={selectedLineIndex}
      locale={locale}
      popoverKey={popoverKey}
      onSelectLine={setSelectedLineIndex}
      onTogglePopover={(key) =>
        setPopoverKey((open) => (open === key ? null : key))
      }
      onClosePopover={() => setPopoverKey(null)}
      onSelectUnit={(key, index) => setLineUnit(key, index)}
      onStepQty={stepQty}
      onRemoveLine={removeLine}
    />
  );
}
