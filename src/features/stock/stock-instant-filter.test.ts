/**
 * Stock overview instant-filter tests (TASK-DSN-050).
 *
 * `StockOverviewPane` owns a `#stock-search-input` that filters the
 * displayed low-stock rows client-side on every keystroke (case-insensitive
 * substring on French designation or SKU — the only text fields carried by
 * `LowStockRow`). Node-only harness: `filterStockRows` is exercised
 * directly, and source-text contracts pin the input wiring.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { filterStockRows } from "@/features/stock/StockOverviewPane";
import type { LowStockRow } from "@/types/reports";
import { t } from "@/lib/i18n";

declare const process: { cwd(): string };

const PANE_SRC = readFileSync(
  join(process.cwd(), "src", "features", "stock", "StockOverviewPane.tsx"),
  "utf8",
);

function row(overrides: Partial<LowStockRow>): LowStockRow {
  return {
    productId: "p-1",
    sku: "VIS-AG-4X35",
    nameFr: "Boîte vis agglo 4x35",
    warehouseId: "w-1",
    warehouseCode: "DEP-01",
    quantityBaseMicro: "0",
    ...overrides,
  };
}

const ROWS: LowStockRow[] = [
  row({}),
  row({ productId: "p-2", sku: "DIS-115", nameFr: "Disque à tronçonner 115mm" }),
  row({ productId: "p-3", sku: "COL-PLB", nameFr: "Coude laiton 1/2" }),
];

describe("filterStockRows", () => {
  it("returns all rows for a blank query", () => {
    expect(filterStockRows(ROWS, "")).toBe(ROWS);
    expect(filterStockRows(ROWS, "   ")).toBe(ROWS);
  });

  it("matches French designations case-insensitively (first character suffices)", () => {
    expect(filterStockRows(ROWS, "o").map((r) => r.productId)).toEqual(["p-1", "p-2", "p-3"]);
    expect(filterStockRows(ROWS, "DISQUE").map((r) => r.productId)).toEqual(["p-2"]);
    expect(filterStockRows(ROWS, "coude").map((r) => r.productId)).toEqual(["p-3"]);
  });

  it("matches SKUs case-insensitively", () => {
    expect(filterStockRows(ROWS, "vis-ag").map((r) => r.productId)).toEqual(["p-1"]);
    expect(filterStockRows(ROWS, "dis-115").map((r) => r.productId)).toEqual(["p-2"]);
  });

  it("returns an empty list when nothing matches", () => {
    expect(filterStockRows(ROWS, "zzz")).toEqual([]);
  });
});

describe("StockOverviewPane search input (source contract)", () => {
  it("renders #stock-search-input wired to live state and memoised rows", () => {
    expect(PANE_SRC).toContain('id="stock-search-input"');
    expect(PANE_SRC).toContain("setStockQuery(event.target.value)");
    expect(PANE_SRC).toContain("filterStockRows(props.rows, stockQuery)");
    expect(PANE_SRC).toContain("filteredRows.map((row)");
    expect(PANE_SRC).toContain("stock.searchPlaceholder");
  });

  it("exposes the placeholder key in both locales", () => {
    expect(t("fr-DZ", "stock.searchPlaceholder")).toBe("Rechercher par nom ou SKU…");
    expect(t("ar-DZ", "stock.searchPlaceholder")).toBe("بحث بالاسم أو الرمز…");
    expect(t("fr-DZ", "cat.depotSelect")).toBe("Dépôt cible du stock initial");
    expect(t("ar-DZ", "cat.depotSelect")).toBe("المستودع المستهدف للمخزون الأولي");
  });
});
