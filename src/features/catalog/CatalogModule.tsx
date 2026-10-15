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
import type { Category, Locale } from "@/types/pos";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { ipcGetStock, ipcListCategories, ipcSearchCatalog, listWarehouses } from "@/lib/ipc";
import { formatMoney, formatQty } from "@/lib/money";
import type { CatalogProduct } from "@/types/pos";
import { CategoryManageModal } from "@/features/catalog/CategoryManageModal";
import { CategoryPillStrip } from "@/features/catalog/CategoryPillStrip";
import { ProductImportModal } from "@/features/catalog/ProductImportModal";
import starterPackCsv from "@/assets/fixtures/quincaillerie_starter_300.csv?raw";
import {
  pickDepotId,
  ProductCreateModal,
} from "@/features/catalog/ProductCreateModal";
import { buildCatalogCsvContent } from "@/features/catalog/catalog-export";
import { downloadTextFile } from "@/features/catalog/desktop-download";
import {
  formFromProduct,
  ProductEditModal,
  submitEditProduct,
} from "@/features/catalog/ProductEditModal";
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

/** Quick-filter tab for the KPI strip. */
export type CatalogFilter = "all" | "in_stock" | "out_of_stock" | "inactive";

/** Sortable hub column. */
export type SortField = "sku" | "name" | "price" | "margin" | "stock";

export type SortDirection = "asc" | "desc";

export interface CatalogSort {
  field: SortField;
  direction: SortDirection;
}

export interface CatalogKpis {
  total: number;
  inStock: number;
  outOfStock: number;
  inactive: number;
}

/** Stock micros as BigInt (`0n` when absent or malformed). Pure. */
export function stockOfBigint(
  product: CatalogProduct,
  stockLevels: Record<string, string>,
): bigint {
  try {
    return BigInt(stockLevels[product.product_id] ?? "0");
  } catch {
    return 0n;
  }
}

function bigintOrZero(raw: string): bigint {
  try {
    return BigInt(raw === "" ? "0" : raw);
  } catch {
    return 0n;
  }
}

/** KPI tallies over the loaded products (BigInt comparisons only). Pure. */
export function computeKpis(
  products: CatalogProduct[],
  stockLevels: Record<string, string>,
): CatalogKpis {
  let inStock = 0;
  let outOfStock = 0;
  let inactive = 0;
  for (const product of products) {
    if (!product.is_active) {
      inactive += 1;
      continue;
    }
    if (stockOfBigint(product, stockLevels) > 0n) {
      inStock += 1;
    } else {
      outOfStock += 1;
    }
  }
  return { total: products.length, inStock, outOfStock, inactive };
}

/** Row predicate for the active KPI filter. Pure. */
export function filterProducts(
  products: CatalogProduct[],
  stockLevels: Record<string, string>,
  filter: CatalogFilter,
): CatalogProduct[] {
  switch (filter) {
    case "in_stock":
      return products.filter(
        (row) => row.is_active && stockOfBigint(row, stockLevels) > 0n,
      );
    case "out_of_stock":
      return products.filter(
        (row) => row.is_active && stockOfBigint(row, stockLevels) <= 0n,
      );
    case "inactive":
      return products.filter((row) => !row.is_active);
    case "all":
    default:
      return [...products];
  }
}

/** Category filter over loaded products (`null` keeps every row). Pure. */
export function filterByCategory(
  products: CatalogProduct[],
  selectedCategoryId: string | null,
): CatalogProduct[] {
  if (selectedCategoryId === null) {
    return [...products];
  }
  return products.filter((row) => row.category_id === selectedCategoryId);
}

/** Per-category product tallies over the loaded rows. Pure. */
export function countProductsByCategory(products: CatalogProduct[]): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const row of products) {
    if (row.category_id === null) {
      continue;
    }
    counts[row.category_id] = (counts[row.category_id] ?? 0) + 1;
  }
  return counts;
}

/** Category lookup by id (`null` when unknown). Pure. */
export function categoryById(
  categories: Category[],
  categoryId: string | null,
): Category | null {
  if (categoryId === null) {
    return null;
  }
  return categories.find((row) => row.id === categoryId) ?? null;
}

