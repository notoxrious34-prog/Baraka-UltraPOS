/**
 * Update Center tab (Settings → System → Updates & Maintenance).
 *
 * Read-only status for everyone; the check/download/install chain is gated
 * by SYSTEM_SETTINGS_MANAGE (Disabled-with-Guidance, never hidden) and the
 * installer additionally requires a CLOSED cash session. Every failure maps
 * to a stable guidance string — the shop never sees a raw exception.
 */

import { useEffect, useState, type ReactElement } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { t, type I18nKey } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import { UPDATE_SIGNING_FINGERPRINT } from "@/lib/update-client";
import { useCashSessionStore, isCheckoutPermitted } from "@/stores/cash-session-store";
import { useSessionStore } from "@/stores/session-store";
import { useUpdateCenterStore } from "@/stores/update-center-store";

interface UpdateCenterTabProps {
  locale: Locale;
  sessionToken: string;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} o`;
  }
  const kb = bytes / 1024;
  if (kb < 1024) {
    return `${kb.toFixed(1)} Ko`;
  }
  return `${(kb / 1024).toFixed(1)} Mo`;
}

function formatStamp(stamp: string, locale: Locale): string {
  const date = new Date(stamp);
  if (Number.isNaN(date.getTime())) {
    return stamp;
  }
  return `${date.toLocaleDateString(locale)} ${date.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit", hourCycle: "h23" })}`;
}

function Card({ children }: { children: ReactElement | ReactElement[] }): ReactElement {
  return (
    <section className="shrink-0 rounded-lg border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900">
      {children}
    </section>
  );
}

function SectionTitle({ textKey, locale }: { textKey: I18nKey; locale: Locale }): ReactElement {
  return <h2 className="pb-2 text-sm font-black">{t(locale, textKey)}</h2>;
}

