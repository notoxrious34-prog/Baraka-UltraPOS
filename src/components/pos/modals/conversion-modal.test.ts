/**
 * Unit tests for the document conversion flow (TASK-BRK-043).
 *
 * Covers the conversion-modal store lifecycle (tabs, search, selection),
 * BigInt-centime settlement arithmetic, the exact-settlement submit gate
 * (including guest credit lockout), single-flight submission with a stable
 * idempotency key, and backend error surfacing — all without floats.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  canSubmitBlToFacture,
  centimesToDisplayUnits,
  computeSettlement,
  isGuestDocument,
  parseTenderCentimes,
  useConversionModalStore,
} from "@/stores/conversion-modal-store";
import { translateConversionError } from "@/components/pos/modals/DocumentConversionModal";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

/** The bridge probe requires a `window` carrying the Tauri marker. */
function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function primeSession(): void {
  fakeTauriWindow();
  const session: SessionInfo = {
    token: "tok-conv-9",
    userId: "55555555-5555-5555-5555-555555555555",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
  };
  useSessionStore.setState({ session });
  useConversionModalStore.getState().reset();
}

const devisRow = {
  id: "22222222-2222-4222-8222-222222222222",
  document_number: "DEV-2026-000001",
  document_type: "DEVIS",
  status: "FINALIZED",
  party_id: null,
  party_name: null,
  total_ttc_centimes: "40000",
  items_count: 2,
  created_at: "2026-01-01T00:00:00Z",
};

const blRow = {
  ...devisRow,
  id: "55555555-5555-4555-8555-555555555555",
  document_number: "BL-2026-000001",
  document_type: "BL",
  party_id: "77777777-7777-4777-8777-777777777777",
  party_name: "Cherif Benz",
};

describe("parseTenderCentimes", () => {
  it("parses decimal input to centime bigints without floats", () => {
    expect(parseTenderCentimes("150")).toBe(15000n);
    expect(parseTenderCentimes("150,50")).toBe(15050n);
    expect(parseTenderCentimes("")).toBe(0n);
    expect(parseTenderCentimes("abc")).toBe(0n);
  });
});

describe("computeSettlement", () => {
  const ttc = "40000";
  it("reports exact settlement at zero remainder", () => {
    const s = computeSettlement(
      { cash: "400", card: "", cheque: "", credit: "" },
      ttc,
    );
    expect(s.allocatedCentimes).toBe(40000n);
    expect(s.remainingCentimes).toBe(0n);
  });

  it("reports positive remainder when under-allocated", () => {
    const s = computeSettlement(
      { cash: "150", card: "", cheque: "", credit: "" },
      ttc,
    );
    expect(s.remainingCentimes).toBe(25000n);
  });

  it("reports negative remainder when over-allocated", () => {
    const s = computeSettlement(
      { cash: "400", card: "50", cheque: "", credit: "" },
      ttc,
    );
    expect(s.remainingCentimes).toBe(-5000n);
  });

  it("sums all four tender legs", () => {
    const s = computeSettlement(
      { cash: "100", card: "100", cheque: "100", credit: "100" },
      ttc,
    );
    expect(s.allocatedCentimes).toBe(40000n);
    expect(s.remainingCentimes).toBe(0n);
  });
});

describe("canSubmitBlToFacture", () => {
  it("opens the gate only on exact settlement without flight or blocks", () => {
    expect(
      canSubmitBlToFacture({
        remainingCentimes: 0n,
        isSubmitting: false,
        hasSelection: true,
        creditBlocked: false,
      }),
    ).toBe(true);
    expect(
      canSubmitBlToFacture({
        remainingCentimes: 1n,
        isSubmitting: false,
        hasSelection: true,
        creditBlocked: false,
      }),
    ).toBe(false);
    expect(
      canSubmitBlToFacture({
        remainingCentimes: -1n,
        isSubmitting: false,
        hasSelection: true,
        creditBlocked: false,
      }),
    ).toBe(false);
    expect(
      canSubmitBlToFacture({
        remainingCentimes: 0n,
        isSubmitting: true,
        hasSelection: true,
        creditBlocked: false,
      }),
    ).toBe(false);
    expect(
      canSubmitBlToFacture({
        remainingCentimes: 0n,
        isSubmitting: false,
        hasSelection: false,
        creditBlocked: false,
      }),
    ).toBe(false);
    expect(
      canSubmitBlToFacture({
        remainingCentimes: 0n,
        isSubmitting: false,
        hasSelection: true,
        creditBlocked: true,
      }),
    ).toBe(false);
  });
});

