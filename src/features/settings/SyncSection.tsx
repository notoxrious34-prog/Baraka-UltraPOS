/**
 * Two-till sync section (TASK-BRK-124, engine ADR-123 v1 scope).
 *
 * Lives at the bottom of the backups tab next to the hardening section:
 * assemble an opaque canonical package of locally created sale docs
 * (read-only export), copy it to a USB file, then merge a received package
 * atomically on the other till. The engine owns every invariant —
 * session gating, terminal binding, SHA-256 integrity, identity index,
 * local renumbering, session-free payments; the UI only carries text.
 * Admin-only rendering is convenience-only: the engine re-validates
 * session, permission (BackupRestoreManage), and preconditions.
 */

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import {
  ipcSyncExport,
  ipcSyncImport,
  listWarehouses,
  type SyncImportOutcome,
  type WarehouseItem,
} from "@/lib/ipc";
import type { Locale } from "@/types/pos";

interface SyncSectionProps {
  locale: Locale;
  sessionToken: string;
}

/** Full-till export cursor: every locally created sale doc, oldest first. */
const FULL_EXPORT_CURSOR = "1970-01-01T00:00:00Z";
/** Backend ceiling (SyncExportPort limit 1..=5000). */
const FULL_EXPORT_LIMIT = 5000;

