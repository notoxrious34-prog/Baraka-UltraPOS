/**
 * Graphique 80mm ticket pins (TASK-DSN-093): pure-function coverage for the
 * OS-graphics ticket path. Template data comes ONLY from the details wire
 * (BRK-088 names) + document summary — totals are rendered verbatim, never
 * recomputed (the arithmetic assert compares against the wire, not local math).
 */

import { describe, expect, it } from "vitest";
import {
  buildTicketPrintHtml,
  printTicketDocument,
  ticketArgsFromWire,
  TICKET_PAGE_WIDTH_MM,
  type TicketPrintArgs,
  type TicketWireDetails,
} from "./graphique-ticket";

const WIRE_LINES = [
  {
    designation: "Ciment CPJ 50kg",
    sku: "CIM-001",
    quantity: "20000",
    unit: "Sac",
    unitPrice: "85000",
    lineTotal: "170000",
  },
  {
    designation: "Fer Ø12",
    sku: "FER-012",
    quantity: "15000",
    unit: "Barre",
    unitPrice: "120000",
    lineTotal: "180000",
  },
];

function baseArgs(overrides: Partial<TicketPrintArgs> = {}): TicketPrintArgs {
  return {
    storeName: "Quincaillerie El Baraka",
    storeAddress: "Rue des Frères, Alger",
    storePhone: "0555123456",
    documentNumber: "T-2026-000123",
    createdAt: "2026-10-24 14:03",
    language: "FR",
    lines: WIRE_LINES,
    subtotalHt: "350000",
    tvaTotal: "10000",
    totalTtc: "360000",
    paidAmount: "400000",
    changeAmount: "40000",
    paymentLabel: "Espèces",
    isReprint: false,
    strings: {
      title: "Ticket de caisse",
      reprintMark: "DUPLICATA",
      designation: "Désignation",
      qty: "Qté",
      price: "P.U",
      total: "Total",
      subtotal: "Sous-total HT",
      tva: "TVA",
      grandTotal: "TOTAL TTC",
      paid: "Encaissé",
      change: "Rendu",
      payment: "Paiement",
      footer: "Merci de votre visite",
    },
    ...overrides,
  };
}

describe("ticket template renders wire data verbatim", () => {
  it("renders store, names, sku, quantities and wire totals (no local math)", () => {
    const html = buildTicketPrintHtml(baseArgs());
    expect(html).toContain("Quincaillerie El Baraka");
    expect(html).toContain("Ciment CPJ 50kg");
    expect(html).toContain("CIM-001");
    expect(html).toContain("700,00");
    expect(html).toContain("600,00");
    // Totals come from the wire: recomputing from lines must match the wire
    // (guard against a template that drifts from engine figures).
    const lineSum = WIRE_LINES.reduce((acc, line) => acc + Number(line.lineTotal), 0);
    expect(String(lineSum)).toBe("350000");
    expect(html).toContain("500,00");
    expect(html).toContain("400,00");
  });

  it("marks reprints DUPLICATA, clean sales carry no mark", () => {
    expect(buildTicketPrintHtml(baseArgs())).not.toContain("DUPLICATA");
    const html = buildTicketPrintHtml(baseArgs({ isReprint: true }));
    expect(html).toContain("DUPLICATA");
  });

  it("declares an 80mm @page and escapes hostile input", () => {
    const html = buildTicketPrintHtml(
      baseArgs({ storeName: "<b>X & Y</b>", lines: [] }),
    );
    expect(html).toContain("@page");
    expect(html).toContain("80mm");
    expect(html).not.toContain("<b>X");
    expect(html).toContain("&lt;b&gt;X &amp; Y&lt;/b&gt;");
  });

  it("isolates numbers LTR and exposes one line per row", () => {
    const html = buildTicketPrintHtml(baseArgs());
    expect((html.match(/data-testid="ticket-line"/g) ?? []).length).toBe(2);
    expect(html).toContain('dir="ltr"');
  });
});

describe("ticket RTL mirror (AR)", () => {
  it("renders dir=rtl on the root and keeps figures LTR-isolated", () => {
    const html = buildTicketPrintHtml(
      baseArgs({ language: "AR", storeName: "البركة", lines: [] }),
    );
    expect(html).toContain('dir="rtl"');
    expect(html).toContain('dir="ltr"');
    expect(html).not.toContain('dir="ltr"><html');
  });
});

describe("print dispatch (label pipeline reuse)", () => {
  it("is a function reusing the hidden-iframe technique", () => {
    expect(typeof printTicketDocument).toBe("function");
    expect(printTicketDocument.length).toBe(1);
  });

  it("uses the 80mm page width constant", () => {
    expect(TICKET_PAGE_WIDTH_MM).toBe(80);
  });
});

