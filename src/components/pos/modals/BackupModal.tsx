/**
 * Backup cockpit modal (TASK-BRK-053): manual snapshots, audit history,
 * and on-disk verification for administrators.
 *
 * TASK-BRK-054 adds the verified restore flow: per-row restore action
 * (ADMIN-only rendering; the Rust SystemSettingsManage gate stays
 * authoritative), a read-only precheck report, an exact-RESTORE
 * confirmation gate, and non-dismissable restoring/success phases.
 *
 * Industrial Trust styling, full keyboard access (Esc closes outside the
 * restore locked phases, capture guard while busy), bilingual FR/AR
 * throughout. Sizes render via formatBytes, timestamps via the locale
 * date formatter. Backend errors surface through translateBackendError;
 * restore errors use the dedicated restore keys; the backend is final.
 */

import { useEffect, type ReactElement } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import {
  canRestoreForRole,
  describeIneligibleReason,
  describeRestoreWarning,
  formatBytes,
  isRestoreExecutable,
  isRestorePhaseDismissable,
  restoreErrorI18nKey,
  useBackupModalStore,
  type RestorePhase,
} from "@/stores/backup-modal-store";
import { t, translateBackendError } from "@/lib/i18n";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { Locale } from "@/types/pos";
import type { PreRestoreReport, RestoreOutcome } from "@/lib/ipc";

/** Close the application window (restore success terminal action). */
export async function closeApplication(): Promise<void> {
  await getCurrentWindow().close();
}

function formatStamp(iso: string, locale: Locale): string {
  const time = Date.parse(iso);
  if (Number.isNaN(time)) {
    return iso;
  }
  try {
    return new Intl.DateTimeFormat(locale === "ar-DZ" ? "ar-DZ" : "fr-DZ", {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(time));
  } catch {
    return iso;
  }
}

function statusTone(status: string): string {
  switch (status) {
    case "SUCCESS":
      return "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200";
    case "FAILED":
      return "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-200";
    case "CORRUPTED":
      return "bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-200";
    default:
      return "bg-slate-200 text-slate-600 dark:bg-slate-700 dark:text-slate-300";
  }
}

function statusKey(status: string): Parameters<typeof t>[1] {
  switch (status) {
    case "SUCCESS":
      return "backups.statusSuccess";
    case "FAILED":
      return "backups.statusFailed";
    case "CORRUPTED":
      return "backups.statusCorrupted";
    default:
      return "backups.statusPruned";
  }
}

function triggerKey(trigger: string): Parameters<typeof t>[1] {
  switch (trigger) {
    case "SHIFT_CLOSE":
      return "backups.triggerShiftClose";
    case "SCHEDULED":
      return "backups.triggerScheduled";
    case "RESET":
      return "backups.triggerReset";
    default:
      return "backups.triggerManual";
  }
}

function CheckRow({ label, passed }: { label: string; passed: boolean }): ReactElement {
  return (
    <li className="flex items-center gap-2 text-xs font-bold">
      <span
        aria-hidden="true"
        className={
          passed
            ? "text-emerald-700 dark:text-emerald-300"
            : "text-red-600 dark:text-red-300"
        }
      >
        {passed ? "✓" : "✗"}
      </span>
      <span>
        {passed ? label : `${label} — ✗`}
      </span>
      <span className="sr-only">{passed ? "pass" : "fail"}</span>
    </li>
  );
}

/** Props for the presentational restore panel (store-free for testability). */
export interface RestoreReportPanelProps {
  locale: Locale;
  phase: RestorePhase;
  report: PreRestoreReport | null;
  confirmationInput: string;
  outcome: RestoreOutcome | null;
  restoreErrorCode: string | null;
  onBack: () => void;
  onConfirmationInput: (value: string) => void;
  onExecute: (backupId: string) => void;
  onCloseApp: () => void;
}

/** Per-row restore action button (ADMIN-gated by the caller). */
export function RestoreRowButton({
  backupId,
  disabled,
  label,
  onStartPrecheck,
}: {
  backupId: string;
  disabled: boolean;
  label: string;
  onStartPrecheck: (backupId: string) => void;
}): ReactElement {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => {
        onStartPrecheck(backupId);
      }}
      aria-label={label}
      className="rounded-md bg-amber-600 px-2 py-1 text-[11px] font-black text-white disabled:opacity-40"
    >
      <span aria-hidden="true">↺ </span>
      {label}
    </button>
  );
}

