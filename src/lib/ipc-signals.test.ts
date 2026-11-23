/**
 * Contract tests for the signal-cockpit IPC wrapper (TASK-BRK-116).
 *
 * Pins the exact backend command name (`reports_signals`), the wire
 * shape field-for-field (snake_case; money as centime strings), the
 * snake_case-to-camelCase mapping, and verbatim error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ipcSignals } from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

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
    username: "audit",
    role: "ADMIN",
    permissions: ["MANAGE_PRODUCTS", "POS_SELL"],
    expiresAt: "2026-01-01T00:00:00Z",
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

function feedWire(): Record<string, unknown> {
  return {
    profile: "apparel_wholesale_retail",
    broken_runs: [
      {
        model_id: "M1",
        cells: [{ axis1_value: "M", axis2_value: "Noir", sibling_units_sold: "20" }],
      },
    ],
    dead_stock: [
      { product_id: "P-OLD", qty_on_hand_micro: "100000", days_since_last_movement: "90" },
    ],
    velocity: { units_per_day: "0", is_slow: true, trend_bps: null },
    model_margins: [
      {
        model_id: "M1",
        revenue_centimes: "375000",
        cogs_centimes: "225000",
        margin_bps: "4000",
      },
    ],
    aging: {
      bucket_0_30_centimes: "100000",
      bucket_31_60_centimes: "0",
      bucket_61_90_centimes: "0",
      bucket_90_plus_centimes: "0",
    },
  };
}

describe("signal-cockpit IPC contract (BRK-116)", () => {
  it("fetches through the exact command with the exact wire shape", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue(feedWire());
    const feed = await ipcSignals("tok", 30);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [string, Record<string, unknown>];
    expect(command).toBe("reports_signals");
    const request = args.request as Record<string, unknown>;
    expect(request.session_token).toBe("tok");
    expect(request.window_days).toBe(30);
    expect(feed.profile).toBe("apparel_wholesale_retail");
    expect(feed.brokenRuns).toHaveLength(1);
    expect(feed.brokenRuns[0]?.cells[0]?.axis1Value).toBe("M");
    expect(feed.brokenRuns[0]?.cells[0]?.siblingUnitsSold).toBe("20");
    expect(feed.deadStock[0]?.productId).toBe("P-OLD");
    expect(feed.velocity?.isSlow).toBe(true);
    expect(feed.modelMargins[0]?.marginBps).toBe("4000");
    expect(feed.aging.bucket030Centimes).toBe("100000");
  });

  it("defaults the window to null and maps a null velocity", async () => {
    primeSession("tok");
    const wire = feedWire();
    wire.velocity = null;
    invokeMock.mockResolvedValue(wire);
    const feed = await ipcSignals("tok");
    const [, args] = invokeMock.mock.calls[0] as unknown as [string, Record<string, unknown>];
    const request = args.request as Record<string, unknown>;
    expect(request.window_days).toBeNull();
    expect(feed.velocity).toBeNull();
  });

  it("propagates backend errors verbatim", async () => {
    primeSession("tok");
    invokeMock.mockRejectedValue("SIGNAL_STORE_UNAVAILABLE");
    await expect(ipcSignals("tok", 30)).rejects.toBe("SIGNAL_STORE_UNAVAILABLE");
  });
});
