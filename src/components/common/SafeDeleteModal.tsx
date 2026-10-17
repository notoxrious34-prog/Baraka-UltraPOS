/**
 * Reusable dual-track safe-deletion dialog (TASK-DSN-051, ADR-092).
 *
 * Explains the purge-vs-archive contract, renders an invariant blocked
 * card (non-zero balance, DEP-01, stocked depot) with the confirm action
 * hard-disabled, and reports the backend `DeleteEntityResult` mode back
 * to the caller. Overlay-only `fixed` positioning (allowed for modals);
 * Escape dismissal + Tab focus trap mirror the catalog modal contract.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import type { DeleteEntityResult } from "@/lib/ipc";
import { usePosUiStore } from "@/stores/pos-ui-store";

export interface SafeDeleteModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => Promise<DeleteEntityResult>;
  onSuccess: (result: DeleteEntityResult) => void;
  titleKey: I18nKey;
  entityName: string;
  entityCode?: string;
  isBlocked?: boolean;
  blockedReason?: string | null;
}

export function SafeDeleteModal(props: SafeDeleteModalProps): ReactElement | null {
  const { isOpen, onClose, onConfirm, onSuccess } = props;
  const locale = usePosUiStore((state) => state.locale);
  const cardRef = useRef<HTMLDivElement | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    setBusy(false);
    setError(null);
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        if (!busy) {
          onClose();
        }
        return;
      }
      if (event.key !== "Tab" || cardRef.current === null) {
        return;
      }
      const focusables = cardRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
      );
      if (focusables.length === 0) {
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (first === undefined || last === undefined) {
        return;
      }
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
    };
  }, [isOpen, busy, onClose]);

  if (!isOpen) {
    return null;
  }

  const blocked = props.isBlocked === true;

  const confirm = (): void => {
    if (busy || blocked) {
      return;
    }
    setBusy(true);
    setError(null);
    onConfirm()
      .then((result) => {
        onSuccess(result);
      })
      .catch((failure: unknown) => {
        if (typeof failure === "string") {
          setError(failure);
        } else if (failure instanceof Error && failure.message !== "") {
          setError(failure.message);
        } else {
          setError("INTERNAL_ERROR");
        }
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={t(locale, props.titleKey)}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) {
          onClose();
        }
      }}
    >
      <div
        ref={cardRef}
        className="flex max-h-[90vh] w-full max-w-md flex-col overflow-hidden rounded-xl border border-slate-200 bg-white shadow-2xl dark:border-slate-700 dark:bg-slate-900"
      >
        <div className="flex h-14 shrink-0 items-center gap-2 bg-red-800 px-4 text-white">
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M3 6h18" />
            <path d="M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2" />
            <path d="M19 6 18 20a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
            <path d="M10 11v6" />
            <path d="M14 11v6" />
          </svg>
          <h2 className="text-base font-extrabold">{t(locale, props.titleKey)}</h2>
        </div>
        <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
          <div className="flex flex-wrap items-center gap-2">
            <span className="min-w-0 flex-1 truncate text-sm font-black">{props.entityName}</span>
            {props.entityCode !== undefined && props.entityCode !== "" && (
              <bdi
                dir="ltr"
                className="rounded-md bg-slate-100 px-2 py-0.5 font-mono text-xs font-bold tabular-nums text-slate-600 dark:bg-slate-800 dark:text-slate-300"
              >
                {props.entityCode}
              </bdi>
            )}
          </div>
          <p className="text-xs leading-relaxed text-slate-600 dark:text-slate-300">
            {t(locale, "del.explain")}
          </p>
          {blocked && (
            <div
              role="alert"
              className="rounded-lg border border-amber-300 bg-amber-50 px-3 py-2 dark:border-amber-700 dark:bg-amber-900/30"
            >
              <p className="text-xs font-black text-amber-800 dark:text-amber-200">
                {t(locale, "del.blockedTitle")}
              </p>
              {props.blockedReason !== null &&
                props.blockedReason !== undefined &&
                props.blockedReason !== "" && (
                  <p className="mt-1 text-xs font-bold text-amber-700 dark:text-amber-300">
                    {props.blockedReason}
                  </p>
                )}
            </div>
          )}
          {error !== null && (
            <p className="rounded-md bg-red-50 px-3 py-2 text-xs font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
              {translateBackendError(error, locale)}
            </p>
          )}
        </div>
        <div className="flex shrink-0 gap-2 border-t border-slate-200 p-3 dark:border-slate-700">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-800 dark:hover:bg-slate-700"
          >
            {t(locale, "common.cancel")}
          </button>
          <button
            type="button"
            onClick={confirm}
            disabled={busy || blocked}
            className="h-10 flex-1 rounded-lg bg-[#C62828] text-sm font-black text-white hover:bg-[#B71C1C] focus:outline-none focus:ring-2 focus:ring-red-400 focus:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {busy ? t(locale, "del.deleting") : t(locale, "del.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}
