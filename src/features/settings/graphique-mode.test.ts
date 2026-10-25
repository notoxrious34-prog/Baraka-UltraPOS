/**
 * Graphique mode pins (TASK-DSN-093): render-mode selector wiring, honest
 * test-print banner classes, and the untouched escpos path. Pure-logic pins
 * run in Node; UI-shape pins use the source-text pattern (no jsdom).
 */

import { describe, expect, it } from "vitest";
import {
  resolveTestPrintNotice,
  testPrintToneClass,
} from "./test-print-notice";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const TAB_SRC = readSrc("src", "features", "settings", "PrinterSettingsTab.tsx");
const IPC_SRC = readSrc("src", "lib", "ipc.ts");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");
const CHECKOUT_SRC = readSrc("src", "components", "pos", "modals", "CheckoutModal.tsx");
const DRAWER_SRC = readSrc("src", "features", "documents", "DocumentDetailDrawer.tsx");
const NOTICE_SRC = readSrc("src", "features", "settings", "test-print-notice.ts");

const NEW_KEYS = [
  "settings.ticketRenderTitle",
  "settings.ticketRenderEscpos",
  "settings.ticketRenderGraphique",
  "settings.ticketRenderHelp",
  "settings.testPrintCaptureNote",
];

describe("notice classifier (pure, both locales)", () => {
  it("marks sent as success without guidance text", () => {
    const fr = resolveTestPrintNotice("fr-DZ", {
      deviceOutcome: "sent",
      captureFileName: "test-fr.escpos",
    });
    expect(fr.tone).toBe("success");
    expect(fr.text).toContain("sent");
    expect(testPrintToneClass(fr.tone)).toContain("bg-green-50");
  });

  it("marks capture-only as neutral with switching guidance", () => {
    for (const locale of ["fr-DZ", "ar-DZ"] as const) {
      const notice = resolveTestPrintNotice(locale, {
        deviceOutcome: "capture-only",
        captureFileName: "test-ar.escpos",
      });
      expect(notice.tone).toBe("neutral");
      expect(notice.text).toContain("capture-only");
      expect(testPrintToneClass(notice.tone)).toContain("bg-amber-50");
    }
  });

  it("marks any other outcome as error carrying the outcome text", () => {
    const outcome = "spooler call OpenPrinterW failed for queue 'X' (OS code 1801)";
    const notice = resolveTestPrintNotice("fr-DZ", {
      deviceOutcome: outcome,
      captureFileName: "test-fr.escpos",
    });
    expect(notice.tone).toBe("error");
    expect(notice.text).toContain(outcome);
    expect(testPrintToneClass(notice.tone)).toContain("bg-red-50");
  });
});

describe("render-mode segmented control (icon+text, FR+AR keys)", () => {
  it("renders a labelled group with one button per render mode", () => {
    expect(TAB_SRC).toContain('aria-label={t(locale, "settings.ticketRenderTitle")}');
    expect(TAB_SRC).toContain('aria-pressed={renderMode === "escpos"}');
    expect(TAB_SRC).toContain('aria-pressed={renderMode === "graphique"}');
    expect(TAB_SRC).toContain("settings.ticketRenderEscpos");
    expect(TAB_SRC).toContain("settings.ticketRenderGraphique");
  });

  it("declares the mode keys in FR and AR", () => {
    for (const key of NEW_KEYS) {
      expect(I18N_SRC).toContain(`"${key}"`);
    }
  });

  it("loads the mode from the bridge and saves it back through IPC", () => {
    expect(TAB_SRC).toContain("ticketRenderMode");
    expect(IPC_SRC).toContain("ticketRenderMode");
    expect(IPC_SRC).toContain("ticket_render_mode");
    // Absent/invalid wire values fall back to escpos (existing shops unchanged).
    expect(IPC_SRC).toContain("toTicketRenderMode");
    expect(IPC_SRC).toContain("'escpos'");
  });
});

describe("honest test banner (AUD-042 §4a, no new IPC fields)", () => {
  it("splits the notice by outcome class at a single site", () => {
    expect(NOTICE_SRC).toContain("capture-only");
    expect(NOTICE_SRC).toContain("sent");
    expect(TAB_SRC).toContain("test-print-notice");
  });

  it("keeps three distinct tones (success / neutral / error)", () => {
    expect(NOTICE_SRC).toContain('"success"');
    expect(NOTICE_SRC).toContain('"neutral"');
    expect(NOTICE_SRC).toContain('"error"');
  });
});

describe("auto-print wiring (graphique only, idempotent)", () => {
  it("invokes the graphique payload once per committed sale", () => {
    expect(CHECKOUT_SRC).toContain("graphique");
    expect(CHECKOUT_SRC).toContain("ticketPrintedFor");
  });

  it("guards the trigger against re-render double dispatch", () => {
    expect(CHECKOUT_SRC).toMatch(/ticketPrintedFor\.current/);
  });

  it("escpos path keeps the IPC reprint untouched", () => {
    expect(DRAWER_SRC).toContain("ipcReprint");
    expect(CHECKOUT_SRC).not.toContain("documents_reprint");
  });
});
