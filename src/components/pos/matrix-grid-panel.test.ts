/**
 * Unit tests for the POS 2D variant grid helpers (DSN-108).
 *
 * `groupMatrixRows`/`sellableUnits` are pure (node-only, no DOM):
 * order-preserving axes, nested cell lookup without key collisions,
 * floor division in micros, and fail-safe null/zero handling.
 * I18n wiring is pinned via source-text assertions, per repo convention.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  groupMatrixRows,
  sellableUnits,
} from "@/components/pos/MatrixGridPanel";
import type {
  VariantMatrixRowWire,
  MatrixGridUnitWire,
} from "@/lib/ipc";

declare const process: { cwd(): string };

function unit(overrides: Partial<MatrixGridUnitWire> = {}): MatrixGridUnitWire {
  return {
    unit_id: "u-1",
    unit_name_ar: "قطعة",
    unit_name_fr: "Pièce",
    factor_micro: "10000",
    unit_barcode: null,
    wholesale_price_centimes: "22000",
    super_gros_price_centimes: "20000",
    is_base_unit: true,
    ...overrides,
  };
}

function row(
  axis1: string,
  axis2: string,
  overrides: Partial<VariantMatrixRowWire> = {},
): VariantMatrixRowWire {
  return {
    axis1_value: axis1,
    axis2_value: axis2,
    product_id: `p-${axis1}-${axis2}`,
    sku: `SKU-${axis1}-${axis2}`,
    name_ar: `سيرية ${axis1} ${axis2}`,
    barcode: null,
    on_hand_micro: "30000",
    sale_price_centimes: "25000",
    default_unit: unit(),
    ...overrides,
  };
}

describe("groupMatrixRows", () => {
  it("preserves display order on both axes", () => {
    const grouped = groupMatrixRows([
      row("M", "BLANC"),
      row("M", "NOIR"),
      row("XXL", "BLANC"),
      row("XXL", "NOIR"),
    ]);
    expect(grouped.axis1).toEqual(["M", "XXL"]);
    expect(grouped.axis2).toEqual(["BLANC", "NOIR"]);
    expect(grouped.cells.get("M")?.get("BLANC")?.sku).toBe("SKU-M-BLANC");
    expect(grouped.cells.get("XXL")?.get("NOIR")?.sku).toBe("SKU-XXL-NOIR");
  });

  it("never collides on values containing spaces", () => {
    const grouped = groupMatrixRows([row("A B", "C"), row("A", "B C")]);
    expect(grouped.cells.get("A B")?.get("C")?.product_id).toBe("p-A B-C");
    expect(grouped.cells.get("A")?.get("B C")?.product_id).toBe("p-A-B C");
  });
});

describe("sellableUnits", () => {
  it("floor-divides on-hand micros by the factor", () => {
    expect(sellableUnits(row("M", "NOIR"))).toBe("3");
    expect(sellableUnits(row("M", "NOIR", { on_hand_micro: "25000" }))).toBe("2");
  });

  it("returns null without a default unit (cell off)", () => {
    expect(sellableUnits(row("M", "NOIR", { default_unit: null }))).toBeNull();
  });

  it("fails safe to zero on zero factor or unparseable figures", () => {
    expect(
      sellableUnits(row("M", "NOIR", { default_unit: unit({ factor_micro: "0" }) })),
    ).toBe("0");
    expect(sellableUnits(row("M", "NOIR", { on_hand_micro: "nope" }))).toBe("0");
  });
});

describe("matrix i18n wiring", () => {
  it("declares every matrix key in both dictionaries", () => {
    const source = readFileSync(join(process.cwd(), "src", "lib", "i18n.ts"), "utf8") as string;
    for (const key of [
      "matrix.toggle",
      "matrix.pickModel",
      "matrix.noModels",
      "matrix.noDefaultUnit",
      "matrix.addedToCart",
    ]) {
      expect(source).toContain(`"${key}"`);
    }
  });
});
