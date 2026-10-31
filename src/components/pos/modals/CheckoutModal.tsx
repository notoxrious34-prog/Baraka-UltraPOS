/**
 * Checkout modal (F12): split tender, fast cash buttons, live change,
 * timbre notice, atomic commit with idempotency command id.
 *
 * All displayed totals (subtotal HT, global discount, TVA, timbre, TTC,
 * change) come EXCLUSIVELY from the backend `sales_preview_totals`
 * command — the same Rust pricing kernel that commits the sale — so the
 * figures the cashier confirms are byte-for-byte the figures persisted
 * (TASK-BRK-031). Zero client-side totals arithmetic exists here.
 *
 * Submission is single-dispatch (TASK-BRK-032): a strict in-flight lock
 * (`createCheckoutSubmission`) blocks repeated F12/Enter/Escape triggers
 * and a session-stable command id makes every attempt of one checkout
 * session one logical backend command (replays return the committed
 * receipt; a fresh modal session mints a fresh id).
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t, translateBackendError } from "@/lib/i18n";
import { addBig, formatMoney, formatQty, parseMoneyToCentimes } from "@/lib/money";
import {
  ipcCheckout,
  ipcPreviewTotals,
  getDocumentDetails,
  getStoreIdentity,
  ipcGetPrinterSettings,
  ipcPrintTsplTicket,
  type CheckoutLineWire,
  type CheckoutPaymentWire,
  type PreviewTotalsWire,
  type StockShortageWarningWire,
} from "@/lib/ipc";
import type { CartLine, SaleReceipt } from "@/types/pos";
import { resolveTicketPrintPath } from "@/lib/ticket-print-path";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import { PrintStatusIndicator } from "@/components/pos/PrintStatusIndicator";
import { useCashSessionStore } from "@/stores/cash-session-store";
import {
  createCheckoutSubmission,
  type CheckoutSubmission,
} from "@/components/pos/modals/checkout-submission";
import { evaluateStockGate } from "@/components/pos/modals/stock-gate";
import {
  buildTicketPrintHtml,
  printTicketDocument,
  ticketArgsFromWire,
  type TicketPrintStrings,
} from "@/features/tickets/graphique-ticket";
import type { Locale } from "@/types/pos";

/** Localised ticket chrome (FR/AR via the document language). */
function ticketStrings(locale: Locale): TicketPrintStrings {
  return {
    title: t(locale, "ticket.title"),
    reprintMark: t(locale, "ticket.reprintMark"),
    designation: t(locale, "ticket.designation"),
    qty: t(locale, "ticket.qty"),
    price: t(locale, "ticket.price"),
    total: t(locale, "ticket.total"),
    subtotal: t(locale, "ticket.subtotal"),
    tva: t(locale, "ticket.tva"),
    grandTotal: t(locale, "ticket.grandTotal"),
    paid: t(locale, "ticket.paid"),
    change: t(locale, "ticket.change"),
    payment: t(locale, "ticket.payment"),
    footer: t(locale, "ticket.footer"),
    depot: t(locale, "ticket.depot"),
    discountBadge: t(locale, "ticket.discountBadge"),
  };
}

function toCentimes(text: string): string {
  if (text.trim() === "") {
    return "0";
  }
  try {
    return parseMoneyToCentimes(text);
  } catch {
    return "0";
  }
}

function isValidAmount(text: string): boolean {
  if (text.trim() === "") {
    return true;
  }
  try {
    parseMoneyToCentimes(text);
    return true;
  } catch {
    return false;
  }
}

function ttcToDisplay(ttcCentimes: string): string {
  try {
    const whole = BigInt(ttcCentimes) / 100n;
    return whole.toString();
  } catch {
    return "0";
  }
}

/** Cart lines on the wire (identical shape for preview and commit).
 * `line_discount_centimes` travels AS the typed per-unit F5 amount
 * (BRK-096: the edge, kernel, and storage all read it per unit) — mapped
 * through verbatim here, never reinterpreted. */
function buildLineWires(lines: CartLine[]): CheckoutLineWire[] {
  return lines.map((line) => {
    const unit = line.units[line.unitIndex];
    return {
      product_id: line.productId,
      unit_id: unit !== undefined ? unit.unitId : "",
      quantity_micro: line.quantityMicro,
      unit_price_centimes: null,
      line_discount_centimes: line.lineDiscountCentimes,
    };
  });
}