describe("isGuestDocument", () => {
  it("treats null party as guest (credit impossible)", () => {
    expect(isGuestDocument(null)).toBe(true);
    expect(isGuestDocument("some-id")).toBe(false);
  });
});

describe("centimesToDisplayUnits", () => {
  it("renders whole dinars for the cash prefill", () => {
    expect(centimesToDisplayUnits("40000")).toBe("400");
    expect(centimesToDisplayUnits("0")).toBe("0");
  });
});

describe("conversion store lifecycle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("opens on a tab and fetches recent finalized candidates", async () => {
    invokeMock.mockResolvedValueOnce({ documents: [devisRow] });
    useConversionModalStore.getState().openModal("devis_to_bl");
    expect(useConversionModalStore.getState().isOpen).toBe(true);
    await vi.waitFor(() => {
      expect(useConversionModalStore.getState().isLoadingSearch).toBe(false);
    });
    expect(invokeMock).toHaveBeenCalledWith("documents_search", {
      request: {
        session_token: "tok-conv-9",
        document_type: "DEVIS",
        status: "FINALIZED",
        query: null,
        limit: 20,
      },
      session_token: "tok-conv-9",
    });
    expect(useConversionModalStore.getState().searchResults).toEqual([devisRow]);
  });

  it("switching tabs resets selection and queries the other type", async () => {
    invokeMock.mockResolvedValue({ documents: [] });
    const store = useConversionModalStore.getState();
    store.openModal("devis_to_bl");
    await vi.waitFor(() => {
      expect(useConversionModalStore.getState().isLoadingSearch).toBe(false);
    });
    useConversionModalStore.getState().setActiveTab("bl_to_facture");
    expect(useConversionModalStore.getState().activeTab).toBe("bl_to_facture");
    expect(useConversionModalStore.getState().selectedDocument).toBeNull();
    await vi.waitFor(() => {
      expect(
        invokeMock.mock.calls.some(
          (call) =>
            (call[1] as { request: { document_type: string } }).request.document_type === "BL",
        ),
      ).toBe(true);
    });
  });

  it("selecting a document prefills cash with the source TTC", async () => {
    invokeMock.mockResolvedValue({ documents: [devisRow] });
    useConversionModalStore.getState().openModal("devis_to_bl");
    await vi.waitFor(() => {
      expect(useConversionModalStore.getState().isLoadingSearch).toBe(false);
    });
    invokeMock.mockResolvedValueOnce({
      document: { document_id: devisRow.id },
      items: [],
      payments: [],
      print_status: "NONE",
    });
    await useConversionModalStore.getState().selectDocument(devisRow);
    expect(useConversionModalStore.getState().tenders.cash).toBe("400");
  });

  it("devis_to_bl submits once with a stable idempotency key", async () => {
    invokeMock.mockResolvedValue({ documents: [devisRow] });
    useConversionModalStore.getState().openModal("devis_to_bl");
    await vi.waitFor(() => {
      expect(useConversionModalStore.getState().isLoadingSearch).toBe(false);
    });
    invokeMock.mockResolvedValueOnce({
      document: { document_id: devisRow.id },
      items: [],
      payments: [],
      print_status: "NONE",
    });
    await useConversionModalStore.getState().selectDocument(devisRow);
    invokeMock.mockResolvedValueOnce({
      new_document_id: "bl-new",
      new_document_number: "BL-2026-000002",
      new_document_status: "FINALIZED",
      total_ttc_centimes: "40000",
      items_count: 2,
      warnings: [],
    });
    // Second concurrent submit is a no-op behind the lock.
    const first = useConversionModalStore.getState().submitDevisToBl();
    const second = useConversionModalStore.getState().submitDevisToBl();
    await Promise.all([first, second]);
    const conversions = invokeMock.mock.calls.filter((call) => call[0] === "convert_devis_to_bl");
    expect(conversions).toHaveLength(1);
    const payload = (conversions[0]?.[1] as { request: Record<string, unknown> }).request;
    expect(payload.source_document_id).toBe(devisRow.id);
    expect(typeof payload.command_id).toBe("string");
    expect(useConversionModalStore.getState().idempotencyKey).toBe(payload.command_id);
    expect(useConversionModalStore.getState().conversionResult?.newDocumentNumber).toBe(
      "BL-2026-000002",
    );
    expect(useConversionModalStore.getState().isSubmitting).toBe(false);
  });

  it("bl_to_facture sends exact tenders and blocks guest credit", async () => {
    invokeMock.mockResolvedValue({ documents: [blRow] });
    useConversionModalStore.getState().openModal("bl_to_facture");
    await vi.waitFor(() => {
      expect(useConversionModalStore.getState().isLoadingSearch).toBe(false);
    });
    invokeMock.mockResolvedValueOnce({
      document: { document_id: blRow.id },
      items: [],
      payments: [],
      print_status: "NONE",
    });
    await useConversionModalStore.getState().selectDocument(blRow);
    // Cash prefill covers the TTC exactly → submittable.
    useConversionModalStore.getState().setTender("cash", "400");
    invokeMock.mockResolvedValueOnce({
      new_document_id: "fac-new",
      new_document_number: "FAC-2026-000001",
      new_document_status: "FINALIZED",
      total_ttc_centimes: "40000",
      items_count: 2,
      customer_new_balance_centimes: null,
    });
    await useConversionModalStore.getState().submitBlToFacture();
    const closings = invokeMock.mock.calls.filter((call) => call[0] === "convert_bl_to_facture");
    expect(closings).toHaveLength(1);
    expect((closings[0]?.[1] as { request: Record<string, unknown> }).request).toMatchObject({
      source_document_id: blRow.id,
      document_language: "FR",
      payments: [{ payment_method: "CASH", amount_centimes: "40000" }],
    });
    expect(useConversionModalStore.getState().conversionResult?.newDocumentNumber).toBe(
      "FAC-2026-000001",
    );
  });

  it("bl_to_facture refuses to dispatch off-exact settlement", async () => {
    invokeMock.mockResolvedValue({ documents: [blRow] });
    useConversionModalStore.getState().openModal("bl_to_facture");
    await vi.waitFor(() => {
      expect(useConversionModalStore.getState().isLoadingSearch).toBe(false);
    });
    invokeMock.mockResolvedValueOnce({
      document: { document_id: blRow.id },
      items: [],
      payments: [],
      print_status: "NONE",
    });
    await useConversionModalStore.getState().selectDocument(blRow);
    useConversionModalStore.getState().setTender("cash", "100");
    vi.clearAllMocks();
    await useConversionModalStore.getState().submitBlToFacture();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(useConversionModalStore.getState().conversionResult).toBeNull();
  });

  it("surfaces backend errors without swallowing", async () => {
    invokeMock.mockResolvedValue({ documents: [devisRow] });
    useConversionModalStore.getState().openModal("devis_to_bl");
    await vi.waitFor(() => {
      expect(useConversionModalStore.getState().isLoadingSearch).toBe(false);
    });
    invokeMock.mockResolvedValueOnce({
      document: { document_id: devisRow.id },
      items: [],
      payments: [],
      print_status: "NONE",
    });
    await useConversionModalStore.getState().selectDocument(devisRow);
    invokeMock.mockRejectedValueOnce("DOCUMENT_ALREADY_CONVERTED");
    await useConversionModalStore.getState().submitDevisToBl();
    expect(useConversionModalStore.getState().error).toBe("DOCUMENT_ALREADY_CONVERTED");
    expect(useConversionModalStore.getState().conversionResult).toBeNull();
    expect(useConversionModalStore.getState().isSubmitting).toBe(false);
  });
});

describe("translateConversionError", () => {
  it("bridges prefixed stock codes in both locales", () => {
    expect(translateConversionError("INSUFFICIENT_STOCK_FOR:Vis:3", "fr-DZ")).toContain("Vis");
    expect(translateConversionError("INSUFFICIENT_STOCK_FOR:Vis:3", "ar-DZ")).toContain("Vis");
  });

  it("delegates plain codes to the backend table", () => {
    expect(translateConversionError("DOCUMENT_ALREADY_CONVERTED", "fr-DZ")).not.toBe(
      "Erreur interne du noyau",
    );
    expect(translateConversionError("DOCUMENT_ALREADY_CONVERTED", "ar-DZ")).not.toBe(
      "خطأ داخلي في النواة",
    );
  });
});
