/**
 * Fast quantity modal (F4): touch-friendly numeric entry fallback.
 * Converts display units to micro strings without floats.
 */

import { useState, type ReactElement } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import { formatQty, parseQtyToMicro } from "@/lib/money";
import { ModalShell } from "@/components/pos/modals/ModalShell";

const PRESETS = ["1", "2", "5", "10"];

export function QuantityModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const lines = useCartStore((state) => state.lines);
  const selectedLineIndex = usePosUiStore((state) => state.selectedLineIndex);
  const setLineQty = useCartStore((state) => state.setLineQty);

  const selected = lines[selectedLineIndex];
  const [text, setText] = useState(() =>
    selected !== undefined ? formatQty(selected.quantityMicro) : "1",
  );
  const [error, setError] = useState<string | null>(null);

  const apply = (): void => {
    if (selected === undefined) {
      closeModal();
      return;
    }
    try {
      const micro = parseQtyToMicro(text);
      if (micro.startsWith("-") || micro === "0") {
        throw new Error("non-positive");
      }
      setLineQty(selected.key, micro);
      closeModal();
    } catch {
      setError(t(locale, "quantity.invalid"));
    }
  };

  return (
    <ModalShell title={t(locale, "quantity.title")} onClose={closeModal}>
      <div className="flex flex-col gap-3">
        <p className="truncate text-sm font-bold text-slate-600 dark:text-slate-300">
          {selected !== undefined
            ? locale === "ar-DZ"
              ? selected.nameAr
              : selected.nameFr
            : "—"}
        </p>
        <input
          value={text}
          onChange={(event) => {
            setText(event.target.value);
          }}
          inputMode="decimal"
          autoComplete="off"
          autoFocus
          className="h-14 w-full rounded-lg border border-slate-300 bg-white px-3 text-center font-mono text-2xl font-black outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
        />
        <div className="grid grid-cols-4 gap-2">
          {PRESETS.map((preset) => (
            <button
              key={preset}
              type="button"
              onClick={() => {
                setText(preset);
              }}
              className="h-11 rounded-lg bg-slate-200 font-mono text-base font-black hover:bg-slate-300 dark:bg-slate-700"
            >
              {preset}
            </button>
          ))}
        </div>
        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700">{error}</p>
        )}
        <button
          type="button"
          onClick={apply}
          className="h-12 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
        >
          {t(locale, "quantity.apply")}
        </button>
      </div>
    </ModalShell>
  );
}
