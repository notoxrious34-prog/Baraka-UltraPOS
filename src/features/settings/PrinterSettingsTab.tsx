/**
 * Printer & hardware tab (DSN-031, BAT-072): loads via
 * `ipcGetPrinterSettings(token)`, persists via
 * `ipcUpdatePrinterSettings(token, serialPort, serialBaud, paperWidthMm)`
 * (paper width 58 | 80), and fires bilingual test prints via
 * `printingTestPrint("fr" | "ar")`.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import {
  ipcGetPrinterSettings,
  ipcUpdatePrinterSettings,
  printingTestPrint,
  type TestPrintLanguage,
} from "@/lib/ipc";
import { BAUD_OPTIONS } from "./settings-helpers";

interface PrinterSettingsTabProps {
  locale: Locale;
  sessionToken: string;
}

export function PrinterSettingsTab({ locale, sessionToken }: PrinterSettingsTabProps): ReactElement {
  const [serialPort, setSerialPort] = useState("");
  const [serialBaud, setSerialBaud] = useState<number>(9600);
  const [paperWidthMm, setPaperWidthMm] = useState<number>(80);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<TestPrintLanguage | null>(null);
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    ipcGetPrinterSettings(sessionToken)
      .then((settings) => {
        if (!cancelled) {
          setSerialPort(settings.serialPort);
          setSerialBaud(settings.serialBaud);
          setPaperWidthMm(settings.paperWidthMm);
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setNotice({ ok: false, text: String(failure) });
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [sessionToken]);

  async function handleSave(): Promise<void> {
    if (loading || saving) {
      return;
    }
    setSaving(true);
    setNotice(null);
    try {
      const settings = await ipcUpdatePrinterSettings(
        sessionToken,
        serialPort,
        serialBaud,
        paperWidthMm,
      );
      setSerialPort(settings.serialPort);
      setSerialBaud(settings.serialBaud);
      setPaperWidthMm(settings.paperWidthMm);
      setNotice({ ok: true, text: t(locale, "settings.saved") });
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setSaving(false);
    }
  }

  async function handleTestPrint(language: TestPrintLanguage): Promise<void> {
    setTesting(language);
    setNotice(null);
    try {
      const result = await printingTestPrint(language);
      setNotice({
        ok: true,
        text: `${t(locale, "settings.testPrintSent")}: ${result.deviceOutcome} — ${result.captureFileName}`,
      });
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setTesting(null);
    }
  }

  const inputClass =
    "w-full rounded border border-slate-300 bg-white px-2 py-1.5 text-sm dark:border-slate-600 dark:bg-slate-800";
  if (loading) {
    return <p className="p-4 text-sm text-slate-500">{t(locale, "settings.loading")}</p>;
  }
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
      <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "settings.serialPort")}</span>
          <bdi dir="ltr" className="contents">
            <input
              className={`${inputClass} tabular-nums`}
              value={serialPort}
              disabled={saving}
              onChange={(event) => {
                setSerialPort(event.target.value);
              }}
            />
          </bdi>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "settings.serialBaud")}</span>
          <select
            className={inputClass}
            value={serialBaud}
            disabled={saving}
            onChange={(event) => {
              setSerialBaud(Number(event.target.value));
            }}
          >
            {BAUD_OPTIONS.map((baud) => (
              <option key={baud} value={baud}>
                {baud}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "settings.paperWidth")}</span>
          <select
            className={inputClass}
            value={paperWidthMm}
            disabled={saving}
            onChange={(event) => {
              setPaperWidthMm(Number(event.target.value));
            }}
          >
            <option value={80}>{t(locale, "settings.paperWidth80")}</option>
            <option value={58}>{t(locale, "settings.paperWidth58")}</option>
          </select>
        </label>
      </div>
      <div>
        <button
          type="button"
          onClick={() => {
            void handleSave();
          }}
          disabled={saving}
          className="rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 disabled:opacity-50"
        >
          {saving ? t(locale, "settings.saving") : t(locale, "settings.save")}
        </button>
      </div>
      <fieldset className="rounded border border-slate-200 p-3 dark:border-slate-700">
        <legend className="px-1 text-xs font-bold text-slate-500">
          {t(locale, "settings.testPrintTitle")}
        </legend>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            disabled={testing !== null}
            onClick={() => {
              void handleTestPrint("fr");
            }}
            className="rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
          >
            {testing === "fr" ? t(locale, "settings.testing") : t(locale, "settings.testPrintFr")}
          </button>
          <button
            type="button"
            disabled={testing !== null}
            onClick={() => {
              void handleTestPrint("ar");
            }}
            className="rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
          >
            {testing === "ar" ? t(locale, "settings.testing") : t(locale, "settings.testPrintAr")}
          </button>
        </div>
      </fieldset>
      {notice !== null && (
        <p
          className={`rounded px-2 py-1.5 text-sm font-bold ${
            notice.ok
              ? "bg-green-50 text-green-700 dark:bg-green-900/30 dark:text-green-200"
              : "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200"
          }`}
        >
          <bdi dir="ltr">{notice.text}</bdi>
        </p>
      )}
    </div>
  );
}
