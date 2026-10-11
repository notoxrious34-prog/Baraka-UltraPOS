/**
 * Backup snapshots tab (DSN-031): admin-only creation via
 * `createBackup(token, "MANUAL")` and history via `listBackups(token)`.
 * Non-admins see an informational restriction note.
 */

import { useCallback, useEffect, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import { createBackup, listBackups, type BackupMetadata } from "@/lib/ipc";
import { checksumPrefix, formatBytes, triggerBadgeClass } from "./settings-helpers";

interface BackupSettingsTabProps {
  locale: Locale;
  sessionToken: string;
  isAdmin: boolean;
}

export function BackupSettingsTab({
  locale,
  sessionToken,
  isAdmin,
}: BackupSettingsTabProps): ReactElement {
  const [rows, setRows] = useState<BackupMetadata[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setRows(await listBackups(sessionToken));
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setLoading(false);
    }
  }, [sessionToken]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function handleCreate(): Promise<void> {
    if (creating) {
      return;
    }
    setCreating(true);
    setNotice(null);
    try {
      const created = await createBackup(sessionToken, "MANUAL");
      setRows((previous) => [created, ...previous]);
      setNotice({ ok: true, text: t(locale, "settings.backupCreated") });
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setCreating(false);
    }
  }

  if (!isAdmin) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center p-8">
        <p className="max-w-md text-center text-sm text-slate-500 dark:text-slate-400">
          {t(locale, "settings.backupRestricted")}
        </p>
      </div>
    );
  }
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 p-4">
      <div className="flex shrink-0 items-center gap-2">
        <button
          type="button"
          onClick={() => {
            void handleCreate();
          }}
          disabled={creating}
          className="rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 disabled:opacity-50"
        >
          {creating ? t(locale, "settings.creating") : t(locale, "settings.createBackup")}
        </button>
        <button
          type="button"
          onClick={() => {
            void refresh();
          }}
          disabled={loading}
          className="rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
        >
          {t(locale, "settings.refresh")}
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
        <p className="text-sm text-slate-500">{t(locale, "settings.loading")}</p>
      ) : rows.length === 0 ? (
        <div className="flex min-h-0 flex-1 items-center justify-center">
          <p className="max-w-md text-center text-sm text-slate-500 dark:text-slate-400">
            {t(locale, "settings.backupEmpty")}
          </p>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto">
          <table className="w-full border-collapse text-sm">
            <thead className="sticky top-0 bg-slate-100 dark:bg-slate-800">
              <tr>
                <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
                  {t(locale, "settings.colFile")}
                </th>
                <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
                  {t(locale, "settings.colDate")}
                </th>
                <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
                  {t(locale, "settings.colType")}
                </th>
                <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
                  {t(locale, "settings.colSize")}
                </th>
                <th className="px-3 py-2 text-start text-xs font-bold text-slate-600 dark:text-slate-300">
                  {t(locale, "settings.colStatus")}
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr
                  key={row.id}
                  className="border-t border-slate-200 dark:border-slate-700 hover:bg-slate-50 dark:hover:bg-slate-800/50"
                >
                  <td className="px-3 py-2 font-mono text-xs font-bold">
                    <bdi dir="ltr">{row.fileName}</bdi>
                  </td>
                  <td className="whitespace-nowrap px-3 py-2 text-xs tabular-nums text-slate-500 dark:text-slate-400">
                    <bdi dir="ltr">{row.createdAt}</bdi>
                  </td>
                  <td className="px-3 py-2">
                    <span
                      className={`inline-block rounded-full px-2 py-0.5 text-xs font-bold ${triggerBadgeClass(row.triggerType)}`}
                    >
                      <bdi dir="ltr">{row.triggerType}</bdi>
                    </span>
                  </td>
                  <td className="whitespace-nowrap px-3 py-2 text-xs tabular-nums">
                    <bdi dir="ltr">{formatBytes(row.fileSizeBytes)}</bdi>
                  </td>
                  <td className="whitespace-nowrap px-3 py-2 font-mono text-xs text-slate-500 dark:text-slate-400">
                    <bdi dir="ltr">{checksumPrefix(row.sha256Checksum)}</bdi>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
