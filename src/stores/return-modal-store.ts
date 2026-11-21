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
  ipcSearchCatalog,
  processSaleExchange,
  processSaleReturn,
  searchDocuments,
  type ProcessExchangeResponse,
  type ProcessReturnResponse,
  type ReturnContextResponse,
} from "@/lib/ipc";
import { parseQtyToMicro } from "@/lib/money";
import { useSessionStore } from "@/stores/session-store";
import type { CatalogProduct } from "@/types/pos";

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
    // Per-unit path (BRK-096/DSN-097): the wire carries the exact net unit
    // price transcribed at the edge — the discount already lives in it, so
    // the preview prices picked x net PU verbatim. Legacy path (null):
    // raw PU x picked minus the lump pro-rating, exactly as before.
    const netPrice =
      item.discounted_unit_price_centimes !== null
        ? parseMicro(item.discounted_unit_price_centimes)
        : null;
    const ht = returnLineHtCents(qty, netPrice ?? parseMicro(item.unit_price_centimes));
    const taxable =
      netPrice !== null
        ? ht
        : (() => {
            const discount = returnLineDiscountCents(
              parseMicro(item.discount_centimes),
              qty,
              parseMicro(item.original_quantity_micro),
            );
            const net = ht - discount;
            return net > 0n ? net : 0n;
          })();
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

/** One alternate size/color sold inside an exchange (canonical strings). */
export interface ExchangeAlternate {
  product_id: string;
  product_name: string;
  unit_id: string;
  unit_name: string;
  /** Selected magnitude in selling-unit micros, canonical string. */
  quantity_micro: string;
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
  /** Alternate sizes/colors picked for an Échange Express. */
  exchangeAlternates: ExchangeAlternate[];
  altSearchQuery: string;
  altSearchResults: CatalogProduct[];
  isSearchingAlts: boolean;
  managerUsername: string;
  managerPin: string;
  exchangeResult: ProcessExchangeResponse | null;
  exchangeError: string | null;
  isExchanging: boolean;
  openModal: (prefillDocumentId?: string) => Promise<void>;
  closeModal: () => void;
  setSearchQuery: (query: string) => void;
  searchAndLoadDocument: (documentNumberOrId: string) => Promise<void>;
  /** Monotonic lookup ticket (TASK-DSN-067 stale guard); not UI state. */
  searchTicket: number;
  setItemQuantity: (itemId: string, qtyMicroStr: string) => void;
  setReturnAll: () => void;
  resetQuantities: () => void;
  setRefundMethod: (method: ReturnRefundMethod) => void;
  setNotes: (notes: string) => void;
  submitReturn: () => Promise<boolean>;
  reset: () => void;
  setAltSearchQuery: (query: string) => void;
  searchAlternates: (query: string) => Promise<void>;
  addAlternate: (product: CatalogProduct, unitId: string, qtyMicroStr: string) => void;
  removeAlternate: (index: number) => void;
  setManagerUsername: (username: string) => void;
  setManagerPin: (pin: string) => void;
  submitExchange: () => Promise<boolean>;
}