export function UpdateCenterTab({ locale, sessionToken }: UpdateCenterTabProps): ReactElement {
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canManage = hasPermission("SYSTEM_SETTINGS_MANAGE");
  const cashSession = useCashSessionStore((state) => state.session);
  const shiftOpen = isCheckoutPermitted(cashSession);

  const phase = useUpdateCenterStore((state) => state.phase);
  const info = useUpdateCenterStore((state) => state.info);
  const progress = useUpdateCenterStore((state) => state.progress);
  const errorCode = useUpdateCenterStore((state) => state.errorCode);
  const lastCheckedAt = useUpdateCenterStore((state) => state.lastCheckedAt);
  const storeVersion = useUpdateCenterStore((state) => state.currentVersion);
  const mode = useUpdateCenterStore((state) => state.mode);
  const setMode = useUpdateCenterStore((state) => state.setMode);
  const checkNow = useUpdateCenterStore((state) => state.checkNow);
  const download = useUpdateCenterStore((state) => state.download);
  const install = useUpdateCenterStore((state) => state.install);
  const dismiss = useUpdateCenterStore((state) => state.dismiss);

  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getVersion()
      .then((version) => {
        if (!cancelled) {
          setAppVersion(version);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setAppVersion(null);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const badge = ((): { className: string; textKey: I18nKey } => {
    switch (phase) {
      case "available":
      case "downloading":
      case "ready":
        return {
          className: "bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-200",
          textKey: "update.available",
        };
      case "uptodate":
        return {
          className: "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200",
          textKey: "update.upToDate",
        };
      case "checking":
        return {
          className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300",
          textKey: "update.checking",
        };
      case "installing":
        return {
          className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300",
          textKey: "update.installing",
        };
      default:
        return {
          className: "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300",
          textKey: "update.currentVersion",
        };
    }
  })();

  const busyPhase = phase === "checking" || phase === "downloading" || phase === "installing" || busy;

  async function handleCheck(): Promise<void> {
    if (!canManage || busyPhase) {
      return;
    }
    setNotice(null);
    await checkNow();
  }

  async function handleDownload(): Promise<void> {
    if (!canManage || busyPhase) {
      return;
    }
    setNotice(null);
    await download();
  }

  async function handleInstall(): Promise<void> {
    if (!canManage || shiftOpen || busyPhase) {
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      const fileName = await install(sessionToken);
      setNotice({ ok: true, text: `${t(locale, "update.backupDone")} — ${fileName}` });
    } catch (failure) {
      const code = failure instanceof Error ? failure.message : String(failure);
      setNotice({ ok: false, text: code });
    } finally {
      setBusy(false);
    }
  }

  const installBlockedReason: I18nKey | null = shiftOpen
    ? "update.installBlockedShift"
    : !canManage
      ? "update.installBlockedPermission"
      : null;

  const percent =
    progress.totalBytes !== null && progress.totalBytes > 0
      ? Math.min(100, Math.round((progress.downloadedBytes / progress.totalBytes) * 100))
      : null;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
      <div className="shrink-0">
        <p className="text-base font-black">{t(locale, "settings.tabUpdate")}</p>
        <p className="text-xs font-semibold text-slate-500 dark:text-slate-400">
          {t(locale, "update.statusTitle")}
        </p>
      </div>

      {notice !== null ? (
        <p
          className={`shrink-0 rounded-md px-3 py-2 text-sm font-bold ${
            notice.ok
              ? "bg-emerald-50 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-200"
              : "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200"
          }`}
        >
          {notice.text}
        </p>
      ) : null}

      {phase === "error" && errorCode !== null ? (
        <p className="shrink-0 rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {errorCode === "UPDATE_OFFLINE"
            ? t(locale, "update.offline")
            : errorCode === "UPDATE_NO_RELEASE"
              ? t(locale, "update.noRelease")
              : errorCode}
        </p>
      ) : null}

      <Card>
        <SectionTitle textKey="update.statusTitle" locale={locale} />
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-3">
            <p className="text-2xl font-black" dir="ltr">
              {appVersion ?? storeVersion ?? t(locale, "update.unknownVersion")}
            </p>
            <span
              className={`rounded-full px-2 py-0.5 text-xs font-black ${badge.className}`}
            >
              {t(locale, badge.textKey)}
            </span>
          </div>
          <p className="text-xs font-semibold text-slate-500 dark:text-slate-400">
            {t(locale, "update.lastCheck")}: {lastCheckedAt !== null ? formatStamp(lastCheckedAt, locale) : t(locale, "update.neverChecked")}
          </p>
          <div className="flex flex-col gap-2 sm:flex-row">
            <div role="group" aria-label={t(locale, "update.statusTitle")} className="flex flex-1 gap-2">
              {(["auto", "manual"] as const).map((choice) => (
                <button
                  key={choice}
                  type="button"
                  aria-pressed={mode === choice}
                  onClick={() => {
                    setMode(choice);
                  }}
                  className={`h-10 flex-1 rounded-lg text-xs font-black ${
                    mode === choice
                      ? "bg-primary-900 text-white"
                      : "bg-slate-200 text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100 dark:hover:bg-slate-600"
                  }`}
                >
                  {t(locale, choice === "auto" ? "update.modeAuto" : "update.modeManual")}
                </button>
              ))}
            </div>
            <button
              type="button"
              onClick={() => {
                void handleCheck();
              }}
              disabled={!canManage || busyPhase}
              title={!canManage ? t(locale, "update.installBlockedPermission") : undefined}
              className="flex h-10 flex-1 items-center justify-center gap-2 rounded-lg bg-accent-500 text-xs font-black text-slate-950 hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {phase === "checking" ? (
                <span
                  className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-slate-900 border-t-transparent"
                  aria-hidden="true"
                />
              ) : null}
              {t(locale, "update.checkNow")}
            </button>
          </div>
        </div>
      </Card>

      {info !== null && (phase === "available" || phase === "downloading" || phase === "ready") ? (
        <Card>
          <SectionTitle textKey="update.releaseNotes" locale={locale} />
          <div className="flex flex-col gap-3">
            <p className="text-sm font-black">
              <span dir="auto">
                {t(locale, "update.versionLabel")}:{" "}
              </span>
              <span dir="ltr">{info.version}</span>
            </p>
            {info.notes !== null && info.notes.trim() !== "" ? (
              <pre
                dir="auto"
                className="max-h-40 overflow-y-auto whitespace-pre-wrap rounded-md bg-slate-50 p-3 text-xs leading-5 dark:bg-slate-800"
              >
                {info.notes}
              </pre>
            ) : null}
            {phase === "available" ? (
              <button
                type="button"
                onClick={() => {
                  void handleDownload();
                }}
                disabled={!canManage || busyPhase}
                title={!canManage ? t(locale, "update.installBlockedPermission") : undefined}
                className="flex h-11 items-center justify-center gap-2 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {t(locale, "update.download")}
              </button>
            ) : null}
            {phase === "downloading" ? (
              <div className="flex flex-col gap-1">
                <div
                  role="progressbar"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={percent ?? undefined}
                  className="h-2 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700"
                >
                  <div
                    className={`h-full rounded-full bg-accent-500 ${percent === null ? "w-1/3 animate-pulse" : ""}`}
                    style={percent !== null ? { width: `${percent}%` } : undefined}
                  />
                </div>
                <p className="text-xs font-bold text-slate-500 dark:text-slate-400" dir="ltr">
                  {formatBytes(progress.downloadedBytes)}
                  {progress.totalBytes !== null ? ` / ${formatBytes(progress.totalBytes)}` : ""}
                </p>
              </div>
            ) : null}
            {phase === "ready" ? (
              <div className="flex flex-col gap-2">
                <p className="text-xs font-bold text-emerald-700 dark:text-emerald-200">
                  {t(locale, "update.ready")} — {t(locale, "update.backupNote")}
                </p>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => {
                      void handleInstall();
                    }}
                    disabled={!canManage || shiftOpen || busyPhase}
                    title={installBlockedReason !== null ? t(locale, installBlockedReason) : undefined}
                    className="flex h-11 flex-1 items-center justify-center gap-2 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {busy ? (
                      <span
                        className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-slate-900 border-t-transparent"
                        aria-hidden="true"
                      />
                    ) : null}
                    {t(locale, "update.installAtShiftClose")}
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      void dismiss();
                    }}
                    className="h-11 shrink-0 rounded-lg bg-slate-200 px-4 text-xs font-black text-slate-700 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100 dark:hover:bg-slate-600"
                  >
                    {t(locale, "update.dismiss")}
                  </button>
                </div>
                {installBlockedReason !== null ? (
                  <p className="text-xs font-bold text-amber-700 dark:text-amber-200">
                    {t(locale, installBlockedReason)}
                  </p>
                ) : null}
              </div>
            ) : null}
          </div>
        </Card>
      ) : null}

      <Card>
        <SectionTitle textKey="update.signatureTitle" locale={locale} />
        <p className="text-xs leading-5 text-slate-600 dark:text-slate-300">{t(locale, "update.signatureLine")}</p>
        <p className="pt-1 font-mono text-[11px] text-slate-500 dark:text-slate-400" dir="ltr">
          {UPDATE_SIGNING_FINGERPRINT}…
        </p>
        <p className="pt-2 text-xs leading-5 text-slate-600 dark:text-slate-300">{t(locale, "update.restoreNote")}</p>
      </Card>
    </div>
  );
}