export function SyncSection({ locale, sessionToken }: SyncSectionProps): ReactElement {
  const [warehouses, setWarehouses] = useState<WarehouseItem[]>([]);
  const [warehouseId, setWarehouseId] = useState("");
  const [packageIn, setPackageIn] = useState("");
  const [packageOut, setPackageOut] = useState("");
  const [outcome, setOutcome] = useState<SyncImportOutcome | null>(null);
  const [busy, setBusy] = useState<"idle" | "export" | "import">("idle");
  const [notice, setNotice] = useState<{ ok: boolean; text: string } | null>(null);
  const outRef = useRef<HTMLTextAreaElement | null>(null);

  const loadWarehouses = useCallback(async () => {
    try {
      const rows = await listWarehouses(sessionToken);
      setWarehouses(rows.filter((row) => row.is_active));
      setWarehouseId((current) => current || rows.find((row) => row.is_active)?.id || "");
    } catch {
      setNotice({ ok: false, text: t(locale, "backups.sync.loadWarehousesError") });
    }
  }, [sessionToken, locale]);

  useEffect(() => {
    void loadWarehouses();
  }, [loadWarehouses]);

  async function handleExport(): Promise<void> {
    if (busy !== "idle") {
      return;
    }
    setBusy("export");
    setNotice(null);
    setOutcome(null);
    try {
      const result = await ipcSyncExport(sessionToken, FULL_EXPORT_CURSOR, FULL_EXPORT_LIMIT);
      setPackageOut(result.packageText);
      setNotice({
        ok: true,
        text: t(locale, "backups.sync.exportedDocs").replace("{n}", String(result.exportedDocs)),
      });
    } catch (failure) {
      setNotice({ ok: false, text: translateBackendError(failure, locale) });
    } finally {
      setBusy("idle");
    }
  }

  async function handleCopy(): Promise<void> {
    if (packageOut === "") {
      return;
    }
    try {
      await navigator.clipboard.writeText(packageOut);
      setNotice({ ok: true, text: t(locale, "backups.sync.copied") });
    } catch {
      outRef.current?.select();
      setNotice({ ok: true, text: t(locale, "backups.sync.copied") });
    }
  }

  async function handleImport(): Promise<void> {
    if (busy !== "idle" || packageIn.trim() === "" || warehouseId === "") {
      return;
    }
    setBusy("import");
    setNotice(null);
    setOutcome(null);
    try {
      const result = await ipcSyncImport(sessionToken, packageIn.trim(), warehouseId);
      setOutcome(result);
      setNotice({
        ok: result.negativeStockWarnings.length === 0,
        text: t(locale, "backups.sync.importedDocs").replace("{n}", String(result.importedDocs)),
      });
    } catch (failure) {
      setNotice({ ok: false, text: translateBackendError(failure, locale) });
    } finally {
      setBusy("idle");
    }
  }

  return (
    <section
      aria-label={t(locale, "backups.sync.title")}
      className="flex shrink-0 flex-col gap-2 rounded-lg border border-slate-300 p-3 dark:border-slate-600"
    >
      <p className="text-sm font-black">{t(locale, "backups.sync.title")}</p>
      <p className="text-xs font-semibold text-slate-500 dark:text-slate-400">
        {t(locale, "backups.sync.scope")}
      </p>
      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => {
            void handleExport();
          }}
          disabled={busy !== "idle"}
          className="flex h-11 flex-1 items-center justify-center rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
        >
          {t(locale, "backups.sync.exportAction")}
        </button>
        <button
          type="button"
          onClick={() => {
            void handleCopy();
          }}
          disabled={packageOut === ""}
          className="flex h-11 items-center justify-center rounded-lg bg-slate-200 px-3 text-sm font-bold hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100 dark:hover:bg-slate-600"
        >
          {t(locale, "backups.sync.copyPackage")}
        </button>
      </div>
      {packageOut !== "" && (
        <textarea
          ref={outRef}
          readOnly
          value={packageOut}
          rows={4}
          dir="ltr"
          aria-label={t(locale, "backups.sync.copyPackage")}
          className="w-full rounded border border-slate-300 bg-slate-50 p-2 font-mono text-[11px] dark:border-slate-600 dark:bg-slate-800"
        />
      )}
      <label className="flex flex-col gap-1 text-xs font-bold">
        {t(locale, "backups.sync.packageLabel")}
        <textarea
          value={packageIn}
          onChange={(event) => setPackageIn(event.target.value)}
          rows={4}
          dir="ltr"
          placeholder="HDR|SYNC1|…"
          className="w-full rounded border border-slate-300 bg-white p-2 font-mono text-[11px] dark:border-slate-600 dark:bg-slate-800"
        />
      </label>
      <div className="flex gap-2">
        <label className="flex flex-1 flex-col gap-1 text-xs font-bold">
          {t(locale, "backups.sync.warehouse")}
          <select
            value={warehouseId}
            onChange={(event) => setWarehouseId(event.target.value)}
            className="h-11 rounded-lg border border-slate-300 bg-white px-2 text-sm dark:border-slate-600 dark:bg-slate-800"
          >
            {warehouses.map((row) => (
              <option key={row.id} value={row.id}>
                {row.code} — {row.name}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          onClick={() => {
            void handleImport();
          }}
          disabled={busy !== "idle" || packageIn.trim() === "" || warehouseId === ""}
          className="flex h-11 items-center justify-center self-end rounded-lg bg-accent-500 px-4 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
        >
          {t(locale, "backups.sync.importAction")}
        </button>
      </div>
      {notice !== null && (
        <p
          className={`rounded px-2 py-1.5 text-sm font-bold ${
            notice.ok
              ? "bg-green-50 text-green-700 dark:bg-green-900/30 dark:text-green-200"
              : "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200"
          }`}
        >
          {notice.text}
        </p>
      )}
      {outcome !== null && (
        <div className="flex flex-col gap-1 text-xs">
          <p className="font-black">{t(locale, "backups.sync.renumbered")}</p>
          {outcome.renumbered.length === 0 ? (
            <p className="text-slate-500">{t(locale, "backups.sync.noRenumber")}</p>
          ) : (
            <ul className="flex flex-col gap-0.5 font-mono" dir="ltr">
              {outcome.renumbered.map((row) => (
                <li key={row.newNumber}>
                  {row.originNumber} → {row.newNumber}
                </li>
              ))}
            </ul>
          )}
          <p className="font-black">{t(locale, "backups.sync.warnings")}</p>
          {outcome.negativeStockWarnings.length === 0 ? (
            <p className="text-slate-500">{t(locale, "backups.sync.noWarnings")}</p>
          ) : (
            <ul className="flex flex-col gap-0.5 font-mono" dir="ltr">
              {outcome.negativeStockWarnings.map((line) => (
                <li key={`${line.sku}-${line.newNumber}`}>
                  {line.sku} @ {line.newNumber}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </section>
  );
}
