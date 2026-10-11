/**
 * Dense documents table (TASK-DSN-028): N° / type / date / client /
 * articles / status / total / actions. Owns the inner scroll region
 * (`min-h-0 flex-1 overflow-auto`); the page itself never scrolls.
 * Amounts are canonical centime strings via formatMoney + <bdi dir="ltr">.
 */

import type { ReactElement } from "react";
import { t } from "@/lib/i18n";
import { formatMoney } from "@/lib/money";
import type { Locale } from "@/types/pos";
import type { DocumentSummaryWireDto } from "@/lib/ipc";
import {
  documentStatusBadge,
  documentStatusKey,
  documentTypeBadge,
  documentTypeKey,
} from "@/features/documents/document-helpers";

interface DocumentTableProps {
  locale: Locale;
  rows: DocumentSummaryWireDto[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  loading: boolean;
}

function fmtAmount(centimes: string, locale: Locale): string {
  try {
    return formatMoney(centimes, locale);
  } catch {
    return "…";
  }
}

function fmtDate(iso: string): string {
  const time = Date.parse(iso);
  if (Number.isNaN(time)) {
    return iso;
  }
  const date = new Date(time);
  const pad = (part: number): string => String(part).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function DocumentTable(props: DocumentTableProps): ReactElement {
  const { locale } = props;
  if (props.loading) {
    return (
      <p className="p-6 text-center text-xs font-bold text-slate-500">
        {t(locale, "documents.searching")}
      </p>
    );
  }
  if (props.rows.length === 0) {
    return (
      <p className="p-6 text-center text-xs font-bold text-slate-500">
        {t(locale, "documents.noResults")}
      </p>
    );
  }
  return (
    <div className="min-h-0 flex-1 overflow-auto">
      <table className="w-full border-collapse text-xs">
        <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
          <tr className="text-start text-slate-500">
            <th className="px-2 py-2 text-start font-bold">{t(locale, "documents.colNumber")}</th>
            <th className="px-2 py-2 text-start font-bold">{t(locale, "documents.colType")}</th>
            <th className="px-2 py-2 text-start font-bold">{t(locale, "documents.colDate")}</th>
            <th className="px-2 py-2 text-start font-bold">{t(locale, "documents.colCustomer")}</th>
            <th className="px-2 py-2 text-end font-bold">{t(locale, "documents.colItems")}</th>
            <th className="px-2 py-2 text-start font-bold">{t(locale, "documents.colStatus")}</th>
            <th className="px-2 py-2 text-end font-bold">{t(locale, "documents.colTotal")}</th>
            <th className="px-2 py-2 text-end font-bold">{t(locale, "documents.colActions")}</th>
          </tr>
        </thead>
        <tbody>
          {props.rows.map((row) => {
            const selected = row.id === props.selectedId;
            return (
              <tr
                key={row.id}
                onClick={() => {
                  props.onSelect(row.id);
                }}
                className={`cursor-pointer border-b border-slate-100 dark:border-slate-800 ${
                  selected ? "bg-accent-500/10" : "hover:bg-slate-50 dark:hover:bg-slate-800/60"
                }`}
              >
                <td className="whitespace-nowrap px-2 py-2 font-mono font-bold">
                  <bdi dir="ltr">{row.document_number}</bdi>
                </td>
                <td className="whitespace-nowrap px-2 py-2">
                  <span
                    className={`rounded-full px-2 py-0.5 text-[10px] font-black ${documentTypeBadge(row.document_type)}`}
                  >
                    {t(locale, documentTypeKey(row.document_type))}
                  </span>
                </td>
                <td className="whitespace-nowrap px-2 py-2 text-slate-500">
                  <bdi dir="ltr">{fmtDate(row.created_at)}</bdi>
                </td>
                <td className="max-w-44 truncate px-2 py-2">
                  {row.party_name ?? t(locale, "documents.walkIn")}
                </td>
                <td className="px-2 py-2 text-end">
                  <bdi dir="ltr" className="font-mono tabular-nums">
                    {row.items_count}
                  </bdi>
                </td>
                <td className="whitespace-nowrap px-2 py-2">
                  <span
                    className={`rounded-full px-2 py-0.5 text-[10px] font-black ${documentStatusBadge(row.status)}`}
                  >
                    {t(locale, documentStatusKey(row.status))}
                  </span>
                </td>
                <td className="whitespace-nowrap px-2 py-2 text-end">
                  <bdi dir="ltr" className="font-mono font-bold tabular-nums">
                    {fmtAmount(row.total_ttc_centimes, locale)}
                  </bdi>
                </td>
                <td className="whitespace-nowrap px-2 py-2 text-end">
                  <button
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      props.onSelect(row.id);
                    }}
                    className="rounded-md bg-primary-900 px-2.5 py-1 text-[11px] font-bold text-white hover:bg-primary-800 dark:bg-primary-700"
                  >
                    {t(locale, "documents.details")}
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
