/**
 * Document detail side pane (TASK-DSN-028): number + badges, metadata and
 * financial cards, and the DUPLICATA reprint action via the existing
 * ipcReprint binding (documents_reprint). Escape closes; caller owns
 * visibility and focus return.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { ipcReprint, type DocumentSummaryWireDto } from "@/lib/ipc";
import { formatMoney } from "@/lib/money";
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

export function DocumentDetailDrawer(props: DocumentDetailDrawerProps): ReactElement {
  const { locale, document } = props;
  const [reprintState, setReprintState] = useState<"idle" | "sending" | "ok" | "fail">("idle");
  const [reprintError, setReprintError] = useState<string | null>(null);

  useEffect(() => {
    setReprintState("idle");
    setReprintError(null);
  }, [document.id]);

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

      <div className="flex flex-col gap-1 rounded-lg border border-slate-200 p-3 dark:border-slate-700">
        <span className="text-[11px] font-bold text-slate-500">{t(locale, "documents.total")}</span>
        <bdi dir="ltr" className="font-mono text-xl font-black tabular-nums">
          {fmtAmount(document.total_ttc_centimes, locale)}
        </bdi>
      </div>

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
