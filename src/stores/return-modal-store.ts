/**
 * Sale-return cockpit store (TASK-BRK-047 re-issue).
 *
 * Lifecycle: candidate search → return-context load (eligibility plus
 * per-line remaining ceilings) → quantity selection clamped to ceilings →
 * refund-method gating (guest sales can never touch CustomerCredit, Cash
 * needs an open drawer session) → single-flight idempotent dispatch
 * through the canonical `documents_process_return` facade.
 *
 * Money and quantities travel exclusively as canonical strings; every
 * computation uses BigInt (zero floats). The wire `tva_rate` is a micro
 * rate (basis points x 100, matching the backend `taxable * bp / 10_000`
 * kernel), hence the 1_000_000 divisor below. The backend remains the
 * final authority; client-side guards are UX-only.
 */

import { create } from "zustand";
import {
  getDocumentReturnContext,
  processSaleReturn,
  searchDocuments,
  type ProcessReturnResponse,
  type ReturnContextResponse,
} from "@/lib/ipc";
import { parseQtyToMicro } from "@/lib/money";
import { useSessionStore } from "@/stores/session-store";

export type ReturnRefundMethod = "CASH" | "CUSTOMER_CREDIT" | "NO_REFUND";

export interface ReturnTotals {
  totalHtCents: bigint;
  totalTvaCents: bigint;
  totalTtcCents: bigint;
  totalQtyMicro: bigint;
}

/** Half-up `(qty * price + 5000) / 10000` for non-negative magnitudes. */
export function returnLineHtCents(qtyMicro: bigint, unitPriceCents: bigint): bigint {
  if (qtyMicro <= 0n || unitPriceCents <= 0n) {
    return 0n;
  }
  return (qtyMicro * unitPriceCents + 5_000n) / 10_000n;
}

/**
 * Line TVA from the wire micro rate, half-up. The backend kernel computes
 * `taxable * bp / 10_000` with basis points; the wire carries micros
 * (bp x 100), so the equivalent divisor here is 1_000_000.
 */
export function returnLineTvaCents(taxableHtCents: bigint, tvaRateMicro: bigint): bigint {
  if (taxableHtCents <= 0n || tvaRateMicro <= 0n) {
    return 0n;
  }
  return (taxableHtCents * tvaRateMicro + 500_000n) / 1_000_000n;
}

/** Pro-rated line discount, half-up over the original quantity. */
export function returnLineDiscountCents(
  originalDiscountCents: bigint,
  qtyMicro: bigint,
  originalQtyMicro: bigint,
): bigint {
  if (originalDiscountCents <= 0n || qtyMicro <= 0n || originalQtyMicro <= 0n) {
    return 0n;
  }
  return (originalDiscountCents * qtyMicro + originalQtyMicro / 2n) / originalQtyMicro;
}

/** Settlement preview over the selected return quantities. */
export function computeReturnTotals(
  context: ReturnContextResponse | null,
  quantities: Record<string, string>,
): ReturnTotals {
  let totalHtCents = 0n;
  let totalTvaCents = 0n;
  let totalQtyMicro = 0n;
  if (context === null) {
    return { totalHtCents, totalTvaCents, totalTtcCents: 0n, totalQtyMicro };
  }
  for (const item of context.items) {
    const qty = parseMicro(quantities[item.source_item_id] ?? "0");
    if (qty <= 0n) {
      continue;
    }
    const unitPrice = parseMicro(item.unit_price_centimes);
    const ht = returnLineHtCents(qty, unitPrice);
    const discount = returnLineDiscountCents(
      parseMicro(item.discount_centimes),
      qty,
      parseMicro(item.original_quantity_micro),
    );
    const taxable = ht - discount > 0n ? ht - discount : 0n;
    totalHtCents += taxable;
    totalTvaCents += returnLineTvaCents(taxable, BigInt(item.tva_rate));
    totalQtyMicro += qty;
  }
  return {
    totalHtCents,
    totalTvaCents,
    totalTtcCents: totalHtCents + totalTvaCents,
    totalQtyMicro,
  };
}

