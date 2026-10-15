/**
 * Left discovery panel (7/12): instant search, scanner indicator,
 * type filter chips, and the quick product grid.
 *
 * Category pills are adapted to product-type filters: the backend
 * contract exposes no category endpoint, so filtering uses the
 * available `is_kit` flag (Tous / Kits / Pièces).
 */

import type { ReactElement } from "react";
import { useEffect, useRef, useState } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useCatalogSearch } from "@/hooks/use-catalog-search";
import { ipcGetStock } from "@/lib/ipc";
import { t } from "@/lib/i18n";
import { formatMoney, formatQty } from "@/lib/money";
import type { CatalogProduct } from "@/types/pos";

type TypeFilter = "all" | "kits" | "goods";

/** Exact scan-to-cart resolution (TASK-DSN-040): barcode/SKU hit or sole result. */
export function resolveScanToCart(
  query: string,
  results: CatalogProduct[],
): { product: CatalogProduct; unitIndex: number } | null {
  const trimmed = query.trim();
  if (trimmed === "") {
    return null;
  }
  const lower = trimmed.toLowerCase();
  const exact = results.find(
    (product) =>
      product.barcode?.toLowerCase() === lower ||
      product.sku.toLowerCase() === lower ||
      product.units.some((unit) => unit.barcode?.toLowerCase() === lower),
  );
  const target = exact ?? (results.length === 1 ? results[0] : null);
  if (target === null || target === undefined) {
    return null;
  }
  const matchedUnit =
    target.units.find((unit) => unit.barcode?.toLowerCase() === lower) ??
    target.units.find((unit) => unit.is_base_unit) ??
    target.units[0];
  if (matchedUnit === undefined) {
    return null;
  }
  return { product: target, unitIndex: target.units.indexOf(matchedUnit) };
}

