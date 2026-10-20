/**
 * Contract tests for the return IPC wrappers (TASK-BRK-047).
 *
 * Pins the returns_get_source_summary flat-arg convention (mirroring
 * sales_get), the returns_process nested-request payload with canonical
 * string quantities and tagged refund destination, verbatim response
 * passthrough, and code-preserving error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  getDocumentReturnContext,
  getReturnableDocumentSummary,
  processReturn,
  processSaleReturn,
  type ProcessReturnPayload,
  type ProcessReturnRequest,
} from "@/lib/ipc";
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

function clearTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  delete globals.window;
  delete globals.__TAURI_INTERNALS__;
}

function primeSession(token: string): void {
  const session: SessionInfo = {
    token,
    userId: "88888888-8888-8888-8888-888888888888",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

const SOURCE_ID = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const LINE_ID = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";

const summaryResponse = {
  source_document_id: SOURCE_ID,
  source_document_number: "TICK-2026-000005",
  document_type: "TICKET",
  party_type: null,
  party_id: null,
  party_name: null,
  warehouse_id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
  total_ht_cents: "15500",
  total_ttc_cents: "15550",
  is_eligible_for_return: true,
  items: [
    {
      source_item_id: LINE_ID,
      product_id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
      product_name: "Vis 8x40",
      sku: "VIS-8X40",
      unit_id: "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee",
      unit_name: "Pièce",
      original_qty_micro: "20000",
      already_returned_qty_micro: "0",
      remaining_qty_micro: "20000",
      unit_price_cents: "7750",
      net_unit_price_cents: "7750",
      tva_rate: "0",
    },
  ],
};

const cashPayload: ProcessReturnPayload = {
  session_token: "tok-ret-1",
  operator_id: "88888888-8888-8888-8888-888888888888",
  source_document_id: SOURCE_ID,
  terminal_id: "TERM-1",
  items: [{ source_item_id: LINE_ID, quantity_micro: "10000", reason: null }],
  refund: { type: "Cash", payload: { cash_session_id: "sess-1" } },
  idempotency_key: "ffffffff-ffff-4fff-8fff-ffffffffffff",
  notes: null,
};

const cashResponse = {
  return_document_id: "99999999-9999-4999-8999-999999999999",
  return_document_number: "AVO-2026-000001",
  source_document_id: SOURCE_ID,
  source_document_number: "TICK-2026-000005",
  total_ht_cents: "7750",
  total_tva_cents: "0",
  total_ttc_cents: "7750",
  refund_type: "CASH",
  lines_count: 1,
};

describe("getReturnableDocumentSummary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls returns_get_source_summary with flat scalar args", async () => {
    primeSession("tok-ret-1");
    invokeMock.mockResolvedValueOnce(summaryResponse);
    const result = await getReturnableDocumentSummary(SOURCE_ID);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("returns_get_source_summary", {
      session_token: "tok-ret-1",
      source_document_id: SOURCE_ID,
    });
    expect(result).toEqual(summaryResponse);
  });

  it("preserves canonical micro/centime strings verbatim", async () => {
    primeSession("tok-ret-1");
    invokeMock.mockResolvedValueOnce(summaryResponse);
    const result = await getReturnableDocumentSummary(SOURCE_ID);
    expect(result.items[0]?.remaining_qty_micro).toBe("20000");
    expect(typeof result.items[0]?.remaining_qty_micro).toBe("string");
    expect(result.total_ttc_cents).toBe("15550");
  });

  it("propagates DOCUMENT_NOT_FOUND without alteration", async () => {
    primeSession("tok-ret-1");
    invokeMock.mockRejectedValueOnce("DOCUMENT_NOT_FOUND");
    await expect(
      getReturnableDocumentSummary("00000000-0000-4000-8000-000000000000"),
    ).rejects.toBe("DOCUMENT_NOT_FOUND");
  });
});

describe("processReturn", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("serializes the cash payload with stringified quantities", async () => {
    primeSession("tok-ret-1");
    invokeMock.mockResolvedValueOnce(cashResponse);
    const result = await processReturn(cashPayload);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("returns_process", {
      request: {
        session_token: "tok-ret-1",
        operator_id: "88888888-8888-8888-8888-888888888888",
        source_document_id: SOURCE_ID,
        terminal_id: "TERM-1",
        items: [{ source_item_id: LINE_ID, quantity_micro: "10000", reason: null }],
        refund: { type: "Cash", payload: { cash_session_id: "sess-1" } },
        idempotency_key: "ffffffff-ffff-4fff-8fff-ffffffffffff",
        notes: null,
      },
      session_token: "tok-ret-1",
    });
    expect(result).toEqual(cashResponse);
  });

  it("maps the customer-credit and no-refund destinations", async () => {
    primeSession("tok-ret-1");
    invokeMock.mockResolvedValueOnce({ ...cashResponse, refund_type: "CUSTOMER_CREDIT" });
    const credit = await processReturn({
      ...cashPayload,
      refund: { type: "CustomerCredit", payload: { customer_id: "cust-1" } },
    });
    expect(credit.refund_type).toBe("CUSTOMER_CREDIT");
    expect(invokeMock).toHaveBeenCalledWith("returns_process", {
      request: {
        ...cashPayload,
        session_token: "tok-ret-1",
        refund: { type: "CustomerCredit", payload: { customer_id: "cust-1" } },
      },
      session_token: "tok-ret-1",
    });
    invokeMock.mockResolvedValueOnce({ ...cashResponse, refund_type: "NO_REFUND" });
    const none = await processReturn({ ...cashPayload, refund: { type: "NoRefund" } });
    expect(none.refund_type).toBe("NO_REFUND");
  });

  it("propagates stable return error codes without alteration", async () => {
    primeSession("tok-ret-1");
    for (const code of [
      "RETURN_QUANTITY_EXCEEDED",
      "CREDIT_CUSTOMER_REQUIRED",
      "NO_OPEN_CASH_SESSION_FOR_TERMINAL",
      "UNAUTHORIZED",
    ]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(processReturn(cashPayload)).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(4);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(processReturn(cashPayload)).rejects.toThrow("IPC_NO_ACTIVE_SESSION");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

const contextResponse = {
  source_document_id: SOURCE_ID,
  document_number: "TICK-2026-000005",
  document_type: "TICKET",
  party_id: null,
  party_name: null,
  warehouse_id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
  created_at: "2026-01-01T00:00:00.000Z",
  is_eligible: true,
  ineligibility_reason: null,
  items: [
    {
      source_item_id: LINE_ID,
      product_id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
      product_name: "Vis 8x40",
      barcode: "6111250000013",
      unit_id: "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee",
      unit_name: "Pièce",
      unit_price_centimes: "7750",
      original_quantity_micro: "20000",
      already_returned_micro: "0",
      remaining_allowable_micro: "20000",
      tva_rate: 0,
      discount_centimes: "0",
    },
  ],
};

const facadePayload: ProcessReturnRequest = {
  session_token: "tok-ret-2",
  source_document_id: SOURCE_ID,
  refund_method: "CASH",
  items: [{ source_item_id: LINE_ID, quantity_micro: "10000" }],
  notes: null,
};

const facadeResponse = {
  return_document_id: "99999999-9999-4999-8999-999999999999",
  return_document_number: "AVO-2026-000001",
  total_refund_ttc_centimes: "7750",
  refund_method: "CASH",
};

describe("getDocumentReturnContext", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls documents_get_return_context with flat scalar args", async () => {
    primeSession("tok-ret-2");
    invokeMock.mockResolvedValueOnce(contextResponse);
    const result = await getDocumentReturnContext(SOURCE_ID);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("documents_get_return_context", {
      session_token: "tok-ret-2",
      source_document_id: SOURCE_ID,
    });
    expect(result).toEqual(contextResponse);
  });

  it("preserves canonical micro strings and numeric tva_rate verbatim", async () => {
    primeSession("tok-ret-2");
    invokeMock.mockResolvedValueOnce(contextResponse);
    const result = await getDocumentReturnContext(SOURCE_ID);
    expect(result.items[0]?.remaining_allowable_micro).toBe("20000");
    expect(typeof result.items[0]?.remaining_allowable_micro).toBe("string");
    expect(result.items[0]?.tva_rate).toBe(0);
    expect(typeof result.items[0]?.tva_rate).toBe("number");
    expect(result.is_eligible).toBe(true);
  });

  it("propagates DOCUMENT_NOT_FOUND without alteration", async () => {
    primeSession("tok-ret-2");
    invokeMock.mockRejectedValueOnce("DOCUMENT_NOT_FOUND");
    await expect(
      getDocumentReturnContext("00000000-0000-4000-8000-000000000000"),
    ).rejects.toBe("DOCUMENT_NOT_FOUND");
  });
});

describe("processSaleReturn", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("serializes the simplified payload with stringified quantities", async () => {
    primeSession("tok-ret-2");
    invokeMock.mockResolvedValueOnce(facadeResponse);
    const result = await processSaleReturn(facadePayload);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("documents_process_return", {
      request: {
        session_token: "tok-ret-2",
        source_document_id: SOURCE_ID,
        refund_method: "CASH",
        items: [{ source_item_id: LINE_ID, quantity_micro: "10000" }],
        notes: null,
      },
      session_token: "tok-ret-2",
    });
    expect(result).toEqual(facadeResponse);
  });

  it("propagates the dedicated return taxonomy without alteration", async () => {
    primeSession("tok-ret-2");
    for (const code of [
      "RETURN_QUANTITY_EXCEEDED",
      "INVALID_SOURCE_DOCUMENT_STATE",
      "NO_ACTIVE_CASH_SESSION",
      "MISSING_CUSTOMER_FOR_CREDIT_REFUND",
      "UNAUTHORIZED",
      "EMPTY_RETURN",
    ]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(processSaleReturn(facadePayload)).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(6);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(processSaleReturn(facadePayload)).rejects.toThrow("IPC_NO_ACTIVE_SESSION");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
