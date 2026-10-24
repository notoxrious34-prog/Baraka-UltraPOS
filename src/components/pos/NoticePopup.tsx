/**
 * Transient success/info notice (TASK-DSN-090): small centered popup that
 * replaces the removed full-width green banner. Visible only when no modal
 * is open (never two success messages simultaneously), auto-dismiss ~4s
 * from visibility, manual close, aria-live polite. z-40 sits below modal
 * overlays (z-50); bottom-center placement never overlaps TOTAL/F12.
 */

import { useEffect, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";

const AUTO_DISMISS_MS = 4000;

export function NoticePopup(): ReactElement | null {
  const notice = usePosUiStore((state) => state.notice);
  const dismissNotice = usePosUiStore((state) => state.dismissNotice);
  const activeModal = usePosUiStore((state) => state.activeModal);

  useEffect(() => {
    if (notice === null || activeModal !== "none") {
      return;
    }
    const timer = window.setTimeout(() => {
      dismissNotice();
    }, AUTO_DISMISS_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [notice, activeModal, dismissNotice]);

  if (notice === null || activeModal !== "none") {
    return null;
  }

  return (
    <div className="pointer-events-none fixed inset-x-0 top-24 z-40 flex justify-center px-4">
      <div
        role="status"
        aria-live="polite"
        className="pointer-events-auto flex max-w-md items-center gap-3 rounded-xl bg-slate-900 px-4 py-3 text-white shadow-2xl dark:bg-white dark:text-slate-950"
      >
        <span
          aria-hidden="true"
          className="inline-block h-2.5 w-2.5 shrink-0 rounded-full bg-emerald-400"
        />
        <div className="min-w-0">
          <p className="text-sm font-black">{notice.title}</p>
          {notice.detail !== undefined && (
            <p className="mt-0.5 truncate text-xs font-bold text-white/80 dark:text-slate-700">
              <bdi dir="ltr">{notice.detail}</bdi>
            </p>
          )}
        </div>
        <button
          type="button"
          onClick={dismissNotice}
          aria-label="Close"
          className="shrink-0 rounded-md bg-white/10 px-2 py-1 text-xs font-black hover:bg-white/20 dark:bg-slate-950/10 dark:hover:bg-slate-950/20"
        >
          ×
        </button>
      </div>
    </div>
  );
}
