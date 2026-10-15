/**
 * Scrollable cart table: keyboard navigation, inline unit cycling,
 * quantity stepping, line deletion. Selected row carries the ring.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import {
  calculateBaseQuantityEquivalent,
  isLineTierPriced,
  lineUnitPriceCentimes,
  previewLineTotal,
  useCartStore,
} from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import { addBig, formatMoney, formatQty, subBig } from "@/lib/money";
import { UnitSelectPopover, factorTag, isBaseUnit } from "@/features/pos/UnitSelectPopover";

import type { CartLine, CustomerTier, LineMargin, Locale } from "@/types/pos";

const STEP_MICRO = "10000";

/** Integer-exact basis-points to percent label (`1764` -> `+17.64%`). */
function formatMarginPercent(basisPoints: number): string {
  const sign = basisPoints < 0 ? "-" : "+";
  const abs = Math.abs(Math.trunc(basisPoints));
  const whole = Math.trunc(abs / 100);
  const frac = (abs % 100).toString().padStart(2, "0").replace(/0+$/, "");
  return `${sign}${frac === "" ? `${whole}` : `${whole}.${frac}`}%`;
}

export interface CartTableViewProps {
  lines: CartLine[];
  selectedLineIndex: number;
  locale: Locale;
  /** Active customer tier (drives PRO/GROS pills + tier totals). */
  customerTier: CustomerTier;
  /** Last preview Margin Guard readout (indexed by cart line order). */
  margins: LineMargin[];
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
  customerTier,
  margins,
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
      <div className="flex flex-col items-center gap-2 p-6 text-center">
        <svg
          aria-hidden="true"
          width="36"
          height="36"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="text-slate-300 dark:text-slate-600"
        >
          <circle cx="9" cy="20" r="1.5" />
          <circle cx="17" cy="20" r="1.5" />
          <path d="M2 3h3l2.6 12.5a1 1 0 0 0 1 .5h8.7a1 1 0 0 0 1-.8L20.5 8H6" />
        </svg>
        <p className="text-sm font-black text-slate-500 dark:text-slate-400">
          {t(locale, "cart.empty")}
        </p>
        <p className="text-xs font-medium text-slate-400 dark:text-slate-500">
          {t(locale, "cart.emptyHint")}
        </p>
        <div className="flex items-center gap-1.5 pt-1">
          <span className="rounded-md bg-slate-100 px-2 py-1 font-mono text-[11px] font-bold text-slate-500 dark:bg-slate-800 dark:text-slate-400">
            [F1] {t(locale, "cart.hintSearch")}
          </span>
          <span className="rounded-md bg-slate-100 px-2 py-1 font-mono text-[11px] font-bold text-slate-500 dark:bg-slate-800 dark:text-slate-400">
            [F2] {t(locale, "cart.hintClient")}
          </span>
          <span className="rounded-md bg-slate-100 px-2 py-1 font-mono text-[11px] font-bold text-slate-500 dark:bg-slate-800 dark:text-slate-400">
            [F12] {t(locale, "cart.hintCheckout")}
          </span>
        </div>
      </div>
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
          const tierPriced = isLineTierPriced(line, customerTier);
          const margin = margins.find((entry) => entry.line_index === index);
          const belowCost = margin?.is_below_cost === true;
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
                {(tierPriced || belowCost) && (
                  <span className="mt-0.5 flex flex-wrap gap-1">
                    {tierPriced && (
                      <span className="rounded bg-amber-100 px-1.5 py-0.5 text-[10px] font-bold text-amber-800 dark:bg-amber-900/40 dark:text-amber-300">
                        {customerTier === "artisan"
                          ? locale === "ar-DZ"
                            ? "سعر مهني"
                            : "PRO"
                          : locale === "ar-DZ"
                            ? "سعر جملة"
                            : "GROS"}
                      </span>
                    )}
                    {belowCost && margin !== undefined && (
                      <span
                        title={`${t(locale, "margin_label")}: ${formatMarginPercent(margin.margin_basis_points)}`}
                        className="animate-pulse rounded border border-rose-300 bg-rose-100 px-2 py-0.5 text-[11px] font-bold text-rose-700 dark:border-rose-700 dark:bg-rose-950/60 dark:text-rose-400"
                      >
                        ⚠️ {t(locale, "below_cost_warning")}
                      </span>
                    )}
                  </span>
                )}
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
                {unit !== undefined ? formatMoney(lineUnitPriceCentimes(line, customerTier), locale) : "—"}
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
                {formatMoney(previewLineTotal(line, customerTier), locale)}
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
  const customer = useCartStore((state) => state.customer);
  const previewMargins = useCartStore((state) => state.previewMargins);
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
      customerTier={customer.kind === "named" ? customer.tier : "retail"}
      margins={previewMargins}
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
