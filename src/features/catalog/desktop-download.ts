/**
 * Desktop-safe text-file download trigger (TASK-REL-055).
 *
 * Root cause of the rc.6 field defect ("Télécharger le modèle CSV standard"
 * and "Exporter CSV" silently doing nothing in desktop WebView2): both
 * callers built a `blob:` URL, dispatched the anchor click, then called
 * `URL.revokeObjectURL(url)` synchronously in the same task. WebView2 picks
 * up blob downloads asynchronously, so the revocation voids the URL before
 * the download pipeline grabs it — the click lands on a dead URL and
 * nothing happens, with no visible feedback either way.
 *
 * This helper is the single hardened path used by both CSV flows:
 * - the anchor stays attached to the DOM for the click dispatch;
 * - revocation is deferred (`DOWNLOAD_URL_LIFETIME_MS`) so the async
 *   download pickup always wins the race;
 * - the outcome (including the filename) is returned so callers render an
 *   instant inline confirmation — the user always sees what happened.
 *
 * No new dependency: pure DOM APIs only (Tauri `plugin-dialog`/`plugin-fs`
 * are not installed and no IPC save command exists in scope).
 */

export const CSV_MIME_TYPE = "text/csv;charset=utf-8";

export interface DesktopDownloadOutcome {
  /** True once the browser download pickup was triggered. */
  ok: boolean;
  /** Proposed save filename (echoed for the confirmation note). */
  filename: string;
  /** Machine-readable cause: `started` | `no-dom` | `trigger-failed`. */
  reason: string;
}

/**
 * Blob-URL lifetime after the click dispatch. Long enough for WebView2's
 * asynchronous download pickup on low-end hardware, short enough to never
 * leak object URLs across a session.
 */
export const DOWNLOAD_URL_LIFETIME_MS = 30_000;

/**
 * Trigger a client-side download of text content. Guarded no-op outside a
 * browser (SSR / node test harness) reporting `{ ok: false }`.
 */
export function downloadTextFile(
  content: string,
  filename: string,
  mime: string = CSV_MIME_TYPE,
): DesktopDownloadOutcome {
  const failed = (reason: string): DesktopDownloadOutcome => ({
    ok: false,
    filename,
    reason,
  });
  if (
    typeof document === "undefined" ||
    typeof URL === "undefined" ||
    typeof URL.createObjectURL !== "function"
  ) {
    return failed("no-dom");
  }
  try {
    const blob = new Blob([content], { type: mime });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.dispatchEvent(
      new MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    anchor.remove();
    // Deferred release: revoking synchronously (the rc.6 pattern) voids the
    // URL before WebView2's async download pickup and kills the download.
    setTimeout(() => {
      URL.revokeObjectURL(url);
    }, DOWNLOAD_URL_LIFETIME_MS);
    return { ok: true, filename, reason: "started" };
  } catch {
    return failed("trigger-failed");
  }
}
