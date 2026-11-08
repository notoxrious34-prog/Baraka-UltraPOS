/**
 * Graphique mirror pins (TASK-DSN-099): the OS-graphics ticket mirrors the
 * BRK-098 TSPL anatomy block-for-block from the same stored wire, and the
 * label preview unifies on the 9-char engine budget.
 *
 * FAIL-before: the frozen dispatch refs (hidden iframe, 80mm page,
 * DUPLICATA) survive the mirror untouched.
 * DOC-05 closed both wire gaps: the sales_get DTO now carries
 * `created_by` (cashier row) and `global_discount_centimes` (Remise
 * totale row), so the mirror pins both — no fabrication. Depot rides
 * `warehouse_id` verbatim; the instant rides `created_at` verbatim.
 */

import { describe, expect, it } from "vitest";
import {
  buildTicketPrintHtml,
  buildTicketPrintHtmlV2,
  ticketArgsFromWire,
  type TicketPrintArgs,
  type TicketWireDetails,
} from "./graphique-ticket";
import { silentLabelPreviewSvg } from "../catalog/SilentLabelModal";

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

function mirrorArgs(overrides: Partial<TicketPrintArgs> = {}): TicketPrintArgs {
  return {
    storeName: "Quincaillerie El Baraka",
    storeAddress: "Rue des Frères, Alger",
    storePhone: "0555123456",
    taxRows: [
      { prefix: "NIF", value: "" },
      { prefix: "NIS", value: "" },
      { prefix: "RC", value: "RC-16-12345" },
      { prefix: "AI", value: "" },
    ],
    documentNumber: "T-2026-000123",
    createdAt: "2026-10-24 14:03",
    templateVersion: 1,
    warehouseText: "11111111-2222-3333-4444-555555555555",
    cashierText: "user-1",
    language: "FR",
    lines: [
      {
        designation: "Vis 8x40",
        sku: "VIS-001",
        quantity: "30000",
        unit: "Pièce",
        unitPrice: "900",
        lineTotal: "2700",
        discountPerUnit: "100",
      },
      {
        designation: "Clou 5cm",
        sku: "—",
        quantity: "10000",
        unit: "",
        unitPrice: "500",
        lineTotal: "500",
        discountPerUnit: null,
      },
    ],
    subtotalHt: "3200",
    discountTotal: "0",
    tvaTotal: "0",
    totalTtc: "3200",
    paidAmount: "3200",
    changeAmount: "0",
    paymentLabel: "CASH",
    isReprint: false,
    ownerFooter: "Échange sous 7 jours",
    strings: {
      title: "Ticket de caisse",
      reprintMark: "DUPLICATA",
      designation: "Désignation",
      qty: "Qté",
      price: "P.U",
      total: "Total",
      subtotal: "Sous-total",
      cashier: "Caissier",
      discountTotal: "Remise totale",
      tva: "TVA",
      grandTotal: "TOTAL TTC",
      paid: "Encaissé",
      change: "Rendu",
      payment: "Payé",
      footer: "Merci de votre visite",
      depot: "Dépôt",
      discountBadge: "remise",
      article: "Article",
      qtyPrice: "Qté x P.U",
    },
    ...overrides,
  };
}

describe("FAIL-before (frozen dispatch refs survive the mirror)", () => {
  it("keeps the hidden-iframe dispatch, the 80mm page and the DUPLICATA mark", () => {
    const template = readSrc("src", "features", "tickets", "graphique-ticket.ts");
    expect(template).toContain("printTicketDocument");
    expect(template).toContain("document.createElement(\"iframe\")");
    expect(template).toContain("win.print()");
    expect(template).toContain("TICKET_PAGE_WIDTH_MM = 80");
    const html = buildTicketPrintHtml({ ...mirrorArgs(), isReprint: true });
    expect(html).toContain("DUPLICATA");
  });
});

