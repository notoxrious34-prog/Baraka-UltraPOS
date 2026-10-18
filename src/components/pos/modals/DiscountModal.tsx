/**
 * Discount modal (F5): global discount editor plus selected-line
 * discount editor. Amounts typed in DZD, stored as centime strings.
 */

import { useState, type ReactElement } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import { parseMoneyToCentimes } from "@/lib/money";
import { ModalShell } from "@/components/pos/modals/ModalShell";

function toDisplay(centimes: string): string {
  try {
    return (BigInt(centimes) / 100n).toString();
  } catch {
    return "0";
  }
}

export function DiscountModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const lines = useCartStore((state) => state.lines);
  const selectedLineIndex = usePosUiStore((state) => state.selectedLineIndex);
  const globalDiscountCentimes = useCartStore((state) => state.globalDiscountCentimes);
  const setGlobalDiscount = useCartStore((state) => state.setGlobalDiscount);
  const setLineDiscount = useCartStore((state) => state.setLineDiscount);

  const selected = lines[selectedLineIndex];
  const [globalText, setGlobalText] = useState(() => toDisplay(globalDiscountCentimes));
  const [lineText, setLineText] = useState(() =>
    toDisplay(selected !== undefined ? selected.lineDiscountCentimes : "0"),
  );
  const [error, setError] = useState<string | null>(null);

  const apply = (): void => {
    try {
      const global = globalText.trim() === "" ? "0" : parseMoneyToCentimes(globalText);
      const line = lineText.trim() === "" ? "0" : parseMoneyToCentimes(lineText);
      if (global.startsWith("-") || line.startsWith("-")) {
        throw new Error("negative");
      }
      setGlobalDiscount(global);
      if (selected !== undefined) {
        setLineDiscount(selected.key, line);
      }
      closeModal();
    } catch {
      setError(t(locale, "discount.invalid"));
    }
  };

  const inputClass =
    "h-11 w-full rounded-lg border border-slate-300 bg-white px-3 font-mono text-base font-black outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800";

  return (
    <ModalShell title={t(locale, "discount.title")} onClose={closeModal}>
      <div className="flex flex-col gap-3">
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-ink-muted">{t(locale, "discount.global")}</span>
          <input
            value={globalText}
            onChange={(event) => {
              setGlobalText(event.target.value);
            }}
            inputMode="decimal"
            autoComplete="off"
            className={inputClass}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-bold text-ink-muted">{t(locale, "discount.line")}</span>
          <input
            value={lineText}
            onChange={(event) => {
              setLineText(event.target.value);
            }}
            inputMode="decimal"
            autoComplete="off"
            className={inputClass}
          />
        </label>
        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700">{error}</p>
        )}
        <button
          type="button"
          onClick={apply}
          className="h-12 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
        >
          {t(locale, "discount.apply")}
        </button>
      </div>
    </ModalShell>
  );
}
