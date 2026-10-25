/**
 * Graphique 80mm ticket (TASK-DSN-093): OS-graphics ticket path for label-class
 * printers that silently discard ESC/POS bytes (ADR-104, XP-410B verdict).
 *
 * Template data comes ONLY from the details wire (BRK-088 names) + document
 * summary — totals are rendered verbatim, never recomputed here. Dispatch
 * reuses the proven label pipeline: hidden iframe → window.print → OS spooler
 * (ProductLabelModal, DSN-083).
 */

import { formatMoney, formatQty } from "@/lib/money";
import type { Locale } from "@/types/pos";

/** 80mm roll width (receipt printers; driver paper size must match). */
export const TICKET_PAGE_WIDTH_MM = 80;

/** One ticket line: all figures are pre-formatted wire strings. */
export interface TicketPrintLine {
  designation: string;
  sku: string;
  quantity: string;
  unit: string;
  unitPrice: string;
  lineTotal: string;
}

/** Localised chrome strings (caller passes t() values for its locale). */
export interface TicketPrintStrings {
  title: string;
  reprintMark: string;
  designation: string;
  qty: string;
  price: string;
  total: string;
  subtotal: string;
  tva: string;
  grandTotal: string;
  paid: string;
  change: string;
  payment: string;
  footer: string;
}

export interface TicketPrintArgs {
  storeName: string;
  storeAddress: string;
  storePhone: string;
  documentNumber: string;
  createdAt: string;
  language: "AR" | "FR";
  lines: TicketPrintLine[];
  subtotalHt: string;
  tvaTotal: string;
  totalTtc: string;
  paidAmount: string;
  changeAmount: string;
  paymentLabel: string;
  isReprint: boolean;
  strings: TicketPrintStrings;
}

