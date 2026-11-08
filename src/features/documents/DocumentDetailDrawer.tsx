/**
 * Document detail side pane (TASK-DSN-028): number + badges, metadata and
 * financial cards, and the DUPLICATA reprint action via the existing
 * ipcReprint binding (documents_reprint). Escape closes; caller owns
 * visibility and focus return.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import {
  getDocumentDetails,
  getStoreIdentity,
  ipcGetPrinterSettings,
  ipcPrintTsplTicket,
  ipcReprint,
  type DocumentDetailsWireResponse,
  type DocumentSummaryWireDto,
} from "@/lib/ipc";
import { resolveTicketPrintPath } from "@/lib/ticket-print-path";
import {
  buildTicketPrintHtml,
  buildTicketPrintHtmlV2,
  printTicketDocument,
  ticketArgsFromWire,
  type TicketPrintStrings,
} from "@/features/tickets/graphique-ticket";
import { formatMoney, formatQty } from "@/lib/money";
import { PrintStatusIndicator } from "@/components/pos/PrintStatusIndicator";
import type { Locale } from "@/types/pos";
import "./document-print.css";
import {
  documentStatusBadge,
  documentStatusKey,
  documentTypeBadge,
  documentTypeKey,
} from "@/features/documents/document-helpers";

interface DocumentDetailDrawerProps {
  locale: Locale;
  token: string;
  document: DocumentSummaryWireDto;
  onClose: () => void;
}

function fmtAmount(centimes: string, locale: Locale): string {
  try {
    return formatMoney(centimes, locale);
  } catch {
    return "…";
  }
}

function fmtQty(micro: string): string {
  try {
    return formatQty(micro);
  } catch {
    return "…";
  }
}

/** Full document view (re-exported wire shape from sales_get). */
export type DocumentDetailsWire = DocumentDetailsWireResponse;

type DetailsLine = DocumentDetailsWireResponse["items"][number];

/**
 * Display designation for one details line (TASK-DSN-089): active-locale
 * first, fallback to the other locale, then SKU, then the raw id (orphans
 * degrade to SKU-or-id with an information badge — never blank).
 */
function itemDisplayName(item: DetailsLine, locale: Locale): string {
  if (item.product_missing) {
    return item.sku ?? item.product_id;
  }
  const primary = locale === "ar-DZ" ? item.product_name_ar : item.product_name_fr;
  const secondary = locale === "ar-DZ" ? item.product_name_fr : item.product_name_ar;
  return primary ?? secondary ?? item.sku ?? item.product_id;
}

/** Selling-unit label in the active locale (null when the unit row is gone). */
function itemUnitLabel(item: DetailsLine, locale: Locale): string | null {
  const primary = locale === "ar-DZ" ? item.unit_name_ar : item.unit_name_fr;
  const secondary = locale === "ar-DZ" ? item.unit_name_fr : item.unit_name_ar;
  return primary ?? secondary ?? null;
}

/**
 * A4 print eligibility (TASK-DSN-085): delivery notes and invoices print on
 * A4; tickets stay on the fiscal thermal path (existing DUPLICATA reprint),
 * quotes and credit notes have no A4 layout in this task.
 */
export function canPrintA4(documentType: string): boolean {
  return documentType === "BL" || documentType === "FACTURE";
}

/** Store-identity subset stamped on the A4 header (empty strings omitted). */
export interface A4Identity {
  storeName: string;
  storeAddress: string;
  storePhone: string;
  nif: string;
  nis: string;
  rc: string;
  ai: string;
  ticketFooterMessage: string;
}

/**
 * Pure A4 invoice/delivery-note sheet. Renders engine values verbatim;
 * sections without wire backing are omitted, never fabricated:
 * no per-rate TVA breakdown, no totals-in-words,
 * no customer fiscal identifiers. Discounted lines reuse the drawer
 * per-unit badge (struck raw PU + net PU + badge).
 */
