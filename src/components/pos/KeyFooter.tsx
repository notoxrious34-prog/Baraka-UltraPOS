/**
 * 56px function-key footer: visual F1–F12 legend.
 * F12 dims when the cart is empty OR no OPEN cash session exists
 * (checkout unavailable; Section 15 bindings unchanged).
 */

import type { ReactElement } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { isCheckoutPermitted, useCashSessionStore } from "@/stores/cash-session-store";
import { t } from "@/lib/i18n";
import type { I18nKey } from "@/lib/i18n";

interface KeyDef {
  code: string;
  labelKey: I18nKey;
  accent?: boolean;
  dimmed?: boolean;
}

export function KeyFooter(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const lineCount = useCartStore((state) => state.lines.length);
  const cashSession = useCashSessionStore((state) => state.session);
  const checkoutReady = lineCount > 0 && isCheckoutPermitted(cashSession);

  const keys: KeyDef[] = [
    { code: "F1", labelKey: "key.f1" },
    { code: "F2", labelKey: "key.f2" },
    { code: "F3", labelKey: "key.f3" },
    { code: "F4", labelKey: "key.f4" },
    { code: "F5", labelKey: "key.f5" },
    { code: "F8", labelKey: "key.f8" },
    { code: "F10", labelKey: "key.f10" },
    { code: "F12", labelKey: "key.f12", accent: true, dimmed: !checkoutReady },
    { code: "Esc", labelKey: "key.esc" },
    { code: "Suppr", labelKey: "key.del" },
    { code: "↑↓", labelKey: "key.arrows" },
  ];

  return (
    <footer className="flex h-14 shrink-0 items-center gap-1 overflow-x-auto bg-slate-900 px-2 dark:bg-black">
      {keys.map((key) => (
        <span
          key={key.code}
          className={`flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-[11px] font-bold ${
            key.accent
              ? key.dimmed
                ? "bg-slate-700 text-slate-400"
                : "bg-accent-500 text-slate-950"
              : "bg-slate-700 text-slate-200"
          }`}
        >
          <kbd className="rounded bg-black/30 px-1 font-mono">{key.code}</kbd>
          {t(locale, key.labelKey)}
        </span>
      ))}
    </footer>
  );
}