describe("mirror anatomy order (TSPL block order == HTML block order)", () => {
  it("lays out store, tax, meta, depot, header, rows+badge, totals, barcode, footers", () => {
    const html = buildTicketPrintHtml(mirrorArgs());
    const order = [
      "Quincaillerie El Baraka",
      "RC: <bdi",
      "T-2026-000123",
      "Caissier:",
      'data-testid="ticket-depot"',
      "Désignation",
      "VIS-001",
      'data-testid="ticket-badge"',
      "Sous-total",
      "Remise totale",
      "TOTAL TTC",
      'data-testid="ticket-barcode"',
      "Échange sous 7 jours",
      "Merci de votre visite",
    ];
    const positions = order.map((needle) => {
      const at = html.indexOf(needle);
      expect(at, `missing ${needle}`).toBeGreaterThan(-1);
      return at;
    });
    const ordered = positions.every(
      (at, index) => index === 0 || (positions[index - 1] as number) < at,
    );
    expect(ordered, `anatomy order broken: ${JSON.stringify(positions)}`).toBe(true);
  });

  it("renders the depot row verbatim and the per-line badge for discounted lines only", () => {
    const html = buildTicketPrintHtml(mirrorArgs());
    expect(html).toContain("Dépôt:");
    expect(html).toContain("11111111-2222-3333-4444-555555555555");
    expect(html).toContain("remise <bdi");
    expect((html.match(/data-testid="ticket-badge"/g) ?? []).length).toBe(1);
  });

  it("renders the doc-number Code128 with quiet-zone CSS and human-readable caption", () => {
    const html = buildTicketPrintHtml(mirrorArgs());
    expect(html).toContain('data-testid="ticket-barcode"');
    expect(html).toContain("<svg");
    expect(html).toContain("T-2026-000123</text>");
    expect(html).toContain(".ticket .barcode { background: #fff; padding:");
  });

  it("collapses blank tax, depot and owner-footer rows without gaps", () => {
    const html = buildTicketPrintHtml(
      mirrorArgs({ taxRows: [{ prefix: "NIF", value: "  " }], warehouseText: "", ownerFooter: "" }),
    );
    expect(html).not.toContain("ticket-depot");
    expect(html).not.toContain("NIF:");
    expect(html).not.toContain("foot owner");
    // The thanks footer always survives.
    expect(html).toContain("Merci de votre visite");
  });
});

