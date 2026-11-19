/**
 * Contract tests for the prep-sheet IPC wrapper (BRK-126).
 *
 * Pins the exact backend command (`sales_build_prep_sheet`), the wire
 * shape field-for-field (quantities as canonical micro strings, shares
 * as integers), and the priceless invariant: NO price key may ride the
 * response — a warehouse checklist can never leak a selling price.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ipcBuildPrepSheet } from "@/lib/ipc";
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
    userId: "44444444-4444-4444-4444-444444444444",
    username: "vendeur1",
    role: "VENDEUR",
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

describe("prep-sheet IPC contract (BRK-126)", () => {
  it("explodes cart lines through sales_build_prep_sheet with exact wire", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue([
      {
        product_id: "11111111-1111-1111-1111-111111111111",
        sku: "SLIP-PREP",
        name_ar: "سليب",
        name_fr: "Slip",
        quantity_base_micro: "1500000",
        shares: [
          {
            unit_id: "33333333-3333-3333-3333-333333333333",
            unit_name_fr: "Douzaine",
            unit_name_ar: "دزينة",
            factor_micro: 120000,
            count: 12,
          },
          {
            unit_id: "22222222-2222-2222-2222-222222222222",
            unit_name_fr: "Paquet",
            unit_name_ar: "حزمة",
            factor_micro: 60000,
            count: 1,
          },
        ],
      },
    ]);
    const sheet = await ipcBuildPrepSheet("tok", [
      {
        product_id: "11111111-1111-1111-1111-111111111111",
        unit_id: "44444444-4444-4444-4444-444444444444",
        quantity_micro: "1500000",
      },
    ]);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      { request: Record<string, unknown> },
    ];
    expect(command).toBe("sales_build_prep_sheet");
    expect(args.request.session_token).toBe("tok");
    expect(args.request.lines).toEqual([
      {
        product_id: "11111111-1111-1111-1111-111111111111",
        unit_id: "44444444-4444-4444-4444-444444444444",
        quantity_micro: "1500000",
      },
    ]);
    expect(sheet).toHaveLength(1);
    expect(sheet[0]!.shares).toHaveLength(2);
    expect(sheet[0]!.shares[0]).toEqual({
      unit_id: "33333333-3333-3333-3333-333333333333",
      unit_name_fr: "Douzaine",
      unit_name_ar: "دزينة",
      factor_micro: 120000,
      count: 12,
    });
  });

  it("carries no price key on any line or share", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue([
      {
        product_id: "p",
        sku: "S",
        name_ar: "س",
        name_fr: "S",
        quantity_base_micro: "10000",
        shares: [],
      },
    ]);
    const sheet = await ipcBuildPrepSheet("tok", [
      { product_id: "p", unit_id: "u", quantity_micro: "10000" },
    ]);
    for (const line of sheet) {
      for (const key of Object.keys(line)) {
        expect(key.toLowerCase()).not.toMatch(/price|prix|montant|total/);
      }
      for (const share of line.shares) {
        for (const key of Object.keys(share)) {
          expect(key.toLowerCase()).not.toMatch(/price|prix|montant|total/);
        }
      }
    }
  });

  it("propagates backend rejections verbatim", async () => {
    primeSession("tok");
    invokeMock.mockRejectedValue("PRODUCT_NOT_FOUND");
    await expect(
      ipcBuildPrepSheet("tok", [{ product_id: "ghost", unit_id: "u", quantity_micro: "1" }]),
    ).rejects.toBe("PRODUCT_NOT_FOUND");
  });
});
