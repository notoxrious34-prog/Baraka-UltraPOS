/**
 * Unified document conversion modal (TASK-BRK-043).
 *
 * Two tabs: Devis→BL (logistics: stock deducted, payment-agnostic) and
 * BL→Facture (accounting: stock already deducted, exact tender settlement
 * mandatory). All money renders via formatMoney and computes via BigInt
 * centimes — zero floats. Backend errors surface through
 * translateBackendError (with an INSUFFICIENT_STOCK_FOR:* prefix bridge);
 * the backend remains the final authority.
 */

import { useEffect, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import {
  canSubmitBlToFacture,
  centimesToDisplayUnits,
  computeSettlement,
  isGuestDocument,
  parseTenderCentimes,
  useConversionModalStore,
  type ConversionTab,
  type TenderMethod,
} from "@/stores/conversion-modal-store";
import { t, translateBackendError } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { Locale } from "@/types/pos";

/** Map prefixed stock-shortage codes to a readable FR/AR sentence. */
export function translateConversionError(raw: string, locale: Locale): string {
  if (raw.startsWith("INSUFFICIENT_STOCK_FOR:")) {
    const rest = raw.slice("INSUFFICIENT_STOCK_FOR:".length);
    const [label, shortfall] = rest.split(":");
    const template =
      locale === "ar-DZ"
        ? "مخزون غير كافٍ لـ {label} — ناقص {shortfall}"
        : "Stock insuffisant pour {label} — manque {shortfall}";
    return template
      .replace("{label}", label ?? "?")
      .replace("{shortfall}", shortfall ?? "?");
  }
  return translateBackendError(raw, locale);
}

const TENDER_FIELDS: Array<{ key: TenderMethod; labelKey: Parameters<typeof t>[1] }> = [
  { key: "cash", labelKey: "conversion.method_cash" },
  { key: "card", labelKey: "conversion.method_card" },
  { key: "cheque", labelKey: "conversion.method_cheque" },
  { key: "credit", labelKey: "conversion.method_credit" },
];

function fmtAmount(centimes: bigint, locale: Locale): string {
  const negative = centimes < 0n;
  const abs = negative ? -centimes : centimes;
  try {
    return (negative ? "-" : "") + formatMoney(abs.toString(), locale);
  } catch {
    return "…";
  }
}

export function DocumentConversionModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const s = useConversionModalStore();

  const selected = s.selectedDocument;
  const guestBlocked = selected !== null && isGuestDocument(selected.party_id);
  const settlement =
    selected !== null
      ? computeSettlement(s.tenders, selected.total_ttc_centimes)
      : { allocatedCentimes: 0n, remainingCentimes: 0n };
  const creditEntered = selected !== null && parseTenderCentimes(s.tenders.credit) > 0n;
  const blSubmittable =
    selected !== null &&
    canSubmitBlToFacture({
      remainingCentimes: settlement.remainingCentimes,
      isSubmitting: s.isSubmitting,
      hasSelection: true,
      creditBlocked: guestBlocked && creditEntered,
    });

  // Capture-phase Escape guard: the global keyboard contract closes any
  // modal on Escape — while a conversion dispatch is in flight that would
  // unmount this controller mid-flight, so Escape is suppressed until the
  // dispatch settles (TASK-BRK-032 precedent from CheckoutModal).
  useEffect(() => {
    if (!s.isSubmitting) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape" || event.key === "Enter" || event.key === "F12") {
        event.preventDefault();
        event.stopImmediatePropagation();
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [s.isSubmitting]);

  const close = (): void => {
    if (s.isSubmitting) {
      return;
    }
    s.closeModal();
    closeModal();
  };

  const tabButton = (tab: ConversionTab, labelKey: Parameters<typeof t>[1]): ReactElement => (
    <button
      key={tab}
      type="button"
      disabled={s.isSubmitting}
      onClick={() => {
        s.setActiveTab(tab);
      }}
      className={`flex-1 rounded-lg px-2 py-2 text-xs font-black disabled:opacity-50 ${
        s.activeTab === tab
          ? "bg-primary-600 text-white"
          : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
      }`}
    >
      {t(locale, labelKey)}
    </button>
  );

  return (
    <ModalShell title={t(locale, "conversion.title")} onClose={close}>
      <div className="flex flex-col gap-3">
        <div className="flex gap-2">
          {tabButton("devis_to_bl", "conversion.tab_devis_bl")}
          {tabButton("bl_to_facture", "conversion.tab_bl_facture")}
        </div>

        {s.conversionResult !== null ? (
          <div className="flex flex-col gap-2">
            <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
              {t(locale, "conversion.success")} —{" "}
              <span className="font-mono">{s.conversionResult.newDocumentNumber}</span>
            </p>
            {(s.conversionResult.warnings ?? []).length > 0 && (
              <div className="rounded-md bg-amber-50 px-3 py-2 text-sm font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
                <p className="font-black">{t(locale, "conversion.warnings_title")}</p>
                <ul className="mt-1 flex flex-col gap-0.5">
                  {(s.conversionResult.warnings ?? []).map((warning) => (
                    <li key={warning}>{warning}</li>
                  ))}
                </ul>
              </div>
            )}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={close}
                className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100"
              >
                {t(locale, "conversion.close")}
              </button>
              <button
                type="button"
                onClick={() => {
                  s.clearResult();
                }}
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
              >
                {t(locale, "conversion.convert_another")}
              </button>
            </div>
          </div>
        ) : (
          <>
            <input
              value={s.searchQuery}
              onChange={(event) => {
                s.setSearchQuery(event.target.value);
              }}
              placeholder={t(locale, "conversion.search_placeholder")}
              autoComplete="off"
              disabled={s.isSubmitting}
              className="h-11 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
            />

            {s.isLoadingSearch ? (
              <p className="text-center text-xs font-bold text-slate-500">
                {t(locale, "conversion.searching")}
              </p>
            ) : s.searchResults.length === 0 ? (
              <p className="text-center text-xs font-bold text-slate-500">
                {t(locale, "conversion.no_documents_found")}
              </p>
            ) : (
              <div className="flex max-h-48 flex-col gap-1 overflow-y-auto">
                {s.searchResults.map((doc) => (
                  <button
                    key={doc.id}
                    type="button"
                    disabled={s.isSubmitting}
                    onClick={() => {
                      void s.selectDocument(doc);
                    }}
                    className={`flex items-center justify-between gap-2 rounded-md px-2 py-2 text-start disabled:opacity-50 ${
                      selected?.id === doc.id
                        ? "bg-primary-600 text-white"
                        : "bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700"
                    }`}
                  >
                    <span className="min-w-0">
                      <span className="block truncate font-mono text-sm font-extrabold">
                        {doc.document_number}
                      </span>
                      <span className="block truncate text-[11px] opacity-70">
                        {doc.party_name ?? t(locale, "conversion.cash_customer")} ·{" "}
                        {t(locale, "conversion.items_count")}: {doc.items_count}
                      </span>
                    </span>
                    <span className="shrink-0 font-mono text-xs font-black">
                      {formatMoney(doc.total_ttc_centimes, locale)}
                    </span>
                  </button>
                ))}
              </div>
            )}

            {selected === null ? (
              <p className="text-center text-xs font-bold text-slate-500">
                {t(locale, "conversion.select_prompt")}
              </p>
            ) : (
              <div className="flex flex-col gap-2 rounded-lg border border-slate-200 p-2 dark:border-slate-700">
                <div className="flex items-center justify-between gap-2 text-sm">
                  <span className="font-mono font-extrabold">{selected.document_number}</span>
                  <span className="font-mono font-black">
                    {formatMoney(selected.total_ttc_centimes, locale)}
                  </span>
                </div>
                <p className="truncate text-xs opacity-70">
                  {selected.party_name ?? t(locale, "conversion.cash_customer")}
                </p>

                {s.activeTab === "devis_to_bl" ? (
                  <>
                    <p className="rounded-md bg-sky-50 px-3 py-2 text-xs font-bold text-sky-800 dark:bg-sky-900/30 dark:text-sky-200">
                      {t(locale, "conversion.stock_notice")}
                    </p>
                    <button
                      type="button"
                      disabled={s.isSubmitting}
                      onClick={() => {
                        void s.submitDevisToBl();
                      }}
                      className="h-11 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
                    >
                      {s.isSubmitting
                        ? t(locale, "conversion.submitting")
                        : t(locale, "conversion.convert_to_bl")}
                    </button>
                  </>
                ) : (
                  <>
                    <p className="rounded-md bg-sky-50 px-3 py-2 text-xs font-bold text-sky-800 dark:bg-sky-900/30 dark:text-sky-200">
                      {t(locale, "conversion.payment_notice")}
                    </p>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-bold text-slate-500">
                        {t(locale, "conversion.language_label")}
                      </span>
                      <div className="flex flex-1 gap-1" role="group">
                        {(["FR", "AR"] as const).map((option) => (
                          <button
                            key={option}
                            type="button"
                            disabled={s.isSubmitting}
                            onClick={() => {
                              s.setDocumentLanguage(option);
                            }}
                            className={`flex-1 rounded-md px-2 py-1 font-mono text-xs font-black disabled:opacity-40 ${
                              s.documentLanguage === option
                                ? "bg-primary-600 text-white"
                                : "bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300"
                            }`}
                          >
                            {t(
                              locale,
                              option === "FR" ? "conversion.lang_fr" : "conversion.lang_ar",
                            )}
                          </button>
                        ))}
                      </div>
                    </div>
                    <div className="grid grid-cols-2 gap-2">
                      {TENDER_FIELDS.map((field) => {
                        const creditOff = field.key === "credit" && guestBlocked;
                        return (
                          <label key={field.key} className="flex flex-col gap-1">
                            <span className="text-xs font-bold text-slate-500">
                              {t(locale, field.labelKey)}
                            </span>
                            <input
                              value={s.tenders[field.key]}
                              onChange={(event) => {
                                s.setTender(field.key, event.target.value);
                              }}
                              inputMode="decimal"
                              autoComplete="off"
                              dir="ltr"
                              disabled={s.isSubmitting || creditOff}
                              placeholder={centimesToDisplayUnits(
                                selected.total_ttc_centimes,
                              )}
                              className="h-10 rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm font-black outline-none focus:border-primary-600 disabled:opacity-40 dark:border-slate-700 dark:bg-slate-800"
                            />
                          </label>
                        );
                      })}
                    </div>
                    {guestBlocked && (
                      <p className="text-[11px] font-bold text-slate-500">
                        {t(locale, "conversion.guest_credit_disabled")}
                      </p>
                    )}
                    <div
                      className={`flex items-center justify-between rounded-lg px-3 py-2 text-sm font-black ${
                        settlement.remainingCentimes === 0n
                          ? "bg-emerald-50 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200"
                          : settlement.remainingCentimes > 0n
                            ? "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200"
                            : "bg-amber-50 text-amber-800 dark:bg-amber-900/30 dark:text-amber-200"
                      }`}
                    >
                      <span>
                        {settlement.remainingCentimes === 0n
                          ? t(locale, "conversion.exact_allocated")
                          : settlement.remainingCentimes > 0n
                            ? t(locale, "conversion.remaining_to_allocate")
                            : t(locale, "conversion.excess_allocated")}
                      </span>
                      <span className="font-mono">
                        {settlement.remainingCentimes === 0n
                          ? fmtAmount(settlement.allocatedCentimes, locale)
                          : fmtAmount(
                              settlement.remainingCentimes < 0n
                                ? -settlement.remainingCentimes
                                : settlement.remainingCentimes,
                              locale,
                            )}
                      </span>
                    </div>
                    <button
                      type="button"
                      disabled={!blSubmittable}
                      onClick={() => {
                        void s.submitBlToFacture();
                      }}
                      className="h-11 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
                    >
                      {s.isSubmitting
                        ? t(locale, "conversion.submitting")
                        : t(locale, "conversion.convert_to_facture")}
                    </button>
                  </>
                )}
              </div>
            )}

            {s.error !== null && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {translateConversionError(s.error, locale)}
              </p>
            )}
          </>
        )}
      </div>
    </ModalShell>
  );
}