describe("DOC-05 engine parity proof (OS == TSPL, data level)", () => {
  // Shared fixture mirrors the Rust `owner_ticket` golden (27.00 line,
  // 1.00 per-unit discount, depot + cashier UUIDs verbatim). The engine
  // contract pinned here: Sous-total / Remise totale / TVA / TOTAL TTC /
  // Payé / Rendu rows in order, dot spelling, remise badge with the
  // per-unit figure, DUPLICATA band on reprints only. Layout differs by
  // medium (HTML table vs TSPL stacked lines) — figures, rows and order
  // must not.
  function engineMirrorArgs(
    overrides: Partial<TicketPrintArgs> = {},
  ): TicketPrintArgs {
    return mirrorArgs({
      storeName: "BARAKA QUINCAILLERIE",
      documentNumber: "T-2026-000123",
      createdAt: "2026-10-27T20:00:00.000Z",
      cashierText: "00000000-0000-0000-0000-0000000000ab",
      lines: [
        {
          designation: "Vis 8x40",
          sku: "VIS-001",
          quantity: "30000",
          unit: "Pièce",
          unitPrice: "900",
          lineTotal: "2700",
          discountPerUnit: "100",
        },
      ],
      subtotalHt: "2700",
      discountTotal: "0",
      tvaTotal: "0",
      totalTtc: "2700",
      paidAmount: "2700",
      changeAmount: "0",
      paymentLabel: "CASH",
      ...overrides,
    });
  }

  function rowOrder(html: string, rows: string[]): number[] {
    return rows.map((row) => {
      const at = html.indexOf(row);
      expect(at, `missing engine row: ${row}`).toBeGreaterThan(-1);
      return at;
    });
  }

  it("prints the engine totals hierarchy in engine order with engine spelling (FR original)", () => {
    const html = buildTicketPrintHtml(engineMirrorArgs());
    const positions = rowOrder(html, [
      "Sous-total",
      "Remise totale",
      "TVA",
      "TOTAL TTC",
      "Payé (",
      "Rendu",
    ]);
    expect(
      positions.every((at, i) => i === 0 || (positions[i - 1] as number) < at),
      "totals hierarchy out of engine order",
    ).toBe(true);
    for (const figure of ["27.00", "0.00"]) {
      expect(html).toContain(figure);
    }
    expect(html).not.toContain("27,00");
    expect(html).not.toContain("DUPLICATA");
  });

  it("renders the remise badge with the per-unit figure and the verbatim line figures", () => {
    const html = buildTicketPrintHtml(engineMirrorArgs());
    expect(html).toContain("remise <bdi");
    expect(html).toContain("1.00");
    expect(html).toContain("9.00");
    expect(html).not.toContain("10.00");
  });

  it("stamps DUPLICATA on reprints only, with depot and cashier verbatim", () => {
    const original = buildTicketPrintHtml(engineMirrorArgs());
    expect(original).not.toContain("DUPLICATA");
    const reprint = buildTicketPrintHtml(engineMirrorArgs({ isReprint: true }));
    expect(reprint).toContain("DUPLICATA");
    expect(reprint).toContain("00000000-0000-0000-0000-0000000000ab");
    expect(reprint).toContain("11111111-2222-3333-4444-555555555555");
    expect(reprint).toContain("T-2026-000123</text>");
  });

  it("mirrors the Arabic engine contract (labels + نسخة mark, ASCII figures)", () => {
    const html = buildTicketPrintHtml(
      engineMirrorArgs({
        language: "AR",
        isReprint: true,
        strings: {
          ...mirrorArgs().strings,
          subtotal: "المجموع الفرعي",
          cashier: "الكاشير",
          discountTotal: "الخصم الإجمالي",
          tva: "الضريبة",
          grandTotal: "الإجمالي",
          paid: "المدفوع",
          change: "الباقي",
          payment: "المدفوع",
          footer: "شكراً لزيارتكم",
          depot: "المستودع",
          reprintMark: "نسخة",
        },
      }),
    );
    const positions = rowOrder(html, [
      "المجموع الفرعي",
      "الخصم الإجمالي",
      "الضريبة",
      ">الإجمالي</span>",
      "المدفوع",
      "الباقي",
    ]);
    expect(
      positions.every((at, i) => i === 0 || (positions[i - 1] as number) < at),
      "arabic totals hierarchy out of engine order",
    ).toBe(true);
    expect(html).toContain("نسخة");
    expect(html).toContain("27.00");
    expect(html).not.toContain("DUPLICATA");
  });
});

describe("legacy-NULL lines keep the old format (raw PU, no badge)", () => {
  it("falls back to the raw PU verbatim and renders no badge", () => {
    const wire: TicketWireDetails = {
      documentNumber: "T-9",
      createdAt: "2026-10-26",
      templateVersion: 1,
      subtotalHt: "500",
      discountTotal: "0",
      tvaTotal: "0",
      totalTtc: "500",
      paidAmount: "500",
      changeAmount: "0",
      paymentLabel: "CASH",
      items: [
        {
          nameFr: "Clou 5cm",
          nameAr: null,
          sku: null,
          unitFr: null,
          unitAr: null,
          quantityMicro: "10000",
          unitPriceCentimes: "500",
          discountPerUnitCentimes: null,
          discountedUnitPriceCentimes: null,
          lineTtcCentimes: "500",
        },
      ],
    };
    const args = ticketArgsFromWire(wire, {
      language: "FR",
      storeName: "S",
      storeAddress: "",
      storePhone: "",
      taxRows: [],
      warehouseText: "",
      cashierText: "user-1",
      ownerFooter: "",
      isReprint: false,
      strings: mirrorArgs().strings,
    });
    expect(args.lines[0]?.unitPrice).toBe("500");
    expect(args.lines[0]?.discountPerUnit).toBeNull();
    const html = buildTicketPrintHtml({ ...mirrorArgs(), lines: args.lines });
    expect(html).toContain("5.00");
    expect(html).not.toContain("ticket-badge");
  });
});