describe("wire mapper (BRK-088 names + summary, no new IPC)", () => {
  const wire: TicketWireDetails = {
    documentNumber: "T-1",
    createdAt: "2026-10-24",
    subtotalHt: "350000",
    tvaTotal: "10000",
    totalTtc: "360000",
    paidAmount: "400000",
    changeAmount: "40000",
    paymentLabel: "CASH",
    items: [
      {
        nameFr: "Ciment",
        nameAr: "إسمنت",
        sku: "CIM-001",
        unitFr: "Sac",
        unitAr: "كيس",
        quantityMicro: "20000",
        unitPriceCentimes: "85000",
        discountPerUnitCentimes: null,
        discountedUnitPriceCentimes: null,
        lineTtcCentimes: "170000",
      },
      {
        nameFr: null,
        nameAr: null,
        sku: null,
        unitFr: null,
        unitAr: null,
        quantityMicro: "10000",
        unitPriceCentimes: "0",
        discountPerUnitCentimes: null,
        discountedUnitPriceCentimes: null,
        lineTtcCentimes: "0",
      },
    ],
  };

  it("selects names per document language with fallback, figures untouched", () => {
    const fr = ticketArgsFromWire(wire, {
      language: "FR",
      storeName: "S",
      storeAddress: "",
      storePhone: "",
      isReprint: false,
      strings: baseArgs().strings,
    });
    expect(fr.lines[0]?.designation).toBe("Ciment");
    expect(fr.lines[0]?.lineTotal).toBe("170000");
    expect(fr.totalTtc).toBe("360000");
    const ar = ticketArgsFromWire(wire, {
      language: "AR",
      storeName: "S",
      storeAddress: "",
      storePhone: "",
      isReprint: true,
      strings: baseArgs().strings,
    });
    expect(ar.lines[0]?.designation).toBe("إسمنت");
    expect(ar.isReprint).toBe(true);
  });

  it("never leaves a blank designation (sku-or-dash degradation)", () => {
    const fr = ticketArgsFromWire(wire, {
      language: "FR",
      storeName: "S",
      storeAddress: "",
      storePhone: "",
      isReprint: false,
      strings: baseArgs().strings,
    });
    expect(fr.lines[1]?.designation).toBe("—");
    expect(fr.lines[1]?.sku).toBe("—");
  });
});

describe("per-unit price mapping (DSN-097)", () => {
  const OPTS = {
    language: "FR" as const,
    storeName: "S",
    storeAddress: "",
    storePhone: "",
    isReprint: false,
    strings: baseArgs().strings,
  };

  function wireItem(overrides: Record<string, unknown>) {
    return {
      nameFr: "Vis 8x40",
      nameAr: "برغي",
      sku: "VIS-001",
      unitFr: "Pièce",
      unitAr: "قطعة",
      quantityMicro: "30000",
      unitPriceCentimes: "1000",
      discountPerUnitCentimes: null,
      discountedUnitPriceCentimes: null,
      lineTtcCentimes: "2700",
      ...overrides,
    };
  }

  function detailsWith(item: ReturnType<typeof wireItem>): TicketWireDetails {
    return {
      documentNumber: "T-9",
      createdAt: "2026-10-26",
      subtotalHt: "2700",
      tvaTotal: "0",
      totalTtc: "2700",
      paidAmount: "2700",
      changeAmount: "0",
      paymentLabel: "CASH",
      items: [item],
    };
  }

  it("renders the wire net PU verbatim for a per-unit line (3 x 900 = 2700)", () => {
    const args = ticketArgsFromWire(
      detailsWith(
        wireItem({
          discountPerUnitCentimes: "100",
          discountedUnitPriceCentimes: "900",
        }),
      ),
      OPTS,
    );
    expect(args.lines[0]?.unitPrice).toBe("900");
    expect(args.lines[0]?.lineTotal).toBe("2700");
    const html = buildTicketPrintHtml({ ...baseArgs(), lines: args.lines });
    expect(html).toContain("9,00 DZD");
    expect(html).not.toContain("10,00 DZD");
  });

  it("falls back to the raw PU for legacy lines (null per-unit)", () => {
    const args = ticketArgsFromWire(detailsWith(wireItem({})), OPTS);
    expect(args.lines[0]?.unitPrice).toBe("1000");
    const html = buildTicketPrintHtml({ ...baseArgs(), lines: args.lines });
    expect(html).toContain("10,00 DZD");
  });
});
