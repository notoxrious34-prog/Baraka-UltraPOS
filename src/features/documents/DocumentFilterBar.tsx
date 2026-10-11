/**
 * Documents filter bar (TASK-DSN-028): live search input, type pills,
 * status selector, and manual refresh. All state is owned by the parent
 * module; this bar is fully controlled.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import { DOCUMENT_TYPES, DOCUMENT_STATUSES, documentTypeKey, documentStatusKey } from "@/features/documents/document-helpers";
import type { DocumentStatusFilter, DocumentTypeFilter } from "@/features/documents/document-helpers";

interface DocumentFilterBarProps {
  locale: Locale;
  query: string;
  onQuery: (query: string) => void;
  docType: DocumentTypeFilter;
  onType: (docType: DocumentTypeFilter) => void;
  status: DocumentStatusFilter;
  onStatus: (status: DocumentStatusFilter) => void;
  onRefresh: () => void;
  loading: boolean;
}

export function DocumentFilterBar(props: DocumentFilterBarProps): ReactElement {
  const { locale } = props;
  return (
    <div className="flex shrink-0 flex-col gap-2 border-b border-slate-200 p-3 dark:border-slate-800">
      <div className="flex gap-2">
        <input
          value={props.query}
          onChange={(event) => {
            props.onQuery(event.target.value);
          }}
          placeholder={t(locale, "documents.searchPlaceholder")}
          autoComplete="off"
          className="h-10 min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 dark:border-slate-700 dark:bg-slate-800"
        />
        <button
          type="button"
          onClick={props.onRefresh}
          disabled={props.loading}
          className="h-10 shrink-0 rounded-lg bg-slate-200 px-4 text-sm font-bold disabled:opacity-50 dark:bg-slate-700"
        >
          {t(locale, "documents.refresh")}
        </button>
      </div>
      <div className="flex flex-wrap items-center gap-1">
        <button
          type="button"
          onClick={() => {
            props.onType("ALL");
          }}
          className={`rounded-full px-3 py-1 text-xs font-bold ${
            props.docType === "ALL"
              ? "bg-primary-900 text-white dark:bg-primary-700"
              : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200"
          }`}
        >
          {t(locale, "documents.typeAll")}
        </button>
        {DOCUMENT_TYPES.map((kind) => (
          <button
            key={kind}
            type="button"
            onClick={() => {
              props.onType(kind);
            }}
            className={`rounded-full px-3 py-1 text-xs font-bold ${
              props.docType === kind
                ? "bg-primary-900 text-white dark:bg-primary-700"
                : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-200"
            }`}
          >
            {t(locale, documentTypeKey(kind))}
          </button>
        ))}
        <select
          value={props.status}
          onChange={(event) => {
            props.onStatus(event.target.value as DocumentStatusFilter);
          }}
          className="ms-2 h-8 rounded-lg border border-slate-300 bg-white px-2 text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
          aria-label={t(locale, "documents.colStatus")}
        >
          <option value="ALL">{t(locale, "documents.statusAll")}</option>
          {DOCUMENT_STATUSES.map((state) => (
            <option key={state} value={state}>
              {t(locale, documentStatusKey(state))}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