/** Signed margin amount in centimes as BigInt (`0n` when unit-less). Pure. */
export function marginOfBigint(product: CatalogProduct): bigint {
  const base = baseUnitOf(product);
  if (base === null) {
    return 0n;
  }
  return bigintOrZero(base.sale_price_centimes) - bigintOrZero(base.purchase_price_centimes);
}

function compareField(
  field: SortField,
  a: CatalogProduct,
  b: CatalogProduct,
  stockLevels: Record<string, string>,
): number {
  switch (field) {
    case "sku":
      return a.sku.localeCompare(b.sku);
    case "name":
      return a.name_fr.localeCompare(b.name_fr);
    case "price": {
      const pa = bigintOrZero(baseUnitOf(a)?.sale_price_centimes ?? "0");
      const pb = bigintOrZero(baseUnitOf(b)?.sale_price_centimes ?? "0");
      return pa < pb ? -1 : pa > pb ? 1 : 0;
    }
    case "margin": {
      const ma = marginOfBigint(a);
      const mb = marginOfBigint(b);
      return ma < mb ? -1 : ma > mb ? 1 : 0;
    }
    case "stock": {
      const sa = stockOfBigint(a, stockLevels);
      const sb = stockOfBigint(b, stockLevels);
      return sa < sb ? -1 : sa > sb ? 1 : 0;
    }
  }
}

/** Sorted copy of the rows (`null` sort keeps load order). Pure. */
export function sortProducts(
  products: CatalogProduct[],
  stockLevels: Record<string, string>,
  sort: CatalogSort | null,
): CatalogProduct[] {
  const rows = [...products];
  if (sort === null) {
    return rows;
  }
  const sign = sort.direction === "asc" ? 1 : -1;
  rows.sort((a, b) => sign * compareField(sort.field, a, b, stockLevels));
  return rows;
}

