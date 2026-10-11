/**
 * Documents & Billing module root (TASK-DSN-028): filter bar, dense table,
 * and detail side pane bound to searchDocuments. Manager-only with
 * defense-in-depth role gating (same rule as App.tsx). Queries are
 * debounced client-side (250ms); type/status narrow server-side via
 * the searchDocuments wire params.
 */

import { useEffect, useState, type ReactElement } from "react";
import { translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { searchDocuments, type DocumentSummaryWireDto } from "@/lib/ipc";
import { DocumentFilterBar } from "@/features/documents/DocumentFilterBar";
import { DocumentTable } from "@/features/documents/DocumentTable";
import { DocumentDetailDrawer } from "@/features/documents/DocumentDetailDrawer";
import type { DocumentStatusFilter, DocumentTypeFilter } from "@/features/documents/document-helpers";

/** Same manager rule as App.tsx isManagerRole (ADMIN or GERANT). */
function isManagerRole(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

const SEARCH_LIMIT = 100;

export function DocumentsModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [docType, setDocType] = useState<DocumentTypeFilter>("ALL");
  const [status, setStatus] = useState<DocumentStatusFilter>("ALL");
  const [rows, setRows] = useState<DocumentSummaryWireDto[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [refreshTick, setRefreshTick] = useState(0);

  const authorized = session !== null && isManagerRole(session.role);
  const token = authorized && session !== null ? session.token : null;

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedQuery(query.trim());
    }, 250);
    return () => {
      window.clearTimeout(timer);
    };
  }, [query]);

  useEffect(() => {
    if (token === null) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    setListError(null);
    searchDocuments({
      session_token: token,
      document_type: docType === "ALL" ? null : docType,
      status: status === "ALL" ? null : status,
      query: debouncedQuery === "" ? null : debouncedQuery,
      limit: SEARCH_LIMIT,
    })
      .then((response) => {
        if (!cancelled) {
          setRows(response.documents);
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setListError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
          setRows([]);
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, docType, status, debouncedQuery, refreshTick]);

  if (!authorized || token === null) {
    return (
      <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError("UNAUTHORIZED", locale)}
        </p>
      </section>
    );
  }

  const selected = rows.find((row) => row.id === selectedId) ?? null;

  return (
    <div className="flex h-full flex-col overflow-hidden bg-slate-50 dark:bg-slate-900">
      <DocumentFilterBar
        locale={locale}
        query={query}
        onQuery={setQuery}
        docType={docType}
        onType={(next) => {
          setDocType(next);
          setSelectedId(null);
        }}
        status={status}
        onStatus={(next) => {
          setStatus(next);
          setSelectedId(null);
        }}
        onRefresh={() => {
          setRefreshTick((tick) => tick + 1);
        }}
        loading={loading}
      />
      {listError !== null && (
        <p className="shrink-0 px-4 pt-2 text-xs font-bold text-red-600">
          {translateBackendError(listError, locale)}
        </p>
      )}
      <div className="flex min-h-0 flex-1 overflow-hidden bg-white dark:bg-slate-950">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col">
          <DocumentTable
            locale={locale}
            rows={rows}
            selectedId={selectedId}
            onSelect={setSelectedId}
            loading={loading}
          />
        </div>
        {selected !== null && (
          <DocumentDetailDrawer
            locale={locale}
            token={token}
            document={selected}
            onClose={() => {
              setSelectedId(null);
            }}
          />
        )}
      </div>
    </div>
  );
}
