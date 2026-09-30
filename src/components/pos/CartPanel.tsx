/**
 * Right active-sale panel (5/12): document switcher, credit HUD,
 * cart table, sticky totals.
 */

import type { ReactElement } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import type { DocType } from "@/types/pos";
import { CartTable } from "@/components/pos/CartTable";
import { CreditHud } from "@/components/pos/CreditHud";
import { TotalsFooter } from "@/components/pos/TotalsFooter";

const DOC_TYPES: DocType[] = ["TICKET", "FACTURE", "BL"];

export function CartPanel(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const docType = useCartStore((state) => state.docType);
  const setDocType = useCartStore((state) => state.setDocType);

  const docLabel = (doc: DocType): string => {
    if (doc === "TICKET") {
      return t(locale, "cart.docTicket");
    }
    if (doc === "FACTURE") {
      return t(locale, "cart.docFacture");
    }
    return t(locale, "cart.docBl");
  };

  return (
    <section className="col-span-5 flex min-h-0 flex-col gap-2">
      <div className="flex shrink-0 gap-1">
        {DOC_TYPES.map((doc) => (
          <button
            key={doc}
            type="button"
            onClick={() => {
              setDocType(doc);
            }}
            className={`flex-1 rounded-lg px-2 py-2 text-xs font-black ${
              docType === doc
                ? "bg-primary-600 text-white"
                : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-300"
            }`}
          >
            {docLabel(doc)}
          </button>
        ))}
      </div>

      <CreditHud />

      <div className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900">
        <CartTable />
      </div>

      <TotalsFooter />
    </section>
  );
}
