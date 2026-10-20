/**
 * Quick-palette wiring contracts (TASK-DSN-069).
 * The hook must intercept Alt+Digit ahead of the printable-key wedge;
 * the palette must mount inside the discovery column.
 */
import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized).
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const HOOK_SRC = readFileSync(
  join(process.cwd(), "src", "hooks", "use-pos-keyboard.ts"),
  "utf8",
);
const DISCOVERY_SRC = readFileSync(
  join(process.cwd(), "src", "components", "pos", "ProductDiscovery.tsx"),
  "utf8",
);
const PALETTE_SRC = readFileSync(
  join(process.cwd(), "src", "features", "pos", "QuickProductPalette.tsx"),
  "utf8",
);
const MODAL_SRC = readFileSync(
  join(process.cwd(), "src", "features", "pos", "QuickProductsConfigModal.tsx"),
  "utf8",
);
const I18N_SRC = readFileSync(
  join(process.cwd(), "src", "lib", "i18n.ts"),
  "utf8",
);

describe("palette wiring (TASK-DSN-069)", () => {
  it("hook intercepts Alt+Digit with preventDefault and dispatches the slot", () => {
    expect(HOOK_SRC).toContain("altDigitSlotIndex");
    expect(HOOK_SRC).toContain("dispatchSlot");
  });

  it("palette mounts inside the discovery column", () => {
    expect(DISCOVERY_SRC).toContain("QuickProductPalette");
  });

  it("palette renders Alt+n badges with the specified kbd styling", () => {
    expect(PALETTE_SRC).toContain("Alt+");
    expect(PALETTE_SRC).toContain("<kbd");
    expect(PALETTE_SRC).toContain("bg-surface-sunken");
  });

  it("hook suppresses Alt+1..8 dispatch when the palette is disabled", () => {
    expect(HOOK_SRC).toContain("useQuickProductsStore.getState().enabled");
    expect(HOOK_SRC).toContain("bypassed entirely");
    expect(HOOK_SRC).not.toMatch(/if \(quickSlot !== null && !modalOpen\) \{\s*event\.preventDefault\(\);/);
  });

  it("palette renders nothing when disabled and exposes the config gear", () => {
    expect(PALETTE_SRC).toContain("if (!enabled)");
    expect(PALETTE_SRC).toContain("return null");
    expect(PALETTE_SRC).toContain("setConfigOpen(true)");
    expect(PALETTE_SRC).toContain("pos.customizeQuickProducts");
  });

  it("discovery mounts the toggle and the config dialog", () => {
    expect(DISCOVERY_SRC).toContain("pos.quickProductsToggle");
    expect(DISCOVERY_SRC).toContain("toggleQuick");
    expect(DISCOVERY_SRC).toContain("aria-pressed={quickEnabled}");
    expect(DISCOVERY_SRC).toContain("QuickProductsConfigModal");
  });

  it("config modal assigns through active-only search and guards inactive", () => {
    expect(MODAL_SRC).toContain("ipcSearchCatalog(token, query, ASSIGN_SEARCH_LIMIT, false)");
    expect(MODAL_SRC).toContain("resetDefaults(rows)");
    expect(MODAL_SRC).toContain("product.is_active !== true");
    expect(MODAL_SRC).toContain("pos.inactiveProductWarning");
    expect(MODAL_SRC).toContain("aria-modal=\"true\"");
    expect(MODAL_SRC).toContain("fixed inset-0");
  });

  it("i18n carries the quick-products keys in French and Arabic", () => {
    for (const key of [
      "pos.quickProductsToggle",
      "pos.showQuickProducts",
      "pos.hideQuickProducts",
      "pos.customizeQuickProducts",
      "pos.quickModalTitle",
      "pos.slotEmpty",
      "pos.assignProduct",
      "pos.changeProduct",
      "pos.clearSlot",
      "pos.resetDefaults",
      "pos.quickClose",
      "pos.quickSearchHint",
      "pos.inactiveProductWarning",
    ]) {
      expect(I18N_SRC).toContain(`"${key}"`);
    }
    expect(I18N_SRC).toContain("\"pos.quickModalTitle\": \"Produits rapides");
    expect(I18N_SRC).toContain("\"pos.quickModalTitle\": \"المنتجات السريعة");
  });
});
