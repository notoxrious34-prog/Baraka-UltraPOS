/**
 * Document conversion modal store (TASK-BRK-043).
 *
 * Owns the Devis→BL / BL→Facture lookup-and-execute lifecycle: candidate
 * search, document selection with fresh details, BigInt-centime tender
 * settlement, and single-flight submission with a session-stable
 * idempotency key. Money travels exclusively as canonical centime strings;
 * all arithmetic uses BigInt (zero floats). The backend remains the final
 * authority; client-side guards are UX-only.
 */

import { create } from "zustand";
import {
  convertBlToFacture,
  convertDevisToBl,
  getDocumentDetails,
  searchDocuments,
  type DocumentDetailsWireResponse,
  type DocumentSummaryWireDto,
} from "@/lib/ipc";
import { newCommandId } from "@/lib/money";
import { useSessionStore } from "@/stores/session-store";

export type ConversionTab = "devis_to_bl" | "bl_to_facture";

export type TenderMethod = "cash" | "card" | "cheque" | "credit";

export interface ConversionTenders {
  cash: string;
  card: string;
  cheque: string;
  credit: string;
}

export interface ConversionResult {
  newDocumentNumber: string;
  newDocumentId: string;
  status: string;
  warnings?: string[];
}

export const EMPTY_TENDERS: ConversionTenders = {
  cash: "",
  card: "",
  cheque: "",
  credit: "",
};

/** Human decimal input → centime bigint; blank/garbage counts as zero. */
export function parseTenderCentimes(text: string): bigint {
  const trimmed = text.trim().replace(",", ".");
  const match = /^(-)?(\d+)(?:\.(\d{1,2}))?$/.exec(trimmed);
  if (match === null) {
    return 0n;
  }
  const sign = match[1] === "-" ? -1n : 1n;
  const whole = BigInt(match[2] ?? "0");
  const frac = BigInt((match[3] ?? "").padEnd(2, "0"));
  return sign * (whole * 100n + frac);
}

/** Centime string → whole-dinar display text (cash-prefill convenience). */
export function centimesToDisplayUnits(centimes: string): string {
  try {
    return (BigInt(centimes) / 100n).toString();
  } catch {
    return "0";
  }
}

export interface Settlement {
  allocatedCentimes: bigint;
  remainingCentimes: bigint;
}

/** Exact-settlement arithmetic: allocated vs source TTC. */
export function computeSettlement(
  tenders: ConversionTenders,
  sourceTtcCentimes: string,
): Settlement {
  const allocated =
    parseTenderCentimes(tenders.cash) +
    parseTenderCentimes(tenders.card) +
    parseTenderCentimes(tenders.cheque) +
    parseTenderCentimes(tenders.credit);
  let target = 0n;
  try {
    target = BigInt(sourceTtcCentimes);
  } catch {
    target = 0n;
  }
  return { allocatedCentimes: allocated, remainingCentimes: target - allocated };
}

/** Guest (comptant) documents carry no party — credit tender is impossible. */
export function isGuestDocument(partyId: string | null): boolean {
  return partyId === null;
}

/** Submit gate for BL→Facture: exact settlement, no flight, selection set. */
export function canSubmitBlToFacture(args: {
  remainingCentimes: bigint;
  isSubmitting: boolean;
  hasSelection: boolean;
  creditBlocked: boolean;
}): boolean {
  return (
    args.remainingCentimes === 0n &&
    !args.isSubmitting &&
    args.hasSelection &&
    !args.creditBlocked
  );
}