/** Tender lines on the wire (identical shape for preview and commit). */
function buildPaymentWires(
  cash: string,
  credit: string,
  cheque: string,
  card: string,
): CheckoutPaymentWire[] {
  const payments: CheckoutPaymentWire[] = [];
  if (cash !== "0") {
    payments.push({ payment_method: "CASH", amount_centimes: cash });
  }
  if (credit !== "0") {
    payments.push({ payment_method: "CREDIT", amount_centimes: credit });
  }
  if (cheque !== "0") {
    payments.push({ payment_method: "CHEQUE", amount_centimes: cheque });
  }
  if (card !== "0") {
    payments.push({ payment_method: "CARD", amount_centimes: card });
  }
  return payments;
}

export function CheckoutModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const setLastReceipt = usePosUiStore((state) => state.setLastReceipt);
  const notify = usePosUiStore((state) => state.notify);
  const lines = useCartStore((state) => state.lines);
  const globalDiscountCentimes = useCartStore((state) => state.globalDiscountCentimes);
  const customer = useCartStore((state) => state.customer);
  const docType = useCartStore((state) => state.docType);
  const clearCart = useCartStore((state) => state.clearCart);
  const session = useSessionStore((state) => state.session);
  const warehouse = useSessionStore((state) => state.warehouse);

  const [cashText, setCashText] = useState("");
  const [creditText, setCreditText] = useState("0");
  const [chequeText, setChequeText] = useState("0");
  const [cardText, setCardText] = useState("0");
  const [cashTouched, setCashTouched] = useState(false);
  const [preview, setPreview] = useState<PreviewTotalsWire | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewPending, setPreviewPending] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  // Explicit sale-on-shortage confirmation (TASK-BRK-033): privileged
  // sessions only, reset whenever the shortage picture changes so every
  // distinct shortage set gets its own affirmative gesture.
  const [allowNegativeStock, setAllowNegativeStock] = useState(false);
  const shortageSignatureRef = useRef("");
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<SaleReceipt | null>(null);
  const [doneLang, setDoneLang] = useState<"AR" | "FR">("FR");
  const [docLang, setDocLang] = useState<"AR" | "FR">(() =>
    usePosUiStore.getState().locale === "ar-DZ" ? "AR" : "FR",
  );
  // Graphique auto-print (TASK-DSN-093): idempotency guard keyed by committed
  // document — the effect below must never fire twice for one sale, even
  // across re-renders (print() has no undo).
  const ticketPrintedFor = useRef<string | null>(null);
  const [ticketHtml, setTicketHtml] = useState<string | null>(null);
  const [graphiqueMode, setGraphiqueMode] = useState(false);
  // Silent TSPL reprint (TASK-DSN-098): spooler owns the paper via the
  // button below; the graphique auto/manual dialog path stays frozen.
  const [spoolerMode, setSpoolerMode] = useState(false);
  const [tsplState, setTsplState] = useState<"idle" | "sending" | "ok" | "fail">("idle");
  const [tsplError, setTsplError] = useState<string | null>(null);
  const [tsplBytes, setTsplBytes] = useState<number | null>(null);
  const previewRequestIdRef = useRef(0);
  const cashPrefilledRef = useRef(false);
  // Single-dispatch submission controller: created once per mounted modal
  // session (the shell renders this modal conditionally), owning the
  // in-flight lock and the session-stable idempotency command id
  // (TASK-BRK-032).
  const submissionRef = useRef<CheckoutSubmission | null>(null);
  if (submissionRef.current === null) {
    submissionRef.current = createCheckoutSubmission(ipcCheckout, setIsSubmitting);
  }

  const cash = toCentimes(cashText);
  const credit = toCentimes(creditText);
  const cheque = toCentimes(chequeText);
  const card = toCentimes(cardText);
  const tendered = addBig(addBig(addBig(cash, credit), cheque), card);
  const creditBlocked = customer.kind === "guest" && credit !== "0";
  const inputsValid =
    isValidAmount(cashText) &&
    isValidAmount(creditText) &&
    isValidAmount(chequeText) &&
    isValidAmount(cardText);
  const warehouseId = warehouse?.id ?? null;
  // Shortage gate (TASK-BRK-033): the backend preview names every leaf
  // short against this warehouse; non-privileged sessions are hard-blocked
  // while privileged sessions must explicitly toggle the confirmation.
  const stockWarnings: StockShortageWarningWire[] = preview?.stock_warnings ?? [];
  const stockGate = evaluateStockGate({
    role: session?.role ?? "",
    warningCount: stockWarnings.length,
    allowNegativeStock,
  });
  // Payment confirmation requires a valid, fresh, backend-computed
  // preview, no dispatch in flight, and a satisfied shortage gate — never
  // stale or locally-guessed totals (TASK-BRK-031), never a second
  // concurrent dispatch (TASK-BRK-032), never a silent sale on shortage
  // (TASK-BRK-033).
  const canConfirm =
    !isSubmitting &&
    inputsValid &&
    !creditBlocked &&
    tendered !== "0" &&
    session !== null &&
    warehouse !== null &&
    preview !== null &&
    previewError === null &&
    !previewPending &&
    !stockGate.blocked;

  // Authoritative totals: fetched from the backend pricing kernel on
  // modal open and on every cart/tender change. Stale responses are
  // dropped by request id; a failed fetch blocks payment confirmation.
  useEffect(() => {
    if (session === null || warehouseId === null) {
      return;
    }
    const requestId = previewRequestIdRef.current + 1;
    previewRequestIdRef.current = requestId;
    setPreviewPending(true);
    ipcPreviewTotals(session.token, {
      warehouse_id: warehouseId,
      document_type: docType,
      lines: buildLineWires(lines),
      global_discount_centimes: globalDiscountCentimes,
      payments: buildPaymentWires(cash, credit, cheque, card),
      customer_id: customer.kind === "named" ? customer.id : null,
    })
      .then((result) => {
        if (previewRequestIdRef.current !== requestId) {
          return;
        }
        setPreview(result);
        setPreviewError(null);
        // Publish the Margin Guard readout for the cart table pills.
        useCartStore.getState().setPreviewMargins(result.line_margins ?? []);
      })
      .catch((failure: unknown) => {
        if (previewRequestIdRef.current !== requestId) {
          return;
        }
        setPreview(null);
        const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
        setPreviewError(translateBackendError(code, usePosUiStore.getState().locale));
      })
      .finally(() => {
        if (previewRequestIdRef.current !== requestId) {
          return;
        }
        setPreviewPending(false);
      });
  }, [session, warehouseId, lines, globalDiscountCentimes, docType, cash, credit, cheque, card, customer]);

  // Shortage-confirmation reset: every distinct shortage set requires its
  // own explicit gesture — a toggle given for one cart must never leak
  // onto a different shortage picture.
  useEffect(() => {
    const warnings = preview?.stock_warnings ?? [];
    const signature = warnings
      .map((warning) => `${warning.product_id}:${warning.requested_qty_micro}`)
      .sort()
      .join("|");
    if (signature !== shortageSignatureRef.current) {
      shortageSignatureRef.current = signature;
      setAllowNegativeStock(false);
    }
  }, [preview]);

  // Graphique auto-print (TASK-DSN-093): once per committed sale, in
  // graphique mode only. The guard is claimed BEFORE any async work so a
  // re-render mid-flight can never dispatch a second print() for the same
  // document. Escpos mode is untouched (engine reprint path as before).
  useEffect(() => {
    if (done === null) {
      return;
    }
    if (ticketPrintedFor.current === done.document_id) {
      return;
    }
    ticketPrintedFor.current = done.document_id;
    setTicketHtml(null);
    setGraphiqueMode(false);
    setSpoolerMode(false);
    setTsplState("idle");
    setTsplError(null);
    setTsplBytes(null);
    const sessionToken = useSessionStore.getState().session?.token ?? null;
    if (sessionToken === null) {
      return;
    }
    const receipt = done;
    const language = doneLang;
    ipcGetPrinterSettings(sessionToken)
      .then((settings) => {
        if (settings.transport === "spooler") {
          // Spooler owns the paper: mark the receipt handled and stop here —
          // no graphique html, no auto window.print; the silent TSPL button
          // below is the only paper path for this receipt.
          setSpoolerMode(true);
          return;
        }
        if (settings.ticketRenderMode !== "graphique") {
          return;
        }
        setGraphiqueMode(true);
        return Promise.all([
          getDocumentDetails(receipt.document_id),
          getStoreIdentity(),
        ]).then(([full, identityState]) => {
          const identity = identityState.identity;
          const args = ticketArgsFromWire(
            {
              documentNumber: full.document.number,
              createdAt: new Date().toLocaleString(
                language === "AR" ? "ar-DZ" : "fr-DZ",
              ),
              subtotalHt: full.document.subtotal_ht_centimes,
              tvaTotal: full.document.tva_total_centimes,
              totalTtc: full.document.total_ttc_centimes,
              paidAmount: full.document.paid_amount_centimes,
              changeAmount: full.document.change_amount_centimes,
              paymentLabel: full.payments.map((p) => p.method_code).join(" + "),
              items: full.items.map((item) => ({
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
              language,
              storeName: identity.storeName,
              storeAddress: identity.storeAddress,
              storePhone: identity.storePhone,
              taxRows: [
                { prefix: "NIF", value: identity.nif },
                { prefix: "NIS", value: identity.nis },
                { prefix: "RC", value: identity.rc },
                { prefix: "AI", value: identity.ai },
              ],
              warehouseText: full.document.warehouse_id,
              ownerFooter: identity.ticketFooterMessage,
              isReprint: false,
              strings: ticketStrings(language === "AR" ? "ar-DZ" : "fr-DZ"),
            },
          );
          const html = buildTicketPrintHtml(args);
          setTicketHtml(html);
          printTicketDocument(html);
        });
      })
      .catch(() => {
        // Sale already committed: a ticket failure must never fail the sale.
        // The fallback button below stays hidden; the engine job row keeps
        // the FRONTEND_RENDERED audit trail for a later reprint.
      });
  }, [done, doneLang]);

  // Mount-time cash prefill: the pre-fix modal pre-filled the cash field
  // with the whole-dinar part of the total at open; preserved here, now
  // sourced from the authoritative backend TTC. Runs at most once (first
  // valid preview) and never fights operator input.
  useEffect(() => {
    if (cashPrefilledRef.current || preview === null) {
      return;
    }
    cashPrefilledRef.current = true;
    if (!cashTouched && cashText === "" && credit === "0" && cheque === "0" && card === "0") {
      setCashText(ttcToDisplay(preview.total_ttc_centimes));
    }
  }, [preview, cashTouched, cashText, credit, cheque, card]);

  const confirm = (): void => {
    if (!canConfirm || session === null || warehouse === null) {
      return;
    }
    const submission = submissionRef.current;
    if (submission === null) {
      return;
    }
    setError(null);
    // Session-stable command id: the controller owns it (minted once per
    // modal session); retries reuse it, so the backend sees one logical
    // command (TASK-BRK-032). Returns null while a dispatch is in flight.
    const dispatch = submission.confirm((commandId) => ({
      command_id: commandId,
      session_token: session.token,
      warehouse_id: warehouse.id,
      customer_id: customer.kind === "named" ? customer.id : null,
      document_type: docType,
      document_language: docLang,
      lines: buildLineWires(lines),
      global_discount_centimes: globalDiscountCentimes,
      payments: buildPaymentWires(cash, credit, cheque, card),
      // Explicit shortage confirmation travels with the command so the
      // atomic engine can enforce it (TASK-BRK-033); the backend ignores
      // it for roles without the standing grant.
      allow_negative_stock: allowNegativeStock,
      notes: null,
    }));
    if (dispatch === null) {
      return;
    }
    dispatch
      .then((receipt) => {
        setLastReceipt(receipt);
        setDone(receipt);
        setDoneLang(docLang);
        clearCart();
        void useCashSessionStore.getState().refresh();
      })
      .catch((failure: unknown) => {
        const code = typeof failure === "string" ? failure : "INTERNAL_ERROR";
        setError(translateBackendError(code, locale));
      });
  };

  const finishSale = (): void => {
    if (done !== null) {
      notify({
        title: t(locale, "checkout.success"),
        detail: `${done.document_number} · ${formatMoney(done.total_ttc_centimes, locale)} · ${t(locale, "print.pending")}`,
      });
    }
    setDone(null);
    closeModal();
    const search = document.getElementById("pos-search-input");
    if (search instanceof HTMLInputElement) {
      search.focus();
    }
  };

  // F12 / Enter confirms from inside the modal (inputs included).
  // Once committed, Enter/F12 starts the next sale instead.
  // Capture phase: the in-flight lock must intercept BEFORE the global
  // keyboard contract (bubble phase at the shell root) can close the
  // modal on Escape — an unlocked Escape mid-dispatch would unmount the
  // controller and let a reopened modal mint a fresh command id, i.e. a
  // twin sale (TASK-BRK-032, the TICK-2026-000001/000002 mechanism).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      // Strict in-flight keyboard lock: while the checkout dispatch is
      // in flight, F12/Enter/Escape are fully suppressed.
      if (
        isSubmitting &&
        (event.key === "F12" || event.key === "Enter" || event.key === "Escape")
      ) {
        event.preventDefault();
        event.stopImmediatePropagation();
        return;
      }
      if (event.key === "F12") {
        event.preventDefault();
        if (done !== null) {
          finishSale();
        } else {
          confirm();
        }
      } else if (event.key === "Enter" && !(event.target instanceof HTMLTextAreaElement)) {
        event.preventDefault();
        if (done !== null) {
          finishSale();
        } else {
          confirm();
        }
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", onKeyDown, { capture: true });
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [canConfirm, cash, credit, cheque, card, lines, globalDiscountCentimes, locale, done, isSubmitting]);

  const tenderField = (
    label: string,
    value: string,
    setValue: (value: string) => void,
    disabled: boolean,
  ): ReactElement => (
    <label className="flex flex-col gap-1 opacity-100">
      <span className="text-xs font-bold text-slate-500">{label}</span>
      <input
        value={value}
        disabled={disabled}
        onChange={(event) => {
          setValue(event.target.value);
        }}
        inputMode="decimal"
        autoComplete="off"
        className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-base font-black outline-none focus:border-primary-600 disabled:opacity-40 dark:border-slate-700 dark:bg-slate-800"
      />
    </label>
  );

  const bumpCash = (centimes: string): void => {
    setCashTouched(true);
    setCashText((current) => {
      try {
        const next = BigInt(addBig(toCentimes(current), centimes)) / 100n;
        return next.toString();
      } catch {
        return current;
      }
    });
  };

  /** Format a backend preview total ("…" while no valid preview exists). */
  const fmt = (value: string | null | undefined): string =>
    value === null || value === undefined ? "…" : formatMoney(value, locale);

  /** Format a backend micro quantity ("…" on non-canonical input). */
  const fmtQty = (value: string | null | undefined): string => {
    if (value === null || value === undefined) {
      return "…";
    }
    try {
      return formatQty(value);
    } catch {
      return "…";
    }
  };

  /** One shortage line: name plus on-hand vs requested quantities. */
  const shortageLine = (warning: StockShortageWarningWire): string =>
    t(locale, "checkout.stockOverrideLine")
      .replace("{name}", `${warning.product_name} (${warning.sku})`)
      .replace("{available}", fmtQty(warning.available_qty_micro))
      .replace("{requested}", fmtQty(warning.requested_qty_micro));

  // Silent TSPL reprint (TASK-DSN-098): spooler-only, never window.print.
  const printTsplTicketSilent = (): void => {
    if (done === null || tsplState === "sending") {
      return;
    }
    const sessionToken = useSessionStore.getState().session?.token ?? null;
    if (sessionToken === null) {
      return;
    }
    setTsplState("sending");
    setTsplError(null);
    setTsplBytes(null);
    ipcPrintTsplTicket(sessionToken, done.document_id)
      .then((bytes) => {
        setTsplBytes(bytes);
        setTsplState("ok");
      })
      .catch((failure: unknown) => {
        setTsplState("fail");
        setTsplError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
      });
  };

  const postSaleTicketPath = resolveTicketPrintPath({
    transport: spoolerMode ? "spooler" : "serial",
    renderMode: graphiqueMode ? "graphique" : "escpos",
    documentType: "TICKET",
  });

  if (done !== null) {
    return (
      <ModalShell title={t(locale, "checkout.title")} onClose={finishSale}>
        <div className="flex flex-col gap-3">
          <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            {t(locale, "checkout.success")} — <span className="font-mono">{done.document_number}</span>
          </p>
          <PrintStatusIndicator documentId={done.document_id} />
          {postSaleTicketPath === "silent-tspl" ? (
            <button
              type="button"
              data-testid="silent-tspl-postsale"
              onClick={printTsplTicketSilent}
              disabled={tsplState === "sending"}
              className="h-11 shrink-0 rounded-lg border border-slate-300 text-sm font-black dark:border-slate-600"
            >
              {tsplState === "sending" ? t(locale, "tspl.ticketSending") : t(locale, "tspl.silentTicket")}
            </button>
          ) : (
            graphiqueMode && ticketHtml !== null && (
              <button
                type="button"
                onClick={() => {
                  printTicketDocument(ticketHtml);
                }}
                className="h-11 shrink-0 rounded-lg border border-slate-300 text-sm font-black dark:border-slate-600"
              >
                {t(locale, "checkout.printTicket")}
              </button>
            )
          )}
          {postSaleTicketPath === "silent-tspl" && tsplState === "ok" && (
            <p className="rounded-lg bg-emerald-50 px-3 py-2 text-center text-sm font-bold text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200" data-testid="silent-tspl-postsale-notice" role="status">
              {t(locale, "tspl.ticketOk").replace("{bytes}", String(tsplBytes ?? 0))}
            </p>
          )}
          {postSaleTicketPath === "silent-tspl" && tsplState === "fail" && (
            <p className="rounded-lg bg-red-50 px-3 py-2 text-center text-sm font-bold text-red-800 dark:bg-red-900/30 dark:text-red-200" data-testid="silent-tspl-postsale-error" role="alert">
              {t(locale, "tspl.ticketFail")}
              {tsplError !== null && ` · ${translateBackendError(tsplError, locale)}`}
            </p>
          )}
          <button
            type="button"
            onClick={finishSale}
            className="h-12 rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400"
          >
            {t(locale, "checkout.newSale")}
          </button>
        </div>
      </ModalShell>
    );
  }

  return (
    <ModalShell
      title={t(locale, "checkout.title")}
      onClose={() => {
        if (!isSubmitting) {
          closeModal();
        }
      }}
    >
      <div className="flex flex-col gap-3">
        <div className="rounded-lg bg-slate-100 p-3 text-sm dark:bg-slate-800">
          <div className="flex justify-between py-0.5">
            <span>{t(locale, "totals.subtotal")}</span>
            <span className="font-mono font-bold">{fmt(preview?.subtotal_ht_centimes)}</span>
          </div>
          <div className="flex justify-between py-0.5">
            <span>{t(locale, "totals.discount")}</span>
            <span className="font-mono font-bold">−{fmt(preview?.global_discount_centimes)}</span>
          </div>
          <div className="flex justify-between py-0.5">
            <span>{t(locale, "totals.tva")}</span>
            <span className="font-mono font-bold">{fmt(preview?.tva_total_centimes)}</span>
          </div>
          <div className="flex justify-between py-0.5">
            <span>{t(locale, "totals.timbre")}</span>
            <span className="font-mono font-bold">{fmt(preview?.timbre_centimes)}</span>
          </div>
          <div className="flex justify-between py-0.5 text-base font-black">
            <span>{t(locale, "totals.total")}</span>
            <span className="font-mono tabular-nums">{fmt(preview?.total_ttc_centimes)}</span>
          </div>
          {cash !== "0" && docType !== "BL" && (
            <p className="mt-1 text-[11px] font-semibold text-amber-700 dark:text-amber-300">
              {t(locale, "checkout.timbreNote")}
            </p>
          )}
          <div className="mt-2 flex items-center justify-between gap-2">
            <span className="text-[11px] font-bold text-slate-500">
              {t(locale, "checkout.docLanguage")}
            </span>
            <span className="flex gap-1" role="group" aria-label={t(locale, "checkout.docLanguage")}>
              {(["AR", "FR"] as const).map((option) => (
                <button
                  key={option}
                  type="button"
                  disabled={isSubmitting}
                  onClick={() => {
                    setDocLang(option);
                  }}
                  className={`rounded-md px-3 py-1 font-mono text-xs font-black disabled:opacity-40 ${
                    docLang === option
                      ? "bg-primary-600 text-white"
                      : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                  }`}
                >
                  {option === "AR" ? "العربية" : "Français"}
                </button>
              ))}
            </span>
          </div>
        </div>

        {previewError !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {previewError}
          </p>
        )}

        {stockGate.hasShortage && !stockGate.isPrivileged && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-black text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {t(locale, "checkout.stockBlocked").replace("{count}", String(stockWarnings.length))}
          </p>
        )}

        {stockGate.needsConfirmation && (
          <div className="rounded-md bg-amber-50 px-3 py-2 text-sm dark:bg-amber-900/30">
            <p className="font-black text-amber-800 dark:text-amber-200">
              {t(locale, "checkout.stockOverrideTitle")}
            </p>
            <ul className="mt-1 flex flex-col gap-0.5">
              {stockWarnings.map((warning) => (
                <li
                  key={warning.product_id}
                  className="font-semibold text-amber-800 dark:text-amber-200"
                >
                  {shortageLine(warning)}
                </li>
              ))}
            </ul>
            <label className="mt-2 flex cursor-pointer items-center gap-2 font-black text-amber-900 dark:text-amber-100">
              <input
                type="checkbox"
                checked={allowNegativeStock}
                disabled={isSubmitting}
                onChange={(event) => {
                  setAllowNegativeStock(event.target.checked);
                }}
                className="h-4 w-4 accent-amber-600"
              />
              {t(locale, "checkout.allowNegativeStock")}
            </label>
            <p className="mt-1 text-[11px] font-semibold text-amber-700 dark:text-amber-300">
              {t(locale, "checkout.stockConfirmHint")}
            </p>
          </div>
        )}

        <div className="grid grid-cols-2 gap-2">
          {tenderField(t(locale, "checkout.cash"), cashText, (value) => {
            setCashTouched(true);
            setCashText(value);
          }, isSubmitting)}
          {tenderField(
            t(locale, "checkout.credit"),
            creditText,
            setCreditText,
            isSubmitting || customer.kind === "guest",
          )}
        </div>
        <div className="grid grid-cols-2 gap-2">
          {tenderField(t(locale, "checkout.cheque"), chequeText, setChequeText, isSubmitting)}
          {tenderField(t(locale, "checkout.card"), cardText, setCardText, isSubmitting)}
        </div>
        {customer.kind === "guest" && (
          <p className="text-[11px] font-semibold text-slate-500">{t(locale, "checkout.creditNeedsCustomer")}</p>
        )}

        <div className="flex flex-wrap gap-1">
          {[
            { label: t(locale, "checkout.fastExact"), set: true, add: "0" },
            { label: "+100", set: false, add: "10000" },
            { label: "+500", set: false, add: "50000" },
            { label: "+1000", set: false, add: "100000" },
            { label: "+2000", set: false, add: "200000" },
          ].map((button) => (
            <button
              key={button.label}
              type="button"
              disabled={isSubmitting}
              onClick={() => {
                setCashTouched(true);
                if (button.set) {
                  if (preview !== null) {
                    setCashText(ttcToDisplay(preview.total_ttc_centimes));
                  }
                } else {
                  bumpCash(button.add);
                }
              }}
              className="rounded-md bg-slate-200 px-2 py-1 font-mono text-xs font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-700"
            >
              {button.label}
            </button>
          ))}
        </div>

        <div className="flex items-center justify-between rounded-lg bg-emerald-50 px-3 py-2 dark:bg-emerald-900/30">
          <span className="text-sm font-black text-emerald-800 dark:text-emerald-200">
            {t(locale, "checkout.change")}
          </span>
          <span className="font-mono text-2xl font-black tabular-nums text-emerald-700 dark:text-emerald-300">
            {fmt(preview?.change_amount_centimes)}
          </span>
        </div>

        {error !== null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {error}
          </p>
        )}

        <div className="flex gap-2">
          <button
            type="button"
            disabled={isSubmitting}
            onClick={() => {
              if (!isSubmitting) {
                closeModal();
              }
            }}
            className="h-12 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-700"
          >
            {t(locale, "checkout.cancel")}
          </button>
          <button
            type="button"
            onClick={confirm}
            disabled={!canConfirm}
            className="h-12 flex-[2] rounded-lg bg-accent-500 text-base font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
          >
            {isSubmitting ? (
              <span className="flex items-center justify-center gap-2">
                <span
                  className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-slate-900 border-t-transparent"
                  aria-hidden="true"
                />
                {t(locale, "checkout.processing")}
              </span>
            ) : (
              t(locale, "checkout.confirm")
            )}
          </button>
        </div>
      </div>
    </ModalShell>
  );
}
