/**
 * Contract tests for the Margin Guard commit path (DSN-115).
 *
 * Pins that `sales_checkout` carries `below_cost_override` verbatim
 * (`null` on the cashier path = every red line stays locked; a filled
 * manager override on the PIN path), and that both lock codes resolve
 * to FR/AR strings that never disclose the purchase cost.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ipcCheckout, type CheckoutRequestWire } from "@/lib/ipc";
import { translateBackendError } from "@/lib/i18n";
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
    userId: "44444444-4444-4444-4444-444444444444",
    username: "caissier1",
    role: "CAISSIER",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

beforeEach(() => {
  fakeTauriWindow();
  invokeMock.mockReset();
});

afterEach(() => {
  clearTauriWindow();
  useSessionStore.setState({ session: null });
});

function cashierRequest(): CheckoutRequestWire {
  return {
    command_id: "cmd-margin-1",
    session_token: "tok",
    warehouse_id: "wh-1",
    customer_id: null,
    document_type: "TICKET",
    document_language: "FR",
    lines: [],
    global_discount_centimes: "0",
    payments: [],
    allow_negative_stock: false,
    notes: null,
    below_cost_override: null,
    cart_opened_at: null,
  };
}

describe("margin-guard checkout contract (DSN-115)", () => {
  it("sends below_cost_override:null on the cashier path", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      document_id: "d1",
      document_number: "TICK-2026-000099",
      subtotal_ht_centimes: "20000",
      tva_total_centimes: "0",
      discount_total_centimes: "0",
      timbre_centimes: "0",
      total_ttc_centimes: "20000",
      paid_amount_centimes: "20000",
      change_amount_centimes: "0",
      print_job_id: "p1",
    });
    await ipcCheckout(cashierRequest());
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      { request: Record<string, unknown> },
    ];
    expect(command).toBe("sales_checkout");
    expect(args.request.below_cost_override).toBeNull();
  });

  it("carries a filled manager override verbatim on the PIN path", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      document_id: "d2",
      document_number: "TICK-2026-000100",
      subtotal_ht_centimes: "20000",
      tva_total_centimes: "0",
      discount_total_centimes: "0",
      timbre_centimes: "0",
      total_ttc_centimes: "20000",
      paid_amount_centimes: "20000",
      change_amount_centimes: "0",
      print_job_id: "p2",
    });
    await ipcCheckout({
      ...cashierRequest(),
      below_cost_override: { username: "gerant1", pin: "2468" },
    });
    const [, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      { request: Record<string, unknown> },
    ];
    expect(args.request.below_cost_override).toEqual({
      username: "gerant1",
      pin: "2468",
    });
  });

  it("resolves both lock codes in FR and AR without disclosing cost", () => {
    for (const locale of ["fr-DZ", "ar-DZ"] as const) {
      const locked = translateBackendError("MARGIN_GUARD_BELOW_COST", locale);
      const rejected = translateBackendError(
        "MARGIN_GUARD_OVERRIDE_REJECTED",
        locale,
      );
      expect(locked.length).toBeGreaterThan(0);
      expect(rejected.length).toBeGreaterThan(0);
      // Silent signal: no figure, no purchase-cost wording may leak.
      for (const message of [locked, rejected]) {
        expect(message).not.toMatch(/[0-9]/);
        expect(message).not.toMatch(/achat|revient|شراء|الشراء/i);
      }
    }
  });
});
