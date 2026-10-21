/**
 * A4 print tests (TASK-DSN-085): per-type button decision, 25-line sheet,
 * missing-field omission, print CSS pins, i18n keys. Node-only harness:
 * renderToStaticMarkup for the pure sheet + source-text pins (no jsdom).
 */

import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  canPrintA4,
  DocumentA4Sheet,
  type A4Identity,
  type DocumentDetailsWire,
} from "@/features/documents/DocumentDetailDrawer";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const DRAWER_SRC = readSrc("src", "features", "documents", "DocumentDetailDrawer.tsx");
const CSS_SRC = readSrc("src", "features", "documents", "document-print.css");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

const FULL_IDENTITY: A4Identity = {
  storeName: "SARL Baraka Commerce",
  storeAddress: "Rue des Frères, Alger",
  storePhone: "0550 00 00 00",
  nif: "123456789012345",
  nis: "0987654321",
  rc: "16/00-1234567A25",
  ai: "1600123",
};

function detailsWithLines(count: number, docType: string, number: string): DocumentDetailsWire {
  return {
    document: {
      document_id: "doc-1",
      number,
      doc_type: docType,
      status: "FINALIZED",
      warehouse_id: "dep-01",
      subtotal_ht_centimes: "100000",
      tva_total_centimes: "19000",
      total_ttc_centimes: "119000",
      paid_amount_centimes: "119000",
      change_amount_centimes: "0",
      payment_status: "PAID",
    },
    items: Array.from({ length: count }, (_, i) => ({
      product_id: `prod-${i}`,
      unit_id: "unit-pcs",
      quantity_micro: "10000",
      unit_price_centimes: "1000",
      line_tva_centimes: "190",
      line_ttc_centimes: "1190",
    })),
    payments: [{ method_code: "CASH", amount_centimes: "119000" }],
    print_status: "NONE",
  };
}

describe("per-type A4 button decision (DSN-085)", () => {
  it("offers A4 for BL and FACTURE only", () => {
    expect(canPrintA4("BL")).toBe(true);
    expect(canPrintA4("FACTURE")).toBe(true);
    expect(canPrintA4("TICKET")).toBe(false);
    expect(canPrintA4("DEVIS")).toBe(false);
    expect(canPrintA4("AVOIR")).toBe(false);
  });

  it("gates the print button on the decision in the drawer", () => {
    expect(DRAWER_SRC).toContain("canPrintA4(document.document_type)");
    expect(DRAWER_SRC).toContain('t(locale, "documents.printA4")');
  });
});

describe("A4 sheet content (DSN-085)", () => {
  it("renders 25 lines with a repeating thead and verbatim totals", () => {
    const html = renderToStaticMarkup(
      DocumentA4Sheet({ locale: "fr-DZ", details: detailsWithLines(25, "FACTURE", "F-2026-0042"), identity: FULL_IDENTITY, createdAt: "2026-10-21T10:02:00" }) as never,
    );
    expect(html).toContain("F-2026-0042");
    expect(html).toContain("SARL Baraka Commerce");
    expect(html).toContain("123456789012345");
    expect((html.match(/data-testid="a4-line"/g) ?? []).length).toBe(25);
    expect(html).toContain("<thead");
    expect(html).toContain("1 190,00 DZD");
  });

  it("omits missing identity fields silently (never fabricates)", () => {
    const html = renderToStaticMarkup(
      DocumentA4Sheet({
        locale: "ar-DZ",
        details: detailsWithLines(1, "BL", "BL-2026-0003"),
        identity: { storeName: "", storeAddress: "", storePhone: "", nif: "", nis: "", rc: "", ai: "" }, createdAt: "2026-10-21T09:12:00",
      }) as never,
    );
    expect(html).toContain("BL-2026-0003");
    expect(html).not.toContain("NIF");
    expect(html).not.toContain("RC");
  });

  it("never renders totals-in-words or discounts (engine provides none)", () => {
    const html = renderToStaticMarkup(
      DocumentA4Sheet({ locale: "fr-DZ", details: detailsWithLines(2, "FACTURE", "F-1"), identity: FULL_IDENTITY, createdAt: "2026-10-21T10:02:00" }) as never,
    );
    expect(html).not.toMatch(/en toutes lettres|بالحروف/);
  });
});

describe("print stylesheet pins (DSN-085)", () => {
  it("declares A4 @page with 12mm margins and hides app chrome", () => {
    expect(CSS_SRC).toContain("@page");
    expect(CSS_SRC).toContain("A4");
    expect(CSS_SRC).toContain("12mm");
    expect(CSS_SRC).toContain("thead");
    expect(CSS_SRC).toContain("table-header-group");
    expect(CSS_SRC).toContain(".a4-sheet");
  });
});

describe("A4 i18n keys (DSN-085)", () => {
  it("declares the print keys in FR and AR", () => {
    for (const entry of [
      '"documents.printA4": "Imprimer A4"',
      '"documents.printFooter": "Document généré électroniquement"',
      '"documents.printA4": "طباعة A4"',
      '"documents.printFooter": "وثيقة مولّدة إلكترونياً"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});
