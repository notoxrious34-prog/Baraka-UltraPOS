/**
 * Silent-print path pins (TASK-DSN-098): FAIL-before on the BRK-097 tree.
 * The engine exposes print_tspl_label/print_tspl_ticket (e07b495) but no
 * UI path reaches them without a dialog: the graphical ticket flows call
 * window.print (drawer + checkout auto-print) and the catalog label modal
 * dispatches through a print() iframe. The silent facade, the shared
 * spooler-gate, and the 40x20 modal do not exist yet — every block below
 * fails until they land.
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

describe("silent-print FAIL-before (dialog-bound baseline)", () => {
  it("quotes the drawer dialog path (window.print, frozen reference)", () => {
    const drawer = readSrc("src", "features", "documents", "DocumentDetailDrawer.tsx");
    expect(drawer).toContain("window.print()");
    expect(drawer).toContain("printTicketDocument");
  });

  it("quotes the checkout dialog path (auto + manual print, frozen reference)", () => {
    const checkout = readSrc("src", "components", "pos", "modals", "CheckoutModal.tsx");
    expect(checkout).toContain("printTicketDocument(html)");
    expect(checkout).toContain("printTicketDocument(ticketHtml)");
  });

  it("quotes the catalog dialog path (iframe win.print, frozen reference)", () => {
    const label = readSrc("src", "features", "catalog", "ProductLabelModal.tsx");
    expect(label).toContain("win.print()");
  });
});

describe("silent-print surface (red until DSN-098 lands)", () => {
  it("exposes a validated silent facade (tspl-print)", async () => {
    const facade = await import("@/lib/tspl-print");
    expect(facade.validateTsplCopies(1)).toBe(1);
    expect(facade.validateTsplCopies(99)).toBe(99);
    expect(() => facade.validateTsplCopies(0)).toThrow("TSPL_COPIES_OUT_OF_RANGE");
    expect(() => facade.validateTsplCopies(100)).toThrow("TSPL_COPIES_OUT_OF_RANGE");
    expect(typeof facade.ipcPrintTsplLabel).toBe("function");
    expect(typeof facade.ipcPrintTsplTicket).toBe("function");
  });

  it("shares one spooler gate (ticket-print-path)", async () => {
    const gate = await import("@/lib/ticket-print-path");
    expect(gate.resolveTicketPrintPath({ transport: "spooler", renderMode: "graphique", documentType: "TICKET" })).toBe("silent-tspl");
    expect(gate.resolveTicketPrintPath({ transport: "serial", renderMode: "graphique", documentType: "TICKET" })).toBe("graphique");
    expect(gate.resolveTicketPrintPath({ transport: "serial", renderMode: "escpos", documentType: "TICKET" })).toBe("engine-reprint");
    expect(gate.resolveTicketPrintPath({ transport: "spooler", renderMode: "graphique", documentType: "FACTURE" })).toBe("existing");
  });

  it("ships a 40x20 silent label modal (never window.print)", () => {
    const modal = readSrc("src", "features", "catalog", "SilentLabelModal.tsx");
    expect(modal).toContain("40");
    expect(modal).not.toContain("window.print");
    expect(modal).not.toContain("printTicketDocument");
    expect(modal).not.toContain("printLabelDocument");
  });

  it("keeps every silent-path module dialog-free", () => {
    for (const file of ["src/lib/tspl-print.ts", "src/lib/ticket-print-path.ts"]) {
      const src = readSrc(...(file.split("/") as [string, string, string]));
      expect(src).not.toContain("window.print");
      expect(src).not.toContain("printTicketDocument");
      expect(src).not.toContain("printLabelDocument");
    }
    const drawer = readSrc("src", "features", "documents", "DocumentDetailDrawer.tsx");
    expect(drawer).toContain("printTsplTicketSilent");
    expect(drawer).not.toContain("printTicketDocument(ticketHtml)");
    const checkout = readSrc("src", "components", "pos", "modals", "CheckoutModal.tsx");
    expect(checkout).toContain("printTsplTicketSilent");
  });

  it("passes copies exactly once (no double multiplication)", () => {
    const facade = readSrc("src", "lib", "tspl-print.ts");
    expect(facade).toContain("validateTsplCopies(input.copies)");
    const bridge = readSrc("src", "lib", "ipc.ts");
    expect(bridge).toContain("copies: job.copies");
    expect(bridge).not.toMatch(/copies\s*\*/);
  });
});
