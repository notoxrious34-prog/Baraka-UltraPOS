/**
 * Direct unit-selection popover for the POS multi-unit cockpit (TASK-DSN-046).
 *
 * Compact `absolute` popover (anchored in a `relative` cart cell — ADR-067:
 * no `fixed`). Number keys `1..9` select instantly, `Esc` closes. Prices via
 * `formatMoney`, factors via BigInt-exact division — zero floats.
 */

import { useEffect, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import type { CartUnitSnapshot, Locale } from "@/types/pos";

export interface UnitSelectPopoverProps {
  isOpen: boolean;
  onClose: () => void;
  units: CartUnitSnapshot[];
  selectedUnitIndex: number;
  onSelectUnit: (index: number) => void;
  locale: Locale;
}

/** Exact factor multiplier tag (`"1000000"` -> `"x100"`). Pure. */
export function factorTag(factorMicro: string): string {
  try {
    return `x${(BigInt(factorMicro) / 10000n).toString()}`;
  } catch {
    return "x?";
  }
}

/** True for the base selling unit (no conversion). Pure. */
export function isBaseUnit(factorMicro: string): boolean {
  return factorMicro === "10000";
}

export function UnitSelectPopover({
  isOpen,
  onClose,
  units,
  selectedUnitIndex,
  onSelectUnit,
  locale,
}: UnitSelectPopoverProps): ReactElement | null {
  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
        return;
      }
      if (/^[1-9]$/.test(event.key)) {
        const index = Number(event.key) - 1;
        if (index >= 0 && index < units.length) {
          event.stopPropagation();
          onSelectUnit(index);
          onClose();
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen, onClose, onSelectUnit, units.length]);

  if (!isOpen) {
    return null;
  }

  return (
    <div
      role="dialog"
      aria-label={t(locale, "pos.unitChanged")}
      className="absolute z-30 min-w-[240px] rounded-lg border border-slate-200 bg-white p-1.5 shadow-xl dark:border-slate-700 dark:bg-slate-900"
    >
      <p className="px-2 py-1 text-[11px] font-black uppercase tracking-wide text-slate-500">
        {t(locale, "pos.unitChanged")} [Esc]
      </p>
      <ul className="space-y-0.5">
        {units.map((unit, index) => {
          const active = index === selectedUnitIndex;
          return (
            <li key={unit.unitId}>
              <button
                type="button"
                onClick={() => {
                  onSelectUnit(index);
                  onClose();
                }}
                aria-pressed={active}
                className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-start text-xs hover:bg-primary-50 dark:hover:bg-primary-950/50 ${
                  active ? "bg-primary-50 font-black dark:bg-primary-950/50" : ""
                }`}
              >
                <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded bg-slate-200 font-mono text-[10px] font-black dark:bg-slate-700">
                  {index + 1}
                </span>
                <span className="min-w-0 flex-1 truncate font-bold">
                  {locale === "ar-DZ" ? unit.nameAr : unit.nameFr}
                </span>
                <span className="shrink-0 rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[10px] font-black text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                  {isBaseUnit(unit.factorMicro)
                    ? t(locale, "pos.baseUnit")
                    : factorTag(unit.factorMicro)}
                </span>
                <span className="shrink-0 font-mono text-[11px] font-black text-primary-600 dark:text-sky-300">
                  {formatMoney(unit.salePriceCentimes, locale)}
                </span>
                {active && (
                  <span className="shrink-0 font-black text-primary-600" aria-hidden="true">
                    ✓
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
