/**
 * POS 2D variant grid (DSN-108): model picker + size grid, one tap to cart.
 *
 * Each cell rides its default-selling unit from `catalog_matrix_view`
 * (single query, no N+1). Tapping a cell fetches the full product once
 * (`catalog_get_product`, the same source as discovery) and adds it with
 * its default unit resolved. Cells without a default unit stay disabled;
 * zero stock stays tappable — shortage policy lives at checkout.
 */

import { useEffect, useState, type ReactElement } from "react";
import {
  ipcGetProduct,
  ipcMatrixModels,
  ipcMatrixView,
  type VariantMatrixRowWire,
  type VariantModelWire,
} from "@/lib/ipc";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";

/** Order-preserving axis grouping of grid rows (unit-tested). */
export function groupMatrixRows(rows: VariantMatrixRowWire[]): {
  axis1: string[];
  axis2: string[];
  cells: Map<string, Map<string, VariantMatrixRowWire>>;
} {
  const axis1: string[] = [];
  const axis2: string[] = [];
  const cells = new Map<string, Map<string, VariantMatrixRowWire>>();
  for (const row of rows) {
    if (!axis1.includes(row.axis1_value)) {
      axis1.push(row.axis1_value);
    }
    if (!axis2.includes(row.axis2_value)) {
      axis2.push(row.axis2_value);
    }
    let column = cells.get(row.axis1_value);
    if (column === undefined) {
      column = new Map<string, VariantMatrixRowWire>();
      cells.set(row.axis1_value, column);
    }
    column.set(row.axis2_value, row);
  }
  return { axis1, axis2, cells };
}

/**
 * Sellable units on hand for one row (`null` = no default unit, cell off).
 * Floor division in micros; unparseable figures fail safe to zero.
 */
export function sellableUnits(row: VariantMatrixRowWire): string | null {
  const unit = row.default_unit;
  if (unit === null) {
    return null;
  }
  try {
    const onHand = BigInt(row.on_hand_micro);
    const factor = BigInt(unit.factor_micro);
    if (factor <= 0n) {
      return "0";
    }
    return (onHand / factor).toString();
  } catch {
    return "0";
  }
}

/** Last-tap feedback chip (auto-clears on the next tap). */
interface TapNote {
  name: string;
  at: number;
}

export function MatrixGridPanel(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const addProduct = useCartStore((state) => state.addProduct);
  const [models, setModels] = useState<VariantModelWire[]>([]);
  const [modelId, setModelId] = useState<string | null>(null);
  const [rows, setRows] = useState<VariantMatrixRowWire[]>([]);
  const [axes, setAxes] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<TapNote | null>(null);

  useEffect(() => {
    if (session === null) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    ipcMatrixModels(session.token)
      .then((list) => {
        if (cancelled) {
          return;
        }
        setModels(list);
        if (list.length > 0 && modelId === null) {
          setModelId(list[0]!.model_id);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
    // Load once per session; model switches fetch their own grid below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.token]);

  useEffect(() => {
    if (session === null || modelId === null) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    ipcMatrixView(session.token, modelId)
      .then((grid) => {
        if (!cancelled) {
          setRows(grid);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [session, modelId]);

  useEffect(() => {
    const model = models.find((entry) => entry.model_id === modelId) ?? null;
    setAxes(model === null ? [] : [model.axis1_name, model.axis2_name]);
  }, [models, modelId]);

  const grouped = groupMatrixRows(rows);

  const tapCell = (row: VariantMatrixRowWire): void => {
    if (session === null || row.default_unit === null) {
      return;
    }
    const token = session.token;
    void ipcGetProduct(token, row.product_id)
      .then((product) => {
        const defaultIndex = product.units.findIndex((unit) => unit.is_default_selling);
        addProduct(product, defaultIndex >= 0 ? defaultIndex : 0);
        setNote({ name: locale === "ar-DZ" ? row.name_ar : row.sku, at: Date.now() });
      })
      .catch((failure: unknown) => {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
      <div className="flex shrink-0 flex-wrap items-center gap-1">
        <span className="self-center text-xs font-medium text-slate-500 dark:text-slate-400">
          {t(locale, "matrix.pickModel")}
        </span>
        {models.map((model) => (
          <button
            key={model.model_id}
            type="button"
            onClick={() => {
              setModelId(model.model_id);
            }}
            className={`rounded-full px-3 py-1 text-xs font-bold ${
              modelId === model.model_id
                ? "bg-primary-600 text-white"
                : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
            }`}
          >
            {locale === "ar-DZ" ? model.name_ar : model.name_fr}
          </button>
        ))}
      </div>
      {loading && (
        <p className="p-4 text-center text-sm font-semibold text-slate-500">
          {t(locale, "search.loading")}
        </p>
      )}
      {!loading && error !== null && (
        <p className="p-4 text-center text-sm font-semibold text-red-600">{error}</p>
      )}
      {!loading && error === null && models.length === 0 && (
        <p className="p-4 text-center text-sm font-semibold text-slate-500">
          {t(locale, "matrix.noModels")}
        </p>
      )}
      {!loading && error === null && modelId !== null && (
        <table className="w-full shrink-0 border-collapse text-center">
          <thead>
            <tr>
              <th className="w-16 px-1 py-1 text-[11px] font-bold text-slate-400">
                {axes[1] ?? ""} \\ {axes[0] ?? ""}
              </th>
              {grouped.axis2.map((second) => (
                <th
                  key={second}
                  className="px-1 py-1 text-xs font-black text-slate-700 dark:text-slate-200"
                >
                  {second}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {grouped.axis1.map((first) => (
              <tr key={first}>
                <th className="px-1 py-1 text-xs font-black text-slate-700 dark:text-slate-200">
                  {first}
                </th>
                {grouped.axis2.map((second) => {
                  const row = grouped.cells.get(first)?.get(second);
                  if (row === undefined) {
                    return <td key={second} className="p-1" />;
                  }
                  const units = sellableUnits(row);
                  const off = units === null;
                  let empty = false;
                  try {
                    empty = !off && BigInt(row.on_hand_micro) <= 0n;
                  } catch {
                    empty = false;
                  }
                  return (
                    <td key={second} className="p-1">
                      <button
                        type="button"
                        disabled={off}
                        title={off ? t(locale, "matrix.noDefaultUnit") : row.sku}
                        onClick={() => {
                          tapCell(row);
                        }}
                        className={`flex w-full flex-col items-center gap-0.5 rounded-lg border px-1 py-1.5 text-center transition-colors ${
                          off
                            ? "cursor-not-allowed border-slate-200 bg-slate-100 text-slate-400 dark:border-slate-800 dark:bg-slate-900"
                            : "border-slate-200 bg-slate-50 hover:border-primary-600 hover:bg-sky-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-sky-950/40"
                        }`}
                      >
                        <span
                          className={`font-mono text-sm font-black ${
                            empty
                              ? "text-red-600 dark:text-red-400"
                              : "text-primary-600 dark:text-sky-300"
                          }`}
                        >
                          {row.sale_price_centimes === null
                            ? "—"
                            : formatMoney(row.sale_price_centimes, locale)}
                        </span>
                        <span className="font-mono text-[11px] text-slate-500">
                          {off ? t(locale, "matrix.noDefaultUnit") : `${units ?? "0"}`}
                        </span>
                      </button>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {note !== null && (
        <p key={note.at} className="shrink-0 px-1 text-xs font-bold text-emerald-700 dark:text-emerald-400">
          {"✓ "}
          {note.name}
          {" — "}
          {t(locale, "matrix.addedToCart")}
        </p>
      )}
    </div>
  );
}
