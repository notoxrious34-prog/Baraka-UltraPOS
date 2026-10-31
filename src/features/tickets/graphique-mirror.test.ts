/**
 * Graphique mirror pins (TASK-DSN-099): the OS-graphics ticket mirrors the
 * BRK-098 TSPL anatomy block-for-block from the same stored wire, and the
 * label preview unifies on the 9-char engine budget.
 *
 * FAIL-before: the frozen dispatch refs (hidden iframe, 80mm page,
 * DUPLICATA) survive the mirror untouched.
 * Block notes (HANDOFF): the sales_get DTO carries neither `created_by`
 * (cashier row) nor `global_discount_centimes` (Remise totale row), so the
 * mirror omits both — no fabrication. Depot rides `warehouse_id` verbatim.
 */

import { describe, expect, it } from "vitest";
import {
  buildTicketPrintHtml,
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
    warehouseText: "11111111-2222-3333-4444-555555555555",
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
      subtotal: "Sous-total HT",
      tva: "TVA",
      grandTotal: "TOTAL TTC",
      paid: "Encaissé",
      change: "Rendu",
      payment: "Paiement",
      footer: "Merci de votre visite",
      depot: "Dépôt",
      discountBadge: "remise",
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
      'data-testid="ticket-depot"',
      "Désignation",
      "VIS-001",
      'data-testid="ticket-badge"',
      "Sous-total HT",
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

describe("legacy-NULL lines keep the old format (raw PU, no badge)", () => {
  it("falls back to the raw PU verbatim and renders no badge", () => {
    const wire: TicketWireDetails = {
      documentNumber: "T-9",
      createdAt: "2026-10-26",
      subtotalHt: "500",
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
      ownerFooter: "",
      isReprint: false,
      strings: mirrorArgs().strings,
    });
    expect(args.lines[0]?.unitPrice).toBe("500");
    expect(args.lines[0]?.discountPerUnit).toBeNull();
    const html = buildTicketPrintHtml({ ...mirrorArgs(), lines: args.lines });
    expect(html).toContain("5,00 DZD");
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