/** Clamp a candidate quantity into `[0, remaining]`. */
export function clampReturnQty(candidate: bigint, remainingMicro: bigint): bigint {
  if (candidate <= 0n) {
    return 0n;
  }
  return candidate > remainingMicro ? remainingMicro : candidate;
}

/** Guest (comptant) contexts carry no party — CustomerCredit is impossible. */
export function isGuestContext(context: ReturnContextResponse | null): boolean {
  if (context === null) {
    return true;
  }
  return context.party_id === null;
}

function parseMicro(value: string): bigint {
  const trimmed = value.trim();
  if (!/^-?\d+$/.test(trimmed)) {
    return 0n;
  }
  try {
    return BigInt(trimmed);
  } catch {
    return 0n;
  }
}

/** Parse human selling-unit input to micros (invalid → 0n, never throws). */
export function parseReturnQtyInput(text: string): bigint {
  try {
    return BigInt(parseQtyToMicro(text));
  } catch {
    return 0n;
  }
}

interface ReturnModalStoreState {
  isOpen: boolean;
  searchQuery: string;
  isSearching: boolean;
  isLoadingContext: boolean;
  isSubmitting: boolean;
  sourceDocumentId: string | null;
  context: ReturnContextResponse | null;
  /** Selected return magnitude per `source_item_id`, canonical micro string. */
  quantities: Record<string, string>;
  refundMethod: ReturnRefundMethod;
  notes: string;
  errorMessage: string | null;
  successResult: ProcessReturnResponse | null;
  openModal: (prefillDocumentId?: string) => Promise<void>;
  closeModal: () => void;
  setSearchQuery: (query: string) => void;
  searchAndLoadDocument: (documentNumberOrId: string) => Promise<void>;
  setItemQuantity: (itemId: string, qtyMicroStr: string) => void;
  setReturnAll: () => void;
  resetQuantities: () => void;
  setRefundMethod: (method: ReturnRefundMethod) => void;
  setNotes: (notes: string) => void;
  submitReturn: () => Promise<boolean>;
  reset: () => void;
}

const initialState = {
  isOpen: false,
  searchQuery: "",
  isSearching: false,
  isLoadingContext: false,
  isSubmitting: false,
  sourceDocumentId: null as string | null,
  context: null as ReturnContextResponse | null,
  quantities: {} as Record<string, string>,
  refundMethod: "CASH" as ReturnRefundMethod,
  notes: "",
  errorMessage: null as string | null,
  successResult: null as ProcessReturnResponse | null,
};

const UUID_RE =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/;

