/**
 * Cockpit layout view: topbar, blocking cash banner, 7/5 grid, key footer.
 * F12 checkout is fully disabled without an OPEN session; F1 search and
 * product browsing stay functional (only the final checkout is blocked).
 */

import type { ReactElement } from "react";
import { Topbar } from "@/components/pos/Topbar";
import { KeyFooter } from "@/components/pos/KeyFooter";
import { ProductDiscovery } from "@/components/pos/ProductDiscovery";
import { CartPanel } from "@/components/pos/CartPanel";
import { isCheckoutPermitted, useCashSessionStore } from "@/stores/cash-session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";

export function PosShellView(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const cashSession = useCashSessionStore((state) => state.session);
  const blocked = !isCheckoutPermitted(cashSession);

  return (
    <>
      <Topbar />
      {blocked && (
        <div
          role="alert"
          className="flex shrink-0 items-center justify-center gap-3 bg-red-700 px-3 py-2 text-center text-xs font-black text-white"
        >
          <span>{t(locale, "cash.noSessionBanner")}</span>
          <span lang="ar" dir="rtl">
            {locale === "ar-DZ"
              ? "يرجى فتح جلسة الصندوق قبل البيع."
              : "Veuillez ouvrir une session de caisse avant de vendre."}
          </span>
        </div>
      )}
      <main className="grid min-h-0 flex-1 grid-cols-12 gap-2 p-2">
        <ProductDiscovery />
        <CartPanel />
      </main>
      <KeyFooter />
    </>
  );
}