interface ConversionModalState {
  isOpen: boolean;
  activeTab: ConversionTab;
  searchQuery: string;
  searchResults: DocumentSummaryWireDto[];
  isLoadingSearch: boolean;
  selectedDocument: DocumentSummaryWireDto | null;
  selectedDetails: DocumentDetailsWireResponse | null;
  isLoadingDetails: boolean;
  documentLanguage: "FR" | "AR";
  tenders: ConversionTenders;
  isSubmitting: boolean;
  submissionLock: boolean;
  idempotencyKey: string | null;
  conversionResult: ConversionResult | null;
  error: string | null;
  openModal: (initialTab?: ConversionTab) => void;
  closeModal: () => void;
  setActiveTab: (tab: ConversionTab) => void;
  setSearchQuery: (query: string) => void;
  runSearch: () => Promise<void>;
  selectDocument: (doc: DocumentSummaryWireDto) => Promise<void>;
  setTender: (method: TenderMethod, value: string) => void;
  setDocumentLanguage: (lang: "FR" | "AR") => void;
  submitDevisToBl: () => Promise<void>;
  submitBlToFacture: () => Promise<void>;
  clearResult: () => void;
  reset: () => void;
}

const initialState = {
  isOpen: false,
  activeTab: "devis_to_bl" as ConversionTab,
  searchQuery: "",
  searchResults: [] as DocumentSummaryWireDto[],
  isLoadingSearch: false,
  selectedDocument: null as DocumentSummaryWireDto | null,
  selectedDetails: null as DocumentDetailsWireResponse | null,
  isLoadingDetails: false,
  documentLanguage: "FR" as "FR" | "AR",
  tenders: { ...EMPTY_TENDERS },
  isSubmitting: false,
  submissionLock: false,
  idempotencyKey: null as string | null,
  conversionResult: null as ConversionResult | null,
  error: null as string | null,
};

function sourceType(tab: ConversionTab): string {
  return tab === "devis_to_bl" ? "DEVIS" : "BL";
}

