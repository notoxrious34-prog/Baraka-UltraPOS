/**
 * Graphique 80mm ticket (TASK-DSN-093): OS-graphics ticket path for label-class
 * printers that silently discard ESC/POS bytes (ADR-104, XP-410B verdict).
 *
 * Template data comes ONLY from the details wire (BRK-088 names) + document
 * summary — totals are rendered verbatim, never recomputed here. Dispatch
 * reuses the proven label pipeline: hidden iframe → window.print → OS spooler
 * (ProductLabelModal, DSN-083).
 */

import { formatQty, isIntString } from "@/lib/money";
import { generateCode128Svg } from "@/lib/barcode-128";

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
  /** Stored per-unit discount centimes (`null` = legacy line, no badge). */
  discountPerUnit: string | null;
}

/** Localised chrome strings (caller passes t() values for its locale). */
export interface TicketPrintStrings {
  title: string;
  /** Cashier row label (`Caissier` / `الكاشير`, engine parity). */
  cashier: string;
  reprintMark: string;
  designation: string;
  qty: string;
  price: string;
  total: string;
  subtotal: string;
  /** Remise totale row (DOC-05): the engine always prints it — omitting
   * it was the OS/TSPL divergence. */
  discountTotal: string;
  tva: string;
  grandTotal: string;
  paid: string;
  change: string;
  payment: string;
  footer: string;
  /** Depot meta label (`Dépôt` / `المستودع`). */
  depot: string;
  /** Per-line badge word (`remise` / `خصم`), amount follows. */
  discountBadge: string;
}

export interface TicketPrintArgs {
  storeName: string;
  storeAddress: string;
  storePhone: string;
  /** Tax-id rows (`{ prefix: "NIF", value: "…" }`); blank values collapse. */
  taxRows: Array<{ prefix: string; value: string }>;
  documentNumber: string;
  createdAt: string;
  /** Selling warehouse identifier (stored verbatim); blank collapses. */
  warehouseText: string;
  /** Acting operator identifier (stored verbatim — engine cashier row). */
  cashierText: string;
  language: "AR" | "FR";
  lines: TicketPrintLine[];
  subtotalHt: string;
  /** Stored global-discount figure (engine `Remise totale` row). */
  discountTotal: string;
  tvaTotal: string;
  totalTtc: string;
  paidAmount: string;
  changeAmount: string;
  paymentLabel: string;
  isReprint: boolean;
  /** Owner footer message (stored verbatim); blank collapses. */
  ownerFooter: string;
  strings: TicketPrintStrings;
}

function escapeTicket(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** Display a centime-string figure in ENGINE spelling (DOC-05): plain
 * ASCII `350.00`, no thousands grouping, dot separator — byte-identical
 * to `format_centimes_plain`, the only spelling the printer emits. The
 * locale comma (`350,00`) was the OS/TSPL divergence this unifies. */
export function ticketMoney(centimes: string): string {
  try {
    const trimmed = centimes.trim();
    if (!isIntString(trimmed)) {
      return "…";
    }
    const value = BigInt(trimmed);
    const negative = value < 0n;
    const abs = negative ? -value : value;
    const whole = (abs / 100n).toString();
    const frac = (abs % 100n).toString().padStart(2, "0");
    return `${negative ? "-" : ""}${whole}.${frac}`;
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
  /** Stored per-unit discount (`null` = legacy line). */
  discountPerUnitCentimes: string | null;
  /** Exact edge-transcribed net PU (`null` = legacy: show raw PU). */
  discountedUnitPriceCentimes: string | null;
  lineTtcCentimes: string;
}

export interface TicketWireDetails {
  documentNumber: string;
  createdAt: string;
  subtotalHt: string;
  /** Stored global-discount figure (engine `Remise totale` row). */
  discountTotal: string;
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
  /** Tax-id rows; blank values collapse in the template. */
  taxRows: Array<{ prefix: string; value: string }>;
  /** Selling warehouse identifier (stored verbatim). */
  warehouseText: string;
  /** Acting operator identifier (stored verbatim — engine cashier row). */
  cashierText: string;
  /** Owner footer message (stored verbatim). */
  ownerFooter: string;
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
      // Discounted PU when the wire carries it, raw PU for legacy lines —
      // both verbatim, zero arithmetic in the template layer.
      unitPrice: item.discountedUnitPriceCentimes ?? item.unitPriceCentimes,
      lineTotal: item.lineTtcCentimes,
      // Stored per-unit discount rides through for the badge (`null` =
      // legacy line, no badge) — never summed, never recomputed.
      discountPerUnit: item.discountPerUnitCentimes,
    })),
    subtotalHt: wire.subtotalHt,
    discountTotal: wire.discountTotal,
    tvaTotal: wire.tvaTotal,
    totalTtc: wire.totalTtc,
    paidAmount: wire.paidAmount,
    changeAmount: wire.changeAmount,
    paymentLabel: wire.paymentLabel,
    taxRows: opts.taxRows,
    warehouseText: opts.warehouseText,
    cashierText: opts.cashierText,
    ownerFooter: opts.ownerFooter,
    isReprint: opts.isReprint,
    strings: opts.strings,
  };
}

