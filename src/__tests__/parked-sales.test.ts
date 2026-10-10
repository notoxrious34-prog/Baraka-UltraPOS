/**
 * Parked sales regression test (TASK-DSN-026): F6 resume modal wiring,
 * cart rehydration action, HUD badge, and symmetric i18n keys.
 *
 * Node-only harness (no jsdom): source-text assertions for wiring plus
 * direct zustand store behavior for loadLines.
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
const readSrc = (...parts: string[]): string =>
  readFileSync(join(ROOT, ...parts), "utf8");

const I18N_SRC = readSrc("src", "lib", "i18n.ts");
const MODAL_SRC = readSrc("src", "components", "pos", "ParkedSalesModal.tsx");
const KEYBOARD_SRC = readSrc("src", "hooks", "use-pos-keyboard.ts");
const FOOTER_SRC = readSrc("src", "components", "pos", "KeyFooter.tsx");
const TOPBAR_SRC = readSrc("src", "components", "pos", "Topbar.tsx");
const SHELL_SRC = readSrc("src", "components", "pos", "PosShell.tsx");

describe("parked sales i18n keys", () => {
  it("declares all parked.* and key.f6 keys in FR", () => {
    for (const entry of [
      '"parked.title": "Ventes en attente"',
      '"parked.empty": "Aucune vente en attente"',
      '"parked.resume": "Reprendre (F6)"',
      '"parked.discard": "Supprimer"',
      '"parked.badge": "En attente"',
      '"parked.resumedOk": "Vente reprise dans le panier"',
      '"parked.discardedOk": "Vente en attente supprimée"',
      '"parked.itemCount": "articles"',
      '"key.f6": "F6 Reprendre"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });

  it("declares all parked.* and key.f6 keys in AR", () => {
    for (const entry of [
      '"parked.title": "السلات المعلقة"',
      '"parked.empty": "لا توجد سلات معلقة"',
      '"parked.resume": "استرجاع (F6)"',
      '"parked.discard": "إلغاء"',
      '"parked.badge": "معلقة"',
      '"parked.resumedOk": "تم استرجاع السلة بنجاح"',
      '"parked.discardedOk": "تم إلغاء السلة المعلقة"',
      '"parked.itemCount": "مواد"',
      '"key.f6": "F6 استرجاع"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});

describe("F6 keyboard and footer wiring", () => {
  it("binds F6 to the parked modal in the sovereign keyboard contract", () => {
    expect(KEYBOARD_SRC).toContain('event.key === "F6"');
    expect(KEYBOARD_SRC).toContain('ui.openModal("parked")');
  });

  it("lists F6 between F5 and F8 in the key footer legend", () => {
    const f5 = FOOTER_SRC.indexOf('"key.f5"');
    const f6 = FOOTER_SRC.indexOf('"key.f6"');
    const f8 = FOOTER_SRC.indexOf('"key.f8"');
    expect(f5).toBeGreaterThanOrEqual(0);
    expect(f6).toBeGreaterThan(f5);
    expect(f8).toBeGreaterThan(f6);
  });

  it("mounts the parked modal in PosShell", () => {
    expect(SHELL_SRC).toContain('activeModal === "parked"');
    expect(SHELL_SRC).toContain("<ParkedSalesModal />");
  });
});

describe("ParkedSalesModal surface", () => {
  it("loads DRAFT documents and renders both empty and populated states", () => {
    expect(MODAL_SRC).toContain('status: "DRAFT"');
    expect(MODAL_SRC).toContain('"parked.empty"');
    expect(MODAL_SRC).toContain('"parked.resume"');
    expect(MODAL_SRC).toContain('"parked.discard"');
    expect(MODAL_SRC).toContain('"parked.itemCount"');
  });

  it("resumes via details fetch, line rehydration, and draft discard", () => {
    expect(MODAL_SRC).toContain("getDocumentDetails");
    expect(MODAL_SRC).toContain("ipcGetProduct");
    expect(MODAL_SRC).toContain("loadLines");
    expect(MODAL_SRC).toContain("ipcDiscardDraft");
  });

  it("shows the HUD badge hook in Topbar with count-gated rendering", () => {
    expect(TOPBAR_SRC).toContain("parkedCount");
    expect(TOPBAR_SRC).toContain('"parked.badge"');
    expect(TOPBAR_SRC).toContain('openModal("parked")');
  });
});

describe("cart-store loadLines", async () => {
  const { useCartStore } = await import("@/stores/cart-store");

  it("declares the loadLines action on the store", () => {
    expect(typeof useCartStore.getState().loadLines).toBe("function");
  });

  it("atomically replaces lines, resets discount, and sets the customer", () => {
    const store = useCartStore.getState();
    store.clearCart();
    store.setGlobalDiscount("500");
    store.loadLines(
      [
        {
          key: "p1:u1:1",
          productId: "p1",
          sku: "SKU-1",
          nameAr: "منتج",
          nameFr: "Produit",
          units: [],
          unitIndex: 0,
          quantityMicro: "20000",
          lineDiscountCentimes: "0",
        },
      ],
      { kind: "named", id: "c1", name: "Client", policy: "LIMITED", limitCentimes: null, balanceCentimes: "0", source: "backend" },
    );
    const state = useCartStore.getState();
    expect(state.lines).toHaveLength(1);
    expect(state.lines[0]?.productId).toBe("p1");
    expect(state.globalDiscountCentimes).toBe("0");
    expect(state.customer).toEqual({
      kind: "named",
      id: "c1",
      name: "Client",
      policy: "LIMITED",
      limitCentimes: null,
      balanceCentimes: "0",
      source: "backend",
    });
    store.clearCart();
  });

  it("defaults to guest customer when omitted", () => {
    useCartStore.getState().loadLines([]);
    expect(useCartStore.getState().customer).toEqual({ kind: "guest" });
    useCartStore.getState().clearCart();
  });
});
