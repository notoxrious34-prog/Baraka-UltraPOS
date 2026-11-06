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
 * TSPL media (LIVE-2026-11-06): gap mm 0..10, density 0..15, speed 1..6 —
 * integers only (plain counts, never money), backend re-validates.
 */

import { useEffect, useState, type ReactElement, type ReactNode } from "react";
import { t } from "@/lib/i18n";
import { useSessionStore } from "@/stores/session-store";
import type { Locale } from "@/types/pos";
import {
  ipcGetPrinterSettings,
  ipcUpdatePrinterSettings,
  printingTestPrint,
  printingTestTsplLabel,
  printingTestTsplTicket,
  type PrinterTransport,
  type TestPrintLanguage,
  type TicketRenderMode,
  type TsplProbeMode,
} from "@/lib/ipc";
import { PrinterIcon, DocumentsIcon, SettingsIcon } from "@/components/icons";
import {
  resolveTestPrintNotice,
  testPrintToneClass,
  type TestPrintTone,
} from "./test-print-notice";
import { BAUD_OPTIONS } from "./settings-helpers";

interface PrinterSettingsTabProps {
  locale: Locale;
  sessionToken: string;
}

// TEMPORARY-DIAGNOSTIC (BRK-100 — remove in lock-in): label/note keys per mode.
const PROBE_LABEL_KEYS = {
  normal: "settings.tsplProbeNormal",
  no_codepage: "settings.tsplProbeNoCodepage",
  ascii_only: "settings.tsplProbeAsciiOnly",
  font1_only: "settings.tsplProbeFont1Only",
} as const;

const PROBE_NOTE_KEYS = {
  normal: "settings.tsplProbeNoteNormal",
  no_codepage: "settings.tsplProbeNoteNoCodepage",
  ascii_only: "settings.tsplProbeNoteAsciiOnly",
  font1_only: "settings.tsplProbeNoteFont1Only",
} as const;

// TASK-PRT-05: numbered section card — title + description + body.
function Section({
  index,
  title,
  desc,
  children,
}: {
  index: string;
  title: string;
  desc: string;
  children: ReactNode;
}): ReactElement {
  return (
    <section className="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-700 dark:bg-slate-900/40">
      <div className="flex items-baseline gap-2">
        <span
          aria-hidden="true"
          className="rounded bg-slate-200 px-1.5 py-0.5 text-xs font-black tabular-nums dark:bg-slate-700"
        >
          {index}
        </span>
        <h2 className="text-sm font-black">{title}</h2>
      </div>
      <p className="mt-0.5 text-xs leading-relaxed text-slate-500 dark:text-slate-400">{desc}</p>
      <div className="mt-3 flex flex-col gap-3">{children}</div>
    </section>
  );
}

