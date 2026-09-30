/**
 * Contract tests for the read-only totals preview IPC call (TASK-BRK-031).
 *
 * Pins the exact backend command name (`sales_preview_totals`), the
 * PreviewTotalsRequest wire shape field-for-field, the automatic
 * session-token injection at the bridge edge, and the verbatim passthrough
 * of the backend totals — the checkout modal's sole totals source. The
 * TICK-2026-000002 figures (TTC 155.50 / change 145.00 for a 300.50
 * tender) ride the wire exactly as the forensic audit demanded.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ipcPreviewTotals } from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { PreviewTotalsRequestWire } from "@/lib/ipc";
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
  };
  useSessionStore.setState({ session });
}

/** The forensic TICK-2026-000002 cart: one 150.50 DZD line, cash+card. */
const forensicCart: PreviewTotalsRequestWire = {
  warehouse_id: "wh-preview-1",
  document_type: "TICKET",
  lines: [
    {
      product_id: "f20f6f21-0000-4000-8000-000000000000",
      unit_id: "78d65d76-0000-4000-8000-000000000000",
      quantity_micro: "10000",
      unit_price_centimes: null,
      line_discount_centimes: "0",
    },
  ],
  global_discount_centimes: "0",
  payments: [
    { payment_method: "CASH", amount_centimes: "15000" },
    { payment_method: "CARD", amount_centimes: "15050" },
  ],
};

/** The committed totals the preview must mirror to the centime. */
const forensicTotals = {
  subtotal_ht_centimes: "15050",
  tva_total_centimes: "0",
  global_discount_centimes: "0",
  timbre_centimes: "500",
  total_ttc_centimes: "15550",
  paid_amount_centimes: "30050",
  change_amount_centimes: "14500",
  remaining_due_centimes: "0",
  stock_warnings: [],
};

describe("ipcPreviewTotals", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls sales_preview_totals with the exact wire payload and injected session token", async () => {
    primeSession("tok-preview-1");
    invokeMock.mockResolvedValueOnce(forensicTotals);
    const result = await ipcPreviewTotals("tok-preview-1", forensicCart);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    // Bridge wire policy: the store token rides BOTH the top level and
    // the nested request DTO (single source of truth, ADR-004 edge).
    expect(invokeMock).toHaveBeenCalledWith("sales_preview_totals", {
      request: {
        session_token: "tok-preview-1",
        warehouse_id: "wh-preview-1",
        document_type: "TICKET",
        lines: forensicCart.lines,
        global_discount_centimes: "0",
        payments: forensicCart.payments,
      },
      session_token: "tok-preview-1",
    });
    expect(result).toEqual(forensicTotals);
  });

  it("returns the TICK-2026-000002 totals verbatim (no client recomputation)", async () => {
    primeSession("tok-preview-2");
    invokeMock.mockResolvedValueOnce(forensicTotals);
    const result = await ipcPreviewTotals("tok-preview-2", {
      ...forensicCart,
      payments: [{ payment_method: "CASH", amount_centimes: "30050" }],
    });
    // The backend-returned values ARE the displayed values — passthrough,
    // byte-for-byte, including the 500-centime timbre.
    expect(result.timbre_centimes).toBe("500");
    expect(result.total_ttc_centimes).toBe("15550");
    expect(result.change_amount_centimes).toBe("14500");
    expect(result).toEqual(forensicTotals);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(ipcPreviewTotals("ignored-token", forensicCart)).rejects.toThrow(
      "IPC_NO_ACTIVE_SESSION",
    );
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
