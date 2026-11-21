/**
 * Sticky totals footer with the Hero Total (JetBrains Mono, fixed height,
 * zero layout shift when amounts change).
 */

import type { ReactElement } from "react";
import { previewTotals, useCartStore } from "@/stores/cart-store";
import { isCheckoutPermitted, useCashSessionStore } from "@/stores/cash-session-store";
import { isCreditBlocked } from "@/hooks/use-pos-keyboard";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import { maskMoney } from "@/lib/money";
import { usePrivacyShieldStore } from "@/stores/privacy-shield-store";

export function TotalsFooter(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const shielded = usePrivacyShieldStore((state) => state.shielded);
  const openModal = usePosUiStore((state) => state.openModal);
  const lines = useCartStore((state) => state.lines);
  const customer = useCartStore((state) => state.customer);
  const cashSession = useCashSessionStore((state) => state.session);
  const globalDiscountCentimes = useCartStore((state) => state.globalDiscountCentimes);
  const totals = previewTotals(
    lines,
    globalDiscountCentimes,
    customer.kind === "named" ? customer.tier : "retail",
  );
  // Single primary checkout action (P1): mirrors the F12 key guard exactly —
  // non-empty cart, OPEN session, no credit block. The KeyFooter stays a legend.
  const checkoutReady =
    lines.length > 0 && isCheckoutPermitted(cashSession) && !isCreditBlocked();

  return (
    <div className="shrink-0 border-t border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
      <div className="flex items-center justify-between text-xs font-semibold text-ink-muted">
        <span>{t(locale, "totals.subtotal")}</span>
        <span className="font-mono">{maskMoney(shielded, totals.gross, locale)}</span>
      </div>
      <div className="flex items-center justify-between text-xs font-semibold text-ink-muted">
        <span>{t(locale, "totals.discount")}</span>
        <span className="font-mono">−{maskMoney(shielded, totals.globalDiscount, locale)}</span>
      </div>
      <div className="flex items-center justify-between text-xs font-semibold text-ink-muted">
        <span>{t(locale, "totals.tva")}</span>
        <span className="font-mono">{maskMoney(shielded, "0", locale)}</span>
      </div>
      <div className="mt-1 flex h-16 items-center justify-between rounded-lg bg-slate-950 px-3">
        <span className="text-sm font-black uppercase tracking-wide text-amber-400">
          {t(locale, "totals.total")}
        </span>
        <span className="font-mono text-4xl font-black tabular-nums text-amber-400">
          {maskMoney(shielded, totals.net, locale)}
        </span>
      </div>
      <button
        type="button"
        disabled={!checkoutReady}
        onClick={() => {
          openModal("checkout");
        }}
        className="mt-2 h-12 w-full rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-40"
      >
        {t(locale, "checkout.title")}
      </button>
    </div>
  );
}
