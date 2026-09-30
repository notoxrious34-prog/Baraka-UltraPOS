/**
 * Catalog module root (TASK-BRK-029, creation flow only): product creation
 * form plus a search/list panel reusing the POS catalog_search bridge, so a
 * freshly created product is immediately verifiable. Manager-only; mounted
 * via the non-POS NavRail view switch in App.tsx. Update/edit, categories,
 * kits, and label printing are deferred to future tasks.
 */

import { useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcSearchCatalog } from "@/lib/ipc";
import { formatMoney } from "@/lib/money";
import type { CatalogProduct } from "@/types/pos";
import { ProductCreateForm } from "@/features/catalog/ProductCreateForm";

/** Same manager rule as App.tsx isManagerRole (ADMIN or GERANT). */
function isManagerRole(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

function baseSalePrice(product: CatalogProduct): string | null {
  const base = product.units.find((unit) => unit.is_base_unit);
  return base === undefined ? null : base.sale_price_centimes;
}

export function CatalogModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [query, setQuery] = useState("");
  const [rows, setRows] = useState<CatalogProduct[]>([]);
  const [loading, setLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);

  // Defense in depth: the screen verifies the role itself, independent of
  // NavRail visibility (URL/state manipulation must never expose the form).
  if (session === null || !isManagerRole(session.role)) {
    return (
      <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError("UNAUTHORIZED", locale)}
        </p>
      </section>
    );
  }
  const token = session.token;

  const runSearch = (term: string): void => {
    const trimmed = term.trim();
    if (trimmed === "") {
      setRows([]);
      setListError(null);
      return;
    }
    setLoading(true);
    // Same bridge the POS Cockpit uses: a hit here is sellable there.
    ipcSearchCatalog(token, trimmed, 50)
      .then((products) => {
        setRows(products);
        setListError(null);
      })
      .catch((failure: unknown) => {
        setListError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        setRows([]);
      })
      .finally(() => {
        setLoading(false);
      });
  };

  return (
    <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
      <h2 className="text-lg font-black">{t(locale, "cat.title")}</h2>
      <ProductCreateForm
        token={token}
        onCreated={(sku) => {
          setQuery(sku);
          runSearch(sku);
        }}
      />
      <div className="flex flex-col gap-2 rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900">
        <h3 className="text-sm font-black">{t(locale, "cat.listTitle")}</h3>
        <div className="flex gap-2">
          <input
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                runSearch(query);
              }
            }}
            placeholder={t(locale, "cat.searchPlaceholder")}
            className="h-10 flex-1 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
          />
          <button
            type="button"
            onClick={() => {
              runSearch(query);
            }}
            className="h-10 rounded-lg bg-primary-600 px-4 text-sm font-black text-white"
          >
            {t(locale, "cat.search")}
          </button>
        </div>
        {loading && <p className="text-xs font-bold">{t(locale, "cash.loading")}</p>}
        {listError !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {translateBackendError(listError, locale)}
          </p>
        )}
        <ul className="flex flex-col gap-1">
          {rows.map((product) => {
            const price = baseSalePrice(product);
            return (
              <li
                key={product.product_id}
                className="flex items-center justify-between gap-2 rounded-md bg-slate-100 px-3 py-1.5 text-xs font-bold dark:bg-slate-800"
              >
                <span>
                  {product.sku} — {locale === "ar-DZ" ? product.name_ar : product.name_fr}
                </span>
                <span>{price === null ? "—" : formatMoney(price, locale)}</span>
              </li>
            );
          })}
        </ul>
      </div>
    </section>
  );
}
