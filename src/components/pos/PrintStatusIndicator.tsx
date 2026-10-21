/**
 * Print status pill: polls `documents_get_print_status` until a terminal
 * state, with an explicit on-screen reprint action (no F-key binding).
 */

import { useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { t, translateBackendError } from "@/lib/i18n";
import { ipcGetPrintStatus, ipcReprint, type PrintStatus } from "@/lib/ipc";

interface PrintStatusIndicatorProps {
  documentId: string;
}

const TERMINAL = new Set(["DONE", "FAILED"]);

export function PrintStatusIndicator({ documentId }: PrintStatusIndicatorProps): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const session = useSessionStore((state) => state.session);
  const [status, setStatus] = useState<PrintStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reprinting, setReprinting] = useState(false);

  useEffect(() => {
    if (session === null) {
      return;
    }
    const token = session.token;
    let stopped = false;
    let polls = 0;
    const tick = (): void => {
      if (stopped || polls >= 10) {
        return;
      }
      polls += 1;
      ipcGetPrintStatus(token, documentId)
        .then((next) => {
          if (stopped) {
            return;
          }
          setStatus(next);
          if (!TERMINAL.has(next.status)) {
            window.setTimeout(tick, 1000);
          }
        })
        .catch(() => {
          if (!stopped) {
            window.setTimeout(tick, 1000);
          }
        });
    };
    tick();
    return () => {
      stopped = true;
    };
  }, [documentId, session]);

  const reprint = (): void => {
    if (session === null || reprinting) {
      return;
    }
    setReprinting(true);
    setError(null);
    ipcReprint(session.token, documentId)
      .then(() => {
        setStatus({ jobId: "", status: "PENDING", attemptCount: "0" });
      })
      .catch((failure: unknown) => {
        setError(
          translateBackendError(
            typeof failure === "string" ? failure : "INTERNAL_ERROR",
            usePosUiStore.getState().locale,
          ),
        );
      })
      .finally(() => {
        setReprinting(false);
      });
  };

  const label =
    status === null
      ? t(locale, "print.pending")
      : status.status === "DONE"
        ? t(locale, "print.done")
        : status.status === "FAILED"
          ? t(locale, "print.failed")
          : status.status === "PROCESSING"
            ? t(locale, "print.processing")
            : status.status === "NONE"
              ? t(locale, "print.none")
              : t(locale, "print.pending");
  const tone =
    status !== null && (status.status === "DONE" || status.status === "NONE")
      ? "bg-emerald-100 text-emerald-900 dark:bg-emerald-900/40 dark:text-emerald-200"
      : status !== null && status.status === "FAILED"
        ? "bg-red-100 text-red-900 dark:bg-red-900/40 dark:text-red-200"
        : "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200";

  return (
    <div className="flex flex-col gap-1" aria-live="polite">
      <div className="flex items-center justify-between gap-2">
        <span className={`rounded-full px-2 py-1 text-[11px] font-black ${tone}`}>{label}</span>
        <button
          type="button"
          onClick={reprint}
          disabled={reprinting}
          className="rounded-md bg-slate-200 px-2 py-1 text-[11px] font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700"
        >
          {reprinting ? t(locale, "print.reprinting") : t(locale, "print.reprint")}
        </button>
      </div>
      {status !== null && status.status === "FAILED" && (
        <p
          role="alert"
          className="rounded-md bg-red-50 px-2 py-1 text-[11px] font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200"
        >
          {t(locale, "print.failedGuidance")}
        </p>
      )}
      {error !== null && (
        <p className="rounded-md bg-red-50 px-2 py-1 text-[11px] font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {error}
        </p>
      )}
    </div>
  );
}