/** Build the standalone 80mm ticket document (BRK-098 mirror order). */
export function buildTicketPrintHtml(args: TicketPrintArgs): string {
  const rtl = args.language === "AR";
  const badgeWord = escapeTicket(args.strings.discountBadge);
  const rows = args.lines
    .map(
      (line) =>
        `<tr data-testid="ticket-line"><td class="des"><bdi dir="ltr">${escapeTicket(line.sku)}</bdi><br>${escapeTicket(line.designation)}` +
        (line.discountPerUnit === null
          ? ""
          : `<div class="badge" data-testid="ticket-badge">${badgeWord} <bdi dir="ltr">${escapeTicket(ticketMoney(line.discountPerUnit))}</bdi></div>`) +
        `</td>` +
        `<td class="num"><bdi dir="ltr">${escapeTicket(ticketQty(line.quantity))} ${escapeTicket(line.unit)}</bdi></td>` +
        `<td class="num"><bdi dir="ltr">${escapeTicket(ticketMoney(line.unitPrice))}</bdi></td>` +
        `<td class="num"><bdi dir="ltr">${escapeTicket(ticketMoney(line.lineTotal))}</bdi></td></tr>`,
    )
    .join("");
  const mark = args.isReprint
    ? `<div class="reprint">${escapeTicket(args.strings.reprintMark)}</div>`
    : "";
  // Blank identity fields collapse (mirror of the TSPL blank-collapse rule).
  const contactParts = [args.storeAddress.trim(), args.storePhone.trim()].filter(
    (part) => part !== "",
  );
  const contact =
    contactParts.length > 0
      ? `<div class="meta">${contactParts
          .map((part) => `<bdi dir="ltr">${escapeTicket(part)}</bdi>`)
          .join(" · ")}</div>`
      : "";
  const taxParts = args.taxRows
    .filter((row) => row.value.trim() !== "")
    .map((row) => `${escapeTicket(row.prefix)}: <bdi dir="ltr">${escapeTicket(row.value.trim())}</bdi>`);
  const tax = taxParts.length > 0 ? `<div class="meta tax">${taxParts.join(" · ")}</div>` : "";
  const depot =
    args.warehouseText.trim() !== ""
      ? `<div class="meta depot" data-testid="ticket-depot">${escapeTicket(args.strings.depot)}: <bdi dir="ltr">${escapeTicket(args.warehouseText.trim())}</bdi></div>`
      : "";
  // Cashier row (DOC-05 engine parity): the operator id rides verbatim,
  // never a display name — same collapse rule as the engine (blank hides).
  const cashier =
    args.cashierText.trim() !== ""
      ? `<div class="meta" data-testid="ticket-cashier">${escapeTicket(args.strings.cashier)}: <bdi dir="ltr">${escapeTicket(args.cashierText.trim())}</bdi></div>`
      : "";
  // Doc-number Code128 with human-readable caption (quiet zones via CSS
  // padding on white; never throws into the print dispatch).
  let barcode = "";
  try {
    barcode =
      `<div class="barcode" data-testid="ticket-barcode">` +
      generateCode128Svg(args.documentNumber, { moduleWidth: 2, height: 48, includeText: true }) +
      `</div>`;
  } catch {
    barcode = "";
  }
  const ownerFooter =
    args.ownerFooter.trim() !== ""
      ? `<div class="foot owner">${escapeTicket(args.ownerFooter.trim())}</div>`
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
    `.ticket .badge { font-size: 7pt; font-weight: bold; margin-top: 0.5mm; }` +
    `.ticket .barcode { background: #fff; padding: 3mm 6mm; margin: 2mm 0; }` +
    `.ticket .barcode svg { display: block; margin: 0 auto; max-width: 100%; height: auto; }` +
    `.ticket .foot { text-align: center; font-size: 8pt; margin-top: 2mm; }` +
    `.ticket .foot.owner { font-weight: bold; }`;
  return (
    `<!DOCTYPE html><html dir="${rtl ? "rtl" : "ltr"}" lang="${rtl ? "ar" : "fr"}"><head><meta charset="utf-8">` +
    `<title>ticket</title><style>${style}</style></head><body>` +
    `<div class="ticket"><div class="shop">${escapeTicket(args.storeName)}</div>` +
    contact +
    tax +
    `<div class="meta"><bdi dir="ltr">${escapeTicket(args.documentNumber)}</bdi> · <bdi dir="ltr">${escapeTicket(args.createdAt)}</bdi></div>` +
    cashier +
    depot +
    `<div class="meta">${escapeTicket(args.strings.title)}</div>${mark}` +
    `<table><thead><tr><th>${escapeTicket(args.strings.designation)}</th>` +
    `<th class="num">${escapeTicket(args.strings.qty)}</th>` +
    `<th class="num">${escapeTicket(args.strings.price)}</th>` +
    `<th class="num">${escapeTicket(args.strings.total)}</th></tr></thead><tbody>${rows}</tbody></table>` +
    `<div class="totals">` +
    `<div class="row"><span>${escapeTicket(args.strings.subtotal)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.subtotalHt))}</bdi></span></div>` +
    `<div class="row"><span>${escapeTicket(args.strings.discountTotal)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.discountTotal))}</bdi></span></div>` +
    `<div class="row"><span>${escapeTicket(args.strings.tva)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.tvaTotal))}</bdi></span></div>` +
    `<div class="row grand"><span>${escapeTicket(args.strings.grandTotal)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.totalTtc))}</bdi></span></div>` +
    `<div class="row"><span>${escapeTicket(args.strings.payment)} (${escapeTicket(args.paymentLabel)})</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.paidAmount))}</bdi></span></div>` +
    `<div class="row"><span>${escapeTicket(args.strings.change)}</span><span><bdi dir="ltr">${escapeTicket(ticketMoney(args.changeAmount))}</bdi></span></div>` +
    `</div>${barcode}${ownerFooter}<div class="foot">${escapeTicket(args.strings.footer)}</div></div></body></html>`
  );
}

/** Dispatch the ticket through an isolated hidden iframe (DSN-083 pipeline).
 *
 * TASK-PRT-04: the dispatch frame must never disturb the app layout —
 * AGENTS.md bans `position: fixed` outside overlays, so the frame rides
 * off-screen (`ticketPrintFrameStyle`) instead of a fixed zero box. The
 * inner window alone is printed, so parking the element off-screen cannot
 * affect the ticket. Spooler transports must never reach this dispatcher
 * (CheckoutModal gates on `resolveTicketPrintPath` first).
 */
export function ticketPrintFrameStyle(): Record<string, string> {
  return {
    position: "absolute",
    left: "-9999px",
    top: "0",
    width: "0",
    height: "0",
    border: "0",
  };
}

export function printTicketDocument(docHtml: string): void {
  if (typeof document === "undefined") {
    return;
  }
  const frame = document.createElement("iframe");
  frame.setAttribute("aria-hidden", "true");
  const style = ticketPrintFrameStyle();
  for (const [key, value] of Object.entries(style)) {
    frame.style.setProperty(key, value);
  }
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
