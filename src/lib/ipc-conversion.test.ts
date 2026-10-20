/**
 * Contract tests for the document-conversion IPC wrappers (TASK-BRK-041).
 *
 * Pins the exact backend command names (`convert_devis_to_bl`,
 * `convert_bl_to_facture`), the request wire shapes field-for-field
 * (snake_case, canonical centime strings), the automatic session-token
 * injection at the bridge edge, verbatim response passthrough, and
 * code-preserving error propagation — so a drift between these wrappers
 * and the Rust DTOs fails here, not in front of the cashier.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  convertBlToFacture,
  convertDevisToBl,
  type ConvertBlToFactureWireRequest,
  type ConvertDevisToBlWireRequest,
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
    userId: "55555555-5555-5555-5555-555555555555",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

/** Derived from devis_to_bl_conversion.rs::convert_finalized_devis_creates_bl_and_seals_source. */
const devisRequest: ConvertDevisToBlWireRequest = {
  session_token: "tok-conv-1",
  command_id: "11111111-1111-4111-8111-111111111111",
  source_document_id: "22222222-2222-4222-8222-222222222222",
};

const devisResponse = {
  new_document_id: "33333333-3333-4333-8333-333333333333",
  new_document_number: "BL-2026-000001",
  new_document_status: "FINALIZED",
  total_ttc_centimes: "40000",
  items_count: 2,
  warnings: [],
};

/** Derived from bl_to_facture_conversion.rs::cash_closing_creates_facture_movement_and_seals_source. */
const blRequest: ConvertBlToFactureWireRequest = {
  session_token: "tok-conv-2",
  command_id: "44444444-4444-4444-8444-444444444444",
  source_document_id: "55555555-5555-4555-8555-555555555555",
  document_language: "FR",
  payments: [{ payment_method: "CASH", amount_centimes: "40000" }],
};

const blResponse = {
  new_document_id: "66666666-6666-4666-8666-666666666666",
  new_document_number: "FAC-2026-000001",
  new_document_status: "FINALIZED",
  total_ttc_centimes: "40000",
  items_count: 2,
  customer_new_balance_centimes: null,
};

describe("convertDevisToBl", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls convert_devis_to_bl with the exact wire payload and injected session token", async () => {
    primeSession("tok-conv-1");
    invokeMock.mockResolvedValueOnce(devisResponse);
    const result = await convertDevisToBl(devisRequest);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("convert_devis_to_bl", {
      request: {
        session_token: "tok-conv-1",
        command_id: "11111111-1111-4111-8111-111111111111",
        source_document_id: "22222222-2222-4222-8222-222222222222",
      },
      session_token: "tok-conv-1",
    });
    expect(result).toEqual(devisResponse);
  });

  it("preserves canonical money strings and warning list verbatim", async () => {
    primeSession("tok-conv-1");
    invokeMock.mockResolvedValueOnce({ ...devisResponse, warnings: ["shortfall"] });
    const result = await convertDevisToBl(devisRequest);
    expect(result.total_ttc_centimes).toBe("40000");
    expect(typeof result.total_ttc_centimes).toBe("string");
    expect(result.warnings).toEqual(["shortfall"]);
  });

  it("propagates backend error codes without alteration", async () => {
    primeSession("tok-conv-1");
    for (const code of [
      "SOURCE_DOCUMENT_NOT_FOUND",
      "INVALID_CONVERSION_SOURCE_TYPE",
      "DOCUMENT_ALREADY_CONVERTED",
      "UNAUTHORIZED",
    ]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(convertDevisToBl(devisRequest)).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(4);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(convertDevisToBl(devisRequest)).rejects.toThrow("IPC_NO_ACTIVE_SESSION");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});

describe("convertBlToFacture", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls convert_bl_to_facture with the exact wire payload and injected session token", async () => {
    primeSession("tok-conv-2");
    invokeMock.mockResolvedValueOnce(blResponse);
    const result = await convertBlToFacture(blRequest);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("convert_bl_to_facture", {
      request: {
        session_token: "tok-conv-2",
        command_id: "44444444-4444-4444-8444-444444444444",
        source_document_id: "55555555-5555-4555-8555-555555555555",
        document_language: "FR",
        payments: [{ payment_method: "CASH", amount_centimes: "40000" }],
      },
      session_token: "tok-conv-2",
    });
    expect(result).toEqual(blResponse);
  });

  it("passes through a credit-closing balance string when present", async () => {
    primeSession("tok-conv-2");
    invokeMock.mockResolvedValueOnce({
      ...blResponse,
      customer_new_balance_centimes: "12000",
    });
    const result = await convertBlToFacture({
      ...blRequest,
      payments: [{ payment_method: "CREDIT", amount_centimes: "40000" }],
    });
    expect(result.customer_new_balance_centimes).toBe("12000");
    expect(typeof result.customer_new_balance_centimes).toBe("string");
    expect(result.total_ttc_centimes).toBe("40000");
  });

  it("propagates backend error codes without alteration", async () => {
    primeSession("tok-conv-2");
    for (const code of [
      "INSUFFICIENT_PAYMENT",
      "EMPTY_CART",
      "CREDIT_LIMIT_EXCEEDED",
      "DOCUMENT_ALREADY_CONVERTED",
      "NO_OPEN_CASH_SESSION_FOR_TERMINAL",
      "UNAUTHORIZED",
    ]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(convertBlToFacture(blRequest)).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(6);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(convertBlToFacture(blRequest)).rejects.toThrow("IPC_NO_ACTIVE_SESSION");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
