/**
 * Fiscal stamp tab (DSN-051, ADR-092, realigned TASK-DSN-079): reads
 * `fiscal.timbre_enabled` via `ipcGetFiscalSettings(token)`, persists via
 * `ipcUpdateFiscalSettings(token, { timbreEnabled })` (SYSTEM_SETTINGS_MANAGE
 * server-side, sales.rs:1395). The switch renders visible but disabled with
 * bilingual guidance unless the operator holds SYSTEM_SETTINGS_MANAGE.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";
import { ipcGetFiscalSettings, ipcUpdateFiscalSettings } from "@/lib/ipc";

interface FiscalSettingsTabProps {
  locale: Locale;
  sessionToken: string;
}

export function FiscalSettingsTab({
  locale,
  sessionToken,
}: FiscalSettingsTabProps): ReactElement {
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canWrite = hasPermission("SYSTEM_SETTINGS_MANAGE");
  const [enabled, setEnabled] = useState(true);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    ipcGetFiscalSettings(sessionToken)
      .then((settings) => {
        if (!cancelled) {
          setEnabled(settings.timbreEnabled);
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setNotice({
            ok: false,
            text: typeof failure === "string" ? failure : "INTERNAL_ERROR",
          });
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [sessionToken]);

  async function handleToggle(): Promise<void> {
    if (loading || saving || !canWrite) {
      return;
    }
    const next = !enabled;
    setSaving(true);
    setNotice(null);
    try {
      const settings = await ipcUpdateFiscalSettings(sessionToken, { timbreEnabled: next });
      setEnabled(settings.timbreEnabled);
      setNotice({ ok: true, text: t(locale, "fiscal.updated") });
    } catch (failure) {
      setNotice({
        ok: false,
        text: typeof failure === "string" ? failure : "INTERNAL_ERROR",
      });
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <p className="p-4 text-sm text-slate-500">{t(locale, "settings.loading")}</p>;
  }

  return (
    <div className="flex flex-col gap-3 p-4">
      <h2 className="text-sm font-black">{t(locale, "fiscal.title")}</h2>
      <div className="flex items-start gap-3 rounded-lg border border-slate-200 p-3 dark:border-slate-700">
        <button
          type="button"
          role="switch"
          aria-checked={enabled}
          aria-label={t(locale, "fiscal.timbreLabel")}
          disabled={saving || !canWrite}
          title={canWrite ? undefined : t(locale, "settings.fiscalWriteReservedGuidance")}
          onClick={() => {
            void handleToggle();
          }}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:opacity-50 ${
            enabled ? "bg-emerald-600" : "bg-slate-300 dark:bg-slate-600"
          }`}
        >
          <span
            aria-hidden="true"
            className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all ${
              enabled ? "start-5" : "start-0.5"
            }`}
          />
        </button>
        <div className="min-w-0 flex-1">
          <p className="text-sm font-black">{t(locale, "fiscal.timbreLabel")}</p>
          <p className="mt-0.5 text-xs leading-relaxed text-slate-500">
            {t(locale, "fiscal.timbreHelp")}
          </p>
        </div>
      </div>
      {notice !== null && (
        <p
          role="status"
          className={`rounded-md px-3 py-2 text-xs font-bold ${
            notice.ok
              ? "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-200"
              : "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200"
          }`}
        >
          {notice.text}
        </p>
      )}
    </div>
  );
}
