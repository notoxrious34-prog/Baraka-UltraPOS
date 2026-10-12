/**
 * Master Catalog Hub table (TASK-DSN-041): dense product table with discovery
 * load on mount, debounced search, stock indicators, BigInt margin math, and
 * creation/label modal integration.
 *
 * The backend `catalog_search` returns discovery rows for blank queries, so
 * an empty search reloads the full list — rows are never blanked. Money and
 * quantities stay canonical strings; the UI converts with BigInt only.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcGetStock, ipcSearchCatalog, listWarehouses } from "@/lib/ipc";
import { formatMoney, formatQty } from "@/lib/money";
import type { CatalogProduct } from "@/types/pos";
import {
  pickDepotId,
  ProductCreateModal,
} from "@/features/catalog/ProductCreateModal";
import { ProductLabelModal } from "@/features/catalog/ProductLabelModal";

export interface MarginView {
  /** Signed margin in centimes (`sale - purchase`), canonical string. */
  amountCentimes: string;
  /** Margin percent with 2 decimals (`null` when purchase is zero). */
  pct: number | null;
}

/**
 * Margin from canonical centime strings using integer BigInt math only.
 * `pct` is truncated (never rounded) per integer division semantics.
 */
export function computeMargin(saleCentimes: string, purchaseCentimes: string): MarginView {
  const sale = BigInt(saleCentimes === "" ? "0" : saleCentimes);
  const purchase = BigInt(purchaseCentimes === "" ? "0" : purchaseCentimes);
  const amount = sale - purchase;
  if (purchase === 0n) {
    return { amountCentimes: amount.toString(), pct: null };
  }
  const pct = Number((amount * 10000n) / purchase) / 100;
  return { amountCentimes: amount.toString(), pct };
}

/** `25.5` -> `"+25.5%"`, `-10` -> `"-10%"`, `0` -> `"0%"`. Pure. */
export function formatMarginPct(pct: number): string {
  return `${pct > 0 ? "+" : ""}${String(pct)}%`;
}

/** Base unit preferred, first unit fallback, `null` when unit-less. Pure. */
export function baseUnitOf(product: CatalogProduct): CatalogProduct["units"][number] | null {
  return product.units.find((unit) => unit.is_base_unit) ?? product.units[0] ?? null;
}

