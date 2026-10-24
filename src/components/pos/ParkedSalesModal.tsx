/**
 * Parked sales modal (TASK-DSN-026): list active DRAFT documents, resume one
 * into the cart (rehydrate lines + customer, then discard the draft record),
 * or discard directly. Mounted via activeModal === "parked".
 */

import { useCallback, useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useCartStore } from "@/stores/cart-store";
import {
  getDocumentDetails,
  ipcDiscardDraft,
  ipcGetCustomer,
  ipcGetProduct,
  searchDocuments,
  type DocumentSummaryWireDto,
} from "@/lib/ipc";
import { t, translateBackendError } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { ActiveCustomer, CartLine, Locale } from "@/types/pos";

const SEARCH_INPUT_ID = "pos-search-input";

function focusSearch(): void {
  const element = document.getElementById(SEARCH_INPUT_ID);
  if (element instanceof HTMLInputElement) {
    element.focus();
    element.select();
  }
}

function fmtAmount(centimes: string, locale: Locale): string {
  try {
    return formatMoney(centimes, locale);
  } catch {
    return "…";
  }
}

/** Rebuild a cart line from a catalog product + draft item snapshot. */
function toCartLine(
  product: {
    product_id: string;
    sku: string;
    name_ar: string;
    name_fr: string;
    units: Array<{
      unit_id: string;
      product_id: string;
      unit_name_ar: string;
      unit_name_fr: string;
      factor_micro: string;
      barcode: string | null;
      sale_price_centimes: string;
      wholesale_price_centimes: string | null;
      is_base_unit: boolean;
      is_default_selling: boolean;
    }>;
  },
  unitId: string,
  quantityMicro: string,
  index: number,
): CartLine {
  let unitIndex = product.units.findIndex((unit) => unit.unit_id === unitId);
  if (unitIndex < 0) {
    unitIndex = 0;
  }
  return {
    key: `${product.product_id}:${unitId}:${Date.now()}:${index}`,
    productId: product.product_id,
    sku: product.sku,
    nameAr: product.name_ar,
    nameFr: product.name_fr,
    units: product.units.map((unit) => ({
      unitId: unit.unit_id,
      productId: unit.product_id,
      nameAr: unit.unit_name_ar,
      nameFr: unit.unit_name_fr,
      factorMicro: unit.factor_micro,
      barcode: unit.barcode,
      salePriceCentimes: unit.sale_price_centimes,
      wholesalePriceCentimes: unit.wholesale_price_centimes,
      isBaseUnit: unit.is_base_unit,
      isDefaultSelling: unit.is_default_selling,
    })),
    unitIndex,
    quantityMicro,
    lineDiscountCentimes: "0",
  };
}

export function ParkedSalesModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const notify = usePosUiStore((state) => state.notify);
  const [drafts, setDrafts] = useState<DocumentSummaryWireDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const loadDrafts = useCallback(() => {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      setError("UNAUTHORIZED");
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    searchDocuments({
      session_token: token,
      document_type: null,
      status: "DRAFT",
      query: null,
      limit: 20,
    })
      .then((response) => {
        setDrafts(response.documents);
        setLoading(false);
      })
      .catch((failure: unknown) => {
        setError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
        setLoading(false);
      });
  }, []);

  useEffect(() => {
    loadDrafts();
  }, [loadDrafts]);

  const fail = (failure: unknown): void => {
    const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
    setError(code);
  };

  const handleDiscard = (draftId: string): void => {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      setError("UNAUTHORIZED");
      return;
    }
    setBusyId(draftId);
    ipcDiscardDraft(token, draftId)
      .then(() => {
        setBusyId(null);
        notify({ title: t(usePosUiStore.getState().locale, "parked.discardedOk") });
        loadDrafts();
      })
      .catch((failure: unknown) => {
        setBusyId(null);
        fail(failure);
      });
  };

  const handleResume = (draft: DocumentSummaryWireDto): void => {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      setError("UNAUTHORIZED");
      return;
    }
    setBusyId(draft.id);
    getDocumentDetails(draft.id)
      .then(async (details) => {
        const lines: CartLine[] = [];
        for (let index = 0; index < details.items.length; index += 1) {
          const item = details.items[index];
          if (item === undefined) {
            continue;
          }
          const product = await ipcGetProduct(token, item.product_id);
          lines.push(toCartLine(product, item.unit_id, item.quantity_micro, index));
        }
        let customer: ActiveCustomer = { kind: "guest" };
        if (draft.party_id !== null) {
          const detail = await ipcGetCustomer(token, draft.party_id);
          customer = {
            kind: "named",
            id: detail.id,
            name: detail.name,
            policy: detail.credit_policy,
            limitCentimes: detail.credit_limit_centimes,
            balanceCentimes: detail.current_balance_centimes,
            tier: detail.customer_tier,
            source: "backend",
          };
        }
        useCartStore.getState().loadLines(lines, customer);
        await ipcDiscardDraft(token, draft.id);
        setBusyId(null);
        notify({ title: t(usePosUiStore.getState().locale, "parked.resumedOk") });
        closeModal();
        focusSearch();
      })
      .catch((failure: unknown) => {
        setBusyId(null);
        fail(failure);
      });
  };

  return (
    <ModalShell title={t(locale, "parked.title")} onClose={closeModal}>
      {loading && <p className="p-4 text-center text-sm">{t(locale, "search.loading")}</p>}
      {!loading && error !== null && (
        <p className="p-4 text-center text-sm font-bold text-red-600">
          {translateBackendError(error, locale)}
        </p>
      )}
      {!loading && error === null && drafts.length === 0 && (
        <div className="flex flex-col items-center justify-center p-8 text-center text-sm text-ink-muted">
          <p className="font-medium">{t(locale, "parked.empty")}</p>
        </div>
      )}
      {!loading && error === null && drafts.length > 0 && (
        <ul className="flex flex-col gap-2">
          {drafts.map((draft) => (
            <li
              key={draft.id}
              className="flex items-center gap-3 rounded-lg border border-slate-200 p-3 dark:border-slate-700"
            >
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-extrabold">{draft.document_number}</p>
                <p className="text-xs text-ink-muted">
                  {draft.items_count} {t(locale, "parked.itemCount")} ·{" "}
                  {fmtAmount(draft.total_ttc_centimes, locale)}
                </p>
                {draft.party_name !== null && (
                  <p className="truncate text-xs text-ink-muted">{draft.party_name}</p>
                )}
              </div>
              <button
                type="button"
                disabled={busyId !== null}
                onClick={() => {
                  handleResume(draft);
                }}
                className="shrink-0 rounded-md bg-accent-500 px-3 py-1.5 text-xs font-bold text-slate-950 disabled:opacity-50"
              >
                {t(locale, "parked.resume")}
              </button>
              <button
                type="button"
                disabled={busyId !== null}
                onClick={() => {
                  handleDiscard(draft.id);
                }}
                className="shrink-0 rounded-md bg-red-700 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
              >
                {t(locale, "parked.discard")}
              </button>
            </li>
          ))}
        </ul>
      )}
    </ModalShell>
  );
}
