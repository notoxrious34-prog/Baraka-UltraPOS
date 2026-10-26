/**
 * Backup snapshots tab (DSN-095 redesign on the BRK-094 wire).
 *
 * Every row renders real engine data — locale date, locale size (`o/Ko/Mo`
 * FR, `ب/ك.ب/م.ب` AR, legacy unknown as "—", never 0), localized trigger
 * pill, and a VERIFIED badge (creation-time SHA-256) with the raw checksum
 * one click away in a `<details>` affordance. One primary create action +
 * one refresh action; the danger zone below keeps its own visually distinct
 * verified-backup entry. Unauthorized operators see the restriction note
 * plus a visible-but-disabled create button (never hidden).
 */

import { useCallback, useEffect, useState, type ReactElement } from "react";
import { t, type I18nKey } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";
import { createBackup, listBackups, type BackupMetadata } from "@/lib/ipc";
import {
  checksumPrefix,
  formatBackupDate,
  formatBytes,
  triggerBadgeClass,
} from "./settings-helpers";
import { FactoryResetSection } from "./FactoryResetSection";

interface BackupSettingsTabProps {
  locale: Locale;
  sessionToken: string;
}

/** Localized trigger pill key (RESET included — BRK-094 codes). */
function triggerKey(triggerType: string): I18nKey {
  switch (triggerType) {
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

/** Localized status pill key. */
function statusKey(status: string): I18nKey {
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

/** Status pill tone (emerald only for the verified SUCCESS state). */
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

/** Plus-in-database primary-action glyph (stroke icon, no emoji). */
function CreateIcon(): ReactElement {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      className="h-4 w-4"
    >
      <ellipse cx={10} cy={5} rx={7} ry={2.5} />
      <path d="M3 5v10c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5V5" />
      <path d="M10 8v6M7 11h6" strokeLinecap="round" />
    </svg>
  );
}

/** Refresh glyph for the single list-refresh action. */
function RefreshIcon(): ReactElement {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      className="h-4 w-4"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M16 10a6 6 0 1 1-1.8-4.3" />
      <path d="M16 2v4h-4" />
    </svg>
  );
}

function BackupRow({ locale, row }: { locale: Locale; row: BackupMetadata }): ReactElement {
  const verified = row.status === "SUCCESS";
  return (
    <li className="flex flex-col gap-1.5 rounded-lg bg-white px-3 py-2.5 shadow-sm ring-1 ring-slate-200 dark:bg-slate-800 dark:ring-slate-700">
      <div className="flex items-center justify-between gap-2">
        <span className="min-w-0 flex-1 truncate font-mono text-xs font-extrabold">
          <bdi dir="ltr">{row.fileName}</bdi>
        </span>
        <span
          className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-black ${statusTone(row.status)}`}
        >
          {t(locale, statusKey(row.status))}
        </span>
      </div>
      <p className="text-[11px] font-semibold text-slate-500 dark:text-slate-400">
        <bdi dir="ltr">
          {formatBackupDate(row.createdAt, locale)} · {formatBytes(row.fileSizeBytes, locale)}
        </bdi>{" "}
        <span
          className={`inline-block rounded-full px-2 py-px text-[10px] font-black ${triggerBadgeClass(row.triggerType)}`}
        >
          {t(locale, triggerKey(row.triggerType))}
        </span>
      </p>
      {verified ? (
        <p className="text-[11px] font-black text-emerald-700 dark:text-emerald-300">
          ✓ {t(locale, "backups.verifiedOk")}
        </p>
      ) : (
        <p className="text-[11px] font-black text-red-600 dark:text-red-300">
          ⚠ {row.errorMessage ?? t(locale, "backups.verifiedFail")}
        </p>
      )}
      <details className="text-[11px] text-slate-500 dark:text-slate-400">
        <summary className="cursor-pointer font-bold hover:underline">
          {checksumPrefix(row.sha256Checksum)} · {t(locale, "backups.checksumDetails")}
        </summary>
        <p className="mt-1 break-all font-mono text-[10px]">
          <bdi dir="ltr">{row.sha256Checksum}</bdi>
        </p>
      </details>
    </li>
  );
}

export function BackupSettingsTab({
  locale,
  sessionToken,
}: BackupSettingsTabProps): ReactElement {
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canManage = hasPermission("SYSTEM_SETTINGS_MANAGE");
  const [rows, setRows] = useState<BackupMetadata[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  const refresh = useCallback(async () => {
    if (!canManage) {
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      setRows(await listBackups(sessionToken));
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setLoading(false);
    }
  }, [sessionToken, canManage]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function handleCreate(): Promise<void> {
    if (creating || !canManage) {
      return;
    }
    setCreating(true);
    setNotice(null);
    try {
      const created = await createBackup(sessionToken, "MANUAL");
      setRows((previous) => [created, ...previous]);
      setNotice({
        ok: true,
        text: t(locale, "backups.successCreated").replace("{file}", created.fileName),
      });
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setCreating(false);
    }
  }

  if (!canManage) {
    return (
      <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-8">
        <p className="max-w-md text-center text-sm text-slate-500 dark:text-slate-400">
          {t(locale, "settings.backupRestricted")}
        </p>
        <button
          type="button"
          disabled
          title={t(locale, "settings.backupCreateReservedGuidance")}
          className="cursor-not-allowed rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 opacity-50"
        >
          {t(locale, "settings.createBackup")}
        </button>
      </div>
    );
  }
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 p-4">
      <div className="shrink-0">
        <p className="text-base font-black">{t(locale, "backups.title")}</p>
        <p className="text-xs font-semibold text-slate-500 dark:text-slate-400">
          {t(locale, "backups.subtitle")}
        </p>
      </div>
      <div className="flex shrink-0 gap-2">
        <button
          type="button"
          onClick={() => {
            void handleCreate();
          }}
          disabled={creating}
          className="flex h-11 flex-1 items-center justify-center gap-2 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
        >
          {creating ? (
            <span
              className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-slate-900 border-t-transparent"
              aria-hidden="true"
            />
          ) : (
            <CreateIcon />
          )}
          {creating ? t(locale, "backups.creating") : t(locale, "backups.createNow")}
        </button>
        <button
          type="button"
          onClick={() => {
            void refresh();
          }}
          disabled={loading || creating}
          title={t(locale, "backups.refresh")}
          aria-label={t(locale, "backups.refresh")}
          className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-slate-200 hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100 dark:hover:bg-slate-600"
        >
          <RefreshIcon />
        </button>
      </div>
      {notice !== null && (
        <p
          className={`shrink-0 rounded px-2 py-1.5 text-sm font-bold ${
            notice.ok
              ? "bg-green-50 text-green-700 dark:bg-green-900/30 dark:text-green-200"
              : "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200"
          }`}
        >
          {notice.text}
        </p>
      )}
      {loading ? (
        <p className="text-sm text-slate-500">{t(locale, "backups.loading")}</p>
      ) : rows.length === 0 ? (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <p className="max-w-md text-center text-sm text-slate-500 dark:text-slate-400">
            {t(locale, "backups.empty")}
          </p>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto">
          <ul className="flex flex-col gap-2">
            {rows.map((row) => (
              <BackupRow key={row.id} locale={locale} row={row} />
            ))}
          </ul>
        </div>
      )}
      <p className="shrink-0 text-[11px] font-semibold text-slate-500 dark:text-slate-400">
        {t(locale, "backups.retentionNotice")}
      </p>
      <FactoryResetSection locale={locale} sessionToken={sessionToken} />
    </div>
  );
}
