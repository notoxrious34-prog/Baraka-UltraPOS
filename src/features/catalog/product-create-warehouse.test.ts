/**
 * Product creation warehouse selection tests (TASK-DSN-050).
 *
 * The modal resolves a default depot via `pickDepotId` (DEP-01 first) but
 * lets the user override it through a target-warehouse `<select>`; the
 * selected id flows into `createProductWithProvisioning` as `warehouse_id`
 * on the single initial-stock `stock.adjust` call. Node-only harness
 * (no DOM library), matching the repo convention: pure units plus
 * source-text contracts.
 */

import { describe, expect, it, vi } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  createProductWithProvisioning,
  pickDepotId,
  PRINCIPAL_DEPOT_CODE,
} from "@/features/catalog/ProductCreateModal";
import type { CatalogProduct, CatalogUnit } from "@/types/pos";
import type { CreateProductWire } from "@/lib/ipc";

declare const process: { cwd(): string };

const MODAL_SRC = readFileSync(
  join(process.cwd(), "src", "features", "catalog", "ProductCreateModal.tsx"),
  "utf8",
);

function unit(overrides: Partial<CatalogUnit>): CatalogUnit {
  return {
    unit_id: "u-1",
    product_id: "p-1",
    unit_name_ar: "قطعة",
    unit_name_fr: "PCS",
    factor_micro: "10000",
    barcode: null,
    sale_price_centimes: "15050",
    purchase_price_centimes: "12000",
    wholesale_price_centimes: null,
    is_base_unit: true,
    is_default_selling: true,
    ...overrides,
  };
}

function product(): CatalogProduct {
  return {
    product_id: "p-1",
    sku: "VIS-8X40",
    barcode: null,
    name_ar: "برغي",
    name_fr: "Vis 8x40",
    category_id: null,
    is_kit: false,
    is_active: true,
    kit_items: [],
    units: [unit({})],
  };
}

const WIRE: CreateProductWire = {
  sku: "VIS-8X40",
  barcode: null,
  name_ar: "برغي",
  name_fr: "Vis 8x40",
  category_id: null,
  is_kit: false,
  units: [],
};

describe("createProductWithProvisioning warehouse targeting", () => {
  it("credits the user-selected warehouse, not a hardcoded depot", async () => {
    const created = product();
    const create = vi.fn().mockResolvedValue(created);
    const adjust = vi.fn().mockResolvedValue({ movement_id: "m-1", new_quantity_base_micro: "50000" });
    const outcome = await createProductWithProvisioning("tok-1", WIRE, "50000", "depot-sud-uuid", {
      create,
      adjust,
    });
    expect(outcome.provisionError).toBeNull();
    expect(adjust).toHaveBeenCalledTimes(1);
    expect(adjust.mock.calls[0]?.[0]).toMatchObject({
      warehouse_id: "depot-sud-uuid",
      product_id: "p-1",
      direction: "INCREASE",
    });
  });

  it("keeps DEP-01 as the default resolution when the user does not override", () => {
    const rows = [
      { id: "w-sud", code: "DEP-02", name: "Dépôt Sud", is_active: true },
      { id: "w-main", code: PRINCIPAL_DEPOT_CODE, name: "Dépôt Principal", is_active: true },
    ];
    expect(pickDepotId(rows)).toBe("w-main");
  });
});

describe("ProductCreateModal warehouse selector (source contract)", () => {
  it("renders a select bound to the depot state with name (code) options", () => {
    expect(MODAL_SRC).toContain("cat.depotSelect");
    expect(MODAL_SRC).toContain("value={depotId ??");
    expect(MODAL_SRC).toContain("setDepotId(event.target.value");
    expect(MODAL_SRC).toContain("${depot.name} (${depot.code})");
    expect(MODAL_SRC).toContain("depots.map((depot)");
  });

  it("loads active warehouses on open and defaults the selection", () => {
    expect(MODAL_SRC).toContain("setDepots(rows.filter((row) => row.is_active))");
    expect(MODAL_SRC).toContain("setDepotId(pickDepotId(rows))");
  });
});