describe("label 9-char budget (preview == engine)", () => {
  it("previews 9-char payloads and refuses the 10th", () => {
    expect(silentLabelPreviewSvg("012345678")).not.toBeNull();
    expect(silentLabelPreviewSvg("0123456789")).toBeNull();
    expect(silentLabelPreviewSvg("VIS-001")).not.toBeNull();
  });

  it("quotes the modal guidance hook and the FR/AR keys", () => {
    const modal = readSrc("src", "features", "catalog", "SilentLabelModal.tsx");
    expect(modal).toContain("SILENT_LABEL_BARCODE_MAX_CHARS = 9");
    expect(modal).toContain("SILENT_LABEL_BARCODE_DOTS = 280");
    expect(modal).toContain('data-testid="silent-label-too-wide"');
    expect(modal).toContain("tspl.labelTooWide");
    const i18n = readSrc("src", "lib", "i18n.ts");
    expect(i18n).toContain('"tspl.labelTooWide": "Code-barres trop long');
    expect(i18n).toContain('"tspl.labelTooWide": "الرمز الشريطي طويل');
    expect(i18n).toContain('"ticket.depot": "Dépôt"');
    expect(i18n).toContain('"ticket.depot": "المستودع"');
    expect(i18n).toContain('"ticket.discountBadge": "remise"');
    expect(i18n).toContain('"ticket.discountBadge": "خصم"');
  });
});

describe("mirror RTL (AR)", () => {
  it("renders dir=rtl with LTR-isolated figures and AR chrome", () => {
    const html = buildTicketPrintHtml({
      ...mirrorArgs(),
      language: "AR",
      strings: {
        ...mirrorArgs().strings,
        depot: "المستودع",
        discountBadge: "خصم",
      },
    });
    expect(html).toContain('dir="rtl"');
    expect(html).toContain('dir="ltr"');
    expect(html).toContain("المستودع:");
    expect(html).toContain("خصم <bdi");
  });
});

describe("DOC-07 v2 mirror (owner redesign)", () => {
  const v2Args = (): TicketPrintArgs => ({
    ...mirrorArgs(),
    templateVersion: 2,
    createdAt: "07/11/2026 19:20",
    cashierText: "Ahmed",
    warehouseText: "Principal",
    lines: [
      {
        designation: "MONCH ISPANA GRIS0958",
        sku: "MONCH-001",
        quantity: "10000",
        unit: "PCS",
        unitPrice: "53000",
        lineTotal: "53000",
        discountPerUnit: "0",
      },
    ],
    subtotalHt: "131000",
    discountTotal: "0",
    tvaTotal: "0",
    totalTtc: "131000",
    paidAmount: "131000",
    changeAmount: "0",
    ownerFooter: "",
    isReprint: true,
  });

  it("renders the v2 anatomy: one-line doc/crew, three columns, zero rows collapsed", () => {
    const html = buildTicketPrintHtmlV2(v2Args());
    // Wall-clock instant + resolved names, never ids or ISO.
    expect(html).toContain("07/11/2026 19:20");
    expect(html).not.toContain("T19:20");
    expect(html).toContain("Caissier:");
    expect(html).toContain("Ahmed");
    expect(html).toContain("Principal");
    // Three-column header.
    expect(html).toContain("Article");
    expect(html).toContain("Qté x P.U");
    expect(html).toContain("Total");
    // Indented detail + line total.
    expect(html).toContain("1 PCS x 530.00");
    expect(html).toContain("530.00");
    // Zero badge / discount / TVA collapse; subtotal stays.
    expect(html).not.toContain("ticket-badge");
    expect(html).toContain("Sous-total");
    expect(html).not.toContain("Remise totale");
    expect(html).not.toContain(">TVA <");
    // Framed reprint band + black grand-total band.
    expect(html).toContain("*** DUPLICATA ***");
    expect(html).toContain("reprint-band");
    expect(html).toContain("TOTAL TTC");
    expect(html).toContain("1310.00");
  });

  it("keeps v1 frozen: zero rows still render on the v1 builder", () => {
    const html = buildTicketPrintHtml(mirrorArgs());
    expect(html).toContain("Remise totale");
    expect(html).toContain("TVA");
  });
});
