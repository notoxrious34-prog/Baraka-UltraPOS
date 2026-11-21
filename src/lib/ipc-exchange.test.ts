/**
 * Contract tests for the exchange IPC wrapper (BRK-125, Échange Express).
 *
 * Pins the documents_process_exchange command name, the nested-request
 * payload with canonical string quantities and the manager-PIN override
 * passthrough, verbatim response passthrough, and code-preserving error
 * propagation (return taxonomy plus exchange and margin-guard codes).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  processSaleExchange,
  type ProcessExchangeRequest,
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
const ALT_PRODUCT_ID = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
const ALT_UNIT_ID = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";

const exchangePayload: ProcessExchangeRequest = {
  session_token: "tok-exch-1",
  source_document_id: SOURCE_ID,
  return_items: [{ source_item_id: LINE_ID, quantity_micro: "10000" }],
  new_items: [
    { product_id: ALT_PRODUCT_ID, unit_id: ALT_UNIT_ID, quantity_micro: "10000" },
  ],
  refund_method: "NO_REFUND",
  below_cost_override: null,
  notes: null,
};

const exchangeResponse = {
  return_document_id: "99999999-9999-4999-8999-999999999999",
  return_document_number: "AVO-2026-000001",
  sale_document_id: "77777777-7777-4777-8777-777777777777",
  sale_document_number: "TICK-2026-000002",
  return_ttc_centimes: "20000",
  sale_ttc_centimes: "20000",
  net_centimes: "0",
};

describe("processSaleExchange", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("serializes the exchange payload with stringified quantities", async () => {
    primeSession("tok-exch-1");
    invokeMock.mockResolvedValueOnce(exchangeResponse);
    const result = await processSaleExchange(exchangePayload);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("documents_process_exchange", {
      request: {
        session_token: "tok-exch-1",
        source_document_id: SOURCE_ID,
        return_items: [{ source_item_id: LINE_ID, quantity_micro: "10000" }],
        new_items: [
          { product_id: ALT_PRODUCT_ID, unit_id: ALT_UNIT_ID, quantity_micro: "10000" },
        ],
        refund_method: "NO_REFUND",
        below_cost_override: null,
        notes: null,
      },
      session_token: "tok-exch-1",
    });
    expect(result).toEqual(exchangeResponse);
  });

  it("passes the manager PIN override through verbatim", async () => {
    primeSession("tok-exch-1");
    invokeMock.mockResolvedValueOnce(exchangeResponse);
    await processSaleExchange({
      ...exchangePayload,
      below_cost_override: { manager_username: "gerant1", manager_pin: "2468" },
    });
    expect(invokeMock).toHaveBeenCalledWith("documents_process_exchange", {
      request: expect.objectContaining({
        below_cost_override: { manager_username: "gerant1", manager_pin: "2468" },
      }),
      session_token: "tok-exch-1",
    });
  });

  it("propagates the exchange taxonomy without alteration", async () => {
    primeSession("tok-exch-1");
    for (const code of [
      "RETURN_QUANTITY_EXCEEDED",
      "EXCHANGE_PRICE_UNKNOWN",
      "EXCHANGE_REFUND_REQUIRED",
      "MARGIN_GUARD_BELOW_COST",
      "MARGIN_GUARD_OVERRIDE_REJECTED",
      "NO_ACTIVE_CASH_SESSION",
      "INVALID_SOURCE_DOCUMENT_STATE",
      "UNAUTHORIZED",
    ]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(processSaleExchange(exchangePayload)).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(8);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(processSaleExchange(exchangePayload)).rejects.toThrow(
      "IPC_NO_ACTIVE_SESSION",
    );
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
