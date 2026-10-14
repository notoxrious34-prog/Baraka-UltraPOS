/**
 * Contract tests for the product-update IPC wrapper (TASK-BRK-050).
 *
 * Pins the exact backend command name (`catalog_update_product`), the
 * full-state request wire shape field-for-field (snake_case, canonical
 * centime strings, SKU absent by design), and verbatim error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { updateProduct, type UpdateProductPayload } from "@/lib/ipc";
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
  };
  useSessionStore.setState({ session });
}

const payload: UpdateProductPayload = {
  session_token: "tok-edit-1",
  product_id: "11111111-1111-4111-8111-111111111111",
  name_ar: "برغي",
  name_fr: "Vis 8x40",
  category_id: null,
  is_active: true,
  units: [
    {
      unit_id: "22222222-2222-4222-8222-222222222222",
      unit_name_ar: "قطعة",
      unit_name_fr: "Pièce",
      factor_micro: "10000",
      barcode: "6111250000013",
      sale_price_centimes: "15050",
      purchase_price_centimes: "12000",
      wholesale_price_centimes: null,
      is_base_unit: true,
      is_default_selling: true,
    },
  ],
};

const updatedDetail = {
  product_id: "11111111-1111-4111-8111-111111111111",
  sku: "VIS-8X40",
  barcode: null,
  name_ar: "برغي",
  name_fr: "Vis 8x40",
  category_id: null,
  is_kit: false,
  is_active: true,
  units: [],
  kit_items: [],
};

describe("updateProduct", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fakeTauriWindow();
  });

  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("calls catalog_update_product with the exact full-state payload", async () => {
    primeSession("tok-edit-1");
    invokeMock.mockResolvedValueOnce(updatedDetail);
    await updateProduct(payload);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("catalog_update_product", {
      request: {
        session_token: "tok-edit-1",
        product_id: "11111111-1111-4111-8111-111111111111",
        name_ar: "برغي",
        name_fr: "Vis 8x40",
        category_id: null,
        is_active: true,
        units: [
          {
            unit_id: "22222222-2222-4222-8222-222222222222",
            unit_name_ar: "قطعة",
            unit_name_fr: "Pièce",
            factor_micro: "10000",
            barcode: "6111250000013",
            sale_price_centimes: "15050",
            purchase_price_centimes: "12000",
            wholesale_price_centimes: null,
            is_base_unit: true,
            is_default_selling: true,
          },
        ],
      },
      session_token: "tok-edit-1",
    });
  });

  it("carries no SKU field (immutability by construction)", async () => {
    primeSession("tok-edit-1");
    invokeMock.mockResolvedValueOnce(updatedDetail);
    await updateProduct(payload);
    const args = invokeMock.mock.calls[0]?.[1] as {
      request: Record<string, unknown>;
    };
    expect("sku" in args.request).toBe(false);
    expect(typeof args.request.sale_price_centimes).toBe("undefined");
  });

  it("propagates backend error codes without alteration", async () => {
    primeSession("tok-edit-1");
    for (const code of ["UNAUTHORIZED", "NOT_FOUND", "INVALID_INPUT"]) {
      invokeMock.mockRejectedValueOnce(code);
      await expect(updateProduct(payload)).rejects.toBe(code);
    }
    expect(invokeMock).toHaveBeenCalledTimes(3);
  });

  it("rejects with IPC_NO_ACTIVE_SESSION before invoking when no session is primed", async () => {
    useSessionStore.setState({ session: null });
    await expect(updateProduct(payload)).rejects.toThrow("IPC_NO_ACTIVE_SESSION");
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
