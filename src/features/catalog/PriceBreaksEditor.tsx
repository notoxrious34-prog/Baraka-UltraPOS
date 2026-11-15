/**
 * Quantity-break ladder editor (BRK-121b): rungs for one product x base unit.
 * Loads via `catalog_list_price_breaks`, persists atomically via
 * `catalog_replace_price_breaks`. All math in canonical strings (BigInt
 * only); quantities shown in selling units, stored in micros.
 */
import { useCallback, useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import { ipcListPriceBreaks, ipcReplacePriceBreaks, type PriceBreakWire } from "@/lib/ipc";
import { formatQty, parseMoneyToCentimes, parseQtyToMicro } from "@/lib/money";
import { centimesToDecimalInput } from "@/features/catalog/ProductEditModal";
import type { PriceBreakRung } from "@/types/pos";

interface BreaksEditorProps {
  locale: Locale;
  token: string;
  productId: string;
  unitId: string;
  inputClass: string;
}

interface Row {
  qty: string;
  price: string;
}

/** Wire ladder → display rows (exported for tests; garbage-safe). */
export function toRows(ladder: PriceBreakWire[]): Row[] {
  return ladder.map((rung) => {
    let qty = "";
    let price = "";
    try {
      qty = formatQty(rung.min_qty_micro);
    } catch {
      qty = "";
    }
    try {
      price = centimesToDecimalInput(rung.price_centimes);
    } catch {
      price = "";
    }
    return { qty, price };
  });
}

export function PriceBreaksEditor(props: BreaksEditorProps): ReactElement {
  const { locale, token, productId, unitId, inputClass } = props;
  const [rows, setRows] = useState<Row[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoaded(false);
    setError(null);
    ipcListPriceBreaks(token, productId, unitId)
      .then((ladder) => {
        if (!cancelled) {
          setRows(toRows(ladder));
          setLoaded(true);
          setDirty(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setError(translateBackendError(failure, locale));
          setLoaded(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, productId, unitId, locale]);

  const patchRow = useCallback((index: number, field: keyof Row, value: string) => {
    setRows((current) =>
      current.map((row, position) => (position === index ? { ...row, [field]: value } : row)),
    );
    setDirty(true);
    setError(null);
  }, []);

  const save = useCallback(() => {
    let rungs: PriceBreakRung[];
    try {
      rungs = rows.map((row) => ({
        minQtyMicro: parseQtyToMicro(row.qty),
        priceCentimes: parseMoneyToCentimes(row.price),
      }));
    } catch {
      setError(t(locale, "cat.invalidPrice"));
      return;
    }
    setSaving(true);
    setError(null);
    ipcReplacePriceBreaks(token, productId, unitId, rungs)
      .then(() => {
        setDirty(false);
        setSaving(false);
      })
      .catch((failure: unknown) => {
        setError(translateBackendError(failure, locale));
        setSaving(false);
      });
  }, [rows, token, productId, unitId, locale]);

  if (!loaded) {
    return <p className="text-xs opacity-60">…</p>;
  }

  return (
    <div className="flex flex-col gap-2">
      {rows.length === 0 && !dirty && (
        <p className="text-xs opacity-60">{t(locale, "breaks_empty")}</p>
      )}
      {rows.map((row, index) => (
        <div key={index} className="flex items-center gap-2">
          <label className="flex flex-1 flex-col gap-1">
            <span className="text-[11px] font-bold opacity-70">{t(locale, "breaks_qty")}</span>
            <input
              value={row.qty}
              inputMode="decimal"
              dir="ltr"
              onChange={(event) => patchRow(index, "qty", event.target.value)}
              className={`${inputClass} font-mono`}
            />
          </label>
          <label className="flex flex-1 flex-col gap-1">
            <span className="text-[11px] font-bold opacity-70">{t(locale, "breaks_price")}</span>
            <input
              value={row.price}
              inputMode="decimal"
              dir="ltr"
              onChange={(event) => patchRow(index, "price", event.target.value)}
              className={`${inputClass} font-mono`}
            />
          </label>
          <button
            type="button"
            onClick={() => {
              setRows((current) => current.filter((_, position) => position !== index));
              setDirty(true);
              setError(null);
            }}
            className="mt-5 shrink-0 rounded-md bg-red-700 px-2 py-1 text-xs font-bold text-white hover:bg-red-800"
          >
            {t(locale, "del.delete")}
          </button>
        </div>
      ))}
      {error !== null && <p className="text-xs font-bold text-red-600">{error}</p>}
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => {
            setRows((current) => [...current, { qty: "", price: "" }]);
            setError(null);
          }}
          className="rounded-md border border-slate-300 px-2 py-1 text-xs font-bold hover:bg-slate-100 dark:border-slate-600 dark:hover:bg-slate-800"
        >
          {t(locale, "breaks_add")}
        </button>
        {dirty && (
          <button
            type="button"
            disabled={saving}
            onClick={save}
            className="rounded-md bg-primary-600 px-3 py-1 text-xs font-black text-white hover:bg-primary-900 disabled:opacity-50"
          >
            {t(locale, "cat.save")}
          </button>
        )}
      </div>
    </div>
  );
}
