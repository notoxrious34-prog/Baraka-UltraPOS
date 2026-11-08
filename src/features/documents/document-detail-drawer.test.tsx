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
    global_discount_centimes: "0",
    created_at: "2026-10-24T14:03:00.000Z",
    created_by: "user-1",
    template_version: 1,
    created_by_name: "Ahmed",
    warehouse_name: "Principal",
    created_at_display: "26/10/2026 10:00",
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
      discount_per_unit_centimes: null,
      discounted_unit_price_centimes: null,
      line_tva_centimes: "483",
      line_ttc_centimes: "3025",
      product_name_fr: "Vis TR 4x40",
      product_name_ar: "برغي 4×40",
      sku: "VIS-440",
      unit_name_fr: "Pièce",
      unit_name_ar: "قطعة",
      product_missing: false,
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
    global_discount_centimes: "0",
    created_at: "2026-10-24T14:03:00.000Z",
    created_by: "user-1",
    template_version: 1,
    created_by_name: "Ahmed",
    warehouse_name: "Principal",
    created_at_display: "26/10/2026 10:00",
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
      discount_per_unit_centimes: null,
      discounted_unit_price_centimes: null,
      line_tva_centimes: "19000",
      line_ttc_centimes: "119000",
      product_name_fr: "Ciment CPJ 50kg",
      product_name_ar: "إسمنت 50كغ",
      sku: "CIM-50",
      unit_name_fr: "Sac",
      unit_name_ar: "كيس",
      product_missing: false,
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
    expect(html).toContain("Vis TR 4x40");
    expect(html).toContain("CASH");
    expect(html).toContain("30,25 DZD");
    expect(html).toContain("Encaissé");
  });

  it("renders FACTURE lines, payments, and totals verbatim", () => {
    const html = renderToStaticMarkup(
      DocumentDetailsView({ locale: "ar-DZ", details: FACTURE }) as never,
    );
    expect(html).toContain("إسمنت 50كغ");
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

const UUID_LINE_1 = "3f4a1c2e-9b5d-4a1f-8c3e-7d2e6a5b4c1d";
const UUID_LINE_2 = "7e2b8f4a-1c3d-4e5f-9a2b-3d4c5e6f7a8b";

function designationWire(): DocumentDetailsWire {
  return {
    document: TICKET.document,
    items: [
      {
        product_id: UUID_LINE_1,
        unit_id: "a1b2c3d4-e5f6-4a7b-8c9d-e0f1a2b3c4d5",
        quantity_micro: "20000",
        unit_price_centimes: "1271",
        discount_per_unit_centimes: null,
        discounted_unit_price_centimes: null,
        line_tva_centimes: "483",
        line_ttc_centimes: "3025",
        product_name_fr: "Vis TR 4x40",
        product_name_ar: "برغي 4×40",
        sku: "VIS-440",
        unit_name_fr: "Pièce",
        unit_name_ar: "قطعة",
        product_missing: false,
      },
      {
        product_id: UUID_LINE_2,
        unit_id: "b2c3d4e5-f6a7-4b8c-9d0e-f1a2b3c4d5e6",
        quantity_micro: "10000",
        unit_price_centimes: "500",
        discount_per_unit_centimes: null,
        discounted_unit_price_centimes: null,
        line_tva_centimes: "95",
        line_ttc_centimes: "595",
        product_name_fr: null,
        product_name_ar: "شاي أخضر 100غ",
        sku: "THE-100",
        unit_name_fr: null,
        unit_name_ar: "علبة معدنية",
        product_missing: false,
      },
    ],
    payments: [],
    print_status: "NONE",
  };
}

describe("drawer product designation (DSN-089)", () => {
  it("shows the FR designation with zero UUID text in fr-DZ", () => {
    const html = renderToStaticMarkup(
      DocumentDetailsView({ locale: "fr-DZ", details: designationWire() }) as never,
    );
    expect(html).toContain("Vis TR 4x40");
    expect(html).toContain("VIS-440");
    expect(html).toContain("Pièce");
    expect(html).not.toMatch(/[0-9a-f]{8}-[0-9a-f]{4}-/);
  });

  it("shows the AR designation RTL-isolated in ar-DZ", () => {
    const html = renderToStaticMarkup(
      DocumentDetailsView({ locale: "ar-DZ", details: designationWire() }) as never,
    );
    expect(html).toContain("برغي 4×40");
    expect(html).toContain("قطعة");
    expect(html).not.toMatch(/[0-9a-f]{8}-[0-9a-f]{4}-/);
  });

  it("falls back across locales, then to SKU, never to UUID", () => {
    const fr = renderToStaticMarkup(
      DocumentDetailsView({ locale: "fr-DZ", details: designationWire() }) as never,
    );
    // Line 2 has no FR name: the AR designation shows instead of the UUID.
    expect(fr).toContain("شاي أخضر 100غ");
    expect(fr).not.toContain(UUID_LINE_2);
  });

  it("degrades orphans to SKU plus an amber information badge", () => {
    const orphan: DocumentDetailsWire = {
      ...designationWire(),
      items: [
        {
          product_id: UUID_LINE_1,
          unit_id: "a1b2c3d4-e5f6-4a7b-8c9d-e0f1a2b3c4d5",
          quantity_micro: "20000",
          unit_price_centimes: "1271",
          discount_per_unit_centimes: null,
          discounted_unit_price_centimes: null,
          line_tva_centimes: "483",
          line_ttc_centimes: "3025",
          product_name_fr: null,
          product_name_ar: null,
          sku: "SUCRE-1KG",
          unit_name_fr: null,
          unit_name_ar: null,
          product_missing: true,
        },
      ],
    };
    const fr = renderToStaticMarkup(
      DocumentDetailsView({ locale: "fr-DZ", details: orphan }) as never,
    );
    expect(fr).toContain("SUCRE-1KG");
    expect(fr).toContain("Produit archivé");
    expect(fr).not.toContain(UUID_LINE_1);
    const ar = renderToStaticMarkup(
      DocumentDetailsView({ locale: "ar-DZ", details: orphan }) as never,
    );
    expect(ar).toContain("منتج مؤرشف");
    expect(ar).not.toMatch(/[0-9a-f]{8}-[0-9a-f]{4}-/);
  });

  it("declares the orphan badge key in FR and AR", () => {
    for (const entry of [
      '"documents.detailsOrphanBadge": "Produit archivé"',
      '"documents.detailsOrphanBadge": "منتج مؤرشف"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});
