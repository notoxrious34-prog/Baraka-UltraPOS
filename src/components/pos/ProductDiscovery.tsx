/**
 * Left discovery panel (7/12): instant search, scanner indicator,
 * type filter chips, and the quick product grid.
 *
 * Category pills are adapted to product-type filters: the backend
 * contract exposes no category endpoint, so filtering uses the
 * available `is_kit` flag (Tous / Kits / Pièces).
 */

import type { ReactElement } from "react";
import { useState } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useProductEditStore } from "@/stores/product-edit-store";
import { useCatalogSearch } from "@/hooks/use-catalog-search";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";

type TypeFilter = "all" | "kits" | "goods";

export function ProductDiscovery(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const searchQuery = usePosUiStore((state) => state.searchQuery);
  const setSearchQuery = usePosUiStore((state) => state.setSearchQuery);
  const warehouse = useSessionStore((state) => state.warehouse);
  const session = useSessionStore((state) => state.session);
  const openModal = usePosUiStore((state) => state.openModal);
  const addProduct = useCartStore((state) => state.addProduct);
  const { results, loading, error, scanFlash } = useCatalogSearch();
  const [typeFilter, setTypeFilter] = useState<TypeFilter>("all");
  const canEditCatalog = session !== null && (session.role === "GERANT" || session.role === "ADMIN");

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
          placeholder={t(locale, "search.placeholder")}
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
            return (
              <div
                key={product.product_id}
                className="flex min-h-24 flex-col items-start gap-1 rounded-lg border border-slate-200 bg-slate-50 p-2 text-start hover:border-primary-600 hover:bg-sky-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-sky-950/40"
              >
                <button
                  type="button"
                  onClick={() => {
                    addProduct(product);
                  }}
                  className="flex w-full min-w-0 flex-col items-start gap-1 text-start"
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
                  </span>
                  <span className="font-mono text-[11px] text-slate-500">{product.sku}</span>
                  <span className="mt-auto font-mono text-sm font-black text-primary-600 dark:text-sky-300">
                    {formatMoney(price, locale)}
                  </span>
                </button>
                {canEditCatalog && (
                  <button
                    type="button"
                    title={t(locale, "editProduct")}
                    aria-label={`${t(locale, "editProduct")} — ${product.sku}`}
                    onClick={() => {
                      useProductEditStore.getState().openForProduct(product);
                      openModal("edit-product");
                    }}
                    className="self-end rounded-md bg-slate-200 px-2 py-0.5 text-[11px] font-black text-slate-600 hover:bg-accent-500 hover:text-slate-950 dark:bg-slate-700 dark:text-slate-300"
                  >
                    ✎ {t(locale, "editProduct")}
                  </button>
                )}
              </div>
            );
          })}
        </div>
      </div>
    </section>
  );
}
