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
  ipcReprint,
  type DocumentDetailsWireResponse,
  type DocumentSummaryWireDto,
} from "@/lib/ipc";
import { formatMoney, formatQty } from "@/lib/money";
import { PrintStatusIndicator } from "@/components/pos/PrintStatusIndicator";
import type { Locale } from "@/types/pos";
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
                    <bdi dir="ltr" className="block font-mono">
                      {item.product_id}
                    </bdi>
                    <bdi dir="ltr" className="block font-mono text-[10px] text-slate-500">
                      {item.unit_id}
                    </bdi>
                  </td>
                  <td className="py-1 pe-2">
                    <bdi dir="ltr" className="font-mono tabular-nums">
                      {fmtQty(item.quantity_micro)}
                    </bdi>
                  </td>
                  <td className="py-1 pe-2">
                    <bdi dir="ltr" className="font-mono tabular-nums">
                      {fmtAmount(item.unit_price_centimes, locale)}
                    </bdi>
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

  useEffect(() => {
    setReprintState("idle");
    setReprintError(null);
    setDetails(null);
    setDetailsError(null);
    setDetailsLoading(true);
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
    return () => {
      cancelled = true;
    };
  }, [document.id, detailsTick]);

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

      <button
        type="button"
        onClick={reprint}
        disabled={reprintState === "sending"}
        className="h-11 shrink-0 rounded-lg bg-accent-500 text-sm font-black text-slate-950 disabled:opacity-50"
      >
        {reprintState === "sending" ? t(locale, "documents.reprinting") : t(locale, "documents.reprint")}
      </button>
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