export function RestoreReportPanel({
  locale,
  phase,
  report,
  confirmationInput,
  outcome,
  restoreErrorCode,
  onBack,
  onConfirmationInput,
  onExecute,
  onCloseApp,
}: RestoreReportPanelProps): ReactElement {

  if (phase === "precheck" || (phase === "report" && report === null)) {
    return (
      <p className="flex items-center justify-center gap-2 rounded-lg bg-slate-100 px-3 py-8 text-center text-sm font-black text-slate-600 dark:bg-slate-800 dark:text-slate-200">
        <span
          className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-slate-500 border-t-transparent"
          aria-hidden="true"
        />
        {t(locale, "backups.restore.precheckLoading")}
      </p>
    );
  }

  if (phase === "restoring") {
    return (
      <p className="flex items-center justify-center gap-2 rounded-lg bg-amber-50 px-3 py-8 text-center text-sm font-black text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
        <span
          className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-amber-600 border-t-transparent"
          aria-hidden="true"
        />
        {t(locale, "backups.restore.restoring")}
      </p>
    );
  }

  if (phase === "success") {
    return (
      <div className="flex flex-col gap-2 rounded-lg border-2 border-emerald-500 bg-emerald-50 px-3 py-4 dark:bg-emerald-900/20">
        <p className="text-sm font-black text-emerald-800 dark:text-emerald-200">
          ✓ {t(locale, "backups.restore.success.title")}
        </p>
        <p className="text-xs font-bold text-emerald-700 dark:text-emerald-300">
          {t(locale, "backups.restore.success.body")}
        </p>
        {outcome !== null && (
          <p className="text-xs font-bold text-emerald-700 dark:text-emerald-300">
            {t(locale, "backups.restore.success.quarantinedTo")}{" "}
            <span className="font-mono">{outcome.quarantinedTo}</span>
          </p>
        )}
        <button
          type="button"
          onClick={onCloseApp}
          className="h-11 rounded-lg bg-emerald-600 text-sm font-black text-white hover:bg-emerald-500"
        >
          {t(locale, "backups.restore.success.closeApp")}
        </button>
      </div>
    );
  }

  if (phase === "restore-error") {
    return (
      <div className="flex flex-col gap-2">
        <div className="rounded-lg border-2 border-red-500 bg-red-50 px-3 py-3 dark:bg-red-900/20">
          <p className="text-sm font-black text-red-800 dark:text-red-200">
            ✗ {t(locale, "backups.restore.error.title")}
          </p>
          <p className="mt-1 text-xs font-bold text-red-700 dark:text-red-300">
            {t(locale, restoreErrorI18nKey(restoreErrorCode ?? ""))}
          </p>
          <p className="mt-1 text-xs font-bold text-red-700 dark:text-red-300">
            {t(locale, "backups.restore.error.untouched")}
          </p>
        </div>
        <button
          type="button"
          onClick={onBack}
          className="h-11 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100"
        >
          {t(locale, "backups.restore.back")}
        </button>
      </div>
    );
  }

  // phase === "report" with a loaded report.
  if (report === null) {
    return (
      <p className="rounded-lg bg-slate-100 px-3 py-6 text-center text-sm font-bold text-slate-500 dark:bg-slate-800">
        {t(locale, "backups.restore.precheckLoading")}
      </p>
    );
  }
  const executable = isRestoreExecutable(report, confirmationInput);
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <p className="text-sm font-black">{t(locale, "backups.restore.reportTitle")}</p>
        <button
          type="button"
          onClick={onBack}
          className="rounded-md bg-slate-200 px-2 py-1 text-[11px] font-black hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100"
        >
          ← {t(locale, "backups.restore.back")}
        </button>
      </div>

      <div className="rounded-md bg-slate-100 px-2 py-2 dark:bg-slate-800">
        <p className="block truncate font-mono text-xs font-extrabold">{report.fileName}</p>
        <p className="block text-[11px] opacity-70">
          {formatStamp(report.createdAt, locale)} · {formatBytes(report.fileSizeBytes)} · v
          {report.schemaVersionBackup} → v{report.schemaVersionApp}
        </p>
      </div>

      <div className="rounded-md bg-slate-100 px-2 py-2 dark:bg-slate-800">
        <p className="text-[11px] font-black uppercase opacity-60">
          {t(locale, "backups.restore.summary.title")}
        </p>
        <p className="text-xs font-bold">
          {t(locale, "backups.restore.summary.products")}: {report.contentSummary.products} ·{" "}
          {t(locale, "backups.restore.summary.customers")}: {report.contentSummary.customers} ·{" "}
          {t(locale, "backups.restore.summary.documents")}: {report.contentSummary.documentsTotal}
        </p>
        {report.contentSummary.lastDocumentAt !== null && (
          <p className="text-[11px] opacity-70">
            {t(locale, "backups.restore.summary.lastDocument")}:{" "}
            {formatStamp(report.contentSummary.lastDocumentAt, locale)}
          </p>
        )}
      </div>

      <ul className="flex flex-col gap-1 rounded-md bg-slate-100 px-2 py-2 dark:bg-slate-800">
        <CheckRow label={t(locale, "backups.restore.checks.fileExists")} passed={report.checks.fileExists} />
        <CheckRow label={t(locale, "backups.restore.checks.sha256")} passed={report.checks.sha256Matches} />
        <CheckRow label={t(locale, "backups.restore.checks.integrity")} passed={report.checks.integrityOk} />
        <CheckRow label={t(locale, "backups.restore.checks.foreignKeys")} passed={report.checks.foreignKeysOk} />
        <CheckRow label={t(locale, "backups.restore.checks.auditChain")} passed={report.checks.auditChainOk} />
        <CheckRow
          label={t(locale, "backups.restore.checks.customerLedger")}
          passed={report.checks.customerLedgerOk}
        />
        <CheckRow
          label={t(locale, "backups.restore.checks.supplierLedger")}
          passed={report.checks.supplierLedgerOk}
        />
        <CheckRow label={t(locale, "backups.restore.checks.stock")} passed={report.checks.stockLevelsOk} />
      </ul>

      {report.warnings.length > 0 && (
        <ul className="flex flex-col gap-1 rounded-md bg-amber-50 px-2 py-2 dark:bg-amber-900/20">
          {report.warnings.map((warning, index) => (
            <li
              key={`${warning.type}-${index}`}
              className="flex items-start gap-2 text-xs font-bold text-amber-800 dark:text-amber-200"
            >
              <span aria-hidden="true">⚠</span>
              <span>{describeRestoreWarning(warning, locale)}</span>
            </li>
          ))}
        </ul>
      )}

      {!report.eligible && (
        <div className="rounded-md border-2 border-red-500 bg-red-50 px-2 py-2 dark:bg-red-900/20">
          <p className="text-xs font-black text-red-800 dark:text-red-200">
            ✗ {t(locale, "backups.restore.ineligible.title")}
          </p>
          <ul className="mt-1 flex flex-col gap-1">
            {report.ineligibleReasons.map((reason, index) => (
              <li key={`${reason.type}-${index}`} className="text-xs font-bold text-red-700 dark:text-red-300">
                · {describeIneligibleReason(reason, locale)}
              </li>
            ))}
          </ul>
        </div>
      )}

      {report.eligible && (
        <div className="flex flex-col gap-2 rounded-md border-2 border-red-500 px-2 py-2">
          <p className="text-xs font-black text-red-800 dark:text-red-200">
            {t(locale, "backups.restore.confirm.title")}
          </p>
          <p className="text-xs font-bold opacity-80">{t(locale, "backups.restore.confirm.body")}</p>
          <input
            type="text"
            value={confirmationInput}
            onChange={(event) => {
              onConfirmationInput(event.target.value);
            }}
            placeholder={t(locale, "backups.restore.confirm.placeholder")}
            aria-label={t(locale, "backups.restore.confirm.placeholder")}
            className="h-11 rounded-lg border border-slate-300 bg-white px-3 text-sm font-bold dark:border-slate-600 dark:bg-slate-800"
          />
          <button
            type="button"
            disabled={!executable}
            onClick={() => {
              onExecute(report.backupId);
            }}
            className="h-11 rounded-lg bg-red-600 text-sm font-black text-white hover:bg-red-500 disabled:opacity-40"
          >
            {t(locale, "backups.restore.confirm.execute")}
          </button>
        </div>
      )}
    </div>
  );
}