const initialState = {
  isOpen: false,
  searchQuery: "",
  searchTicket: 0,
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
  exchangeAlternates: [] as ExchangeAlternate[],
  altSearchQuery: "",
  altSearchResults: [] as CatalogProduct[],
  isSearchingAlts: false,
  managerUsername: "",
  managerPin: "",
  exchangeResult: null as ProcessExchangeResponse | null,
  exchangeError: null as string | null,
  isExchanging: false,
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
        // A freshly loaded source invalidates any staged exchange.
        exchangeAlternates: [],
        altSearchQuery: "",
        altSearchResults: [],
        exchangeResult: null,
        exchangeError: null,
        managerPin: "",
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
      // Monotonic ticket (TASK-DSN-067): a newer lookup supersedes this one.
      const ticket = get().searchTicket + 1;
      set({ searchTicket: ticket });
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
        if (get().searchTicket !== ticket) {
          return;
        }
        const first = response.documents[0];
        if (first === undefined) {
          set({ errorMessage: "DOCUMENT_NOT_FOUND" });
          return;
        }
        await loadContext(first.id);
      } catch (failure: unknown) {
        if (get().searchTicket === ticket) {
          set({ errorMessage: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        }
      } finally {
        if (get().searchTicket === ticket) {
          set({ isSearching: false });
        }
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

    setAltSearchQuery: (query) => {
      set({ altSearchQuery: query });
    },

    searchAlternates: async (query) => {
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ exchangeError: "IPC_NO_ACTIVE_SESSION" });
        return;
      }
      const trimmed = query.trim();
      if (trimmed.length < 2) {
        set({ altSearchResults: [] });
        return;
      }
      set({ isSearchingAlts: true, exchangeError: null });
      try {
        const results = await ipcSearchCatalog(token, trimmed, 10, false);
        set({ altSearchResults: results });
      } catch (failure: unknown) {
        set({
          exchangeError: typeof failure === "string" ? failure : "INTERNAL_ERROR",
          altSearchResults: [],
        });
      } finally {
        set({ isSearchingAlts: false });
      }
    },

    addAlternate: (product, unitId, qtyMicroStr) => {
      const unit = product.units.find((entry) => entry.unit_id === unitId);
      const qty = parseMicro(qtyMicroStr);
      if (unit === undefined || qty <= 0n) {
        return;
      }
      const name = product.name_fr !== "" ? product.name_fr : product.name_ar;
      const unitName =
        unit.unit_name_fr !== "" ? unit.unit_name_fr : unit.unit_name_ar;
      set((state) => ({
        exchangeAlternates: [
          ...state.exchangeAlternates,
          {
            product_id: product.product_id,
            product_name: name,
            unit_id: unit.unit_id,
            unit_name: unitName,
            quantity_micro: qty.toString(),
          },
        ],
        altSearchQuery: "",
        altSearchResults: [],
      }));
    },

    removeAlternate: (index) => {
      set((state) => ({
        exchangeAlternates: state.exchangeAlternates.filter((_, i) => i !== index),
      }));
    },

    setManagerUsername: (username) => {
      set({ managerUsername: username });
    },

    setManagerPin: (pin) => {
      set({ managerPin: pin });
    },

    submitExchange: async () => {
      const state = get();
      if (state.isExchanging || state.context === null) {
        return false;
      }
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ exchangeError: "IPC_NO_ACTIVE_SESSION" });
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
      if (picked.length === 0 || state.exchangeAlternates.length === 0) {
        return false;
      }
      if (state.refundMethod === "CUSTOMER_CREDIT" && isGuestContext(state.context)) {
        return false;
      }
      const username = state.managerUsername.trim();
      // Synchronous lock: set before the first await so same-tick re-entry
      // observes isExchanging (TASK-BRK-032 precedent).
      set({ isExchanging: true, exchangeError: null });
      try {
        const trimmedNotes = state.notes.trim();
        const result = await processSaleExchange({
          session_token: token,
          source_document_id: state.context.source_document_id,
          return_items: picked,
          new_items: state.exchangeAlternates.map((alt) => ({
            product_id: alt.product_id,
            unit_id: alt.unit_id,
            quantity_micro: alt.quantity_micro,
          })),
          refund_method: state.refundMethod,
          below_cost_override:
            username === "" || state.managerPin === ""
              ? null
              : { manager_username: username, manager_pin: state.managerPin },
          notes: trimmedNotes === "" ? null : trimmedNotes,
        });
        set({ exchangeResult: result, managerPin: "" });
        return true;
      } catch (failure: unknown) {
        set({ exchangeError: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
        return false;
      } finally {
        set({ isExchanging: false });
      }
    },

    reset: () => {
      set({ ...initialState, quantities: {} });
    },
  };
});
