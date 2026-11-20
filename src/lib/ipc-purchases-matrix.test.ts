/**
 * Contract tests for the DSN-109 purchase IPC wrappers.
 *
 * Pins the exact backend commands (`purchases_preview_receive_deviations`,
 * `purchases_receive_matrix_goods`), the wire shapes field-for-field
 * (snake_case; money as centime strings; quantities as micro strings),
 * and verbatim error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  ipcPreviewReceiveDeviations,
  ipcReceiveMatrixGoods,
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
    userId: "33333333-3333-3333-3333-333333333333",
    username: "gerant1",
    role: "GERANT",
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

describe("purchases_preview_receive_deviations", () => {
  it("reviews lines through the exact command and wire shape", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue([
      {
        product_id: "11111111-1111-1111-1111-111111111111",
        last_cost_centimes: "8000",
        average90_cost_centimes: "8000",
        delta_vs_last_bps: 2500,
        delta_vs_avg_bps: 2500,
        flagged: true,
      },
    ]);
    const verdicts = await ipcPreviewReceiveDeviations(
      "tok",
      [
        {
          productId: "11111111-1111-1111-1111-111111111111",
          unitId: "22222222-2222-2222-2222-222222222222",
          quantityMicro: "1000000",
          unitCostCentimes: "10000",
        },
      ],
      1000,
    );
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      { request: Record<string, unknown> },
    ];
    expect(command).toBe("purchases_preview_receive_deviations");
    expect(args.request.session_token).toBe("tok");
    expect(args.request.threshold_bps).toBe(1000);
    expect(args.request.lines).toEqual([
      {
        product_id: "11111111-1111-1111-1111-111111111111",
        unit_id: "22222222-2222-2222-2222-222222222222",
        quantity_micro: "1000000",
        unit_cost_centimes: "10000",
      },
    ]);
    expect(verdicts).toEqual([
      {
        productId: "11111111-1111-1111-1111-111111111111",
        lastCostCentimes: "8000",
        average90CostCentimes: "8000",
        deltaVsLastBps: 2500,
        deltaVsAvgBps: 2500,
        flagged: true,
      },
    ]);
  });

  it("propagates backend rejections verbatim", async () => {
    primeSession("tok");
    invokeMock.mockRejectedValue("UNAUTHORIZED");
    await expect(ipcPreviewReceiveDeviations("tok", [], 1000)).rejects.toBe(
      "UNAUTHORIZED",
    );
  });
});

describe("purchases_receive_matrix_goods", () => {
  it("receives a size run through the exact command and wire shape", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      id: "br-1",
      number: "BR-0001",
      doc_type: "BR",
      status: "FINALIZED",
      supplier_id: "sup-1",
      warehouse_id: "wh-1",
      source_document_id: null,
      total_centimes: "1600000",
      lines: [],
      created_at: "2026-01-01T00:00:00Z",
      created_by: "gerant1",
    });
    await ipcReceiveMatrixGoods("tok", "cmd-1", null, "sup-1", "wh-1", "model-1", [
      { axis1: "M", axis2: "Rouge", quantityMicro: "50000", unitCostCentimes: "8000" },
    ]);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      { request: Record<string, unknown> },
    ];
    expect(command).toBe("purchases_receive_matrix_goods");
    expect(args.request).toEqual({
      session_token: "tok",
      command_id: "cmd-1",
      source_bc_id: null,
      supplier_id: "sup-1",
      warehouse_id: "wh-1",
      model_id: "model-1",
      cells: [
        {
          axis1: "M",
          axis2: "Rouge",
          quantity_micro: "50000",
          unit_cost_centimes: "8000",
        },
      ],
    });
  });

  it("propagates backend rejections verbatim", async () => {
    primeSession("tok");
    invokeMock.mockRejectedValue("EMPTY_MATRIX_RUN");
    await expect(
      ipcReceiveMatrixGoods("tok", "cmd-2", null, "sup-1", "wh-1", "model-1", []),
    ).rejects.toBe("EMPTY_MATRIX_RUN");
  });
});