/** Next sort state for a header click (toggle or fresh asc). Pure. */
export function nextSort(current: CatalogSort | null, field: SortField): CatalogSort {
  if (current !== null && current.field === field) {
    return { field, direction: current.direction === "asc" ? "desc" : "asc" };
  }
  return { field, direction: "asc" };
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
  togglingId: string | null;
  filter: CatalogFilter;
  kpis: CatalogKpis;
  sort: CatalogSort | null;
  onQuery: (value: string) => void;
  onClear: () => void;
  onRefresh: () => void;
  onNew: () => void;
  onPrintLabel: (product: CatalogProduct) => void;
  onEdit: (product: CatalogProduct) => void;
  onToggleActive: (product: CatalogProduct) => void;
  onFilter: (filter: CatalogFilter) => void;
  onSort: (field: SortField) => void;
  onExport: () => void;
  onImport?: () => void;
  /**
   * Last exported filename for the inline confirmation note (TASK-REL-055).
   * Filename only — no authored copy, per the UI-string law.
   */
  exportNote?: string | null;
  /** BRK-069: preload the 300-item starter pack into the import modal. */
  onStarterPack?: () => void;
  /** Taxonomy rows for the pill strip + row badges (empty = strip hidden details). */
  categories?: Category[];
  selectedCategoryId?: string | null;
  onSelectCategory?: (id: string | null) => void;
  onManageCategories?: () => void;
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
  const { locale, products, stockLevels, loading, error, query, filter, kpis, sort } = props;
  const categories = props.categories ?? [];
  const selectedCategoryId = props.selectedCategoryId ?? null;

  const sortMark = (field: SortField): string => {
    if (sort === null || sort.field !== field) {
      return "↕";
    }
    return sort.direction === "asc" ? "▲" : "▼";
  };

  const sortButton = (
    label: string,
    field: SortField,
    alignEnd: boolean,
  ): ReactElement => (
    <button
      type="button"
      onClick={() => {
        props.onSort(field);
      }}
      aria-label={`${label} (${sort !== null && sort.field === field ? sort.direction : "unsorted"})`}
      className={`inline-flex items-center gap-1 font-black hover:text-primary-600 ${alignEnd ? "w-full justify-end" : ""}`}
    >
      <span>{label}</span>
      <span aria-hidden="true" className="text-[10px] opacity-60">
        {sortMark(field)}
      </span>
    </button>
  );

  const kpiCards: Array<{
    key: CatalogFilter;
    title: string;
    count: number;
    dot: string;
    ring: string;
  }> = [
    {
      key: "all",
      title: t(locale, "cat.kpiAll"),
      count: kpis.total,
      dot: "bg-slate-400",
      ring: "ring-slate-500",
    },
    {
      key: "in_stock",
      title: t(locale, "cat.kpiInStock"),
      count: kpis.inStock,
      dot: "bg-emerald-500",
      ring: "ring-emerald-500",
    },
    {
      key: "out_of_stock",
      title: t(locale, "cat.kpiOutOfStock"),
      count: kpis.outOfStock,
      dot: "bg-red-500",
      ring: "ring-red-500",
    },
    {
      key: "inactive",
      title: t(locale, "cat.kpiInactive"),
      count: kpis.inactive,
      dot: "bg-amber-500",
      ring: "ring-amber-500",
    },
  ];
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
            id="catalog-search-input"
            autoFocus
            onChange={(event) => {
              props.onQuery(event.target.value);
            }}
            placeholder={t(locale, "cat.searchPlaceholder")}
            aria-label={t(locale, "cat.search")}
            title={t(locale, "cat.quickSearchHelp")}
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
          onClick={() => props.onImport?.()}
          className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
        >
          {"⤒ "}
          {t(locale, "cat.importCsv")}
        </button>
        <button
          type="button"
          onClick={() => props.onStarterPack?.()}
          title={t(locale, "catalog.starterPackLoaded")}
          className="h-10 rounded-lg bg-amber-100 px-4 text-sm font-black text-amber-900 hover:bg-amber-200 dark:bg-amber-900/40 dark:text-amber-200 dark:hover:bg-amber-900/60"
        >
          {"✦ "}
          {t(locale, "catalog.starterPackBtn")}
        </button>
        <button
          type="button"
          onClick={props.onExport}
          className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
        >
          {"⬇️ "}
          {t(locale, "cat.exportCsv")}
        </button>
        {props.exportNote !== null &&
          props.exportNote !== undefined &&
          props.exportNote !== "" && (
            <span
              role="status"
              className="inline-flex h-10 items-center rounded-lg bg-emerald-100 px-3 font-mono text-xs font-black text-emerald-800 dark:bg-emerald-900 dark:text-emerald-200"
            >
              {"✓ "}
              {props.exportNote}
            </span>
          )}
        <button
          type="button"
          onClick={props.onNew}
          className="h-10 rounded-lg bg-accent-500 px-4 text-sm font-black text-slate-950 hover:bg-amber-400"
        >
          {"➕ "}
          {t(locale, "cat.newProduct")}
        </button>
      </div>

      <div className="grid shrink-0 grid-cols-2 gap-3 sm:grid-cols-4">
        {kpiCards.map((card) => {
          const selected = filter === card.key;
          return (
            <button
              key={card.key}
              type="button"
              onClick={() => {
                props.onFilter(card.key);
              }}
              aria-pressed={selected}
              className={`flex items-center gap-2 rounded-lg border bg-white p-3 text-start transition-all hover:shadow dark:bg-slate-900 ${
                selected
                  ? `border-transparent ring-2 ${card.ring}`
                  : "border-slate-200 dark:border-slate-700"
              }`}
            >
              <span aria-hidden="true" className={`h-2.5 w-2.5 shrink-0 rounded-full ${card.dot}`} />
              <span className="flex min-w-0 flex-col">
                <span className="truncate text-xs font-bold opacity-70">{card.title}</span>
                <span className="text-xl font-black leading-none">{card.count}</span>
              </span>
            </button>
          );
        })}
      </div>

      {loading && <p className="shrink-0 text-xs font-bold">{t(locale, "cash.loading")}</p>}
      {error !== null && (
        <p className="shrink-0 rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(error, locale)}
        </p>
      )}
      {props.onSelectCategory !== undefined && props.onManageCategories !== undefined && (
        <CategoryPillStrip
          locale={locale}
          categories={categories}
          selectedCategoryId={selectedCategoryId}
          onSelectCategory={props.onSelectCategory}
          onManageCategories={props.onManageCategories}
          productCounts={countProductsByCategory(products)}
          totalProductsCount={products.length}
        />
      )}

      <div className="min-h-0 flex-1 overflow-y-auto rounded-xl border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900">
        <table className="w-full border-collapse text-xs">
          <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
            <tr>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {sortButton(t(locale, "cat.colSku"), "sku", false)}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colBarcode")}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {sortButton(t(locale, "cat.colName"), "name", false)}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colCategory")}
              </th>
              <th scope="col" className="px-3 py-2 text-start font-black">
                {t(locale, "cat.colBaseUnit")}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {t(locale, "cat.colPurchasePrice")}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {sortButton(t(locale, "cat.colSalePrice"), "price", true)}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {sortButton(t(locale, "cat.colMargin"), "margin", true)}
              </th>
              <th scope="col" className="px-3 py-2 text-end font-black">
                {sortButton(t(locale, "cat.colStock"), "stock", true)}
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
                  <td className="px-3 py-1.5">
                    {(() => {
                      const category = categoryById(categories, product.category_id);
                      if (category === null) {
                        return <span className="font-bold text-slate-400">—</span>;
                      }
                      return (
                        <span
                          title={category.code}
                          className="rounded-md bg-slate-100 px-2 py-0.5 font-bold text-slate-600 dark:bg-slate-800 dark:text-slate-300"
                        >
                          {locale === "ar-DZ" ? category.name_ar : category.name_fr}
                        </span>
                      );
                    })()}
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
                      <button
                        type="button"
                        disabled={props.togglingId === product.product_id}
                        onClick={() => {
                          props.onToggleActive(product);
                        }}
                        title={t(locale, "cat.productActiveLabel")}
                        aria-label={t(locale, "cat.productActiveLabel")}
                        className="rounded-md bg-green-100 px-2 py-0.5 font-black text-green-700 hover:bg-green-200 disabled:opacity-50 dark:bg-green-900/40 dark:text-green-200 dark:hover:bg-green-900/60"
                      >
                        {t(locale, "cat.active")}
                      </button>
                    ) : (
                      <button
                        type="button"
                        disabled={props.togglingId === product.product_id}
                        onClick={() => {
                          props.onToggleActive(product);
                        }}
                        title={t(locale, "cat.productActiveLabel")}
                        aria-label={t(locale, "cat.productActiveLabel")}
                        className="rounded-md bg-slate-200 px-2 py-0.5 font-black text-slate-500 hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-300 dark:hover:bg-slate-600"
                      >
                        {t(locale, "cat.inactive")}
                      </button>
                    )}
                  </td>
                  <td className="px-3 py-1.5">
                    <span className="flex items-center gap-1.5">
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
                      <button
                        type="button"
                        onClick={() => {
                          props.onEdit(product);
                        }}
                        aria-label={t(locale, "cat.editProduct")}
                        title={t(locale, "cat.editProduct")}
                        className="rounded-md bg-slate-200 px-2 py-1 text-xs font-black hover:bg-slate-300 dark:bg-slate-700 dark:hover:bg-slate-600"
                      >
                        ✏️
                      </button>
                    </span>
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
  const [editProduct, setEditProduct] = useState<CatalogProduct | null>(null);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [filter, setFilter] = useState<CatalogFilter>("all");
  /** Last exported filename for the inline confirmation note (TASK-REL-055). */
  const [exportNote, setExportNote] = useState<string | null>(null);
  const [sort, setSort] = useState<CatalogSort | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null);
  const [isCategoryModalOpen, setIsCategoryModalOpen] = useState(false);
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);
  const [importPreset, setImportPreset] = useState<{
    key: number;
    filename: string;
    content: string;
  } | null>(null);
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

  // F1 focuses + selects the search input for hands-free wedge scanners.
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.key !== "F1") {
        return;
      }
      event.preventDefault();
      const input = document.getElementById("catalog-search-input");
      if (input instanceof HTMLInputElement) {
        input.focus();
        input.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  // Category taxonomy for the pill strip + row badges (inactive included
  // so reassigned rows still resolve their label).
  useEffect(() => {
    if (token === null) {
      return;
    }
    const activeToken: string = token;
    let cancelled = false;
    ipcListCategories(activeToken, true)
      .then((rows) => {
        if (!cancelled) {
          setCategories(rows);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setCategories([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

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

  const kpis = computeKpis(products, stockLevels);
  const visible = sortProducts(
    filterByCategory(filterProducts(products, stockLevels, filter), selectedCategoryId),
    stockLevels,
    sort,
  );

  const reloadCategories = (): void => {
    ipcListCategories(activeToken, true)
      .then((rows) => {
        setCategories(rows);
      })
      .catch(() => {
        // The hub stays usable on taxonomy failures; rows show "—".
      });
  };

  const handleFilter = (next: CatalogFilter): void => {
    setFilter((current) => (current === next ? "all" : next));
  };

  const handleSort = (field: SortField): void => {
    setSort((current) => nextSort(current, field));
  };

  const handleExport = (): void => {
    const stamp = new Date().toISOString().slice(0, 10);
    const outcome = downloadTextFile(
      buildCatalogCsvContent(visible, stockLevels, locale),
      `${t(locale, "cat.csvExportFilename")}-${stamp}.csv`,
    );
    setExportNote(outcome.ok ? outcome.filename : null);
    if (outcome.ok) {
      window.setTimeout(() => {
        setExportNote(null);
      }, 8000);
    }
  };

  const handleUpdated = (updated: CatalogProduct): void => {
    setProducts((prev) =>
      prev.map((row) => (row.product_id === updated.product_id ? updated : row)),
    );
    setEditProduct(null);
  };

  // Quick status toggle: full-state update with the flag flipped, then the
  // fresh row swapped in place (stock cache untouched).
  const toggleActive = (target: CatalogProduct): void => {
    if (togglingId !== null) {
      return;
    }
    setTogglingId(target.product_id);
    const data = { ...formFromProduct(target), isActive: !target.is_active };
    submitEditProduct(activeToken, target, data)
      .then((updated) => {
        setProducts((prev) =>
          prev.map((row) => (row.product_id === updated.product_id ? updated : row)),
        );
      })
      .catch((failure: unknown) => {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      })
      .finally(() => {
        setTogglingId(null);
      });
  };

  return (
    <>
      <CatalogHubView
        locale={locale}
        products={visible}
        stockLevels={stockLevels}
        loading={loading}
        error={error}
        query={query}
        togglingId={togglingId}
        filter={filter}
        kpis={kpis}
        sort={sort}
        onQuery={setQuery}
        onClear={() => {
          setQuery("");
        }}
        onRefresh={refresh}
        onNew={() => {
          setIsCreateOpen(true);
        }}
        onPrintLabel={setLabelProduct}
        onEdit={setEditProduct}
        onToggleActive={toggleActive}
        onFilter={handleFilter}
        onSort={handleSort}
        onExport={handleExport}
        exportNote={exportNote}
        onImport={() => {
          setImportPreset(null);
          setIsImportModalOpen(true);
        }}
        onStarterPack={() => {
          setImportPreset({
            key: Date.now(),
            filename: "quincaillerie_starter_300.csv",
            content: starterPackCsv,
          });
          setIsImportModalOpen(true);
        }}
        categories={categories}
        selectedCategoryId={selectedCategoryId}
        onSelectCategory={setSelectedCategoryId}
        onManageCategories={() => {
          setIsCategoryModalOpen(true);
        }}
      />
      <CategoryManageModal
        isOpen={isCategoryModalOpen}
        token={activeToken}
        onClose={() => {
          setIsCategoryModalOpen(false);
        }}
        onChanged={() => {
          reloadCategories();
          refresh();
        }}
      />
      <ProductImportModal
        isOpen={isImportModalOpen}
        token={activeToken}
        onClose={() => {
          setIsImportModalOpen(false);
        }}
        onSuccess={() => {
          reloadCategories();
          refresh();
        }}
        categories={categories}
        preset={importPreset}
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
      <ProductEditModal
        isOpen={editProduct !== null}
        product={editProduct}
        token={activeToken}
        onClose={() => {
          setEditProduct(null);
        }}
        onUpdated={handleUpdated}
      />
    </>
  );
}
