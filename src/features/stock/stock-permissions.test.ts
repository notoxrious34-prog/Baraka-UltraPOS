/**
 * Stock module root & action permission gating (TASK-DSN-077).
 *
 * "Visible but Disabled with Guidance": the module root follows VIEW_STOCK
 * (all store roles); mutating actions render disabled with a localized
 * `title` naming the reservation — never silently hidden. Rail visibility
 * already migrated (DSN-072). Permission sets mirror
 * `crates/domain/src/role.rs:121-174` (SCREAMING_SNAKE_CASE wire form).
 *
 * Harness: node-only `renderToStaticMarkup` (repo convention — no DOM lib).
 * Effects never run under SSR, so no IPC fires; only the static gate markup
 * is asserted.
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { describe, expect, it } from "vitest";
declare const process: { cwd(): string };
import { renderToStaticMarkup } from "react-dom/server";
import { createElement } from "react";
import { StockOverviewPane } from "@/features/stock/StockOverviewPane";
import type { Locale } from "@/types/pos";
import type { LowStockRow } from "@/types/reports";

const ROOT = process.cwd();
const MODULE_SRC: string = readFileSync(
  join(ROOT, "src", "features", "stock", "StockModule.tsx"),
  "utf8",
);
const OVERVIEW_SRC: string = readFileSync(
  join(ROOT, "src", "features", "stock", "StockOverviewPane.tsx"),
  "utf8",
);

const LOCALE: Locale = "fr-DZ";

const ROW: LowStockRow = {
  warehouseId: "w-1",
  warehouseCode: "DEP-01",
  productId: "p-1",
  sku: "VIS-001",
  nameFr: "Vis à bois",
  quantityBaseMicro: "0",
};

function pane(canAdjust: boolean): string {
  return renderToStaticMarkup(
    createElement(StockOverviewPane, {
      locale: LOCALE,
      warehouseCount: 1,
      rows: [ROW],
      loading: false,
      canAdjust,
      onAdjust: () => undefined,
    }),
  );
}

describe("stock root gate (TASK-DSN-077)", () => {
  // NOTE: module roots subscribe to the zustand session store, whose
  // seeded state is invisible to renderToStaticMarkup (SSR server
  // snapshot). Per repo convention (catalog-hub.test.tsx header),
  // container wiring is pinned via source-text assertions; the
  // store-independent StockOverviewPane renders below via props.
  it("authorizes the root view via VIEW_STOCK, not a manager role check", () => {
    expect(MODULE_SRC).toContain('hasPermission("VIEW_STOCK")');
    expect(MODULE_SRC).not.toContain("isManagerRole");
  });

  it("keeps the UNAUTHORIZED fallback and renders the stock chrome past the gate", () => {
    expect(MODULE_SRC).toContain('translateBackendError("UNAUTHORIZED"');
    expect(MODULE_SRC).toContain('"stock.title"');
    expect(MODULE_SRC).toContain("<StockOverviewPane");
  });
});

describe("stock action gating with guidance (TASK-DSN-077)", () => {
  it("wires header actions to ADJUST_STOCK / TRANSFER_STOCK / MANAGE_WAREHOUSES", () => {
    expect(MODULE_SRC).toContain('hasPermission("ADJUST_STOCK")');
    expect(MODULE_SRC).toContain('hasPermission("TRANSFER_STOCK")');
    expect(MODULE_SRC).toContain('hasPermission("MANAGE_WAREHOUSES")');
    expect(MODULE_SRC).toContain("stock.adjustReservedGuidance");
    expect(MODULE_SRC).toContain("stock.transferReservedGuidance");
    expect(MODULE_SRC).toContain("stock.warehouseCreateReservedGuidance");
    expect(MODULE_SRC).toContain("canAdjust={canAdjust}");
    expect(OVERVIEW_SRC).toContain("canAdjust: boolean");
    expect(OVERVIEW_SRC).toContain("stock.adjustReservedGuidance");
  });

  it("renders every header action visibly (never silently hidden) with disabled + guidance wiring", () => {
    expect(MODULE_SRC).toContain("disabled={!canAdjust}");
    expect(MODULE_SRC).toContain("disabled={!canTransfer}");
    expect(MODULE_SRC).toContain("disabled={!canManageWarehouses}");
    expect(MODULE_SRC).not.toContain("isAdmin && (");
    expect(MODULE_SRC).toContain('"stock.newWarehouse"');
  });

  it("disables row-level adjust buttons with guidance when unauthorized", () => {
    const html = pane(false);
    expect(html).toContain('disabled=""');
    expect(html).toContain("ajustement manuel des stocks");
  });

  it("enables row-level adjust buttons when authorized", () => {
    const html = pane(true);
    expect(html).not.toContain('disabled=""');
  });
});
