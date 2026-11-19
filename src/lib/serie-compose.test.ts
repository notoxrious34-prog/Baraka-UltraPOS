/**
 * Contract tests for the série-composer IPC wrapper (TASK-BRK-115).
 *
 * Pins the exact backend command name (`catalog_compose_serie`), the wire
 * shape field-for-field (snake_case; micros as canonical strings), the
 * tolerant shortfall report, and verbatim error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  ipcComposeSerie,
  type ComposeSerieWire,
} from "@/lib/ipc";
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

function serieInput(): ComposeSerieWire {
  return {
    kit_id: "44444444-4444-4444-4444-444444444444",
    model_id: "22222222-2222-2222-2222-222222222222",
    axis2_value: "NOIR",
    lines: [
      { axis1_value: "M", pieces: 2 },
      { axis1_value: "L", pieces: 4 },
      { axis1_value: "XL", pieces: 4 },
      { axis1_value: "XXL", pieces: 2 },
    ],
    warehouse_id: "66666666-6666-6666-6666-666666666666",
    serie_count: 2,
  };
}

describe("série-composer IPC contract (BRK-115)", () => {
  it("composes through the exact command with the exact wire shape", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      kit_id: "44444444-4444-4444-4444-444444444444",
      component_count: 4,
      total_pieces: 12,
      shortfalls: [],
    });
    const result = await ipcComposeSerie("tok", serieInput());
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [string, Record<string, unknown>];
    expect(command).toBe("catalog_compose_serie");
    const request = args.request as Record<string, unknown>;
    expect(request.session_token).toBe("tok");
    expect(request.kit_id).toBe("44444444-4444-4444-4444-444444444444");
    expect(request.axis2_value).toBe("NOIR");
    expect(request.lines).toEqual([
      { axis1_value: "M", pieces: 2 },
      { axis1_value: "L", pieces: 4 },
      { axis1_value: "XL", pieces: 4 },
      { axis1_value: "XXL", pieces: 2 },
    ]);
    expect(request.warehouse_id).toBe("66666666-6666-6666-6666-666666666666");
    expect(request.serie_count).toBe(2);
    expect(result.component_count).toBe(4);
    expect(result.total_pieces).toBe(12);
  });

  it("surfaces tolerant shortfalls with canonical micros strings", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      kit_id: "44444444-4444-4444-4444-444444444444",
      component_count: 4,
      total_pieces: 12,
      shortfalls: [
        {
          product_id: "77777777-7777-7777-7777-777777777777",
          sku: "CTN-100-L-NOIR",
          required_micro: "80000",
          on_hand_micro: "10000",
        },
      ],
    });
    const result = await ipcComposeSerie("tok", serieInput());
    expect(result.shortfalls).toHaveLength(1);
    expect(result.shortfalls[0]!.sku).toBe("CTN-100-L-NOIR");
    expect(result.shortfalls[0]!.required_micro).toBe("80000");
    expect(result.shortfalls[0]!.on_hand_micro).toBe("10000");
  });

  it("propagates backend rejections verbatim", async () => {
    primeSession("tok");
    invokeMock.mockRejectedValue("SERIE_UNKNOWN_SIZE");
    await expect(ipcComposeSerie("tok", serieInput())).rejects.toBe("SERIE_UNKNOWN_SIZE");
  });
});