export function DocumentA4Sheet(props: {
  locale: Locale;
  details: DocumentDetailsWire;
  identity: A4Identity;
  /** Display date from the summary row (the details wire carries no date). */
  createdAt: string;
}): ReactElement {
  const { locale, details, identity, createdAt } = props;
  const doc = details.document;
  const fiscal = [
    identity.nif !== "" ? `NIF ${identity.nif}` : null,
    identity.nis !== "" ? `NIS ${identity.nis}` : null,
    identity.rc !== "" ? `RC ${identity.rc}` : null,
    identity.ai !== "" ? `AI ${identity.ai}` : null,
  ].filter((part): part is string => part !== null);
  return (
    <div className="a4-sheet" data-testid="a4-sheet">
      <header>
        {identity.storeName !== "" && <h1>{identity.storeName}</h1>}
        {identity.storeAddress !== "" && <p>{identity.storeAddress}</p>}
        {identity.storePhone !== "" && (
          <p>
            <bdi dir="ltr">{identity.storePhone}</bdi>
          </p>
        )}
        {fiscal.length > 0 && (
          <p>
            <bdi dir="ltr">{fiscal.join(" · ")}</bdi>
          </p>
        )}
        <h2>
          {t(locale, documentTypeKey(doc.doc_type))} <bdi dir="ltr">{doc.number}</bdi>
        </h2>
        <p>
          {t(locale, "documents.colDate")}: <bdi dir="ltr">{createdAt}</bdi>
        </p>
      </header>
      <table>
        <thead>
          <tr>
            <th scope="col">{t(locale, "documents.colProduct")}</th>
            <th scope="col">{t(locale, "documents.colQty")}</th>
            <th scope="col">{t(locale, "documents.colUnitPrice")}</th>
            <th scope="col">{t(locale, "documents.colLineTva")}</th>
            <th scope="col">{t(locale, "documents.colLineTotal")}</th>
          </tr>
        </thead>
        <tbody>
          {details.items.map((item, index) => (
            <tr key={`${item.product_id}-${item.unit_id}-${index}`} data-testid="a4-line">
              <td>
                <bdi className="a4-num">
                  {itemDisplayName(item, locale)}
                </bdi>
              </td>
              <td>
                <bdi dir="ltr" className="a4-num">
                  {fmtQty(item.quantity_micro)}
                </bdi>
              </td>
              <td>
                {item.discounted_unit_price_centimes !== null &&
                item.discounted_unit_price_centimes !== item.unit_price_centimes ? (
                  <bdi dir="ltr" className="a4-num">
                    <s>{fmtAmount(item.unit_price_centimes, locale)}</s>{" "}
                    {fmtAmount(item.discounted_unit_price_centimes, locale)}
                    <span className="a4-badge">
                      {t(locale, "documents.perUnitDiscount")}{" "}
                      <bdi dir="ltr">
                        {fmtAmount(item.discount_per_unit_centimes ?? "0", locale)}
                      </bdi>
                    </span>
                  </bdi>
                ) : (
                  <bdi dir="ltr" className="a4-num">
                    {fmtAmount(item.unit_price_centimes, locale)}
                  </bdi>
                )}
              </td>
              <td>
                <bdi dir="ltr" className="a4-num">
                  {fmtAmount(item.line_tva_centimes, locale)}
                </bdi>
              </td>
              <td>
                <bdi dir="ltr" className="a4-num">
                  {fmtAmount(item.line_ttc_centimes, locale)}
                </bdi>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <footer>
        <p>
          {t(locale, "documents.detailsSubtotal")}:{" "}
          <bdi dir="ltr" className="a4-num">
            {fmtAmount(doc.subtotal_ht_centimes, locale)}
          </bdi>
        </p>
        <p>
          {t(locale, "documents.detailsTaxTotal")}:{" "}
          <bdi dir="ltr" className="a4-num">
            {fmtAmount(doc.tva_total_centimes, locale)}
          </bdi>
        </p>
        <p>
          {t(locale, "documents.total")}:{" "}
          <bdi dir="ltr" className="a4-num">
            {fmtAmount(doc.total_ttc_centimes, locale)}
          </bdi>
        </p>
        <p>{t(locale, "documents.printFooter")}</p>
      </footer>
    </div>
  );
}

/**
 * Pure full-document render (lines, payments, tax, totals) — no fetching.
 * All money/quantity strings render engine-provided values verbatim
 * (formatMoney/formatQty display only; zero arithmetic in React).
 * Totals-in-words are intentionally absent: the engine provides none.
 */
export function DocumentDetailsView(props: {
  locale: Locale;
  details: DocumentDetailsWire;
}): ReactElement {
  const { locale, details } = props;
  const doc = details.document;
  return (
    <div className="flex flex-col gap-3">
      <section aria-label={t(locale, "documents.detailsLines")}>
        <p className="mb-1 text-[11px] font-bold text-slate-500">
          {t(locale, "documents.detailsLines")}
        </p>
        {details.items.length === 0 ? (
          <p className="rounded-md bg-amber-50 px-3 py-2 text-xs font-bold text-amber-700 dark:bg-amber-900/30 dark:text-amber-200">
            {t(locale, "documents.detailsEmptyLines")}
          </p>
        ) : (
          <table className="w-full border-collapse text-xs">
            <thead>
              <tr className="text-left text-[10px] uppercase text-slate-500">
                <th scope="col" className="py-1 pe-2 font-bold">
                  {t(locale, "documents.colProduct")}
                </th>
                <th scope="col" className="py-1 pe-2 font-bold">
                  {t(locale, "documents.colQty")}
                </th>
                <th scope="col" className="py-1 pe-2 font-bold">
                  {t(locale, "documents.colUnitPrice")}
                </th>
                <th scope="col" className="py-1 font-bold">
                  {t(locale, "documents.colLineTotal")}
                </th>
              </tr>
            </thead>
            <tbody>
              {details.items.map((item, index) => (
                <tr
                  key={`${item.product_id}-${item.unit_id}-${index}`}
                  data-testid="details-lines"
                  className="border-t border-slate-100 dark:border-slate-800"
                >
                  <td className="py-1 pe-2">
                    {item.product_missing ? (
                      <>
                        <bdi dir="ltr" className="block font-medium">
                          {item.sku ?? item.product_id}
                        </bdi>
                        <span className="mt-0.5 inline-block rounded-full bg-amber-50 px-2 py-0.5 text-[10px] font-bold text-amber-700 dark:bg-amber-900/30 dark:text-amber-200">
                          {t(locale, "documents.detailsOrphanBadge")}
                        </span>
                      </>
                    ) : (
                      <bdi className="block font-medium">{itemDisplayName(item, locale)}</bdi>
                    )}
                    <span className="block text-[10px] text-slate-500">
                      <bdi dir="ltr">
                        {[
                          item.sku,
                          itemUnitLabel(item, locale),
                          fmtQty(item.quantity_micro),
                        ]
                          .filter((part): part is string => part !== null)
                          .join(" · ")}
                      </bdi>
                    </span>
                  </td>
                  <td className="py-1 pe-2">
                    <bdi dir="ltr" className="font-mono tabular-nums">
                      {fmtQty(item.quantity_micro)}
                    </bdi>
                  </td>
                  <td className="py-1 pe-2">
                    {item.discounted_unit_price_centimes !== null &&
                    item.discounted_unit_price_centimes !== item.unit_price_centimes ? (
                      <>
                        <s className="font-mono tabular-nums text-slate-400">
                          <bdi dir="ltr">{fmtAmount(item.unit_price_centimes, locale)}</bdi>
                        </s>{" "}
                        <bdi dir="ltr" className="font-mono font-bold tabular-nums">
                          {fmtAmount(item.discounted_unit_price_centimes, locale)}
                        </bdi>
                        <span className="block text-[10px] font-bold text-emerald-700 dark:text-emerald-300">
                          {t(locale, "documents.perUnitDiscount")}{" "}
                          <bdi dir="ltr" className="font-mono tabular-nums">
                            {fmtAmount(item.discount_per_unit_centimes ?? "0", locale)}
                          </bdi>
                        </span>
                      </>
                    ) : (
                      <bdi dir="ltr" className="font-mono tabular-nums">
                        {fmtAmount(item.unit_price_centimes, locale)}
                      </bdi>
                    )}
                    <span className="block text-[10px] text-slate-500">
                      {t(locale, "documents.colLineTva")}{" "}
                      <bdi dir="ltr" className="font-mono tabular-nums">
                        {fmtAmount(item.line_tva_centimes, locale)}
                      </bdi>
                    </span>
                  </td>
                  <td className="py-1">
                    <bdi dir="ltr" className="font-mono font-bold tabular-nums">
                      {fmtAmount(item.line_ttc_centimes, locale)}
                    </bdi>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section aria-label={t(locale, "documents.detailsPayments")}>
        <p className="mb-1 text-[11px] font-bold text-slate-500">
          {t(locale, "documents.detailsPayments")}
        </p>
        <ul className="flex flex-col gap-1">
          {details.payments.map((payment, index) => (
            <li
              key={`${payment.method_code}-${index}`}
              className="flex items-center justify-between rounded-md bg-slate-50 px-2 py-1 text-xs dark:bg-slate-800"
            >
              <bdi dir="ltr" className="font-mono font-bold">
                {payment.method_code}
              </bdi>
              <bdi dir="ltr" className="font-mono tabular-nums">
                {fmtAmount(payment.amount_centimes, locale)}
              </bdi>
            </li>
          ))}
        </ul>
      </section>

      <section
        aria-label={t(locale, "documents.total")}
        className="flex flex-col gap-1 rounded-lg border border-slate-200 p-3 text-xs dark:border-slate-700"
      >
        <div className="flex items-center justify-between">
          <span className="font-bold text-slate-500">{t(locale, "documents.detailsSubtotal")}</span>
          <bdi dir="ltr" className="font-mono tabular-nums">
            {fmtAmount(doc.subtotal_ht_centimes, locale)}
          </bdi>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-bold text-slate-500">{t(locale, "documents.detailsTaxTotal")}</span>
          <bdi dir="ltr" className="font-mono tabular-nums">
            {fmtAmount(doc.tva_total_centimes, locale)}
          </bdi>
        </div>
        <div className="flex items-center justify-between text-sm">
          <span className="font-black">{t(locale, "documents.total")}</span>
          <bdi dir="ltr" className="font-mono text-xl font-black tabular-nums">
            {fmtAmount(doc.total_ttc_centimes, locale)}
          </bdi>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-bold text-slate-500">{t(locale, "documents.detailsPaid")}</span>
          <bdi dir="ltr" className="font-mono tabular-nums">
            {fmtAmount(doc.paid_amount_centimes, locale)}
          </bdi>
        </div>
        <div className="flex items-center justify-between">
          <span className="font-bold text-slate-500">{t(locale, "documents.detailsChange")}</span>
          <bdi dir="ltr" className="font-mono tabular-nums">
            {fmtAmount(doc.change_amount_centimes, locale)}
          </bdi>
        </div>
      </section>
    </div>
  );
}

export function DocumentDetailDrawer(props: DocumentDetailDrawerProps): ReactElement {
  const { locale, document } = props;
  const [reprintState, setReprintState] = useState<"idle" | "sending" | "ok" | "fail">("idle");
  const [reprintError, setReprintError] = useState<string | null>(null);
  const [details, setDetails] = useState<DocumentDetailsWire | null>(null);
  const [detailsLoading, setDetailsLoading] = useState<boolean>(true);
  const [detailsError, setDetailsError] = useState<string | null>(null);
  const [detailsTick, setDetailsTick] = useState<number>(0);
  const [identity, setIdentity] = useState<A4Identity | null>(null);
  const [printing, setPrinting] = useState<boolean>(false);
  // Graphique ticket mode (TASK-DSN-093): resolved per drawer open; escpos
  // keeps the engine DUPLICATA reprint below untouched.
  const [renderMode, setRenderMode] = useState<"escpos" | "graphique">("escpos");
  // Silent TSPL ticket (TASK-DSN-098): spooler transport owns the silent
  // button; anything else keeps the frozen graphique/engine paths below.
  const [transport, setTransport] = useState<"spooler" | "serial">("serial");
  const [tsplState, setTsplState] = useState<"idle" | "sending" | "ok" | "fail">("idle");
  const [tsplError, setTsplError] = useState<string | null>(null);
  const [tsplBytes, setTsplBytes] = useState<number | null>(null);

  useEffect(() => {
    setReprintState("idle");
    setReprintError(null);
    setDetails(null);
    setDetailsError(null);
    setDetailsLoading(true);
    setIdentity(null);
    setPrinting(false);
    setRenderMode("escpos");
    setTransport("serial");
    setTsplState("idle");
    setTsplError(null);
    setTsplBytes(null);
    let cancelled = false;
    getDocumentDetails(document.id)
      .then((full) => {
        if (!cancelled) {
          setDetails(full);
          setDetailsLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setDetailsError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
          setDetailsLoading(false);
        }
      });
    ipcGetPrinterSettings(props.token)
      .then((settings) => {
        if (!cancelled) {
          setRenderMode(settings.ticketRenderMode);
          setTransport(settings.transport);
        }
      })
      .catch(() => {
        // Settings unreadable: stay on the escpos engine path.
      });
    return () => {
      cancelled = true;
    };
  }, [document.id, detailsTick, props.token]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        props.onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [props]);

  useEffect(() => {
    const done = (): void => {
      globalThis.document.body.classList.remove("printing-a4");
      setPrinting(false);
    };
    window.addEventListener("afterprint", done);
    return () => {
      window.removeEventListener("afterprint", done);
    };
  }, []);

  useEffect(() => {
    if (!printing || details === null || identity === null) {
      return;
    }
    globalThis.document.body.classList.add("printing-a4");
    const frame = window.requestAnimationFrame(() => {
      window.print();
    });
    return () => {
      window.cancelAnimationFrame(frame);
    };
  }, [printing, details, identity]);

  const printA4 = (): void => {
    if (details === null || printing || !canPrintA4(document.document_type)) {
      return;
    }
    setPrinting(true);
    getStoreIdentity()
      .then((state) => {
        const found = state.identity;
        setIdentity({
          storeName: found.storeName,
          storeAddress: found.storeAddress,
          storePhone: found.storePhone,
          nif: found.nif,
          nis: found.nis,
          rc: found.rc,
          ai: found.ai,
          ticketFooterMessage: found.ticketFooterMessage,
        });
      })
      .catch(() => {
        setIdentity({
          storeName: "",
          storeAddress: "",
          storePhone: "",
          nif: "",
          nis: "",
          rc: "",
          ai: "",
          ticketFooterMessage: "",
        });
      });
  };

  /** Graphique ticket reprint (TASK-DSN-093): same HTML payload as the
   * checkout auto-print, always marked DUPLICATA (the drawer only opens
   * committed documents). Language falls back to the UI locale — the
   * details wire carries no document_language (no engine change). */
  const printTicket = (): void => {
    if (details === null) {
      return;
    }
    const snapshot = details;
    const docLang: "AR" | "FR" = locale === "ar-DZ" ? "AR" : "FR";
    const strings: TicketPrintStrings = {
      title: t(locale, "ticket.title"),
      reprintMark: t(locale, "ticket.reprintMark"),
      designation: t(locale, "ticket.designation"),
      cashier: t(locale, "ticket.cashier"),
      qty: t(locale, "ticket.qty"),
      price: t(locale, "ticket.price"),
      total: t(locale, "ticket.total"),
      subtotal: t(locale, "ticket.subtotal"),
      discountTotal: t(locale, "ticket.discountTotal"),
      tva: t(locale, "ticket.tva"),
      grandTotal: t(locale, "ticket.grandTotal"),
      paid: t(locale, "ticket.paid"),
      change: t(locale, "ticket.change"),
      payment: t(locale, "ticket.payment"),
      footer: t(locale, "ticket.footer"),
      depot: t(locale, "ticket.depot"),
      discountBadge: t(locale, "ticket.discountBadge"),
      article: t(locale, "ticket.article"),
      qtyPrice: t(locale, "ticket.qtyPrice"),
    };
    const dispatchWith = (id: A4Identity): void => {
      // DOC-07: the wire resolves display names + the wall-clock instant
      // once — the template renders them verbatim, like the engine.
      const args = ticketArgsFromWire(
        {
          documentNumber: snapshot.document.number,
          createdAt: snapshot.document.created_at_display,
          templateVersion: snapshot.document.template_version,
          subtotalHt: snapshot.document.subtotal_ht_centimes,
              discountTotal: snapshot.document.global_discount_centimes,
              tvaTotal: snapshot.document.tva_total_centimes,
              totalTtc: snapshot.document.total_ttc_centimes,
              paidAmount: snapshot.document.paid_amount_centimes,
              changeAmount: snapshot.document.change_amount_centimes,
              paymentLabel: snapshot.payments.map((p) => p.method_code).join(" + "),
              items: snapshot.items.map((item) => ({
                nameFr: item.product_name_fr,
                nameAr: item.product_name_ar,
                sku: item.sku,
                unitFr: item.unit_name_fr,
                unitAr: item.unit_name_ar,
                quantityMicro: item.quantity_micro,
                unitPriceCentimes: item.unit_price_centimes,
                discountPerUnitCentimes: item.discount_per_unit_centimes,
                discountedUnitPriceCentimes: item.discounted_unit_price_centimes,
                lineTtcCentimes: item.line_ttc_centimes,
              })),
            },
            {
              language: docLang,
              storeName: id.storeName,
              storeAddress: id.storeAddress,
              storePhone: id.storePhone,
              taxRows: [
                { prefix: "NIF", value: id.nif },
                { prefix: "NIS", value: id.nis },
                { prefix: "RC", value: id.rc },
                { prefix: "AI", value: id.ai },
              ],
              warehouseText: snapshot.document.warehouse_name,
              cashierText: snapshot.document.created_by_name,
              isReprint: true,
              ownerFooter: id.ticketFooterMessage,
              strings,
            },
          );
          // DOC-07: the pinned layout rides the stored document — v1 keeps
          // its frozen mirror, v2 the redesign.
          printTicketDocument(
            args.templateVersion === 2 ? buildTicketPrintHtmlV2(args) : buildTicketPrintHtml(args),
          );
    };
    if (identity !== null) {
      dispatchWith(identity);
      return;
    }
    getStoreIdentity()
      .then((state) => {
        const found = state.identity;
        dispatchWith({
          storeName: found.storeName,
          storeAddress: found.storeAddress,
          storePhone: found.storePhone,
          nif: found.nif,
          nis: found.nis,
          rc: found.rc,
          ai: found.ai,
          ticketFooterMessage: found.ticketFooterMessage,
        });
      })
      .catch(() => {
        dispatchWith({
          storeName: "",
          storeAddress: "",
          storePhone: "",
          nif: "",
          nis: "",
          rc: "",
          ai: "",
          ticketFooterMessage: "",
        });
      });
  };

  const reprint = (): void => {
    if (reprintState === "sending") {
      return;
    }
    setReprintState("sending");
    setReprintError(null);
    ipcReprint(props.token, document.id)
      .then(() => {
        setReprintState("ok");
      })
      .catch((failure: unknown) => {
        setReprintState("fail");
        setReprintError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      });
  };

  // Silent TSPL ticket (TASK-DSN-098): spooler-only, never window.print.
  const printTsplTicketSilent = (): void => {
    if (tsplState === "sending") {
      return;
    }
    setTsplState("sending");
    setTsplError(null);
    setTsplBytes(null);
    ipcPrintTsplTicket(props.token, document.id, true)
      .then((bytes) => {
        setTsplBytes(bytes);
        setTsplState("ok");
      })
      .catch((failure: unknown) => {
        setTsplState("fail");
        setTsplError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      });
  };

  const ticketPath = resolveTicketPrintPath({
    transport,
    renderMode,
    documentType: document.document_type,
  });

  return (
    <aside className="flex h-full w-[400px] min-h-0 flex-none flex-col gap-3 overflow-y-auto border-s border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-900">
      <div className="flex items-start gap-2">
        <div className="min-w-0 flex-1">
          <bdi dir="ltr" className="block truncate font-mono text-base font-black">
            {document.document_number}
          </bdi>
          <div className="mt-1 flex flex-wrap gap-1">
            <span
              className={`rounded-full px-2 py-0.5 text-[10px] font-black ${documentTypeBadge(document.document_type)}`}
            >
              {t(locale, documentTypeKey(document.document_type))}
            </span>
            <span
              className={`rounded-full px-2 py-0.5 text-[10px] font-black ${documentStatusBadge(document.status)}`}
            >
              {t(locale, documentStatusKey(document.status))}
            </span>
          </div>
        </div>
        <button
          type="button"
          onClick={props.onClose}
          className="shrink-0 rounded-md bg-slate-200 px-2.5 py-1 text-xs font-bold dark:bg-slate-700"
        >
          {t(locale, "documents.close")}
        </button>
      </div>

      <div className="flex flex-col gap-1 rounded-lg border border-slate-200 p-3 text-xs dark:border-slate-700">
        <p>
          <span className="font-bold text-slate-500">{t(locale, "documents.customer")}: </span>
          {document.party_name ?? t(locale, "documents.walkIn")}
        </p>
        <p>
          <span className="font-bold text-slate-500">{t(locale, "documents.colDate")}: </span>
          <bdi dir="ltr">{document.created_at}</bdi>
        </p>
        <p>
          <span className="font-bold text-slate-500">{t(locale, "documents.colItems")}: </span>
          <bdi dir="ltr" className="font-mono tabular-nums">
            {document.items_count}
          </bdi>
        </p>
      </div>

      {detailsLoading ? (
        <div aria-busy="true" className="flex animate-pulse flex-col gap-2">
          <div className="h-4 rounded bg-slate-200 dark:bg-slate-700" />
          <div className="h-16 rounded bg-slate-200 dark:bg-slate-700" />
          <div className="h-4 w-2/3 rounded bg-slate-200 dark:bg-slate-700" />
          <span className="sr-only">{t(locale, "documents.detailsLoading")}</span>
        </div>
      ) : detailsError !== null ? (
        <div className="flex flex-col gap-2 rounded-lg border border-red-200 p-3 dark:border-red-900">
          <p
            role="alert"
            className="rounded-md bg-red-50 px-3 py-2 text-xs font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200"
          >
            {t(locale, "documents.detailsError")}
            {` · ${translateBackendError(detailsError, locale)}`}
          </p>
          {(detailsError === "UNAUTHORIZED" || detailsError === "FORBIDDEN") && (
            <p className="px-1 text-xs font-bold text-red-600 dark:text-red-300">
              {t(locale, "documents.detailsNoPermission")}
            </p>
          )}
          <button
            type="button"
            onClick={() => {
              setDetailsTick((tick) => tick + 1);
            }}
            className="h-9 shrink-0 rounded-lg bg-slate-200 text-xs font-black dark:bg-slate-700"
          >
            {t(locale, "documents.detailsRetry")}
          </button>
        </div>
      ) : details !== null ? (
        <DocumentDetailsView locale={locale} details={details} />
      ) : null}

      <PrintStatusIndicator documentId={document.id} />

      {details !== null && canPrintA4(document.document_type) && (
        <button
          type="button"
          onClick={printA4}
          disabled={printing}
          data-testid="print-a4"
          className="h-11 shrink-0 rounded-lg border border-slate-300 text-sm font-black dark:border-slate-600"
        >
          {t(locale, "documents.printA4")}
        </button>
      )}
      {details !== null && (
        <DocumentA4Sheet
          locale={locale}
          details={details}
          identity={
            identity ?? {
              storeName: "",
              storeAddress: "",
              storePhone: "",
              nif: "",
              nis: "",
              rc: "",
              ai: "",
              ticketFooterMessage: "",
            }
          }
          createdAt={document.created_at}
        />
      )}

      {ticketPath === "silent-tspl" ? (
        <button
          type="button"
          data-testid="silent-tspl-ticket"
          onClick={printTsplTicketSilent}
          disabled={tsplState === "sending"}
          className="h-11 shrink-0 rounded-lg bg-accent-500 text-sm font-black text-slate-950 disabled:opacity-50"
        >
          {tsplState === "sending" ? t(locale, "tspl.ticketSending") : t(locale, "tspl.silentTicket")}
        </button>
      ) : renderMode === "graphique" ? (
        <button
          type="button"
          onClick={printTicket}
          disabled={details === null}
          className="h-11 shrink-0 rounded-lg bg-accent-500 text-sm font-black text-slate-950 disabled:opacity-50"
        >
          {t(locale, "documents.printTicket")}
        </button>
      ) : (
        <button
          type="button"
          onClick={reprint}
          disabled={reprintState === "sending"}
          className="h-11 shrink-0 rounded-lg bg-accent-500 text-sm font-black text-slate-950 disabled:opacity-50"
        >
          {reprintState === "sending" ? t(locale, "documents.reprinting") : t(locale, "documents.reprint")}
        </button>
      )}
      {ticketPath === "silent-tspl" && tsplState === "ok" && (
        <p className="rounded-md bg-emerald-50 px-3 py-2 text-xs font-bold text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-200" data-testid="silent-tspl-ticket-notice" role="status">
          {t(locale, "tspl.ticketOk").replace("{bytes}", String(tsplBytes ?? 0))}
        </p>
      )}
      {ticketPath === "silent-tspl" && tsplState === "fail" && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-xs font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200" data-testid="silent-tspl-ticket-error" role="alert">
          {t(locale, "tspl.ticketFail")}
          {tsplError !== null && ` · ${translateBackendError(tsplError, locale)}`}
        </p>
      )}
      {reprintState === "ok" && (
        <p className="rounded-md bg-emerald-50 px-3 py-2 text-xs font-bold text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-200">
          {t(locale, "documents.reprintOk")}
        </p>
      )}
      {reprintState === "fail" && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-xs font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {t(locale, "documents.reprintFail")}
          {reprintError !== null && ` · ${translateBackendError(reprintError, locale)}`}
        </p>
      )}
    </aside>
  );
}
