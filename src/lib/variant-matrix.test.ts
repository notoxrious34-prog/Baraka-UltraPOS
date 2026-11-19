/**
 * Contract tests for the variant-matrix IPC wrappers (TASK-BRK-114).
 *
 * Pins the exact backend command names (`catalog_generate_variant_matrix`,
 * `catalog_matrix_view`), the wire shapes field-for-field (snake_case;
 * money/micros as canonical strings), and verbatim error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  ipcGenerateVariantMatrix,
  ipcMatrixModels,
  ipcMatrixView,
  type GenerateVariantMatrixWire,
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

function generationInput(): GenerateVariantMatrixWire {
  return {
    name_ar: "سيرية قطن",
    name_fr: "SERIE COTON",
    axis1_name: "Taille",
    axis2_name: "Couleur",
    axis1_values: ["M", "XL"],
    axis2_values: ["NOIR", "BLANC"],
    base_sku: "CTN-100",
    category_id: null,
    template_product_id: "11111111-1111-1111-1111-111111111111",
    overrides: [
      { axis1: "M", axis2: "BLANC", sale_price_centimes: "27000", barcode: "CELL-M-BLANC" },
    ],
  };
}

describe("variant-matrix IPC contract (BRK-114)", () => {
  it("generates through the exact command with the exact wire shape", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      model_id: "22222222-2222-2222-2222-222222222222",
      cell_count: 4,
      skus: ["CTN-100-M-BLANC", "CTN-100-M-NOIR", "CTN-100-XL-BLANC", "CTN-100-XL-NOIR"],
    });
    const result = await ipcGenerateVariantMatrix("tok", generationInput());
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [string, Record<string, unknown>];
    expect(command).toBe("catalog_generate_variant_matrix");
    const request = args.request as Record<string, unknown>;
    expect(request.session_token).toBe("tok");
    expect(request.base_sku).toBe("CTN-100");
    expect(request.overrides).toEqual([
      { axis1: "M", axis2: "BLANC", sale_price_centimes: "27000", barcode: "CELL-M-BLANC" },
    ]);
    expect(result.cell_count).toBe(4);
    expect(result.skus).toHaveLength(4);
  });

  it("reads the grid through catalog_matrix_view with model_id", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue([
      {
        axis1_value: "M",
        axis2_value: "BLANC",
        product_id: "33333333-3333-3333-3333-333333333333",
        sku: "CTN-100-M-BLANC",
        name_ar: "سيرية قطن M BLANC",
        barcode: "CELL-M-BLANC",
        on_hand_micro: "0",
        sale_price_centimes: "27000",
        default_unit: {
          unit_id: "44444444-4444-4444-4444-444444444444",
          unit_name_ar: "قطعة",
          unit_name_fr: "Pièce",
          factor_micro: "10000",
          unit_barcode: null,
          wholesale_price_centimes: "22000",
          super_gros_price_centimes: "20000",
          is_base_unit: true,
        },
      },
    ]);
    const rows = await ipcMatrixView("tok", "22222222-2222-2222-2222-222222222222");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [string, Record<string, unknown>];
    expect(command).toBe("catalog_matrix_view");
    expect((args.request as Record<string, unknown>).model_id).toBe(
      "22222222-2222-2222-2222-222222222222",
    );
    expect(rows[0]!.on_hand_micro).toBe("0");
    expect(rows[0]!.sale_price_centimes).toBe("27000");
    expect(rows[0]!.default_unit!.unit_name_ar).toBe("قطعة");
    expect(rows[0]!.default_unit!.factor_micro).toBe("10000");
  });

  it("lists models through catalog_matrix_models for the grid picker", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue([
      {
        model_id: "22222222-2222-2222-2222-222222222222",
        name_ar: "سيرية قطن",
        name_fr: "SERIE COTON",
        axis1_name: "Taille",
        axis2_name: "Couleur",
      },
    ]);
    const models = await ipcMatrixModels("tok");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [string, Record<string, unknown>];
    expect(command).toBe("catalog_matrix_models");
    expect((args.request as Record<string, unknown>).session_token).toBe("tok");
    expect(models[0]!.axis2_name).toBe("Couleur");
  });

  it("propagates backend rejections verbatim", async () => {
    primeSession("tok");
    invokeMock.mockRejectedValue("VARIANT_AXIS_DUPLICATE");
    await expect(ipcGenerateVariantMatrix("tok", generationInput())).rejects.toBe(
      "VARIANT_AXIS_DUPLICATE",
    );
  });
});
