/**
 * Sale-return cockpit modal (TASK-BRK-047 re-issue).
 *
 * Two phases: source lookup (UUID direct or human-number resolution) with
 * an eligibility banner, then the return workstation — per-line quantities
 * hard-clamped to remaining ceilings, refund-method gating (guest lockout
 * on CustomerCredit, closed-drawer hint on Cash), BigInt settlement
 * preview, single-flight idempotent dispatch, and a success view with
 * reprint. All money renders via formatMoney and computes via BigInt —
 * zero floats. Backend errors surface through translateBackendError; the
 * backend is final.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { isCheckoutPermitted, useCashSessionStore } from "@/stores/cash-session-store";
import {
  computeReturnTotals,
  isGuestContext,
  parseReturnQtyInput,
  returnLineHtCents,
  returnLineTvaCents,
  useReturnModalStore,
  type ReturnRefundMethod,
} from "@/stores/return-modal-store";
import { t, translateBackendError } from "@/lib/i18n";
import { createDebouncedSearchRunner } from "@/lib/debounced-search";
import { formatMoney, formatQty } from "@/lib/money";
import { ipcReprint } from "@/lib/ipc";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { Locale } from "@/types/pos";

function fmtQty(micro: bigint): string {
  try {
    return formatQty(micro.toString());
  } catch {
    return "…";
  }
}

function fmtCents(cents: bigint, locale: Locale): string {
  try {
    return formatMoney(cents.toString(), locale);
  } catch {
    return "…";
  }
}

const REFUND_OPTIONS: Array<{ key: ReturnRefundMethod; labelKey: Parameters<typeof t>[1] }> = [
  { key: "CASH", labelKey: "returns.cashRefund" },
  { key: "CUSTOMER_CREDIT", labelKey: "returns.creditRefund" },
  { key: "NO_REFUND", labelKey: "returns.noRefund" },
];

export function ReturnModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const cashSession = useCashSessionStore((state) => state.session);
  const s = useReturnModalStore();
  const [printing, setPrinting] = useState(false);
  // Instant debounced document lookup (TASK-DSN-067): 200ms, min length 2.
  // Enter keeps its direct immediate path (barcode-safe, bypasses the gate).
  const lookupSearchRef = useRef<ReturnType<typeof createDebouncedSearchRunner> | null>(null);
  if (lookupSearchRef.current === null) {
    lookupSearchRef.current = createDebouncedSearchRunner({
      delayMs: 200,
      shouldSearch: (firedQuery) => firedQuery.trim().length >= 2,
      onSearch: (firedQuery) => {
        void s.searchAndLoadDocument(firedQuery);
      },
    });
  }
  useEffect(() => {
    lookupSearchRef.current?.search(s.searchQuery);
  }, [s.searchQuery]);
  useEffect(
    () => () => {
      lookupSearchRef.current?.dispose();
    },
    [],
  );

  const token = session !== null ? session.token : null;
  const guest = isGuestContext(s.context);
  const totals = computeReturnTotals(s.context, s.quantities);
  const canSubmit =
    !s.isSubmitting &&
    s.context !== null &&
    totals.totalQtyMicro > 0n &&
    !(s.refundMethod === "CUSTOMER_CREDIT" && guest);
  const cashOpen = isCheckoutPermitted(cashSession);

  // Capture-phase keyboard guard: global contract closes on Escape and
  // confirms on F12/Enter — mid-dispatch that would unmount this
  // controller, so while submitting every trigger is suppressed; when
  // idle, F12 confirms and Escape closes (CheckoutModal precedent).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (s.isSubmitting) {
        if (event.key === "Escape" || event.key === "Enter" || event.key === "F12") {
          event.preventDefault();
          event.stopImmediatePropagation();
        }
        return;
      }
      if (event.key === "F12") {
        event.preventDefault();
        void s.submitReturn();
      } else if (event.key === "Escape") {
        event.preventDefault();
        s.closeModal();
        closeModal();
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [s.isSubmitting, s, closeModal]);

  const close = (): void => {
    if (s.isSubmitting) {
      return;
    }
    s.closeModal();
    closeModal();
  };

  const printAvoir = (): void => {
    if (token === null || s.successResult === null || printing) {
      return;
    }
    setPrinting(true);
    ipcReprint(token, s.successResult.return_document_id)
      .catch(() => {
        // Reprint failures are outbox-tracked server-side; the AVOIR
        // itself is already committed, so stay on the success view.
      })
      .finally(() => {
        setPrinting(false);
      });
  };

  return (
    <ModalShell title={t(locale, "returns.title")} onClose={close}>
      <div className="flex max-h-[70vh] flex-col gap-3 overflow-y-auto">
        {s.successResult !== null ? (
          <div className="flex flex-col gap-2">
            <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
              {t(locale, "returns.returnSuccess").replace(
                "{number}",
                s.successResult.return_document_number,
              )}
            </p>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={printAvoir}
                disabled={printing}
                className="h-11 flex-1 rounded-lg bg-primary-600 text-sm font-black text-white disabled:opacity-50"
              >
                {printing ? t(locale, "returns.printing") : t(locale, "returns.printAvoir")}
              </button>
              <button
                type="button"
                onClick={close}
                className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100"
              >
                {t(locale, "returns.close")}
              </button>
            </div>
          </div>
        ) : s.context === null ? (
          <>
            <div className="flex gap-2">
              <input
                value={s.searchQuery}
                onChange={(event) => {
                  s.setSearchQuery(event.target.value);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    lookupSearchRef.current?.reset();
                    void s.searchAndLoadDocument(s.searchQuery);
                  }
                }}
                placeholder={t(locale, "returns.searchPlaceholder")}
                autoComplete="off"
                dir="ltr"
                disabled={s.isSearching || s.isLoadingContext}
                className="h-11 min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm font-bold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
            </div>
            {(s.isSearching || s.isLoadingContext) && (
              <p className="text-center text-xs font-bold text-slate-500">
                {t(locale, "returns.searching")}
              </p>
            )}
          </>
        ) : (
          <>
            <div className="rounded-lg bg-slate-100 px-3 py-2 text-sm dark:bg-slate-800">
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono font-extrabold">
                  {s.context.document_number}
                </span>
                <span className="rounded-full bg-slate-300 px-2 py-0.5 font-mono text-[10px] font-black dark:bg-slate-600">
                  {s.context.document_type}
                </span>
              </div>
              <p className="truncate text-xs opacity-70">
                {s.context.party_name ?? t(locale, "return.cashCustomer")} ·{" "}
                {s.context.warehouse_id.slice(0, 8)}
              </p>
            </div>

            {!s.context.is_eligible && (
              <p className="rounded-md bg-amber-50 px-3 py-2 text-sm font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
                {t(locale, "returns.ineligibleDocument")}
                {s.context.ineligibility_reason !== null &&
                  ` — ${s.context.ineligibility_reason}`}
              </p>
            )}

            <div className="flex gap-2">
              <button
                type="button"
                disabled={s.isSubmitting}
                onClick={() => {
                  s.setReturnAll();
                }}
                className="h-9 flex-1 rounded-lg bg-primary-600 text-xs font-black text-white disabled:opacity-50"
              >
                {t(locale, "returns.selectAll")}
              </button>
              <button
                type="button"
                disabled={s.isSubmitting}
                onClick={() => {
                  s.resetQuantities();
                }}
                className="h-9 flex-1 rounded-lg bg-slate-200 text-xs font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
              >
                {t(locale, "returns.resetQuantities")}
              </button>
              <button
                type="button"
                disabled={s.isSubmitting}
                onClick={() => {
                  useReturnModalStore.setState({
                    context: null,
                    quantities: {},
                    sourceDocumentId: null,
                    successResult: null,
                  });
                }}
                className="h-9 flex-1 rounded-lg bg-slate-200 text-xs font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
              >
                {t(locale, "returns.backToSearch")}
              </button>
            </div>

            <div className="flex flex-col gap-1">
              <div className="grid grid-cols-[1fr_auto] gap-2 px-2 text-[10px] font-black uppercase text-slate-400">
                <span>{t(locale, "returns.colArticle")}</span>
                <span>{t(locale, "returns.colLineTotal")}</span>
              </div>
              {s.context.items.map((item) => {
                const remaining = tryBigInt(item.remaining_allowable_micro);
                const picked = tryBigInt(s.quantities[item.source_item_id] ?? "0");
                const pickedHt = returnLineHtCents(picked, tryBigInt(item.unit_price_centimes));
                const lineTtcPreview =
                  pickedHt +
                  returnLineTvaCents(pickedHt, tryBigInt(String(item.tva_rate)));
                return (
                  <div
                    key={item.source_item_id}
                    className="flex flex-col gap-1 rounded-md bg-slate-100 px-2 py-2 dark:bg-slate-800"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="min-w-0 truncate text-sm font-extrabold">
                        {item.product_name}
                      </span>
                      <span className="shrink-0 font-mono text-xs font-black">
                        {fmtQty(picked)} · {fmtCents(lineTtcPreview, locale)}
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-x-3 font-mono text-[11px] opacity-80">
                      <span>
                        {t(locale, "returns.colUnitPrice")}:{" "}
                        {fmtCents(tryBigInt(item.unit_price_centimes), locale)}
                      </span>
                      <span>
                        {t(locale, "returns.colPurchased")}:{" "}
                        {fmtQty(tryBigInt(item.original_quantity_micro))}
                      </span>
                      <span>
                        {t(locale, "returns.colReturned")}:{" "}
                        {fmtQty(tryBigInt(item.already_returned_micro))}
                      </span>
                      <span className="font-black">
                        {t(locale, "returns.colRemaining")}: {fmtQty(remaining)}
                      </span>
                    </div>
                    <label className="flex items-center gap-2">
                      <span className="text-xs font-bold text-slate-500">
                        {t(locale, "returns.colReturnQty")}
                      </span>
                      <input
                        value={fmtQty(picked)}
                        onChange={(event) => {
                          s.setItemQuantity(
                            item.source_item_id,
                            parseReturnQtyInput(event.target.value).toString(),
                          );
                        }}
                        inputMode="decimal"
                        autoComplete="off"
                        dir="ltr"
                        disabled={s.isSubmitting || remaining <= 0n}
                        className="h-9 w-28 rounded-md border border-slate-300 bg-white px-2 font-mono text-sm font-black outline-none focus:border-primary-600 disabled:opacity-40 dark:border-slate-700 dark:bg-slate-900"
                      />
                    </label>
                    {picked > remaining && (
                      <p className="text-[11px] font-bold text-red-600">
                        {t(locale, "returns.cannotExceedAvailable")}
                      </p>
                    )}
                  </div>
                );
              })}
            </div>

            <div className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">
                {t(locale, "returns.refundMethod")}
              </span>
              <div className="grid grid-cols-3 gap-2">
                {REFUND_OPTIONS.map((option) => {
                  const off =
                    (option.key === "CUSTOMER_CREDIT" && guest) ||
                    (option.key === "CASH" && !cashOpen);
                  return (
                    <button
                      key={option.key}
                      type="button"
                      disabled={s.isSubmitting || off}
                      title={
                        option.key === "CUSTOMER_CREDIT" && guest
                          ? t(locale, "returns.clientRequired")
                          : option.key === "CASH" && !cashOpen
                            ? t(locale, "returns.cashSessionClosed")
                            : undefined
                      }
                      onClick={() => {
                        s.setRefundMethod(option.key);
                      }}
                      className={`rounded-lg px-2 py-2 text-xs font-black disabled:opacity-40 ${
                        s.refundMethod === option.key
                          ? "bg-primary-600 text-white"
                          : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                      }`}
                    >
                      {t(locale, option.labelKey)}
                    </button>
                  );
                })}
              </div>
              {guest && (
                <p className="text-[11px] font-bold text-slate-500">
                  {t(locale, "returns.clientRequired")}
                </p>
              )}
              {!cashOpen && (
                <p className="text-[11px] font-bold text-slate-500">
                  {t(locale, "returns.cashSessionClosed")}
                </p>
              )}
            </div>

            <input
              value={s.notes}
              onChange={(event) => {
                s.setNotes(event.target.value);
              }}
              placeholder={t(locale, "returns.notesPlaceholder")}
              autoComplete="off"
              disabled={s.isSubmitting}
              className="h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
            />

            <div className="rounded-lg bg-amber-50 px-3 py-2 text-sm dark:bg-amber-900/30">
              <div className="flex justify-between py-0.5 font-bold text-amber-900 dark:text-amber-100">
                <span>{t(locale, "returns.totalHt")}</span>
                <span className="font-mono">{fmtCents(totals.totalHtCents, locale)}</span>
              </div>
              <div className="flex justify-between py-0.5 font-bold text-amber-900 dark:text-amber-100">
                <span>{t(locale, "returns.totalTva")}</span>
                <span className="font-mono">{fmtCents(totals.totalTvaCents, locale)}</span>
              </div>
              <div className="flex justify-between py-0.5 text-base font-black text-amber-900 dark:text-amber-100">
                <span>{t(locale, "returns.totalTtc")}</span>
                <span className="font-mono text-2xl tabular-nums">
                  {fmtCents(totals.totalTtcCents, locale)}
                </span>
              </div>
            </div>

            {s.errorMessage !== null && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {translateBackendError(s.errorMessage, locale)}
              </p>
            )}

            <div className="flex gap-2">
              <button
                type="button"
                onClick={close}
                disabled={s.isSubmitting}
                className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
              >
                {t(locale, "returns.cancel")}
              </button>
              <button
                type="button"
                onClick={() => {
                  void s.submitReturn();
                }}
                disabled={!canSubmit}
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
              >
                {s.isSubmitting
                  ? t(locale, "returns.submitting")
                  : t(locale, "returns.confirmReturn")}
              </button>
            </div>
          </>
        )}

        {s.successResult === null && s.errorMessage !== null && s.context === null && (
          <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
            {translateBackendError(s.errorMessage, locale)}
          </p>
        )}
      </div>
    </ModalShell>
  );
}

function tryBigInt(value: string): bigint {
  try {
    return BigInt(value);
  } catch {
    return 0n;
  }
}
