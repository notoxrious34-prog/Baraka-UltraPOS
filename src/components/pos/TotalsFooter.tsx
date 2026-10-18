/**
 * Sticky totals footer with the Hero Total (JetBrains Mono, fixed height,
 * zero layout shift when amounts change).
 */

import type { ReactElement } from "react";
import { previewTotals, useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";

export function TotalsFooter(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const lines = useCartStore((state) => state.lines);
  const customer = useCartStore((state) => state.customer);
  const globalDiscountCentimes = useCartStore((state) => state.globalDiscountCentimes);
  const totals = previewTotals(
    lines,
    globalDiscountCentimes,
    customer.kind === "named" ? customer.tier : "retail",
  );

  return (
    <div className="shrink-0 border-t border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
      <div className="flex items-center justify-between text-xs font-semibold text-ink-muted">
        <span>{t(locale, "totals.subtotal")}</span>
        <span className="font-mono">{formatMoney(totals.gross, locale)}</span>
      </div>
      <div className="flex items-center justify-between text-xs font-semibold text-ink-muted">
        <span>{t(locale, "totals.discount")}</span>
        <span className="font-mono">−{formatMoney(totals.globalDiscount, locale)}</span>
      </div>
      <div className="flex items-center justify-between text-xs font-semibold text-ink-muted">
        <span>{t(locale, "totals.tva")}</span>
        <span className="font-mono">{formatMoney("0", locale)}</span>
      </div>
      <div className="mt-1 flex h-16 items-center justify-between rounded-lg bg-slate-950 px-3">
        <span className="text-sm font-black uppercase tracking-wide text-amber-400">
          {t(locale, "totals.total")}
        </span>
        <span className="font-mono text-4xl font-black tabular-nums text-amber-400">
          {formatMoney(totals.net, locale)}
        </span>
      </div>
    </div>
  );
}