function escapeTicket(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** Display a centime-string figure in the ticket locale. */
export function ticketMoney(centimes: string, locale: Locale): string {
  try {
    return formatMoney(centimes, locale);
  } catch {
    return "…";
  }
}

/** Display a micro-quantity wire string. */
export function ticketQty(micro: string): string {
  try {
    return formatQty(micro);
  } catch {
    return "…";
  }
}

/** Minimal ticket source (details wire + document summary, no new IPC). */
export interface TicketWireItem {
  nameFr: string | null;
  nameAr: string | null;
  sku: string | null;
  unitFr: string | null;
  unitAr: string | null;
  quantityMicro: string;
  unitPriceCentimes: string;
  lineTtcCentimes: string;
}

export interface TicketWireDetails {
  documentNumber: string;
  createdAt: string;
  subtotalHt: string;
  tvaTotal: string;
  totalTtc: string;
  paidAmount: string;
  changeAmount: string;
  paymentLabel: string;
  items: TicketWireItem[];
}

export interface TicketWireOptions {
  language: "AR" | "FR";
  storeName: string;
  storeAddress: string;
  storePhone: string;
  isReprint: boolean;
  strings: TicketPrintStrings;
}

/**
 * Map the details wire onto template args. Names follow the document
 * language with cross-locale fallback (never fabricated); figures travel
 * untouched for the template to render verbatim.
 */
export function ticketArgsFromWire(
  wire: TicketWireDetails,
  opts: TicketWireOptions,
): TicketPrintArgs {
  const fr = opts.language === "FR";
  return {
    storeName: opts.storeName,
    storeAddress: opts.storeAddress,
    storePhone: opts.storePhone,
    documentNumber: wire.documentNumber,
    createdAt: wire.createdAt,
    language: opts.language,
    lines: wire.items.map((item) => ({
      designation: (fr ? item.nameFr : item.nameAr) ?? (fr ? item.nameAr : item.nameFr) ?? "—",
      sku: item.sku ?? "—",
      quantity: item.quantityMicro,
      unit: (fr ? item.unitFr : item.unitAr) ?? (fr ? item.unitAr : item.unitFr) ?? "",
      unitPrice: item.unitPriceCentimes,
      lineTotal: item.lineTtcCentimes,
    })),
    subtotalHt: wire.subtotalHt,
    tvaTotal: wire.tvaTotal,
    totalTtc: wire.totalTtc,
    paidAmount: wire.paidAmount,
    changeAmount: wire.changeAmount,
    paymentLabel: wire.paymentLabel,
    isReprint: opts.isReprint,
    strings: opts.strings,
  };
}

/** Build the standalone 80mm ticket document. */
export function buildTicketPrintHtml(args: TicketPrintArgs): string {
  const rtl = args.language === "AR";
  const locale: Locale = rtl ? "ar-DZ" : "fr-DZ";
  const rows = args.lines
    .map(
      (line) =>
        `<tr data-testid="ticket-line"><td class="des"><bdi dir="ltr">${escapeTicket(line.sku)}</bdi><br>${escapeTicket(line.designation)}</td>` +
        `<td class="num"><bdi dir="ltr">${escapeTicket(ticketQty(line.quantity))} ${escapeTicket(line.unit)}</bdi></td>` +
        `<td class="num"><bdi dir="ltr">${escapeTicket(ticketMoney(line.unitPrice, locale))}</bdi></td>` +
        `<td class="num"><bdi dir="ltr">${escapeTicket(ticketMoney(line.lineTotal, locale))}</bdi></td></tr>`,
    )
    .join("");
  const mark = args.isReprint
    ? `<div class="reprint">${escapeTicket(args.strings.reprintMark)}</div>`
    : "";
  const style =
    `@page { size: ${TICKET_PAGE_WIDTH_MM}mm auto; margin: 0; }` +
    `html,body { margin: 0; padding: 0; }` +
    `.ticket { width: ${TICKET_PAGE_WIDTH_MM}mm; max-width: 100%; margin: 0 auto; padding: 2mm 3mm; ` +
    `font-family: Arial, sans-serif; color: #000; background: #fff; font-size: 9pt; }` +
    `.ticket .shop { text-align: center; font-weight: bold; font-size: 11pt; }` +
    `.ticket .meta { text-align: center; font-size: 8pt; margin: 1mm 0; }` +
    `.ticket .reprint { text-align: center; font-weight: bold; font-size: 12pt; ` +
    `border: 1px solid #000; margin: 2mm 0; padding: 1mm; }` +
    `.ticket table { width: 100%; border-collapse: collapse; margin: 2mm 0; }` +
    `.ticket th, .ticket td { padding: 1mm 0; vertical-align: top; text-align: start; }` +
    `.ticket th.num, .ticket td.num { text-align: end; font-family: "Courier New", monospace; }` +
    `.ticket .totals { margin-top: 2mm; border-top: 1px dashed #000; padding-top: 1mm; }` +
    `.ticket .totals .row { display: flex; justify-content: space-between; }` +
    `.ticket .totals .grand { font-weight: bold; font-size: 12pt; }` +
    `.ticket .foot { text-align: center; font-size: 8pt; margin-top: 2mm; }`;
  return (
    `<!DOCTYPE html><html dir="${rtl ? "rtl" : "ltr"}" lang="${rtl ? "ar" : "fr"}"><head><meta charset="utf-8">` +
    `<title>ticket</title><style>${style}</style></head><body>` +
    `<div class="ticket"><div class="shop">${escapeTicket(args.storeName)}</div>` +
    `<div class="meta">${escapeTicket(args.storeAddress)} · <bdi dir="ltr">${escapeTicket(args.storePhone)}</bdi><br>` +
    `<bdi dir="ltr">${escapeTicket(args.documentNumber)}</bdi> · <bdi dir="ltr">${escapeTicket(args.createdAt)}</bdi></div>` +
    `<div class="meta">${escapeTicket(args.strings.title)}</div>${mark}` +
    `<table><thead><tr><th>${escapeTicket(args.strings.designation)}</th>` +
    `<th class="num">${escapeTicket(args.strings.qty)}</th>` +
    `<th class="num">${escapeTicket(args.strings.price)}</th>` +
    `<th class="num">${escapeTicket(args.strings.total)}</th></tr></thead><tbody>${rows}</tbody></table>` +
    `<div class="totals">` +
    `<div class="row"><span>${escapeTicket(args.strings.subtotal)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.subtotalHt, locale))}</bdi></span></div>` +
    `<div class="row"><span>${escapeTicket(args.strings.tva)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.tvaTotal, locale))}</bdi></span></div>` +
    `<div class="row grand"><span>${escapeTicket(args.strings.grandTotal)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.totalTtc, locale))}</bdi></span></div>` +
    `<div class="row"><span>${escapeTicket(args.strings.payment)} (${escapeTicket(args.paymentLabel)})</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.paidAmount, locale))}</bdi></span></div>` +
    `<div class="row"><span>${escapeTicket(args.strings.change)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.changeAmount, locale))}</bdi></span></div>` +
    `</div><div class="foot">${escapeTicket(args.strings.footer)}</div></div></body></html>`
  );
}

/** Dispatch the ticket through an isolated hidden iframe (DSN-083 pipeline). */
export function printTicketDocument(docHtml: string): void {
  if (typeof document === "undefined") {
    return;
  }
  const frame = document.createElement("iframe");
  frame.setAttribute("aria-hidden", "true");
  frame.style.position = "fixed";
  frame.style.right = "0";
  frame.style.bottom = "0";
  frame.style.width = "0";
  frame.style.height = "0";
  frame.style.border = "0";
  document.body.appendChild(frame);
  const cleanup = (): void => {
    if (frame.parentNode !== null) {
      frame.parentNode.removeChild(frame);
    }
  };
  try {
    const doc = frame.contentDocument;
    const win = frame.contentWindow;
    if (doc === null || win === null) {
      cleanup();
      return;
    }
    doc.open();
    doc.write(docHtml);
    doc.close();
    win.focus();
    win.print();
  } finally {
    window.setTimeout(cleanup, 1000);
  }
}
