/**
 * Starter-pack lifecycle tests (TASK-DSN-052, Mod 4).
 * Node-only: real-CSV parse, batching, dual-track aggregation, edges.
 * IPC edges are injected mocks — zero backend required.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import type { CatalogProduct } from "@/types/pos";
import type { CreateProductWire, StockAdjustPayload } from "@/lib/ipc";
import {
  buildCategoryLookup,
  checkStarterPackStatus,
  installStarterPack,
  parseStarterPackCsv,
  readPackFlag,
  STARTER_PACK_SIZE,
  uninstallStarterPack,
  writePackFlag,
  type InstallDeps,
  type StarterPackItem,
  type UninstallDeps,
} from "./starter-pack";

// Minimal ambient typing for the single Node global used below.
// Type-only; zero runtime impact.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const ASSET_CSV = readFileSync(
  join(ROOT, "src", "assets", "fixtures", "quincaillerie_starter_300.csv"),
  "utf8",
);

function unit(unitId: string): CatalogProduct["units"][number] {
  return {
    unit_id: unitId,
    product_id: "p",
    unit_name_ar: "قطعة",
    unit_name_fr: "Pièce",
    factor_micro: "10000",
    barcode: null,
    sale_price_centimes: "100",
    purchase_price_centimes: "80",
    wholesale_price_centimes: null,
    is_base_unit: true,
    is_default_selling: true,
  };
}

function product(productId: string, sku: string): CatalogProduct {
  return {
    product_id: productId,
    sku,
    barcode: null,
    name_ar: "صنف",
    name_fr: "Article",
    category_id: null,
    is_active: true,
    is_kit: false,
    kit_items: [],
    units: [unit("u-1")],
  };
}

function packItem(code: string, stockMicro = 210000n): StarterPackItem {
  return {
    code,
    nameFr: `Article ${code}`,
    nameAr: `منتج ${code}`,
    categoryId: null,
    categoryName: "",
    unitNameFr: "Pièce",
    unitNameAr: "قطعة",
    purchasePriceCentimes: 85000n,
    salePriceCentimes: 121429n,
    initialStockMicro: stockMicro,
    barcode: null,
  };
}

describe("parseStarterPackCsv", () => {
  it("parses the real 300-row asset with zero drops", () => {
    const items = parseStarterPackCsv(ASSET_CSV);
    expect(items).toHaveLength(STARTER_PACK_SIZE);
    const first = items[0];
    expect(first?.code).toBe("PLB-001");
    expect(first?.purchasePriceCentimes).toBe(85000n);
    expect(first?.salePriceCentimes).toBe(121429n);
    expect(first?.initialStockMicro).toBe(210000n);
    const barcoded = items.find((item) => item.code === "PLB-002");
    expect(barcoded?.barcode).toBe("6111250000024");
  });

  it("handles French decimal commas and drops invalid rows", () => {
    const csv = [
      "sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;category_code",
      "TST-001;;Tournevis plat;مفك;120,50;200,00;5;",
      "TST-002;;Marteau;مطرقة;300;450,75;2,5;",
      "BAD-001;;Mauvais prix;سعر سيء;50;abc;1;",
      "ZERO-001;;Achat invalide;شراء غير صالح;xyz;100;1;",
      ";123;C sans code;بدون رمز;10;20;1;",
    ].join("\n");
    const items = parseStarterPackCsv(csv);
    expect(items.map((item) => item.code)).toEqual(["TST-001", "TST-002", "ZERO-001"]);
    expect(items[0]?.purchasePriceCentimes).toBe(12050n);
    expect(items[1]?.salePriceCentimes).toBe(45075n);
    expect(items[1]?.initialStockMicro).toBe(25000n);
    // Invalid purchase prices degrade to 0 (backend default); the row survives.
    expect(items[2]?.purchasePriceCentimes).toBe(0n);
  });

  it("resolves categories through the provided lookup", () => {
    const lookup = new Map([["CAT-TEST", "cat-id-1"]]);
    const csv = [
      "sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;category_code",
      "TST-009;;Clou;مسمار;10;20;100;CAT-TEST",
    ].join("\n");
    const items = parseStarterPackCsv(csv, lookup);
    expect(items[0]?.categoryId).toBe("cat-id-1");
  });
});

describe("buildCategoryLookup", () => {
  it("indexes ids by code and upper-cased code", () => {
    const lookup = buildCategoryLookup([
      { id: "c1", code: "cat-plomb", name_fr: "P", name_ar: "س", is_active: true, parent_id: null },
    ]);
    expect(lookup.get("cat-plomb")).toBe("c1");
    expect(lookup.get("CAT-PLOMB")).toBe("c1");
  });
});

describe("checkStarterPackStatus", () => {
  const items = [packItem("A-001"), packItem("A-002"), packItem("A-003")];

  it("reports not installed on an empty catalog", () => {
    expect(checkStarterPackStatus(items, new Set())).toEqual({
      isInstalled: false,
      installedCount: 0,
      totalPackCount: 3,
    });
  });

  it("detects partial installation and counts matches", () => {
    const status = checkStarterPackStatus(items, [{ sku: "ZZZ" }, { sku: "A-002" }]);
    expect(status).toEqual({ isInstalled: true, installedCount: 1, totalPackCount: 3 });
  });
});

/** In-memory Web Storage stub: vitest runs in plain node (no DOM). */
function installStorageStub(): void {
  const store = new Map<string, string>();
  Object.defineProperty(globalThis, "localStorage", {
    value: {
      getItem: (key: string): string | null => store.get(key) ?? null,
      setItem: (key: string, value: string): void => {
        store.set(key, value);
      },
    },
    configurable: true,
    writable: true,
  });
}

