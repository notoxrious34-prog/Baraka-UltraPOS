/**
 * Unit tests for the sale-return cockpit (TASK-BRK-047 re-issue).
 *
 * Covers search rendering, context load with line ceilings, ceiling
 * clamping, return-all fill, cash/credit gating, canonical dispatch
 * payloads, the single-flight lock, and bilingual error mapping — all
 * without floats.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  clampReturnQty,
  computeReturnTotals,
  isGuestContext,
  returnLineHtCents,
  returnLineTvaCents,
  useReturnModalStore,
} from "@/stores/return-modal-store";
import { translateBackendError } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import { useCashSessionStore } from "@/stores/cash-session-store";
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
    token: "tok-ret-9",
    userId: "88888888-8888-8888-8888-888888888888",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
  useCashSessionStore.setState({
    session: {
      id: "sess-9",
      warehouseId: "wh-9",
      terminalId: "TERM-9",
      status: "OPEN",
      openedBy: "gerant1",
      openedByUsername: "gerant1",
      openedAt: "2026-01-01T00:00:00Z",
      openingFloatCentimes: "0",
      closedBy: null,
      closedByUsername: null,
      closedAt: null,
      actualCountedCentimes: null,
      systemExpectedCentimes: null,
      varianceCentimes: null,
      closingNotes: null,
      revision: "1",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
  });
  useReturnModalStore.getState().reset();
}

const SOURCE_ID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const LINE_ID = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";

const contextWire = {
  source_document_id: SOURCE_ID,
  document_number: "TICK-2026-000005",
  document_type: "TICKET",
  party_id: null,
  party_name: null,
  warehouse_id: "wh-9",
  created_at: "2026-01-01T00:00:00.000Z",
  is_eligible: true,
  ineligibility_reason: null,
  items: [
    {
      source_item_id: LINE_ID,
      product_id: "prod-1",
      product_name: "Vis 8x40",
      barcode: null,
      unit_id: "unit-1",
      unit_name: "Pièce",
      unit_price_centimes: "7750",
      discount_per_unit_centimes: null,
      discounted_unit_price_centimes: null,
      original_quantity_micro: "20000",
      already_returned_micro: "5000",
      remaining_allowable_micro: "15000",
      tva_rate: 0,
      discount_centimes: "0",
    },
  ],
};

describe("return store lifecycle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("initializes closed and empty, opens on demand", () => {
    const state = useReturnModalStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.context).toBeNull();
    expect(state.quantities).toEqual({});
    expect(state.refundMethod).toBe("CASH");
  });

  it("loads context and exposes remaining ceilings", async () => {
    invokeMock.mockResolvedValueOnce(contextWire);
    await useReturnModalStore.getState().searchAndLoadDocument(SOURCE_ID);
    const state = useReturnModalStore.getState();
    expect(state.context?.document_number).toBe("TICK-2026-000005");
    expect(state.context?.is_eligible).toBe(true);
    expect(state.isLoadingContext).toBe(false);
  });
});

describe("ceiling clamping", () => {
  it("clamps into [0, remaining]", () => {
    expect(clampReturnQty(5000n, 15000n)).toBe(5000n);
    expect(clampReturnQty(15000n, 15000n)).toBe(15000n);
    expect(clampReturnQty(15001n, 15000n)).toBe(15000n);
    expect(clampReturnQty(0n, 15000n)).toBe(0n);
    expect(clampReturnQty(-5n, 15000n)).toBe(0n);
  });

  it("setItemQuantity never stores more than remaining", async () => {
    vi.clearAllMocks();
    primeSession();
    invokeMock.mockResolvedValueOnce(contextWire);
    await useReturnModalStore.getState().searchAndLoadDocument(SOURCE_ID);
    useReturnModalStore.getState().setItemQuantity(LINE_ID, "999999");
    expect(useReturnModalStore.getState().quantities[LINE_ID]).toBe("15000");
  });

  it("setReturnAll fills every remaining ceiling", async () => {
    vi.clearAllMocks();
    primeSession();
    invokeMock.mockResolvedValueOnce(contextWire);
    await useReturnModalStore.getState().searchAndLoadDocument(SOURCE_ID);
    useReturnModalStore.getState().setReturnAll();
    expect(useReturnModalStore.getState().quantities[LINE_ID]).toBe("15000");
    useReturnModalStore.getState().resetQuantities();
    expect(useReturnModalStore.getState().quantities).toEqual({});
  });
});

describe("settlement math", () => {
  it("computes HT/TVA/TTC with integer precision", () => {
    expect(returnLineHtCents(20000n, 7750n)).toBe(15500n);
    expect(returnLineTvaCents(15500n, 190000n)).toBe(2945n);
    expect(returnLineTvaCents(15500n, 0n)).toBe(0n);
    const totals = computeReturnTotals(
      {
        ...contextWire,
        items: [
          {
            ...contextWire.items[0]!,
            remaining_allowable_micro: "20000",
            unit_price_centimes: "7750",
            discount_per_unit_centimes: null,
            discounted_unit_price_centimes: null,
            discount_centimes: "0",
            tva_rate: 190000,
          },
        ],
      },
      { [LINE_ID]: "20000" },
    );
    expect(totals.totalHtCents).toBe(15500n);
    expect(totals.totalTvaCents).toBe(2945n);
    expect(totals.totalTtcCents).toBe(18445n);
    expect(totals.totalQtyMicro).toBe(20000n);
  });
});

describe("refund gating", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("disables CustomerCredit for guest contexts", async () => {
    invokeMock.mockResolvedValueOnce(contextWire);
    await useReturnModalStore.getState().searchAndLoadDocument(SOURCE_ID);
    expect(isGuestContext(useReturnModalStore.getState().context)).toBe(true);
    useReturnModalStore.getState().setRefundMethod("CUSTOMER_CREDIT");
    expect(useReturnModalStore.getState().refundMethod).toBe("CASH");
    useReturnModalStore.getState().setRefundMethod("NO_REFUND");
    expect(useReturnModalStore.getState().refundMethod).toBe("NO_REFUND");
  });

  it("accepts CustomerCredit for named-customer contexts", async () => {
    invokeMock.mockResolvedValueOnce({ ...contextWire, party_id: "cust-1" });
    await useReturnModalStore.getState().searchAndLoadDocument(SOURCE_ID);
    useReturnModalStore.getState().setRefundMethod("CUSTOMER_CREDIT");
    expect(useReturnModalStore.getState().refundMethod).toBe("CUSTOMER_CREDIT");
  });
});

describe("submit dispatch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  async function selectGuestDoc(): Promise<void> {
    invokeMock.mockResolvedValueOnce(contextWire);
    await useReturnModalStore.getState().searchAndLoadDocument(SOURCE_ID);
    vi.clearAllMocks();
  }

  it("sends the canonical stringified payload", async () => {
    await selectGuestDoc();
    useReturnModalStore.getState().setItemQuantity(LINE_ID, "10000");
    invokeMock.mockResolvedValueOnce({
      return_document_id: "avo-1",
      return_document_number: "AVO-2026-000001",
      total_refund_ttc_centimes: "7750",
      refund_method: "CASH",
    });
    const ok = await useReturnModalStore.getState().submitReturn();
    expect(ok).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("documents_process_return", {
      request: {
        session_token: "tok-ret-9",
        source_document_id: SOURCE_ID,
        refund_method: "CASH",
        items: [{ source_item_id: LINE_ID, quantity_micro: "10000" }],
        notes: null,
      },
      session_token: "tok-ret-9",
    });
    expect(useReturnModalStore.getState().successResult?.return_document_number).toBe(
      "AVO-2026-000001",
    );
    expect(useReturnModalStore.getState().isSubmitting).toBe(false);
  });

  it("blocks concurrent double submission behind the lock", async () => {
    await selectGuestDoc();
    useReturnModalStore.getState().setItemQuantity(LINE_ID, "10000");
    invokeMock.mockResolvedValue({
      return_document_id: "avo-1",
      return_document_number: "AVO-2026-000001",
      total_refund_ttc_centimes: "7750",
      refund_method: "CASH",
    });
    const first = useReturnModalStore.getState().submitReturn();
    const second = useReturnModalStore.getState().submitReturn();
    const [a, b] = await Promise.all([first, second]);
    expect(a).toBe(true);
    expect(b).toBe(false);
    expect(
      invokeMock.mock.calls.filter((call) => call[0] === "documents_process_return"),
    ).toHaveLength(1);
  });

  it("refuses to dispatch with zero selected quantity", async () => {
    await selectGuestDoc();
    vi.clearAllMocks();
    const ok = await useReturnModalStore.getState().submitReturn();
    expect(ok).toBe(false);
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("error mapping", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    primeSession();
  });

  it("maps return backend codes bilingually without swallowing", async () => {
    invokeMock.mockResolvedValueOnce(contextWire);
    await useReturnModalStore.getState().searchAndLoadDocument(SOURCE_ID);
    useReturnModalStore.getState().setItemQuantity(LINE_ID, "10000");
    invokeMock.mockRejectedValueOnce("RETURN_QUANTITY_EXCEEDED");
    const ok = await useReturnModalStore.getState().submitReturn();
    expect(ok).toBe(false);
    expect(useReturnModalStore.getState().errorMessage).toBe("RETURN_QUANTITY_EXCEEDED");
    expect(useReturnModalStore.getState().successResult).toBeNull();
    expect(useReturnModalStore.getState().isSubmitting).toBe(false);
    for (const code of [
      "RETURN_QUANTITY_EXCEEDED",
      "INVALID_SOURCE_DOCUMENT_STATE",
      "NO_ACTIVE_CASH_SESSION",
      "MISSING_CUSTOMER_FOR_CREDIT_REFUND",
      "EMPTY_RETURN",
    ]) {
      expect(translateBackendError(code, "fr-DZ")).not.toBe("Erreur interne du noyau");
      expect(translateBackendError(code, "ar-DZ")).not.toBe("خطأ داخلي في النواة");
    }
  });
});

describe("per-unit settlement (DSN-097)", () => {
  function perUnitContext() {
    return {
      ...contextWire,
      items: [
        {
          ...contextWire.items[0]!,
          unit_price_centimes: "1000",
          discount_per_unit_centimes: "100",
          discounted_unit_price_centimes: "900",
          original_quantity_micro: "30000",
          remaining_allowable_micro: "30000",
          already_returned_micro: "0",
          discount_centimes: "300",
          tva_rate: 0,
        },
      ],
    };
  }

  it("prices picked x net PU verbatim from the wire (1 of 3 x (1000-100) = 900)", () => {
    const totals = computeReturnTotals(perUnitContext(), { [LINE_ID]: "10000" });
    expect(totals.totalHtCents).toBe(900n);
    expect(totals.totalTvaCents).toBe(0n);
    expect(totals.totalTtcCents).toBe(900n);
    expect(totals.totalQtyMicro).toBe(10000n);
  });

  it("keeps the legacy lump pro-rating path byte-identical when per-unit is null", () => {
    const totals = computeReturnTotals(
      {
        ...contextWire,
        items: [
          {
            ...contextWire.items[0]!,
            unit_price_centimes: "1000",
            discount_per_unit_centimes: null,
            discounted_unit_price_centimes: null,
            original_quantity_micro: "30000",
            remaining_allowable_micro: "30000",
            already_returned_micro: "0",
            discount_centimes: "300",
            tva_rate: 0,
          },
        ],
      },
      { [LINE_ID]: "10000" },
    );
    // Legacy: 1 x 1000 gross, pro-rated lump 300/3 = 100 -> net 900.
    expect(totals.totalHtCents).toBe(900n);
    expect(totals.totalTtcCents).toBe(900n);
  });
});