export function ProductDiscovery(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const searchQuery = usePosUiStore((state) => state.searchQuery);
  const setSearchQuery = usePosUiStore((state) => state.setSearchQuery);
  const warehouse = useSessionStore((state) => state.warehouse);
  const session = useSessionStore((state) => state.session);
  const addProduct = useCartStore((state) => state.addProduct);
  const { results, loading, error, scanFlash } = useCatalogSearch();
  const [typeFilter, setTypeFilter] = useState<TypeFilter>("all");
  /** Base-micro stock per visible product (`undefined` = not loaded yet). */
  const [stockById, setStockById] = useState<Record<string, string>>({});
  const [lastScanned, setLastScanned] = useState<{
    name: string;
    sku: string;
    timestamp: number;
  } | null>(null);
  const scanTimer = useRef<number | null>(null);

  // Tile stock chips: best-effort per-product levels for the visible rows,
  // same `stock_get` source as the catalog hub. Failures stay "…".
  useEffect(() => {
    if (session === null || warehouse === null) {
      return;
    }
    const token = session.token;
    const depotId = warehouse.id;
    const rows = results.filter((product) => {
      if (typeFilter === "kits") {
        return product.is_kit;
      }
      if (typeFilter === "goods") {
        return !product.is_kit;
      }
      return true;
    });
    let cancelled = false;
    void Promise.all(
      rows.map(async (product) => {
        try {
          const level = await ipcGetStock(token, product.product_id, depotId);
          return [product.product_id, level] as const;
        } catch {
          return null;
        }
      }),
    ).then((entries) => {
      if (cancelled) {
        return;
      }
      setStockById((prev) => {
        const next = { ...prev };
        for (const entry of entries) {
          if (entry !== null) {
            next[entry[0]] = entry[1];
          }
        }
        return next;
      });
    });
    return () => {
      cancelled = true;
    };
  }, [results, typeFilter, session, warehouse]);

  const handleSearchKeyDown = (event: React.KeyboardEvent<HTMLInputElement>): void => {
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();
    const target = resolveScanToCart(searchQuery, results);
    if (target === null) {
      return;
    }
    addProduct(target.product, target.unitIndex);
    setSearchQuery("");
    event.currentTarget.focus();
    setLastScanned({
      name: locale === "ar-DZ" ? target.product.name_ar : target.product.name_fr,
      sku: target.product.sku,
      timestamp: Date.now(),
    });
    if (scanTimer.current !== null) {
      window.clearTimeout(scanTimer.current);
    }
    scanTimer.current = window.setTimeout(() => {
      setLastScanned(null);
    }, 2000);
  };

  const visible = results.filter((product) => {
    if (typeFilter === "kits") {
      return product.is_kit;
    }
    if (typeFilter === "goods") {
      return !product.is_kit;
    }
    return true;
  });

  const chips: Array<{ id: TypeFilter; label: string }> = [
    { id: "all", label: locale === "ar-DZ" ? "الكل" : "Tous" },
    { id: "kits", label: locale === "ar-DZ" ? "أطقم" : "Kits" },
    { id: "goods", label: locale === "ar-DZ" ? "قطع" : "Pièces" },
  ];

  return (
    <section className="col-span-7 flex min-h-0 flex-col gap-2">
      <div className="flex items-center gap-2">
        <input
          id="pos-search-input"
          value={searchQuery}
          onChange={(event) => {
            setSearchQuery(event.target.value);
          }}
          onKeyDown={handleSearchKeyDown}
          placeholder={t(locale, "pos.scanPrompt")}
          autoComplete="off"
          className="h-11 min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 focus:ring-2 focus:ring-primary-600/30 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
        />
        <span
          className={`flex h-11 shrink-0 items-center gap-1 rounded-lg px-3 text-xs font-bold ${
            scanFlash
              ? "bg-emerald-600 text-white"
              : "bg-slate-200 text-slate-600 dark:bg-slate-800 dark:text-slate-300"
          }`}
        >
          <span className="font-mono">▮▮▮</span>
          {scanFlash ? t(locale, "search.scanning") : t(locale, "search.barcodeHint")}
        </span>
      </div>
      {lastScanned !== null && (
        <div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-xs font-bold text-emerald-700 dark:text-emerald-400 animate-in fade-in duration-150">
          {"✓ "}
          {lastScanned.name}
          {" — "}
          {t(locale, "pos.itemScannedSuccess")}
        </div>
      )}

      <div className="flex shrink-0 gap-1">
        {searchQuery.trim() === "" && visible.length > 0 && (
          <span className="self-center text-xs font-medium text-slate-500 dark:text-slate-400">
            {t(locale, "search.discoveryTitle")}
          </span>
        )}
        {chips.map((chip) => (
          <button
            key={chip.id}
            type="button"
            onClick={() => {
              setTypeFilter(chip.id);
            }}
            className={`rounded-full px-3 py-1 text-xs font-bold ${
              typeFilter === chip.id
                ? "bg-primary-600 text-white"
                : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-200"
            }`}
          >
            {chip.label}
          </button>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-slate-200 bg-white p-2 dark:border-slate-800 dark:bg-slate-900">
        {warehouse === null && (
          <p className="p-4 text-center text-sm font-semibold text-slate-500">
            {t(locale, "search.noWarehouse")}
          </p>
        )}
        {warehouse !== null && loading && (
          <p className="p-4 text-center text-sm font-semibold text-slate-500">
            {t(locale, "search.loading")}
          </p>
        )}
        {warehouse !== null && !loading && error !== null && (
          <p className="p-4 text-center text-sm font-semibold text-red-600">{error}</p>
        )}
        {warehouse !== null && !loading && error === null && visible.length === 0 && (
          <div className="flex flex-col items-center justify-center p-8 text-center text-sm text-slate-500">
            <p className="font-medium">
              {searchQuery.trim() === ""
                ? t(locale, "search.catalogEmpty")
                : t(locale, "search.noResults")}
            </p>
          </div>
        )}
        <div className="grid grid-cols-2 gap-2 xl:grid-cols-3">
          {visible.map((product) => {
            const priced = product.units.find((unit) => unit.is_default_selling) ?? product.units[0];
            const price = priced !== undefined ? priced.sale_price_centimes : "0";
            const unitName =
              priced === undefined
                ? ""
                : locale === "ar-DZ"
                  ? priced.unit_name_ar
                  : priced.unit_name_fr;
            const micro = stockById[product.product_id];
            let chip: ReactElement;
            if (micro === undefined) {
              chip = <span className="font-mono text-[11px] text-slate-400">…</span>;
            } else {
              let empty = false;
              try {
                empty = BigInt(micro) <= 0n;
              } catch {
                empty = false;
              }
              chip = empty ? (
                <span className="rounded bg-red-100 px-1.5 py-0.5 text-[11px] font-semibold text-red-700 dark:bg-red-900/40 dark:text-red-300">
                  0{unitName === "" ? "" : ` ${unitName}`}
                </span>
              ) : (
                <span className="rounded bg-emerald-50 px-1.5 py-0.5 text-[11px] font-semibold text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-400">
                  {formatQty(micro)}
                  {unitName === "" ? "" : ` ${unitName}`}
                </span>
              );
            }
            return (
              <div
                key={product.product_id}
                onClick={() => {
                  addProduct(product);
                }}
                className="flex min-h-24 cursor-pointer flex-col items-start gap-1 rounded-lg border border-slate-200 bg-slate-50 p-2 text-start transition-colors hover:border-primary-600 hover:bg-sky-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-sky-950/40"
              >
                <span className="flex w-full items-center gap-1">
                  <span className="truncate text-sm font-extrabold text-slate-900 dark:text-slate-100">
                    {locale === "ar-DZ" ? product.name_ar : product.name_fr}
                  </span>
                  {product.is_kit && (
                    <span className="shrink-0 rounded bg-violet-600 px-1 text-[10px] font-black text-white">
                      {t(locale, "search.kit")}
                    </span>
                  )}
                  {product.units.length > 1 && (
                    <span
                      title={product.units
                        .map((unit) =>
                          locale === "ar-DZ" ? unit.unit_name_ar : unit.unit_name_fr,
                        )
                        .join(", ")}
                      className="shrink-0 rounded bg-sky-600 px-1 text-[10px] font-black text-white"
                    >
                      {locale === "ar-DZ"
                        ? `${product.units.length} وحدات`
                        : `${product.units.length} unités`}
                    </span>
                  )}
                </span>
                <span className="font-mono text-[11px] text-slate-500">{product.sku}</span>
                {chip}
                <span className="mt-auto font-mono text-sm font-black text-primary-600 dark:text-sky-300">
                  {formatMoney(price, locale)}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