export function PrinterSettingsTab({ locale, sessionToken }: PrinterSettingsTabProps): ReactElement {
  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canSaveSettings = hasPermission("MANAGE_PRINTER_SETTINGS");
  const canTestPrint = hasPermission("SYSTEM_SETTINGS_MANAGE");
  const [serialPort, setSerialPort] = useState("");
  const [serialBaud, setSerialBaud] = useState<number>(9600);
  const [paperWidthMm, setPaperWidthMm] = useState<number>(80);
  const [transport, setTransport] = useState<PrinterTransport>("serial");
  const [renderMode, setRenderMode] = useState<TicketRenderMode>("escpos");
  const [queueName, setQueueName] = useState("");
  // LIVE-2026-11-06: TSPL media for the 40x20 label stock. Plain integer
  // counts (digits only, backend re-validates ranges) — never money.
  const [tsplGapMm, setTsplGapMm] = useState("3");
  const [tsplDensity, setTsplDensity] = useState("8");
  const [tsplSpeed, setTsplSpeed] = useState("4");
  // TEMPORARY-DIAGNOSTIC (BRK-100 — remove in lock-in): probe switch state.
  const [probeMode, setProbeMode] = useState<TsplProbeMode>("normal");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<TestPrintLanguage | "tspl" | "tspl-ticket" | null>(null);
  const [notice, setNotice] = useState<{ tone: TestPrintTone; text: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    ipcGetPrinterSettings(sessionToken)
      .then((settings) => {
        if (!cancelled) {
          setSerialPort(settings.serialPort);
          setSerialBaud(settings.serialBaud);
          setPaperWidthMm(settings.paperWidthMm);
          setTransport(settings.transport);
          setRenderMode(settings.ticketRenderMode);
          setQueueName(settings.queueName);
          setProbeMode(settings.tsplProbeMode);
          setTsplGapMm(String(settings.tsplGapMm));
          setTsplDensity(String(settings.tsplDensity));
          setTsplSpeed(String(settings.tsplSpeed));
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setNotice({ tone: "error", text: String(failure) });
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
      setNotice({ tone: "error", text: t(locale, "settings.printerQueueRequired") });
      return;
    }
    setSaving(true);
    setNotice(null);
    // LIVE-2026-11-06: TSPL media are plain integer counts — digits only,
    // clamped to the backend ranges (gap 0..10, density 0..15, speed 1..6).
    const gap = /^\d+$/.test(tsplGapMm) ? Math.min(10, Number(tsplGapMm)) : NaN;
    const density = /^\d+$/.test(tsplDensity) ? Math.min(15, Number(tsplDensity)) : NaN;
    const speed = /^\d+$/.test(tsplSpeed)
      ? Math.min(6, Math.max(1, Number(tsplSpeed)))
      : NaN;
    if (!Number.isInteger(gap) || !Number.isInteger(density) || !Number.isInteger(speed)) {
      setSaving(false);
      setNotice({ tone: "error", text: t(locale, "settings.tsplMediaRange") });
      return;
    }
    try {
      const settings = await ipcUpdatePrinterSettings(
        sessionToken,
        serialPort,
        serialBaud,
        paperWidthMm,
        transport,
        trimmedQueue,
        renderMode,
        // TEMPORARY-DIAGNOSTIC (BRK-100): typed union — invalid unselectable.
        probeMode,
        gap,
        density,
        speed,
      );
      setSerialPort(settings.serialPort);
      setSerialBaud(settings.serialBaud);
      setPaperWidthMm(settings.paperWidthMm);
      setTransport(settings.transport);
      setRenderMode(settings.ticketRenderMode);
      setQueueName(settings.queueName);
      setProbeMode(settings.tsplProbeMode);
      setTsplGapMm(String(settings.tsplGapMm));
      setTsplDensity(String(settings.tsplDensity));
      setTsplSpeed(String(settings.tsplSpeed));
      setNotice({ tone: "success", text: t(locale, "settings.saved") });
    } catch (failure) {
      setNotice({ tone: "error", text: String(failure) });
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
      setNotice(resolveTestPrintNotice(locale, result));
    } catch (failure) {
      setNotice({ tone: "error", text: String(failure) });
    } finally {
      setTesting(null);
    }
  }

  // TASK-PRT-01: the ESC/POS ticket test is silently eaten by TSPL-only
  // printers — this button speaks the production label renderer instead.
  async function handleTestTsplLabel(): Promise<void> {
    if (!canTestPrint) {
      return;
    }
    setTesting("tspl");
    setNotice(null);
    try {
      const result = await printingTestTsplLabel();
      setNotice({
        tone: "success",
        text: `${t(locale, "settings.testPrintTsplSent")} — ${result.bytesSent} o`,
      });
    } catch (failure) {
      setNotice({ tone: "error", text: String(failure) });
    } finally {
      setTesting(null);
    }
  }

  async function handleTestTsplTicket(): Promise<void> {
    if (!canTestPrint) {
      return;
    }
    setTesting("tspl-ticket");
    setNotice(null);
    try {
      const result = await printingTestTsplTicket();
      setNotice({
        tone: "success",
        text: `${t(locale, "settings.testPrintTsplTicketSent")} — ${result.bytesSent} o`,
      });
    } catch (failure) {
      setNotice({ tone: "error", text: String(failure) });
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

  // TASK-PRT-05: live summary chips — effective config at a glance.
  const connectionValue =
    transport === "serial"
      ? `${t(locale, "settings.printerTransportSerial")} · ${serialPort === "" ? "—" : serialPort}`
      : `${t(locale, "settings.printerTransportSpooler")} · ${queueName.trim() === "" ? "—" : queueName.trim()}`;
  const ticketValue = `${t(locale, renderMode === "escpos" ? "settings.ticketRenderEscpos" : "settings.ticketRenderGraphique")} · ${paperWidthMm} mm`;
  const labelValue =
    transport === "spooler" ? `${tsplGapMm} / ${tsplDensity} / ${tsplSpeed} mm` : "—";
  const labelIndex = "04";
  const testsIndex = transport === "spooler" ? "05" : "04";

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
      <header>
        <h1 className="text-base font-black">{t(locale, "settings.tabPrinter")}</h1>
        <p className="mt-0.5 text-xs leading-relaxed text-slate-500 dark:text-slate-400">
          {t(locale, "settings.printerPageSubtitle")}
        </p>
      </header>

      <dl className="grid grid-cols-1 gap-2 sm:grid-cols-3">
        <div className="rounded-lg border border-slate-200 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-900/40">
          <dt className="text-[11px] font-bold uppercase tracking-wide text-slate-500 dark:text-slate-400">
            {t(locale, "settings.summaryConnection")}
          </dt>
          <dd className="mt-0.5 truncate text-sm font-black" title={connectionValue}>
            {connectionValue}
          </dd>
        </div>
        <div className="rounded-lg border border-slate-200 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-900/40">
          <dt className="text-[11px] font-bold uppercase tracking-wide text-slate-500 dark:text-slate-400">
            {t(locale, "settings.summaryTicket")}
          </dt>
          <dd className="mt-0.5 truncate text-sm font-black" title={ticketValue}>
            {ticketValue}
          </dd>
        </div>
        <div className="rounded-lg border border-slate-200 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-900/40">
          <dt className="text-[11px] font-bold uppercase tracking-wide text-slate-500 dark:text-slate-400">
            {t(locale, "settings.summaryLabel")}
          </dt>
          <dd className="mt-0.5 truncate text-sm font-black tabular-nums" title={labelValue}>
            {labelValue}
          </dd>
        </div>
      </dl>

      <Section
        index="01"
        title={t(locale, "settings.sectionConnection")}
        desc={t(locale, "settings.sectionConnectionDesc")}
      >
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
      <div className="rounded-lg bg-slate-50 p-3 text-sm dark:bg-slate-800/50">
        <p className="font-black">{t(locale, "settings.printerComGuideTitle")}</p>
        <ul className="mt-1 list-disc space-y-1 ps-5 text-slate-600 dark:text-slate-300">
          <li>{t(locale, "settings.printerComGuideCom")}</li>
          <li>{t(locale, "settings.printerComGuideUsb")}</li>
          <li>{t(locale, "settings.printerComGuideTransport")}</li>
        </ul>
      </div>
      </Section>

      <Section
        index="02"
        title={t(locale, "settings.sectionTicket")}
        desc={t(locale, "settings.sectionTicketDesc")}
      >
      <div
        role="group"
        aria-label={t(locale, "settings.ticketRenderTitle")}
        className="flex gap-1 rounded border border-slate-300 p-1 dark:border-slate-600"
      >
        <button
          type="button"
          aria-pressed={renderMode === "escpos"}
          onClick={() => {
            setRenderMode("escpos");
          }}
          className={transportButton(renderMode === "escpos")}
        >
          <DocumentsIcon className="h-4 w-4" />
          <span>{t(locale, "settings.ticketRenderEscpos")}</span>
        </button>
        <button
          type="button"
          aria-pressed={renderMode === "graphique"}
          onClick={() => {
            setRenderMode("graphique");
          }}
          className={transportButton(renderMode === "graphique")}
        >
          <PrinterIcon className="h-4 w-4" />
          <span>{t(locale, "settings.ticketRenderGraphique")}</span>
        </button>
      </div>
      <p className="text-xs text-slate-500 dark:text-slate-400">
        {t(locale, "settings.ticketRenderHelp")}
      </p>
      <label className="flex max-w-xs flex-col gap-1 text-sm">
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
      </Section>

      <Section
        index="03"
        title={t(locale, "settings.sectionDiag")}
        desc={t(locale, "settings.sectionDiagDesc")}
      >
      {/* TEMPORARY-DIAGNOSTIC probe switch (BRK-100 — remove in lock-in). */}
      <div className="rounded border-2 border-dashed border-amber-500 p-2 dark:border-amber-400">
        <p className="flex flex-wrap items-center gap-2 text-sm">
          <span className="rounded bg-amber-500 px-1.5 py-0.5 text-xs font-black text-slate-950">
            {t(locale, "settings.tsplProbeBadge")}
          </span>
          <span className="font-bold">{t(locale, "settings.tsplProbeTitle")}</span>
        </p>
        <div
          role="group"
          aria-label={t(locale, "settings.tsplProbeTitle")}
          className="mt-2 grid grid-cols-2 gap-1 rounded border border-slate-300 p-1 dark:border-slate-600"
        >
          {(Object.keys(PROBE_LABEL_KEYS) as TsplProbeMode[]).map((mode) => (
            <button
              key={mode}
              type="button"
              aria-pressed={probeMode === mode}
              onClick={() => {
                setProbeMode(mode);
              }}
              className={transportButton(probeMode === mode)}
            >
              <span>{t(locale, PROBE_LABEL_KEYS[mode])}</span>
            </button>
          ))}
        </div>
        <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
          {t(locale, PROBE_NOTE_KEYS[probeMode])}
        </p>
        {probeMode === "ascii_only" && (
          <p className="mt-1 rounded bg-amber-100 px-2 py-1 text-xs font-bold text-amber-900 dark:bg-amber-900/40 dark:text-amber-200">
            {t(locale, "settings.tsplProbeAsciiWarning")}
          </p>
        )}
      </div>
      </Section>

      {transport === "spooler" ? (
        <Section
          index={labelIndex}
          title={t(locale, "settings.sectionLabel")}
          desc={t(locale, "settings.sectionLabelDesc")}
        >
      <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
        <label className="flex flex-col gap-1 text-sm">
          <span className="font-bold">{t(locale, "settings.tsplGapMm")}</span>
            <bdi dir="ltr" className="contents">
              <input
                className={`${inputClass} tabular-nums`}
                value={tsplGapMm}
                disabled={saving}
                inputMode="numeric"
                onChange={(event) => {
                  setTsplGapMm(event.target.value);
                }}
              />
            </bdi>
            <span className="text-xs text-slate-500 dark:text-slate-400">
              {t(locale, "settings.tsplGapHelp")}
            </span>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="font-bold">{t(locale, "settings.tsplDensity")}</span>
            <bdi dir="ltr" className="contents">
              <input
                className={`${inputClass} tabular-nums`}
                value={tsplDensity}
                disabled={saving}
                inputMode="numeric"
                onChange={(event) => {
                  setTsplDensity(event.target.value);
                }}
              />
            </bdi>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="font-bold">{t(locale, "settings.tsplSpeed")}</span>
            <bdi dir="ltr" className="contents">
              <input
                className={`${inputClass} tabular-nums`}
                value={tsplSpeed}
                disabled={saving}
                inputMode="numeric"
                onChange={(event) => {
                  setTsplSpeed(event.target.value);
                }}
              />
            </bdi>
          </label>
        </div>
        </Section>
      ) : null}
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

      <Section
        index={testsIndex}
        title={t(locale, "settings.sectionTests")}
        desc={t(locale, "settings.sectionTestsDesc")}
      >
        <div className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-slate-200 p-2.5 dark:border-slate-700">
            <div className="min-w-0 flex-1">
              <p className="text-sm font-black">{t(locale, "settings.testPrintFr")}</p>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                {t(locale, "settings.testFrHelp")}
              </p>
            </div>
          <button
            type="button"
            disabled={testing !== null || !canTestPrint}
            title={canTestPrint ? undefined : t(locale, "settings.printerManageReservedGuidance")}
            onClick={() => {
              void handleTestPrint("fr");
            }}
            className="shrink-0 rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
          >
            {testing === "fr" ? t(locale, "settings.testing") : t(locale, "settings.testPrintFr")}
          </button>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-slate-200 p-2.5 dark:border-slate-700">
            <div className="min-w-0 flex-1">
              <p className="text-sm font-black">{t(locale, "settings.testPrintAr")}</p>
              <p className="text-xs text-slate-500 dark:text-slate-400">
                {t(locale, "settings.testArHelp")}
              </p>
            </div>
          <button
            type="button"
            disabled={testing !== null || !canTestPrint}
            title={canTestPrint ? undefined : t(locale, "settings.printerManageReservedGuidance")}
            onClick={() => {
              void handleTestPrint("ar");
            }}
            className="shrink-0 rounded bg-slate-200 px-3 py-1.5 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
          >
            {testing === "ar" ? t(locale, "settings.testing") : t(locale, "settings.testPrintAr")}
          </button>
          </div>
          <div className="rounded-lg border border-accent-500/50 p-2.5 dark:border-accent-400/40">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="min-w-0 flex-1">
                <p className="text-sm font-black">{t(locale, "settings.testPrintTsplLabel")}</p>
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  {t(locale, "settings.testTsplHelp")}
                </p>
              </div>
          <button
            type="button"
            disabled={testing !== null || !canTestPrint}
            title={
              canTestPrint ? undefined : t(locale, "settings.printerManageReservedGuidance")
            }
            onClick={() => {
              void handleTestTsplLabel();
            }}
            className="shrink-0 rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 hover:brightness-110 disabled:opacity-50"
          >
            {testing === "tspl"
              ? t(locale, "settings.testing")
              : t(locale, "settings.testPrintTsplLabel")}
          </button>
            </div>
          </div>
          <div className="rounded-lg border border-accent-500/50 p-2.5 dark:border-accent-400/40">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="min-w-0 flex-1">
                <p className="text-sm font-black">{t(locale, "settings.testPrintTsplTicket")}</p>
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  {t(locale, "settings.testTsplTicketHelp")}
                </p>
              </div>
          <button
            type="button"
            disabled={testing !== null || !canTestPrint}
            title={
              canTestPrint ? undefined : t(locale, "settings.printerManageReservedGuidance")
            }
            onClick={() => {
              void handleTestTsplTicket();
            }}
            className="shrink-0 rounded bg-accent-500 px-3 py-1.5 text-sm font-bold text-slate-950 hover:brightness-110 disabled:opacity-50"
          >
            {testing === "tspl-ticket"
              ? t(locale, "settings.testing")
              : t(locale, "settings.testPrintTsplTicket")}
          </button>
            </div>
          </div>
        </div>
      </Section>
      {notice !== null && (
        <p
          className={`rounded px-2 py-1.5 text-sm font-bold ${testPrintToneClass(notice.tone)}`}
        >
          <bdi dir="ltr">{notice.text}</bdi>
        </p>
      )}
    </div>
  );
}
