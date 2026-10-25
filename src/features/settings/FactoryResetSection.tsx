/**
 * Factory-reset danger-zone section (TASK-DSN-094, engine TASK-BRK-091).
 *
 * Lives at the bottom of the backups tab: precheck display (open-session
 * block, backup state + verified-backup entry point reusing the `createBackup`
 * flow), typed-token double-confirm (SafeDeleteModal convention — expected
 * token derived from the store-identity wire, precheck deliberately omits
 * it), execute → progress → restart-required notice with a post-success
 * state lock. Admin-only rendering is convenience-only: the engine
 * re-validates session, permission, token, and preconditions.
 */

import { useCallback, useEffect, useState, type ReactElement } from "react";
import { t, translateBackendError, type I18nKey } from "@/lib/i18n";
import {
  createBackup,
  executeFactoryReset,
  getFactoryResetPrecheck,
  getStoreIdentity,
  type FactoryResetOutcome,
  type FactoryResetPrecheck,
} from "@/lib/ipc";
import type { Locale } from "@/types/pos";
import { canExecuteReset, expectedResetToken } from "./factory-reset";

interface FactoryResetSectionProps {
  locale: Locale;
  sessionToken: string;
}

export function FactoryResetSection({
  locale,
  sessionToken,
}: FactoryResetSectionProps): ReactElement {
  const [storeName, setStoreName] = useState<string | null>(null);
  const [precheck, setPrecheck] = useState<FactoryResetPrecheck | null>(null);
  const [checking, setChecking] = useState(true);
  const [creatingBackup, setCreatingBackup] = useState(false);
  const [typedToken, setTypedToken] = useState("");
  const [executing, setExecuting] = useState(false);
  const [outcome, setOutcome] = useState<FactoryResetOutcome | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setChecking(true);
    setError(null);
    try {
      const [identity, report] = await Promise.all([
        getStoreIdentity(),
        getFactoryResetPrecheck(sessionToken),
      ]);
      setStoreName(identity.identity.storeName);
      setPrecheck(report);
    } catch (failure) {
      setError(translateBackendError(failure, locale));
    } finally {
      setChecking(false);
    }
  }, [sessionToken, locale]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function handleCreateBackup(): Promise<void> {
    if (creatingBackup || outcome !== null) {
      return;
    }
    setCreatingBackup(true);
    setError(null);
    try {
      await createBackup(sessionToken, "MANUAL");
      await refresh();
    } catch (failure) {
      setError(translateBackendError(failure, locale));
    } finally {
      setCreatingBackup(false);
    }
  }

  async function handleExecute(): Promise<void> {
    if (precheck === null || storeName === null || executing || outcome !== null) {
      return;
    }
    const expected = expectedResetToken(storeName);
    if (!canExecuteReset(precheck.eligible, typedToken, expected)) {
      return;
    }
    setExecuting(true);
    setError(null);
    try {
      const result = await executeFactoryReset(sessionToken, typedToken);
      setOutcome(result);
    } catch (failure) {
      setError(translateBackendError(failure, locale));
    } finally {
      setExecuting(false);
    }
  }

  const locked = outcome !== null;
  const expected = storeName === null ? null : expectedResetToken(storeName);
  const executable =
    precheck !== null &&
    expected !== null &&
    !executing &&
    !locked &&
    canExecuteReset(precheck.eligible, typedToken, expected);

  return (
    <section
      aria-label={t(locale, "settings.resetDangerTitle")}
      className="shrink-0 rounded-lg border-2 border-red-500 bg-red-50/50 p-4 dark:bg-red-900/10"
    >
      <p className="text-sm font-black text-red-800 dark:text-red-200">
        {"\u26A0 "}{t(locale, "settings.resetDangerTitle")}
      </p>
      <p className="mt-1 text-xs text-red-700 dark:text-red-300">
        {t(locale, "settings.resetDangerBody")}
      </p>

      {checking || precheck === null ? (
        <p className="mt-3 text-sm text-slate-500">{t(locale, "settings.resetChecking")}</p>
      ) : (
        <div className="mt-3 flex flex-col gap-2">
          <div className="flex items-center justify-between gap-2">
            <p className="text-sm font-black text-slate-800 dark:text-slate-100">
              {t(locale, "settings.resetPrecheckTitle")}
            </p>
            <button
              type="button"
              onClick={() => { void refresh(); }}
              disabled={checking || executing || locked}
              className="rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
            >
              {t(locale, "settings.refresh")}
            </button>
          </div>

          <ul className="flex flex-col gap-1 text-sm">
            <li
              className={`rounded px-2 py-1 font-bold ${
                precheck.openSession
                  ? "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-200"
                  : "bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-200"
              }`}
            >
              {precheck.openSession
                ? t(locale, "settings.resetOpenSessionBlocked")
                : t(locale, "settings.resetNoOpenSession")}
            </li>
            <li
              className={`rounded px-2 py-1 font-bold ${
                precheck.backupDirWritable
                  ? "bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-200"
                  : "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-200"
              }`}
            >
              {precheck.backupDirWritable
                ? t(locale, "settings.resetBackupDirOk")
                : t(locale, "settings.resetBackupDirBlocked")}
            </li>
            <li className="rounded bg-slate-100 px-2 py-1 font-bold text-slate-700 dark:bg-slate-800 dark:text-slate-200">
              {precheck.newestBackup === null
                ? t(locale, "settings.resetNoBackup")
                : t(locale, "settings.resetNewestBackup")
                    .replace("{file}", precheck.newestBackup.fileName)
                    .replace("{status}", precheck.newestBackup.status)}
            </li>
          </ul>

          {precheck.reasons.length > 0 && (
            <ul className="flex flex-col gap-1">
              {precheck.reasons.map((reason) => (
                <li
                  key={reason}
                  className="rounded bg-red-100 px-2 py-1 font-mono text-xs font-bold text-red-800 dark:bg-red-900/40 dark:text-red-200"
                >
                  {reason}
                </li>
              ))}
            </ul>
          )}

          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={() => { void handleCreateBackup(); }}
              disabled={creatingBackup || locked}
              className="rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 disabled:opacity-50"
            >
              {creatingBackup
                ? t(locale, "settings.creating")
                : t(locale, "settings.resetCreateBackup")}
            </button>
          </div>

          {expected !== null && (
            <div className="flex flex-col gap-2 rounded border border-red-300 bg-white p-3 dark:border-red-800 dark:bg-slate-900">
              <label
                htmlFor="factory-reset-token"
                className="text-sm font-bold text-slate-800 dark:text-slate-100"
              >
                {t(locale, "settings.resetTypeTokenLabel").replace("{token}", expected)}
              </label>
              <input
                id="factory-reset-token"
                type="text"
                value={typedToken}
                disabled={locked || executing}
                autoComplete="off"
                onChange={(event) => { setTypedToken(event.target.value); }}
                className="w-full rounded-md border border-slate-300 bg-white px-2 py-1.5 font-mono text-sm font-bold outline-none focus:border-red-400 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
              <button
                type="button"
                onClick={() => { void handleExecute(); }}
                disabled={!executable}
                className="rounded bg-red-600 px-3 py-1.5 text-sm font-black text-white hover:bg-red-500 disabled:opacity-50"
              >
                {executing
                  ? t(locale, "settings.resetExecuting")
                  : t(locale, "settings.resetExecuteButton")}
              </button>
            </div>
          )}
        </div>
      )}

      {error !== null && (
        <p className="mt-2 rounded bg-red-100 px-2 py-1.5 font-mono text-xs font-bold text-red-800 dark:bg-red-900/40 dark:text-red-200">
          {error}
        </p>
      )}

      {outcome !== null && (
        <div className="mt-2 rounded-lg border-2 border-amber-500 bg-amber-50 px-3 py-3 dark:bg-amber-900/20">
          <p className="text-sm font-black text-amber-900 dark:text-amber-100">
            {t(locale, "settings.resetRestartTitle")}
          </p>
          <p className="mt-1 text-xs font-bold text-amber-800 dark:text-amber-200">
            {t(locale, "settings.resetRestartBody")
              .replace("{file}", outcome.resetBackupFileName)
              .replace("{quarantine}", outcome.quarantinedTo)}
          </p>
        </div>
      )}
    </section>
  );
}

/** I18n key list for the reset pins (FR/AR symmetry). */
export const FACTORY_RESET_I18N_KEYS: I18nKey[] = [
  "settings.resetDangerTitle",
  "settings.resetDangerBody",
  "settings.resetChecking",
  "settings.resetPrecheckTitle",
  "settings.resetOpenSessionBlocked",
  "settings.resetNoOpenSession",
  "settings.resetBackupDirOk",
  "settings.resetBackupDirBlocked",
  "settings.resetNoBackup",
  "settings.resetNewestBackup",
  "settings.resetCreateBackup",
  "settings.resetTypeTokenLabel",
  "settings.resetExecuteButton",
  "settings.resetExecuting",
  "settings.resetRestartTitle",
  "settings.resetRestartBody",
];