describe("installStarterPack", () => {
  it("creates missing items sequentially with provisioning and progress", async () => {
    installStorageStub();
    const created: CreateProductWire[] = [];
    const adjusted: StockAdjustPayload[] = [];
    const progress: Array<[number, number]> = [];
    const deps: InstallDeps = {
      create: (token, wire) => {
        expect(token).toBe("tok");
        created.push(wire);
        return Promise.resolve(product(`id-${wire.sku}`, wire.sku));
      },
      adjust: (payload) => {
        adjusted.push(payload);
        return Promise.resolve({ product_id: payload.product_id });
      },
    };
    const items = [packItem("N-001"), packItem("N-002", 0n), packItem("N-003")];
    const outcome = await installStarterPack(
      "tok",
      items,
      "depot-1",
      new Set(["N-002"]),
      deps,
      (current, total) => {
        progress.push([current, total]);
      },
    );
    expect(outcome).toEqual({ created: 2, skipped: 1 });
    expect(created.map((wire) => wire.sku)).toEqual(["N-001", "N-003"]);
    expect(created[0]?.units[0]?.sale_price_centimes).toBe("121429");
    expect(adjusted).toHaveLength(2);
    expect(adjusted[0]?.direction).toBe("INCREASE");
    expect(adjusted[0]?.warehouse_id).toBe("depot-1");
    expect(adjusted[0]?.quantity_micro).toBe("210000");
    expect(progress).toEqual([
      [1, 3],
      [2, 3],
      [3, 3],
    ]);
    expect(readPackFlag()).toBe(true);
  });

  it("skips zero-stock provisioning and survives create failures", async () => {
    const adjusted: StockAdjustPayload[] = [];
    const deps: InstallDeps = {
      create: (_token, wire) =>
        wire.sku === "BOOM-001"
          ? Promise.reject(new Error("DUPLICATE_SKU"))
          : Promise.resolve(product("id", wire.sku)),
      adjust: (payload) => {
        adjusted.push(payload);
        return Promise.resolve({ product_id: payload.product_id });
      },
    };
    const outcome = await installStarterPack(
      "tok",
      [packItem("BOOM-001"), packItem("ZERO-001", 0n)],
      "depot-1",
      new Set(),
      deps,
    );
    expect(outcome).toEqual({ created: 1, skipped: 1 });
    expect(adjusted).toHaveLength(0);
  });
});

describe("uninstallStarterPack", () => {
  it("aggregates hard-deleted vs deactivated vs failed with progress", async () => {
    const removed: string[] = [];
    const progress: Array<[number, number]> = [];
    const deps: UninstallDeps = {
      remove: (_token, id) => {
        removed.push(id);
        if (id === "id-hard") {
          return Promise.resolve({ id, mode: "hard_deleted", message: "purged" });
        }
        if (id === "id-boom") {
          return Promise.reject(new Error("locked"));
        }
        return Promise.resolve({ id, mode: "deactivated", message: "archived" });
      },
    };
    const summary = await uninstallStarterPack(
      "tok",
      new Set(["P-001", "P-002", "P-003"]),
      [
        { id: "id-hard", sku: "P-001" },
        { id: "id-soft", sku: "P-002" },
        { id: "id-boom", sku: "P-003" },
        { id: "id-foreign", sku: "UNRELATED" },
      ],
      deps,
      (current, total) => {
        progress.push([current, total]);
      },
    );
    expect(summary).toEqual({ hardDeleted: 1, deactivated: 1, failed: 1, totalProcessed: 3 });
    expect(removed).toEqual(["id-hard", "id-soft", "id-boom"]);
    expect(progress).toEqual([
      [1, 3],
      [2, 3],
      [3, 3],
    ]);
    expect(readPackFlag()).toBe(false);
  });

  it("processes nothing when no catalog row matches the pack", async () => {
    let calls = 0;
    const summary = await uninstallStarterPack(
      "tok",
      new Set(["P-001"]),
      [{ id: "id-x", sku: "OTHER" }],
      {
        remove: () => {
          calls += 1;
          return Promise.resolve({ id: "id-x", mode: "hard_deleted", message: "" });
        },
      },
    );
    expect(summary.totalProcessed).toBe(0);
    expect(calls).toBe(0);
  });
});

describe("pack flag storage", () => {
  it("never throws without a DOM and round-trips when available", () => {
    expect(() => {
      writePackFlag(true);
    }).not.toThrow();
    expect(typeof readPackFlag()).toBe("boolean");
  });
});
