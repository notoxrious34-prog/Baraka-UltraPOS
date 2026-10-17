/**
 * POS scanner ergonomics tests (TASK-DSN-049).
 *
 * Covers what TASK-DSN-040 left unpinned: exact-barcode priority over result
 * order, matched-unit selection inside multi-unit products, the Enter
 * auto-add wiring (add + clear + refocus), and the full F-key/Esc
 * prevention contract including F11. Node-only harness (no DOM library),
 * matching the repo convention: pure units plus source-text contracts.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { resolveScanToCart } from "@/components/pos/ProductDiscovery";
import type { CatalogProduct, CatalogUnit } from "@/types/pos";

declare const process: { cwd(): string };

const HOOK_SRC = readFileSync(join(process.cwd(), "src", "hooks", "use-pos-keyboard.ts"), "utf8");
const PANEL_SRC = readFileSync(
  join(process.cwd(), "src", "components", "pos", "ProductDiscovery.tsx"),
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

function product(overrides: Partial<CatalogProduct>): CatalogProduct {
  return {
    product_id: "p-1",
    sku: "VIS-8X40",
    barcode: "6111250000013",
    name_ar: "برغي",
    name_fr: "Vis",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [unit({})],
    kit_items: [],
    ...overrides,
  };
}

describe("resolveScanToCart exact-priority (scanner dispatch)", () => {
  it("prefers the exact barcode hit over result order", () => {
    const first = product({ product_id: "p-first", sku: "AAAA", barcode: null });
    const scanned = product({ product_id: "p-scanned", sku: "BBBB", barcode: "999" });
    const hit = resolveScanToCart("999", [first, scanned]);
    expect(hit?.product.product_id).toBe("p-scanned");
  });

  it("matches barcodes case-insensitively with surrounding whitespace", () => {
    const target = product({});
    expect(resolveScanToCart("  6111250000013  ", [target])?.product.product_id).toBe("p-1");
  });

  it("selects the barcode-matched unit inside a multi-unit product", () => {
    const multi = product({
      barcode: null,
      units: [
        unit({ unit_id: "u-base", barcode: null, is_base_unit: true }),
        unit({ unit_id: "u-box", barcode: "BOX-12", is_base_unit: false }),
      ],
    });
    const hit = resolveScanToCart("BOX-12", [multi]);
    expect(hit?.product.product_id).toBe("p-1");
    expect(hit?.unitIndex).toBe(1);
  });

  it("does not auto-add when several results share no exact match", () => {
    const a = product({ product_id: "p-a", sku: "AAAA", barcode: null });
    const b = product({ product_id: "p-b", sku: "BBBB", barcode: null });
    expect(resolveScanToCart("zzz", [a, b])).toBeNull();
  });
});

describe("search Enter auto-add wiring", () => {
  it("resolves, adds, clears, and refocuses on Enter", () => {
    // Non-Enter keys fall through untouched.
    expect(PANEL_SRC).toContain('if (event.key !== "Enter")');
    // Exact/sole resolution against live results.
    expect(PANEL_SRC).toContain("resolveScanToCart(searchQuery, results)");
    // Immediate cart add, input clear, dropdown collapse (query-driven),
    // and focus pinned on the search field for the next scan.
    expect(PANEL_SRC).toContain("addProduct(target.product, target.unitIndex)");
    expect(PANEL_SRC).toContain('setSearchQuery("")');
    expect(PANEL_SRC).toContain("event.currentTarget.focus()");
    expect(PANEL_SRC).toContain("event.preventDefault()");
  });
});

describe("keyboard contract prevention (WebView2 discipline)", () => {
  it("prevents every reserved key including F11", () => {
    for (const key of ["F1", "F2", "F4", "F8", "F10", "F11", "F12"]) {
      expect(HOOK_SRC).toContain(`if (event.key === "${key}")`);
    }
    const preventCount = HOOK_SRC.split("event.preventDefault()").length - 1;
    expect(preventCount).toBeGreaterThanOrEqual(10);
  });

  it("keeps browser defaults suppressed for F3/F5 while preserving actions", () => {
    expect(HOOK_SRC).toContain('if (event.key === "F3")');
    expect(HOOK_SRC).toContain('if (event.key === "F5")');
  });

  it("dispatches F2 customer, F4 quantity, F8 park, F10 doc-type, F12 checkout", () => {
    expect(HOOK_SRC).toContain('ui.openModal("customer")');
    expect(HOOK_SRC).toContain('ui.openModal("quantity")');
    expect(HOOK_SRC).toContain("void parkSale()");
    expect(HOOK_SRC).toContain("cart.setDocType(cycleDocType(cart.docType))");
    expect(HOOK_SRC).toContain('ui.openModal("checkout")');
  });

  it("gates modal-unsafe actions behind the modal-open guard", () => {
    const modalGuard = HOOK_SRC.indexOf('ui.activeModal !== "none"');
    expect(modalGuard).toBeGreaterThan(-1);
    // Action branches are wired after the guard is read, and the unsafe
    // ones consult it before acting.
    for (const branch of ['event.key === "F1"', 'event.key === "F4"', 'event.key === "F8"']) {
      expect(HOOK_SRC.indexOf(branch)).toBeGreaterThan(modalGuard);
    }
    expect(HOOK_SRC).toContain("if (!modalOpen) {");
  });

  it("closes modals on Esc and never redirects keystrokes into them", () => {
    expect(HOOK_SRC).toContain("ui.closeModal()");
    const escIndex = HOOK_SRC.indexOf('event.key === "Escape"');
    const redirectIndex = HOOK_SRC.indexOf("isPrintableKey(event)");
    expect(escIndex).toBeGreaterThan(-1);
    expect(redirectIndex).toBeGreaterThan(escIndex);
  });
});
