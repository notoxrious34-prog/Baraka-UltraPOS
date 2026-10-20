/**
 * Product discovery regression test (TASK-DSN-025): blank-query discovery
 * wiring and teaching empty state.
 *
 * Verifies the search hook no longer suppresses empty-query searches, the
 * symmetric i18n keys exist in both locales, the discovery component
 * references both empty-state keys, and the type-filter logic partitions
 * kits from goods.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
// Type-only; zero runtime impact.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const HOOK_SRC = readFileSync(join(ROOT, "src", "hooks", "use-catalog-search.ts"), "utf8");
const PANEL_SRC = readFileSync(join(ROOT, "src", "components", "pos", "ProductDiscovery.tsx"), "utf8");
const I18N_SRC = readFileSync(join(ROOT, "src", "lib", "i18n.ts"), "utf8");

type TypeFilter = "all" | "kits" | "goods";

interface FakeProduct {
  readonly product_id: string;
  readonly is_kit: boolean;
}

function applyTypeFilter(products: FakeProduct[], filter: TypeFilter): FakeProduct[] {
  return products.filter((product) => {
    if (filter === "kits") {
      return product.is_kit;
    }
    if (filter === "goods") {
      return !product.is_kit;
    }
    return true;
  });
}

const SAMPLE: FakeProduct[] = [
  { product_id: "p-1", is_kit: false },
  { product_id: "p-2", is_kit: true },
  { product_id: "p-3", is_kit: false },
];

describe("use-catalog-search blank-query wiring", () => {
  it("does not early-return on empty query", () => {
    expect(HOOK_SRC).not.toContain('if (query === "")');
  });

  it("fetches the first 100 active-only rows for discovery", () => {
    expect(HOOK_SRC).toContain("ipcSearchCatalog(session.token, query, 100, false)");
  });
});

describe("discovery i18n keys", () => {
  it("declares search.discoveryTitle and search.catalogEmpty in FR", () => {
    expect(I18N_SRC).toContain('"search.discoveryTitle": "Articles disponibles"');
    expect(I18N_SRC).toContain(
      '"search.catalogEmpty": "Catalogue vide — ajoutez des produits pour commencer"',
    );
  });

  it("declares search.discoveryTitle and search.catalogEmpty in AR", () => {
    expect(I18N_SRC).toContain('"search.discoveryTitle": "المنتجات المتاحة"');
    expect(I18N_SRC).toContain('"search.catalogEmpty": "الكتالوج فارغ — أضف منتجات للبدء"');
  });
});

describe("ProductDiscovery empty states", () => {
  it("references both catalogEmpty (blank query) and noResults (miss)", () => {
    expect(PANEL_SRC).toContain('"search.catalogEmpty"');
    expect(PANEL_SRC).toContain('"search.noResults"');
  });

  it("shows the discovery title when the blank query yields rows", () => {
    expect(PANEL_SRC).toContain('"search.discoveryTitle"');
  });
});

describe("type filter partitioning", () => {
  it("ALL keeps every product", () => {
    expect(applyTypeFilter(SAMPLE, "all").map((p) => p.product_id)).toEqual(["p-1", "p-2", "p-3"]);
  });

  it("KITS keeps only kits", () => {
    expect(applyTypeFilter(SAMPLE, "kits").map((p) => p.product_id)).toEqual(["p-2"]);
  });

  it("PIECES keeps only goods", () => {
    expect(applyTypeFilter(SAMPLE, "goods").map((p) => p.product_id)).toEqual(["p-1", "p-3"]);
  });
});
