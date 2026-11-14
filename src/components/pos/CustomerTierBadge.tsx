/**
 * Customer pricing-tier badge (Pillar 4, DSN-047; Super-Gros DSN-105).
 * Retail: neutral slate. Artisan: amber PRO. Wholesale: indigo GROS.
 * Super-Gros: emerald bulk.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { CustomerTier, Locale } from "@/types/pos";

const TIER_STYLES: Record<CustomerTier, string> = {
  retail:
    "bg-slate-100 text-slate-700 border-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:border-slate-600",
  artisan:
    "bg-amber-50 text-amber-700 border-amber-300 font-medium dark:bg-amber-950/40 dark:text-amber-400 dark:border-amber-700",
  wholesale:
    "bg-indigo-50 text-indigo-700 border-indigo-300 font-medium dark:bg-indigo-950/40 dark:text-indigo-400 dark:border-indigo-700",
  super_gros:
    "bg-emerald-50 text-emerald-700 border-emerald-300 font-medium dark:bg-emerald-950/40 dark:text-emerald-400 dark:border-emerald-700",
};

const TIER_KEYS = {
  retail: "tier_retail",
  artisan: "tier_artisan",
  wholesale: "tier_wholesale",
  super_gros: "tier_super_gros",
} as const;

export function CustomerTierBadge({
  tier,
  locale,
}: {
  tier: CustomerTier;
  locale: Locale;
}): ReactElement {
  return (
    <span
      className={`inline-flex items-center rounded border px-1.5 py-0.5 text-[10px] font-bold ${TIER_STYLES[tier]}`}
    >
      {t(locale, TIER_KEYS[tier])}
    </span>
  );
}
