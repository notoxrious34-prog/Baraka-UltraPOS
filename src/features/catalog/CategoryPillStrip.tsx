/**
 * Category pill filter strip (TASK-DSN-044): horizontal high-density
 * category filter for the Master Catalog Hub.
 *
 * Pure presentational component: active categories render as selectable
 * pills (locale-aware label + optional product count), prefixed by the
 * "Tous / الكل" pill and optionally suffixed by the management action pill
 * (rendered only when `onManageCategories` is provided).
 * No floats, no fixed positioning, design tokens only.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Category, Locale } from "@/types/pos";

export interface CategoryPillStripProps {
  locale: Locale;
  categories: Category[];
  selectedCategoryId: string | null;
  onSelectCategory: (id: string | null) => void;
  /** Management CTA (Settings-owned); omit to render a pure filter strip. */
  onManageCategories?: () => void;
  productCounts?: Record<string, number>;
  totalProductsCount: number;
}

/** Locale-aware display name for a category. Pure. */
export function categoryLabel(category: Category, locale: Locale): string {
  return locale === "ar-DZ" ? category.name_ar : category.name_fr;
}

/** Product count for a category (`0` when absent). Pure. */
export function countForCategory(
  productCounts: Record<string, number> | undefined,
  categoryId: string,
): number {
  if (productCounts === undefined) {
    return 0;
  }
  const count = productCounts[categoryId];
  return typeof count === "number" ? count : 0;
}

const ACTIVE_PILL =
  "shrink-0 rounded-full bg-primary-600 px-3 py-1.5 text-xs font-black text-white shadow-sm hover:bg-primary-900";
const IDLE_PILL =
  "shrink-0 rounded-full border border-slate-200 bg-slate-100 px-3 py-1.5 text-xs font-black text-slate-700 hover:bg-slate-200 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700";

export function CategoryPillStrip(props: CategoryPillStripProps): ReactElement {
  const { locale, categories, selectedCategoryId } = props;
  const active = categories.filter((category) => category.is_active);
  return (
    <div
      className="flex shrink-0 items-center gap-2 overflow-x-auto pb-1 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      role="group"
      aria-label={t(locale, "cat.categoryLabel")}
    >
      <button
        type="button"
        onClick={() => {
          props.onSelectCategory(null);
        }}
        aria-pressed={selectedCategoryId === null}
        className={selectedCategoryId === null ? ACTIVE_PILL : IDLE_PILL}
      >
        {t(locale, "cat.allPill")}
        <span className="ms-1 rounded-full bg-black/10 px-1.5 py-0.5 font-mono text-[11px]">
          {props.totalProductsCount}
        </span>
      </button>
      {active.map((category) => {
        const selected = selectedCategoryId === category.id;
        const count =
          props.productCounts === undefined ? null : countForCategory(props.productCounts, category.id);
        return (
          <button
            key={category.id}
            type="button"
            onClick={() => {
              props.onSelectCategory(category.id);
            }}
            aria-pressed={selected}
            title={category.code}
            className={selected ? ACTIVE_PILL : IDLE_PILL}
          >
            {categoryLabel(category, locale)}
            {count !== null && (
              <span className="ms-1 rounded-full bg-black/10 px-1.5 py-0.5 font-mono text-[11px]">
                {count}
              </span>
            )}
          </button>
        );
      })}
      {props.onManageCategories !== undefined && (
        <button
          type="button"
          onClick={props.onManageCategories}
          className="shrink-0 rounded-full border border-dashed border-slate-300 px-3 py-1.5 text-xs font-black text-slate-500 hover:bg-slate-100 dark:border-slate-600 dark:text-slate-400 dark:hover:bg-slate-800"
        >
          {"⚙ "}
          {t(locale, "cat.manageCta")}
        </button>
      )}
    </div>
  );
}
