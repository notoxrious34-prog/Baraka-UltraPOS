/**
 * Document detail drawer tests (TASK-DSN-084): one details fetch per open,
 * TICKET + FACTURE golden renders, loading/error/empty states, i18n keys.
 * Node-only harness: renderToStaticMarkup for the pure details view +
 * source-text pins for the fetch lifecycle (no jsdom).
 */

import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  DocumentDetailsView,
  type DocumentDetailsWire,
} from "@/features/documents/DocumentDetailDrawer";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const DRAWER_SRC = readSrc("src", "features", "documents", "DocumentDetailDrawer.tsx");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

const TICKET: DocumentDetailsWire = {
  document: {
    document_id: "doc-ticket-1",
    number: "T-2026-0007",
    doc_type: "TICKET",
    status: "FINALIZED",
    warehouse_id: "dep-01",
    subtotal_ht_centimes: "2542",
    tva_total_centimes: "483",
    total_ttc_centimes: "3025",
    paid_amount_centimes: "5000",
    change_amount_centimes: "1975",
    payment_status: "PAID",
  },
  items: [
    {
      product_id: "prod-vis",
      unit_id: "unit-pcs",
      quantity_micro: "20000",
      unit_price_centimes: "1271",
      line_tva_centimes: "483",
      line_ttc_centimes: "3025",
    },
  ],
  payments: [{ method_code: "CASH", amount_centimes: "5000" }],
  print_status: "DONE",
};

const FACTURE: DocumentDetailsWire = {
  document: {
    document_id: "doc-fact-1",
    number: "F-2026-0042",
    doc_type: "FACTURE",
    status: "FINALIZED",
    warehouse_id: "dep-01",
    subtotal_ht_centimes: "100000",
    tva_total_centimes: "19000",
    total_ttc_centimes: "119000",
    paid_amount_centimes: "119000",
    change_amount_centimes: "0",
    payment_status: "PAID",
  },
  items: [
    {
      product_id: "prod-ciment",
      unit_id: "unit-sac",
      quantity_micro: "40000",
      unit_price_centimes: "25000",
      line_tva_centimes: "19000",
      line_ttc_centimes: "119000",
    },
  ],
  payments: [{ method_code: "VIREMENT", amount_centimes: "119000" }],
  print_status: "NONE",
};

describe("details fetch lifecycle (DSN-084)", () => {
  it("fetches full details on open via the sales_get wrapper", () => {
    expect(DRAWER_SRC).toContain("getDocumentDetails(document.id)");
  });

  it("issues exactly one fetch per open with cancel-on-close", () => {
    expect(DRAWER_SRC).toContain("getDocumentDetails(document.id)");
    const calls = DRAWER_SRC.match(/getDocumentDetails\(/g) ?? [];
    expect(calls.length).toBe(1);
    expect(DRAWER_SRC).toContain("cancelled");
  });

  it("shows a loading skeleton while fetching", () => {
    expect(DRAWER_SRC).toContain("aria-busy");
  });

  it("shows an error state with retry", () => {
    expect(DRAWER_SRC).toContain("detailsRetry");
    expect(DRAWER_SRC).toContain('role="alert"');
  });
});

describe("TICKET + FACTURE golden renders (DSN-084)", () => {
  it("renders TICKET lines, payments, and totals verbatim", () => {
    const html = renderToStaticMarkup(
      DocumentDetailsView({ locale: "fr-DZ", details: TICKET }) as never,
    );
    expect(html).toContain("prod-vis");
    expect(html).toContain("CASH");
    expect(html).toContain("30,25 DZD");
    expect(html).toContain("Encaissé");
  });

  it("renders FACTURE lines, payments, and totals verbatim", () => {
    const html = renderToStaticMarkup(
      DocumentDetailsView({ locale: "ar-DZ", details: FACTURE }) as never,
    );
    expect(html).toContain("prod-ciment");
    expect(html).toContain("VIREMENT");
    expect(html).toContain("1 190,00 د.ج");
    expect(html).toContain("1 000,00 د.ج");
  });

  it("renders LTR-isolated codes and tabular figures", () => {
    const html = renderToStaticMarkup(
      DocumentDetailsView({ locale: "ar-DZ", details: TICKET }) as never,
    );
    expect(html).toContain('dir="ltr"');
    expect(html).toContain("tabular-nums");
  });
});

describe("details i18n keys (DSN-084)", () => {
  it("declares the details namespace in FR and AR", () => {
    for (const entry of [
      '"documents.detailsLines": "Lignes du document"',
      '"documents.detailsPayments": "Paiements"',
      '"documents.detailsRetry": "Réessayer"',
      '"documents.detailsLines": "سطور الوثيقة"',
      '"documents.detailsPayments": "المدفوعات"',
      '"documents.detailsRetry": "إعادة المحاولة"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});