/** Same manager rule as App.tsx isManagerRole (ADMIN or GERANT). */
function isManagerRole(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

const SEARCH_LIMIT = 100;
const SEARCH_DEBOUNCE_MS = 150;

export interface CatalogHubViewProps {
  locale: Locale;
  products: CatalogProduct[];
  stockLevels: Record<string, string>;
  loading: boolean;
  error: string | null;
  query: string;
  onQuery: (value: string) => void;
  onClear: () => void;
  onRefresh: () => void;
  onNew: () => void;
  onPrintLabel: (product: CatalogProduct) => void;
}

function marginBadgeClass(pct: number): string {
  if (pct > 0) {
    return "rounded-md bg-green-100 px-2 py-0.5 font-mono text-green-700 dark:bg-green-900/40 dark:text-green-200";
  }
  if (pct < 0) {
    return "rounded-md bg-red-100 px-2 py-0.5 font-mono text-red-700 dark:bg-red-900/40 dark:text-red-200";
  }
  return "rounded-md bg-slate-200 px-2 py-0.5 font-mono text-slate-600 dark:bg-slate-700 dark:text-slate-300";
}

function stockCell(
  product: CatalogProduct,
  stockLevels: Record<string, string>,
  locale: Locale,
): ReactElement {
  const micro = stockLevels[product.product_id];
  if (micro === undefined) {
    return <span className="font-mono text-slate-400">…</span>;
  }
  let empty = false;
  try {
    empty = BigInt(micro) <= 0n;
  } catch {
    return <span className="font-mono text-slate-400">—</span>;
  }
  if (empty) {
    return (
      <span className="rounded-md bg-red-100 px-2 py-0.5 font-mono text-red-700 dark:bg-red-900/40 dark:text-red-200">
        0 - {t(locale, "cat.outOfStock")}
      </span>
    );
  }
  const base = baseUnitOf(product);
  const suffix = base === null ? "" : ` ${locale === "ar-DZ" ? base.unit_name_ar : base.unit_name_fr}`;
  return (
    <span className="font-mono">
      {formatQty(micro)}
      {suffix}
    </span>
  );
}

/** Pure presentational hub (SSR-safe for tests; the container owns IPC). */
export function CatalogHubView(props: CatalogHubViewProps): ReactElement {
  const { locale, products, stockLevels, loading, error, query } = props;
  return (
    <section className="flex h-full flex-col gap-3 p-4">
      <div className="flex shrink-0 flex-wrap items-center gap-2">
        <h2 className="text-lg font-black">{t(locale, "cat.masterTitle")}</h2>
        <span className="rounded-full bg-slate-200 px-2 py-0.5 text-xs font-black dark:bg-slate-700">
          {products.length} {t(locale, "cat.productsCount")}
        </span>
        <div className="flex min-w-52 flex-1 items-center gap-1">
          <input
            value={query}
            autoFocus
            onChange={(event) => {
              props.onQuery(event.target.value);
            }}
            placeholder={t(locale, "cat.searchPlaceholder")}
            aria-label={t(locale, "cat.search")}
            className="h-10 flex-1 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          />
          {query !== "" && (
            <button
              type="button"
              onClick={props.onClear}
              aria-label={t(locale, "cat.clearSearch")}
              className="h-10 rounded-lg bg-slate-200 px-3 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
            >
              ✕
            </button>
          )}
        </div>
        <button
          type="button"
          onClick={props.onRefresh}
          className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
        >
          {t(locale, "cat.refresh")}
        </button>
        <button
          type="button"
          onClick={props.onNew}
          className="h-10 rounded-lg bg-accent-500 px-4 text-sm font-black text-slate-950 hover:bg-amber-400"
        >
          {"➕ "}
          {t(locale, "cat.newProduct")}
        </button>
      </div>

      {loading && <p className="shrink-0 text-xs font-bold">{t(locale, "cash.loading")}</p>}
      {error !== null && (
        <p className="shrink-0 rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(error, locale)}
        </p>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto rounded-xl border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900">
        <table className="w-full border-collapse text-xs">
          <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
            <tr>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colSku")}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colBarcode")}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colName")}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colBaseUnit")}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {t(locale, "cat.colPurchasePrice")}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {t(locale, "cat.colSalePrice")}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {t(locale, "cat.colMargin")}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {t(locale, "cat.colStock")}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colStatus")}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colActions")}
              </th>
            </tr>
          </thead>
          <tbody>
            {products.map((product) => {
              const base = baseUnitOf(product);
              const margin =
                base === null
                  ? null
                  : computeMargin(base.sale_price_centimes, base.purchase_price_centimes);
              return (
                <tr
                  key={product.product_id}
                  className="border-t border-slate-100 hover:bg-slate-50 dark:border-slate-800 dark:hover:bg-slate-800/50"
                >
                  <td className="px-3 py-1.5 font-mono font-black">{product.sku}</td>
                  <td className="px-3 py-1.5 font-mono">{product.barcode ?? "—"}</td>
                  <td className="px-3 py-1.5">
                    <span className="block font-bold">
                      {locale === "ar-DZ" ? product.name_ar : product.name_fr}
                    </span>
                    {product.name_ar !== "" && (
                      <span className="block text-[11px] opacity-60">
                        {locale === "ar-DZ" ? product.name_fr : product.name_ar}
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-1.5 font-bold">
                    {base === null ? "—" : `${base.unit_name_fr} · ${base.unit_name_ar}`}
                  </td>
                  <td className="px-3 py-1.5 text-end font-mono">
                    {base === null ? "—" : formatMoney(base.purchase_price_centimes, locale)}
                  </td>
                  <td className="px-3 py-1.5 text-end font-black">
                    {base === null ? "—" : formatMoney(base.sale_price_centimes, locale)}
                  </td>
                  <td className="px-3 py-1.5 text-end">
                    {margin === null || margin.pct === null ? (
                      <span className="font-mono text-slate-400">—</span>
                    ) : (
                      <span className="inline-flex items-center gap-1">
                        <span className="font-mono">{formatMoney(margin.amountCentimes, locale)}</span>
                        <span className={marginBadgeClass(margin.pct)}>
                          {formatMarginPct(margin.pct)}
                        </span>
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-1.5 text-end font-bold">
                    {stockCell(product, stockLevels, locale)}
                  </td>
                  <td className="px-3 py-1.5">
                    {product.is_active ? (
                      <span className="rounded-md bg-green-100 px-2 py-0.5 font-black text-green-700 dark:bg-green-900/40 dark:text-green-200">
                        {t(locale, "cat.active")}
                      </span>
                    ) : (
                      <span className="rounded-md bg-slate-200 px-2 py-0.5 font-black text-slate-500 dark:bg-slate-700 dark:text-slate-300">
                        {t(locale, "cat.inactive")}
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        props.onPrintLabel(product);
                      }}
                      aria-label={t(locale, "cat.printLabel")}
                      title={t(locale, "cat.printLabel")}
                      className="rounded-md bg-slate-200 px-2 py-1 text-xs font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
                    >
                      🏷️
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {products.length === 0 && !loading && (
          <p className="px-4 py-6 text-center text-sm font-bold opacity-60">
            {t(locale, "cat.noProducts")}
          </p>
        )}
      </div>
    </section>
  );
}

export function CatalogModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [products, setProducts] = useState<CatalogProduct[]>([]);
  const [stockLevels, setStockLevels] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [labelProduct, setLabelProduct] = useState<CatalogProduct | null>(null);
  const depotRef = useRef<string | null | undefined>(undefined);
  const skipFirstDebounce = useRef(true);
  const authed = session !== null && isManagerRole(session.role);
  const token: string | null = authed && session !== null ? session.token : null;

  const ensureDepot = async (tok: string, alive: () => boolean): Promise<string | null> => {
    if (depotRef.current !== undefined) {
      return depotRef.current;
    }
    try {
      const rows = await listWarehouses(tok);
      if (!alive()) {
        return null;
      }
      depotRef.current = pickDepotId(rows);
    } catch {
      if (!alive()) {
        return null;
      }
      depotRef.current = null;
    }
    return depotRef.current;
  };

  const loadCatalog = async (tok: string, term: string, alive: () => boolean): Promise<void> => {
    setLoading(true);
    try {
      const rows = await ipcSearchCatalog(tok, term, SEARCH_LIMIT);
      if (!alive()) {
        return;
      }
      setProducts(rows);
      setError(null);
      const depot = await ensureDepot(tok, alive);
      if (!alive() || depot === null) {
        return;
      }
      const entries = await Promise.all(
        rows.map(async (product) => {
          try {
            const level = await ipcGetStock(tok, product.product_id, depot);
            return [product.product_id, level] as const;
          } catch {
            return null;
          }
        }),
      );
      if (!alive()) {
        return;
      }
      const next: Record<string, string> = {};
      for (const entry of entries) {
        if (entry !== null) {
          next[entry[0]] = entry[1];
        }
      }
      setStockLevels(next);
    } catch (failure: unknown) {
      if (alive()) {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        setProducts([]);
      }
    } finally {
      if (alive()) {
        setLoading(false);
      }
    }
  };

  // Discovery load on mount; 150ms debounced search after. An empty query
  // reloads discovery rows — the table is never blanked by clearing input.
  useEffect(() => {
    if (token === null) {
      return;
    }
    const activeToken: string = token;
    let cancelled = false;
    const alive = (): boolean => !cancelled;
    if (skipFirstDebounce.current) {
      skipFirstDebounce.current = false;
      void loadCatalog(activeToken, "", alive);
      return () => {
        cancelled = true;
      };
    }
    const timer = window.setTimeout(() => {
      void loadCatalog(activeToken, query.trim(), alive);
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
    // loadCatalog/ensureDepot take the token as a parameter; query re-triggers.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, token]);

  // Defense in depth: the screen verifies the role itself, independent of
  // NavRail visibility (URL/state manipulation must never expose the hub).
  if (!authed) {
    return (
      <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError("UNAUTHORIZED", locale)}
        </p>
      </section>
    );
  }
  const activeToken: string = session.token;

  const refresh = (): void => {
    const cancelled = false;
    void loadCatalog(activeToken, query.trim(), () => !cancelled);
  };

  return (
    <>
      <CatalogHubView
        locale={locale}
        products={products}
        stockLevels={stockLevels}
        loading={loading}
        error={error}
        query={query}
        onQuery={setQuery}
        onClear={() => {
          setQuery("");
        }}
        onRefresh={refresh}
        onNew={() => {
          setIsCreateOpen(true);
        }}
        onPrintLabel={setLabelProduct}
      />
      <ProductCreateModal
        isOpen={isCreateOpen}
        token={activeToken}
        onClose={() => {
          setIsCreateOpen(false);
        }}
        onCreated={(product, printLabel) => {
          setIsCreateOpen(false);
          setLabelProduct(printLabel ? product : null);
          refresh();
        }}
      />
      {labelProduct !== null && (
        <ProductLabelModal
          product={labelProduct}
          isOpen
          onClose={() => {
            setLabelProduct(null);
          }}
        />
      )}
    </>
  );
}