export const useConversionModalStore = create<ConversionModalState>()((set, get) => {
  async function fetchRecent(): Promise<void> {
    const token = useSessionStore.getState().getSessionToken();
    if (token === null) {
      set({ error: "IPC_NO_ACTIVE_SESSION", searchResults: [] });
      return;
    }
    set({ isLoadingSearch: true, error: null });
    try {
      const response = await searchDocuments({
        session_token: token,
        document_type: sourceType(get().activeTab),
        status: "FINALIZED",
        query: null,
        limit: 20,
      });
      set({ searchResults: response.documents });
    } catch (failure: unknown) {
      set({
        error: typeof failure === "string" ? failure : "INTERNAL_ERROR",
        searchResults: [],
      });
    } finally {
      set({ isLoadingSearch: false });
    }
  }

  return {
    ...initialState,

    openModal: (initialTab) => {
      set({
        ...initialState,
        isOpen: true,
        activeTab: initialTab ?? "devis_to_bl",
      });
      void fetchRecent();
    },

    closeModal: () => {
      if (get().isSubmitting) {
        return;
      }
      set({ ...initialState });
    },

    setActiveTab: (tab) => {
      set({
        ...initialState,
        isOpen: true,
        activeTab: tab,
      });
      void fetchRecent();
    },

    setSearchQuery: (query) => {
      set({ searchQuery: query });
      void get().runSearch();
    },

    runSearch: async () => {
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ error: "IPC_NO_ACTIVE_SESSION", searchResults: [] });
        return;
      }
      const trimmed = get().searchQuery.trim();
      set({ isLoadingSearch: true, error: null });
      try {
        const response = await searchDocuments({
          session_token: token,
          document_type: sourceType(get().activeTab),
          status: "FINALIZED",
          query: trimmed === "" ? null : trimmed,
          limit: 20,
        });
        set({ searchResults: response.documents });
      } catch (failure: unknown) {
        set({
          error: typeof failure === "string" ? failure : "INTERNAL_ERROR",
          searchResults: [],
        });
      } finally {
        set({ isLoadingSearch: false });
      }
    },

    selectDocument: async (doc) => {
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ error: "IPC_NO_ACTIVE_SESSION" });
        return;
      }
      set({
        selectedDocument: doc,
        selectedDetails: null,
        tenders: { ...EMPTY_TENDERS, cash: centimesToDisplayUnits(doc.total_ttc_centimes) },
        conversionResult: null,
        error: null,
        isLoadingDetails: true,
      });
      try {
        const details = await getDocumentDetails(doc.id);
        if (get().selectedDocument?.id === doc.id) {
          set({ selectedDetails: details });
        }
      } catch (failure: unknown) {
        // Details are enrichment only: the summary already carries every
        // field the settlement form needs, so a details failure surfaces
        // as a banner without clearing the selection.
        set({ error: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
      } finally {
        if (get().selectedDocument?.id === doc.id) {
          set({ isLoadingDetails: false });
        }
      }
    },

    setTender: (method, value) => {
      set((state) => ({ tenders: { ...state.tenders, [method]: value } }));
    },

    setDocumentLanguage: (lang) => {
      set({ documentLanguage: lang });
    },

    submitDevisToBl: async () => {
      const state = get();
      if (state.isSubmitting || state.selectedDocument === null) {
        return;
      }
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ error: "IPC_NO_ACTIVE_SESSION" });
        return;
      }
      const key = newCommandId();
      // Synchronous lock: the ref-equivalent here is the zustand flag set
      // before the first await — same-tick re-entry sees isSubmitting.
      set({ isSubmitting: true, submissionLock: true, idempotencyKey: key, error: null });
      try {
        const response = await convertDevisToBl({
          session_token: token,
          command_id: key,
          source_document_id: state.selectedDocument.id,
        });
        set({
          conversionResult: {
            newDocumentNumber: response.new_document_number,
            newDocumentId: response.new_document_id,
            status: response.new_document_status,
            warnings: response.warnings,
          },
        });
      } catch (failure: unknown) {
        set({ error: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
      } finally {
        set({ isSubmitting: false, submissionLock: false });
      }
    },

    submitBlToFacture: async () => {
      const state = get();
      if (state.isSubmitting || state.selectedDocument === null) {
        return;
      }
      const token = useSessionStore.getState().getSessionToken();
      if (token === null) {
        set({ error: "IPC_NO_ACTIVE_SESSION" });
        return;
      }
      const settlement = computeSettlement(
        state.tenders,
        state.selectedDocument.total_ttc_centimes,
      );
      const creditBlocked = isGuestDocument(state.selectedDocument.party_id);
      if (
        !canSubmitBlToFacture({
          remainingCentimes: settlement.remainingCentimes,
          isSubmitting: state.isSubmitting,
          hasSelection: true,
          creditBlocked:
            creditBlocked && parseTenderCentimes(state.tenders.credit) > 0n,
        })
      ) {
        return;
      }
      const key = newCommandId();
      set({ isSubmitting: true, submissionLock: true, idempotencyKey: key, error: null });
      try {
        const wire = [
          { payment_method: "CASH" as const, amount: parseTenderCentimes(state.tenders.cash) },
          { payment_method: "CARD" as const, amount: parseTenderCentimes(state.tenders.card) },
          { payment_method: "CHEQUE" as const, amount: parseTenderCentimes(state.tenders.cheque) },
          { payment_method: "CREDIT" as const, amount: parseTenderCentimes(state.tenders.credit) },
        ]
          .filter((line) => line.amount > 0n)
          .map((line) => ({
            payment_method: line.payment_method,
            amount_centimes: line.amount.toString(),
          }));
        const response = await convertBlToFacture({
          session_token: token,
          command_id: key,
          source_document_id: state.selectedDocument.id,
          document_language: state.documentLanguage,
          payments: wire,
        });
        set({
          conversionResult: {
            newDocumentNumber: response.new_document_number,
            newDocumentId: response.new_document_id,
            status: response.new_document_status,
          },
        });
      } catch (failure: unknown) {
        set({ error: typeof failure === "string" ? failure : "INTERNAL_ERROR" });
      } finally {
        set({ isSubmitting: false, submissionLock: false });
      }
    },

    clearResult: () => {
      set({
        conversionResult: null,
        selectedDocument: null,
        selectedDetails: null,
        tenders: { ...EMPTY_TENDERS },
        error: null,
      });
      void fetchRecent();
    },

    reset: () => {
      set({ ...initialState, tenders: { ...EMPTY_TENDERS } });
    },
  };
});
