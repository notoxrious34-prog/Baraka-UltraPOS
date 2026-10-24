/**
 * Printer & hardware tab (DSN-031, BAT-072, DSN-091): loads via
 * `ipcGetPrinterSettings(token)`, persists via
 * `ipcUpdatePrinterSettings(token, serialPort, serialBaud, paperWidthMm, transport, queueName)`
 * (paper width 58 | 80), and fires bilingual test prints via
 * `printingTestPrint("fr" | "ar")`.
 * Transport selection (serial COM vs USB spooler RAW) with conditional
 * fields; saving on the spooler arm with an empty queue is blocked with
 * an inline message (no IPC call). Both arms' stored values are kept
 * independently — switching never clears the other field.
 */

import { useEffect, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";
import {
  ipcGetPrinterSettings,
  ipcUpdatePrinterSettings,
  printingTestPrint,
  type PrinterTransport,
  type TestPrintLanguage,
} from "@/lib/ipc";
import { PrinterIcon, SettingsIcon } from "@/components/icons";
import { BAUD_OPTIONS } from "./settings-helpers";

interface PrinterSettingsTabProps {
  locale: Locale;
  sessionToken: string;
}

export function PrinterSettingsTab({ locale, sessionToken }: PrinterSettingsTabProps): ReactElement {
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canSaveSettings = hasPermission("MANAGE_PRINTER_SETTINGS");
  const canTestPrint = hasPermission("SYSTEM_SETTINGS_MANAGE");
  const [serialPort, setSerialPort] = useState("");
  const [serialBaud, setSerialBaud] = useState<number>(9600);
  const [paperWidthMm, setPaperWidthMm] = useState<number>(80);
  const [transport, setTransport] = useState<PrinterTransport>("serial");
  const [queueName, setQueueName] = useState("");
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
          setTransport(settings.transport);
          setQueueName(settings.queueName);
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
    if (loading || saving || !canSaveSettings) {
      return;
    }
    const trimmedQueue = queueName.trim();
    if (transport === "spooler" && trimmedQueue === "") {
      setNotice({ ok: false, text: t(locale, "settings.printerQueueRequired") });
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
        transport,
        trimmedQueue,
      );
      setSerialPort(settings.serialPort);
      setSerialBaud(settings.serialBaud);
      setPaperWidthMm(settings.paperWidthMm);
      setTransport(settings.transport);
      setQueueName(settings.queueName);
      setNotice({ ok: true, text: t(locale, "settings.saved") });
    } catch (failure) {
      setNotice({ ok: false, text: String(failure) });
    } finally {
      setSaving(false);
    }
  }

  async function handleTestPrint(language: TestPrintLanguage): Promise<void> {
    if (!canTestPrint) {
      return;
    }
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
  const transportButton = (active: boolean): string =>
    `flex flex-1 items-center justify-center gap-1.5 rounded px-3 py-1.5 text-sm font-bold ${
      active
        ? "bg-accent-500 text-slate-950"
        : "text-slate-600 hover:bg-slate-200 dark:text-slate-300 dark:hover:bg-slate-700"
    }`;
  if (loading) {
    return <p className="p-4 text-sm text-slate-500">{t(locale, "settings.loading")}</p>;
  }
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
      <div
        role="group"
        aria-label={t(locale, "settings.printerTransportTitle")}
        className="flex gap-1 rounded border border-slate-300 p-1 dark:border-slate-600"
      >
        <button
          type="button"
          aria-pressed={transport === "serial"}
          onClick={() => {
            setTransport("serial");
          }}
          className={transportButton(transport === "serial")}
        >
          <SettingsIcon className="h-4 w-4" />
          <span>{t(locale, "settings.printerTransportSerial")}</span>
        </button>
        <button
          type="button"
          aria-pressed={transport === "spooler"}
          onClick={() => {
            setTransport("spooler");
          }}
          className={transportButton(transport === "spooler")}
        >
          <PrinterIcon className="h-4 w-4" />
          <span>{t(locale, "settings.printerTransportSpooler")}</span>
        </button>
      </div>
      {transport === "serial" ? (
        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
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
        </div>
      ) : (
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "settings.printerQueueName")}</span>
          <bdi dir="ltr" className="contents">
            <input
              className={`${inputClass} tabular-nums`}
              value={queueName}
              disabled={saving}
              placeholder={t(locale, "settings.printerQueuePlaceholder")}
              onChange={(event) => {
                setQueueName(event.target.value);
              }}
            />
          </bdi>
          <span className="text-xs text-slate-500 dark:text-slate-400">
            {t(locale, "settings.printerQueueHelp")}
          </span>
        </label>
      )}
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
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
      <div className="rounded border border-slate-200 bg-slate-50 p-3 text-sm dark:border-slate-700 dark:bg-slate-800/50">
        <p className="font-black">{t(locale, "settings.printerComGuideTitle")}</p>
        <ul className="mt-1 list-disc space-y-1 ps-5 text-slate-600 dark:text-slate-300">
          <li>{t(locale, "settings.printerComGuideCom")}</li>
          <li>{t(locale, "settings.printerComGuideUsb")}</li>
          <li>{t(locale, "settings.printerComGuideTransport")}</li>
        </ul>
      </div>
      <div>
        <button
          type="button"
          onClick={() => {
            void handleSave();
          }}
          disabled={saving || !canSaveSettings}
          title={canSaveSettings ? undefined : t(locale, "settings.printerManageReservedGuidance")}
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
            disabled={testing !== null || !canTestPrint}
            title={canTestPrint ? undefined : t(locale, "settings.printerManageReservedGuidance")}
            onClick={() => {
              void handleTestPrint("fr");
            }}
            className="rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
          >
            {testing === "fr" ? t(locale, "settings.testing") : t(locale, "settings.testPrintFr")}
          </button>
          <button
            type="button"
            disabled={testing !== null || !canTestPrint}
            title={canTestPrint ? undefined : t(locale, "settings.printerManageReservedGuidance")}
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