export const useReturnModalStore = create<ReturnModalStoreState>()((set, get) => {
  async function loadContext(documentId: string): Promise<void> {
    set({
      sourceDocumentId: documentId,
      isLoadingContext: true,
      errorMessage: null,
      successResult: null,
    });
    try {
      const context = await getDocumentReturnContext(documentId);
      if (get().sourceDocumentId !== documentId) {
        return;
      }
      const guest = context.party_id === null;
      set({
        context,
        quantities: {},
        // Guest sales can never touch CustomerCredit: fall back to Cash.
        refundMethod:
          guest && get().refundMethod === "CUSTOMER_CREDIT" ? "CASH" : get().refundMethod,
      });
    } catch (failure: unknown) {
      if (get().sourceDocumentId !== documentId) {
        return;
      }
      set({
        errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR",
        context: null,
        quantities: {},
      });
    } finally {
      if (get().sourceDocumentId === documentId) {
        set({ isLoadingContext: false });
      }
    }
  }

  return {
    ...initialState,

    openModal: async (prefillDocumentId) => {
      set({ ...initialState, isOpen: true });
      if (prefillDocumentId !== undefined && prefillDocumentId.trim() !== "") {
        await get().searchAndLoadDocument(prefillDocumentId.trim());
      }
    },

    closeModal: () => {
      if (get().isSubmitting) {
        return;
      }
      set({ ...initialState });
    },

    setSearchQuery: (query) => {
      set({ searchQuery: query });
    },

    searchAndLoadDocument: async (documentNumberOrId) => {
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
        return;
      }
      const trimmed = documentNumberOrId.trim();
      if (trimmed === "") {
        return;
      }
      // UUIDs load directly; human numbers resolve via the search index.
      if (UUID_RE.test(trimmed)) {
        await loadContext(trimmed);
        return;
      }
      set({ isSearching: true, errorMessage: null });
      try {
        const response = await searchDocuments({
          session_token: token,
          document_type: null,
          status: "FINALIZED",
          query: trimmed,
          limit: 5,
        });
        const first = response.documents[0];
        if (first === undefined) {
          set({ errorMessage: "DOCUMENT_NOT_FOUND" });
          return;
        }
        await loadContext(first.id);
      } catch (failure: unknown) {
        set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
      } finally {
        set({ isSearching: false });
      }
    },

    setItemQuantity: (itemId, qtyMicroStr) => {
      const context = get().context;
      const line = context?.items.find((entry) => entry.source_item_id === itemId);
      const remaining = line !== undefined ? parseMicro(line.remaining_allowable_micro) : 0n;
      const clamped = clampReturnQty(parseMicro(qtyMicroStr), remaining);
      set((state) => ({
        quantities: { ...state.quantities, [itemId]: clamped.toString() },
      }));
    },

    setReturnAll: () => {
      const context = get().context;
      if (context === null) {
        return;
      }
      const quantities: Record<string, string> = {};
      for (const item of context.items) {
        quantities[item.source_item_id] = parseMicro(
          item.remaining_allowable_micro,
        ).toString();
      }
      set({ quantities });
    },

    resetQuantities: () => {
      set({ quantities: {} });
    },

    setRefundMethod: (method) => {
      // Hard gates at the store edge: guest contexts reject CustomerCredit.
      if (method === "CUSTOMER_CREDIT" && isGuestContext(get().context)) {
        return;
      }
      // Cash needs an open drawer session; otherwise fall back to NoRefund
      // is left to the UI hint — the backend re-validates authoritatively.
      set({ refundMethod: method });
    },

    setNotes: (notes) => {
      set({ notes });
    },

    submitReturn: async () => {
      const state = get();
      if (state.isSubmitting || state.context === null) {
        return false;
      }
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ errorMessage: "IPC_NO_ACTIVE_SESSION" });
        return false;
      }
      const picked = state.context.items
        .map((item) => ({
          source_item_id: item.source_item_id,
          quantity_micro: parseMicro(state.quantities[item.source_item_id] ?? "0"),
        }))
        .filter((entry) => entry.quantity_micro > 0n)
        .map((entry) => ({
          source_item_id: entry.source_item_id,
          quantity_micro: entry.quantity_micro.toString(),
        }));
      if (picked.length === 0) {
        return false;
      }
      if (state.refundMethod === "CUSTOMER_CREDIT" && isGuestContext(state.context)) {
        return false;
      }
      // Synchronous lock: set before the first await so same-tick re-entry
      // observes isSubmitting (TASK-BRK-032 precedent).
      set({ isSubmitting: true, errorMessage: null });
      try {
        const trimmedNotes = state.notes.trim();
        const result = await processSaleReturn({
          session_token: token,
          source_document_id: state.context.source_document_id,
          refund_method: state.refundMethod,
          items: picked,
          notes: trimmedNotes === "" ? null : trimmedNotes,
        });
        set({ successResult: result });
        return true;
      } catch (failure: unknown) {
        set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        return false;
      } finally {
        set({ isSubmitting: false });
      }
    },

    reset: () => {
      set({ ...initialState, quantities: {} });
    },
  };
});