function RestoreView({ locale, phase }: { locale: Locale; phase: RestorePhase }): ReactElement {
  const s = useBackupModalStore();
  return (
    <RestoreReportPanel
      locale={locale}
      phase={phase}
      report={s.restoreReport}
      confirmationInput={s.confirmationInput}
      outcome={s.restoreOutcome}
      restoreErrorCode={s.restoreError}
      onBack={() => {
        s.resetToIdle();
      }}
      onConfirmationInput={(value) => {
        s.setConfirmationInput(value);
      }}
      onExecute={(backupId) => {
        void s.executeRestore(backupId);
      }}
      onCloseApp={() => {
        void closeApplication();
      }}
    />
  );
}

export function BackupModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const sessionRole = useSessionStore((state) => state.session?.role);
  const s = useBackupModalStore();
  const busy = s.isLoading || s.isCreating;
  const canRestore = canRestoreForRole(sessionRole);
  const dismissable = !s.isCreating && isRestorePhaseDismissable(s.restorePhase);
  const inRestoreFlow = s.restorePhase !== "idle";

  // Capture-phase Escape guard while creating or inside a restore locked
  // phase (precheck, restoring, success): dismissal is blocked there.
  // CheckoutModal precedent for the capture guard.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (s.isCreating || !isRestorePhaseDismissable(s.restorePhase)) {
        if (event.key === "Escape" || event.key === "Enter" || event.key === "F12") {
          event.preventDefault();
          event.stopImmediatePropagation();
        }
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        s.closeModal();
        closeModal();
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [s.isCreating, s.restorePhase, s, closeModal]);

  const close = (): void => {
    if (!dismissable) {
      return;
    }
    s.closeModal();
    closeModal();
  };

  return (
    <ModalShell title={t(locale, "backups.title")} onClose={close}>
      <div className="flex max-h-[70vh] flex-col gap-3 overflow-y-auto">
        <p className="text-xs font-semibold text-slate-500">{t(locale, "backups.subtitle")}</p>

        {inRestoreFlow ? (
          <RestoreView locale={locale} phase={s.restorePhase} />
        ) : (
          <>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => {
                  void s.triggerManualBackup();
                }}
                disabled={busy}
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
              >
                {s.isCreating ? (
                  <span className="flex items-center justify-center gap-2">
                    <span
                      className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-slate-900 border-t-transparent"
                      aria-hidden="true"
                    />
                    {t(locale, "backups.creating")}
                  </span>
                ) : (
                  `💾 ${t(locale, "backups.createNow")}`
                )}
              </button>
              <button
                type="button"
                onClick={() => {
                  void s.fetchBackups();
                }}
                disabled={busy}
                title={t(locale, "backups.refresh")}
                aria-label={t(locale, "backups.refresh")}
                className="h-11 shrink-0 rounded-lg bg-slate-200 px-3 text-base font-black hover:bg-slate-300 disabled:opacity-40 dark:bg-slate-700 dark:text-slate-100"
              >
                <span aria-hidden="true">↻</span>
              </button>
            </div>

            {s.successMessage !== null && (
              <p className="rounded-lg bg-emerald-50 px-3 py-2 text-center text-sm font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
                {t(locale, "backups.successCreated").replace("{file}", s.successMessage)}
              </p>
            )}
            {s.errorMessage !== null && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {translateBackendError(s.errorMessage, locale)}
              </p>
            )}

            {s.isLoading ? (
              <p className="text-center text-xs font-bold text-slate-500">
                {t(locale, "backups.loading")}
              </p>
            ) : s.backups.length === 0 ? (
              <p className="rounded-lg bg-slate-100 px-3 py-6 text-center text-sm font-bold text-slate-500 dark:bg-slate-800">
                {t(locale, "backups.empty")}
              </p>
            ) : (
              <div className="flex flex-col gap-1 overflow-y-auto">
                <div className="grid grid-cols-[1fr_auto] gap-2 px-2 text-[10px] font-black uppercase text-slate-400">
                  <span>{t(locale, "backups.colFile")}</span>
                  <span>{t(locale, "backups.colActions")}</span>
                </div>
                {s.backups.map((item) => {
                  const verdict = s.verificationResults[item.id];
                  const verifying = s.verifyingId === item.id;
                  return (
                    <div
                      key={item.id}
                      className="flex flex-col gap-1 rounded-md bg-slate-100 px-2 py-2 dark:bg-slate-800"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="min-w-0">
                          <span className="block truncate font-mono text-xs font-extrabold">
                            {item.fileName}
                          </span>
                          <span className="block text-[11px] opacity-70">
                            {formatStamp(item.createdAt, locale)} · {formatBytes(item.fileSizeBytes)} ·{" "}
                            {t(locale, triggerKey(item.triggerType))}
                          </span>
                        </span>
                        <span
                          className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-black ${statusTone(item.status)}`}
                        >
                          {t(locale, statusKey(item.status))}
                        </span>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          type="button"
                          disabled={verifying || s.isCreating}
                          onClick={() => {
                            void s.verifyBackupIntegrity(item.id);
                          }}
                          className="rounded-md bg-primary-600 px-2 py-1 text-[11px] font-black text-white disabled:opacity-40"
                        >
                          {verifying ? t(locale, "backups.verifying") : t(locale, "backups.verify")}
                        </button>
                        {canRestore && (
                          <RestoreRowButton
                            backupId={item.id}
                            disabled={verifying || s.isCreating}
                            label={t(locale, "backups.restore.action")}
                            onStartPrecheck={(backupId) => {
                              void s.startPrecheck(backupId);
                            }}
                          />
                        )}
                        {verdict !== undefined &&
                          (verdict.isValid ? (
                            <span className="text-[11px] font-black text-emerald-700 dark:text-emerald-300">
                              ✓ {t(locale, "backups.verifiedOk")}
                            </span>
                          ) : (
                            <span className="text-[11px] font-black text-red-600">
                              ⚠ {t(locale, "backups.verifiedFail")}
                            </span>
                          ))}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            <p className="text-[11px] font-semibold text-slate-500">
              {t(locale, "backups.retentionNotice")}
            </p>
          </>
        )}

        <button
          type="button"
          onClick={close}
          disabled={!dismissable}
          className="h-11 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
        >
          {t(locale, "backups.close")}
        </button>
      </div>
    </ModalShell>
  );
}
