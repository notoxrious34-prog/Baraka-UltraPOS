/**
 * Shell restyle pins (TASK-DSN-090): icon set, nav icon+text, strip zones,
 * cash menu in-viewport, banner removal, notice popup, printer guidance.
 * Node-only harness: source-text pins + store flow (no jsdom).
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { usePosUiStore } from "@/stores/pos-ui-store";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const ICONS_SRC = readSrc("src", "components", "icons.tsx");
const APP_SRC = readSrc("src", "App.tsx");
const TOPBAR_SRC = readSrc("src", "components", "pos", "Topbar.tsx");
const CASH_SRC = readSrc("src", "components", "pos", "CashSessionIndicator.tsx");
const SHELL_SRC = readSrc("src", "components", "pos", "PosShell.tsx");
const STORE_SRC = readSrc("src", "stores", "pos-ui-store.ts");
const NOTICE_SRC = readSrc("src", "components", "pos", "NoticePopup.tsx");
const OPS_SRC = readSrc("src", "components", "pos", "OperationsMenu.tsx");
const CHECKOUT_SRC = readSrc("src", "components", "pos", "modals", "CheckoutModal.tsx");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

const ICONS = [
  "CashIcon",
  "DepotIcon",
  "OperationsIcon",
  "UserIcon",
  "ClockIcon",
  "LangIcon",
  "MoonIcon",
  "SunIcon",
  "LogoutIcon",
  "PosIcon",
  "CatalogIcon",
  "StockIcon",
  "CustomersIcon",
  "DocumentsIcon",
  "SuppliersIcon",
  "ReportsIcon",
  "SettingsIcon",
];

describe("shell restyle (DSN-090)", () => {
  it("ships the inline SVG icon set (1.75px stroke, currentColor, no emoji)", () => {
    for (const name of ICONS) {
      expect(ICONS_SRC).toContain(`export function ${name}`);
    }
    expect(ICONS_SRC).toContain("strokeWidth={1.75}");
    expect(ICONS_SRC).toContain('stroke="currentColor"');
  });

  it("renders the full brand name with icon+text nav pills", () => {
    expect(APP_SRC).toContain("Baraka UltraPOS");
    for (const icon of [
      "PosIcon",
      "CatalogIcon",
      "StockIcon",
      "CustomersIcon",
      "DocumentsIcon",
      "SuppliersIcon",
      "ReportsIcon",
      "SettingsIcon",
      "UserIcon",
    ]) {
      expect(APP_SRC).toContain(icon);
    }
  });

  it("uses SVG moon/sun instead of the cryptic glyph toggle", () => {
    expect(TOPBAR_SRC).toContain("MoonIcon");
    expect(TOPBAR_SRC).toContain("SunIcon");
    expect(TOPBAR_SRC).not.toContain('"☀"');
    expect(TOPBAR_SRC).not.toContain('"☾"');
  });

  it("opens the cash menu in-viewport (start-aligned, never end-0 off-screen)", () => {
    expect(CASH_SRC).toContain("absolute start-0 top-full");
    expect(CASH_SRC).not.toContain("absolute end-0 top-full");
  });

  it("removes the banner element and its state (no dead code)", () => {
    expect(SHELL_SRC).not.toContain("bg-emerald-700");
    expect(SHELL_SRC).not.toContain("banner");
    expect(STORE_SRC).not.toContain("setBanner");
    expect(STORE_SRC).not.toContain("banner");
    expect(CHECKOUT_SRC).not.toContain("setBanner");
  });

  it("routes checkout success through notify on modal close (no double message)", () => {
    expect(CHECKOUT_SRC).toContain("notify({");
    expect(CHECKOUT_SRC).toContain('detail: `${done.document_number}');
  });

  it("gates the popup below modals with auto-dismiss and live region", () => {
    for (const marker of [
      'activeModal !== "none"',
      "4000",
      'role="status"',
      'aria-live="polite"',
      "z-40",
      "NoticePopup",
    ]) {
      expect(NOTICE_SRC).toContain(marker);
    }
    expect(SHELL_SRC).toContain("NoticePopup");
  });

  it("declares the printer COM guidance keys in FR and AR", () => {
    for (const entry of [
      '"settings.printerComGuideTitle": "Brancher l',
      '"settings.printerComGuideCom": "Port COM',
      '"settings.printerComGuideUsb": "Imprimante USB',
      '"settings.printerComGuideTransport"',
      '"settings.printerComGuideTitle": "توصيل طابعة التذاكر"',
      '"settings.printerComGuideCom": "منفذ COM',
      '"settings.printerComGuideUsb": "طابعة USB',
      '"settings.printerComGuideTransport"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });

  it("maps every operations entry to an inline SVG (zero emoji icons)", () => {
    for (const [entry, icon] of [
      ["conversion: DocumentsIcon", "DocumentsIcon"],
      ["return: LogoutIcon", "LogoutIcon"],
      ['"warehouse-transfer": DepotIcon', "DepotIcon"],
      ['"stock-adjust": StockIcon', "StockIcon"],
      ["backup: ShieldIcon", "ShieldIcon"],
      ["identity: PrinterIcon", "PrinterIcon"],
    ]) {
      expect(OPS_SRC).toContain(entry);
      expect(ICONS_SRC).toContain(`export function ${icon}`);
    }
    for (const glyph of ["⇄", "↩", "⚖", "🛡", "🖨"]) {
      expect(OPS_SRC).not.toContain(glyph);
    }
  });

  it("drives the notice lifecycle through the store (notify → visible → dismiss)", () => {
    const store = usePosUiStore.getState();
    expect(store.notice).toBeNull();
    store.notify({ title: "Vente validée", detail: "T-1 · 100 — attente" });
    expect(usePosUiStore.getState().notice?.title).toBe("Vente validée");
    usePosUiStore.getState().dismissNotice();
    expect(usePosUiStore.getState().notice).toBeNull();
  });

  it("clears a pending notice when a modal opens (never stacked under a modal)", () => {
    const store = usePosUiStore.getState();
    store.notify({ title: "Vente validée" });
    store.openModal("checkout");
    expect(usePosUiStore.getState().notice).toBeNull();
    usePosUiStore.getState().closeModal();
  });
});
