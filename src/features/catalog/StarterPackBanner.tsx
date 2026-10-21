/**
 * Quincaillerie starter-pack lifecycle cockpit (TASK-DSN-052, Mod 4).
 *
 * Dynamic install detection over catalog SKUs, depot-targeted chunked
 * installation with a live progress modal, and dual-track uninstallation
 * confirmed through `SafeDeleteModal` with a hard-deleted/archived
 * summary. Mounted at the top of `CatalogModule`.
 */

import { useMemo, useRef, useState, type ReactElement } from "react";
import { t } from "@/lib/i18n";
import type { Locale } from "@/types/pos";
import type { CatalogProduct } from "@/types/pos";
import type { WarehouseItem } from "@/lib/ipc";
import {
  adjustStock,
  ipcCreateProduct,
  ipcDeleteProduct,
  ipcListCategories,
  ipcSeedStarterCategories,
} from "@/lib/ipc";
import { pickDepotId } from "@/features/catalog/ProductCreateModal";
import { SafeDeleteModal } from "@/components/common/SafeDeleteModal";
import {
  buildCategoryLookup,
  checkStarterPackStatus,
  installStarterPack,
  parseStarterPackCsv,
  uninstallStarterPack,
  type StarterPackItem,
  type UninstallSummary,
} from "@/lib/starter-pack";
import starterPackCsv from "@/assets/fixtures/quincaillerie_starter_300.csv?raw";

interface StarterPackBannerProps {
  locale: Locale;
  token: string;
  warehouses: WarehouseItem[];
  products: CatalogProduct[];
  onChanged: () => void;
}

type Phase =
  | { kind: "idle" }
  | { kind: "install-confirm" }
  | { kind: "installing"; current: number; total: number; name: string }
  | { kind: "uninstalling"; current: number; total: number; name: string }
  | { kind: "summary"; summary: UninstallSummary }
  | { kind: "done"; created: number; skipped: number };

function percent(current: number, total: number): number {
  if (total <= 0) {
    return 0;
  }
  return Math.min(100, Math.round((current / total) * 100));
}

export function StarterPackBanner(props: StarterPackBannerProps): ReactElement {
  const { locale, token, warehouses, products, onChanged } = props;
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [depotId, setDepotId] = useState<string | null>(null);
  const [confirmUninstall, setConfirmUninstall] = useState(false);
  const pendingReasonRef = useRef("");
  const [note, setNote] = useState<string | null>(null);

  const activeWarehouses = useMemo(
    () => warehouses.filter((row) => row.is_active),
    [warehouses],
  );
  const resolvedDepot = depotId ?? pickDepotId(activeWarehouses);

  const status = useMemo(() => {
    const items = parseStarterPackCsv(starterPackCsv);
    return {
      items,
      status: checkStarterPackStatus(
        items,
        products.map((row) => ({ sku: row.sku })),
      ),
    };
  }, [products]);

  const flashNote = (text: string): void => {
    setNote(text);
    window.setTimeout(() => {
      setNote(null);
    }, 8000);
  };

  const startInstall = (): void => {
    if (resolvedDepot === null || status.items.length === 0) {
      return;
    }
    const items: StarterPackItem[] = status.items;
    const target = resolvedDepot;
    const existing = new Set(products.map((row) => row.sku));
    setPhase({ kind: "installing", current: 0, total: items.length, name: "" });
    void (async () => {
      try {
        await ipcSeedStarterCategories(token);
        const categories = await ipcListCategories(token, true);
        const parsed = parseStarterPackCsv(starterPackCsv, buildCategoryLookup(categories));
        const outcome = await installStarterPack(
          token,
          parsed.length > 0 ? parsed : items,
          target,
          existing,
          { create: ipcCreateProduct, adjust: adjustStock },
          (current, total) => {
            const currentItem = (parsed.length > 0 ? parsed : items)[current - 1];
            setPhase({
              kind: "installing",
              current,
              total,
              name:
                currentItem === undefined
                  ? ""
                  : locale === "ar-DZ"
                    ? currentItem.nameAr
                    : currentItem.nameFr,
            });
          },
        );
        setPhase({ kind: "done", created: outcome.created, skipped: outcome.skipped });
        onChanged();
      } catch {
        setPhase({ kind: "idle" });
      }
    })();
  };

  const startUninstall = (reason: string): void => {
    const packCodes = new Set(status.items.map((item) => item.code));
    const rows = products.map((row) => ({ id: row.product_id, sku: row.sku }));
    setConfirmUninstall(false);
    setPhase({ kind: "uninstalling", current: 0, total: rows.length, name: "" });
    void (async () => {
      try {
        const summary = await uninstallStarterPack(
          token,
          packCodes,
          rows,
          { remove: ipcDeleteProduct },
          (current, total) => {
            setPhase({ kind: "uninstalling", current, total, name: "" });
          },
          reason,
        );
        setPhase({ kind: "summary", summary });
        onChanged();
      } catch {
        setPhase({ kind: "idle" });
      }
    })();
  };

  const busy = phase.kind === "installing" || phase.kind === "uninstalling";
  const installed = status.status;

  return (
    <>
      <section
        aria-label={t(locale, installed.isInstalled ? "pack.activeTitle" : "pack.title")}
        className={`shrink-0 rounded-xl border-2 bg-white p-3 dark:bg-slate-900 ${
          installed.isInstalled
            ? "border-emerald-500/60"
            : "border-amber-500/60 dark:border-teal-500/50"
        }`}
      >
        <div className="flex flex-wrap items-center gap-3">
          <svg
            width="28"
            height="28"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            className="shrink-0 text-slate-500"
          >
            <path d="M21 8 12 3 3 8v8l9 5 9-5V8Z" />
            <path d="m3 8 9 5 9-5" />
            <path d="M12 13v8" />
          </svg>
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-sm font-black">
              {t(locale, installed.isInstalled ? "pack.activeTitle" : "pack.title")}
            </h2>
            {!installed.isInstalled && (
              <p className="mt-0.5 text-xs text-slate-500">{t(locale, "pack.subtitle")}</p>
            )}
            {installed.isInstalled && (
              <p className="mt-0.5 font-mono text-xs font-bold tabular-nums text-emerald-700 dark:text-emerald-300">
                <bdi dir="ltr">
                  {installed.installedCount} / {installed.totalPackCount}
                </bdi>{" "}
                {t(locale, "pack.inCatalog")}
              </p>
            )}
          </div>
          {!installed.isInstalled && (
            <label className="flex shrink-0 items-center gap-1.5 text-xs font-bold">
              <span className="text-slate-500">{t(locale, "pack.depotLabel")}</span>
              <select
                value={resolvedDepot ?? ""}
                disabled={busy || activeWarehouses.length === 0}
                onChange={(event) => {
                  setDepotId(event.target.value === "" ? null : event.target.value);
                }}
                className="h-10 rounded-lg border border-slate-300 bg-white px-2 text-xs font-bold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              >
                {activeWarehouses.map((warehouse) => (
                  <option key={warehouse.id} value={warehouse.id}>
                    {warehouse.code} · {warehouse.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          {!installed.isInstalled ? (
            <button
              type="button"
              disabled={busy || resolvedDepot === null}
              onClick={() => {
                setPhase({ kind: "install-confirm" });
              }}
              className="h-10 shrink-0 rounded-lg bg-accent-500 px-4 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-50"
            >
              {t(locale, "pack.install")}
            </button>
          ) : (
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                setConfirmUninstall(true);
              }}
              className="h-10 shrink-0 rounded-lg border border-red-300 px-4 text-sm font-black text-red-700 hover:bg-red-50 disabled:opacity-50 dark:border-red-700 dark:text-red-200 dark:hover:bg-red-900/30"
            >
              {t(locale, "pack.uninstall")}
            </button>
          )}
        </div>
        {note !== null && (
          <p role="status" className="mt-2 text-xs font-black text-emerald-700 dark:text-emerald-300">
            {note}
          </p>
        )}
      </section>

      {phase.kind === "install-confirm" && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          role="dialog"
          aria-modal="true"
          aria-label={t(locale, "pack.install")}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
              setPhase({ kind: "idle" });
            }
          }}
        >
          <div className="w-full max-w-md rounded-xl border border-slate-200 bg-white p-4 shadow-2xl dark:border-slate-700 dark:bg-slate-900">
            <h2 className="text-sm font-black">{t(locale, "pack.install")}</h2>
            <p className="mt-2 text-xs leading-relaxed text-slate-600 dark:text-slate-300">
              {t(locale, "pack.installConfirm")}
            </p>
            <div className="mt-3 flex gap-2">
              <button
                type="button"
                onClick={() => {
                  setPhase({ kind: "idle" });
                }}
                className="h-10 rounded-lg bg-slate-200 px-4 text-sm font-bold hover:bg-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700"
              >
                {t(locale, "common.cancel")}
              </button>
              <button
                type="button"
                onClick={startInstall}
                className="h-10 flex-1 rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
              >
                {t(locale, "pack.start")}
              </button>
            </div>
          </div>
        </div>
      )}

      {(phase.kind === "installing" || phase.kind === "uninstalling") && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          role="dialog"
          aria-modal="true"
          aria-label={t(
            locale,
            phase.kind === "installing" ? "pack.installing" : "pack.uninstalling",
          )}
        >
          <div className="w-full max-w-md rounded-xl border border-slate-200 bg-white p-4 shadow-2xl dark:border-slate-700 dark:bg-slate-900">
            <h2 className="text-sm font-black">
              {t(locale, phase.kind === "installing" ? "pack.installing" : "pack.uninstalling")}
            </h2>
            <div
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={phase.total}
              aria-valuenow={phase.current}
              className="mt-3 h-3 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700"
            >
              <div
                className="h-full rounded-full bg-accent-500 transition-all"
                style={{ width: `${percent(phase.current, phase.total)}%` }}
              />
            </div>
            <p className="mt-2 font-mono text-xs font-bold tabular-nums">
              <bdi dir="ltr">
                {phase.current} / {phase.total} ({percent(phase.current, phase.total)}%)
              </bdi>
            </p>
            {phase.name !== "" && (
              <p className="mt-1 truncate text-xs text-slate-500">{phase.name}</p>
            )}
          </div>
        </div>
      )}

      {confirmUninstall && (
        <SafeDeleteModal
          isOpen
          titleKey="pack.uninstallTitle"
          entityName={t(locale, "pack.uninstallName")}
          entityCode="300"
          onClose={() => {
            setConfirmUninstall(false);
          }}
          onConfirm={(reason: string) => {
            pendingReasonRef.current = reason;
            return Promise.resolve({ id: "pack", mode: "deactivated", message: "pack" });
          }}
          onSuccess={() => {
            startUninstall(pendingReasonRef.current);
          }}
        />
      )}

      {phase.kind === "summary" && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          role="dialog"
          aria-modal="true"
          aria-label={t(locale, "pack.summaryTitle")}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
              setPhase({ kind: "idle" });
            }
          }}
        >
          <div className="w-full max-w-md rounded-xl border border-slate-200 bg-white p-4 shadow-2xl dark:border-slate-700 dark:bg-slate-900">
            <h2 className="text-sm font-black">{t(locale, "pack.summaryTitle")}</h2>
            <ul className="mt-2 flex flex-col gap-1 text-xs font-bold">
              <li className="font-mono tabular-nums">
                <bdi dir="ltr">{phase.summary.hardDeleted}</bdi> {t(locale, "pack.summaryDeleted")}
              </li>
              <li className="font-mono tabular-nums">
                <bdi dir="ltr">{phase.summary.deactivated}</bdi> {t(locale, "pack.summaryArchived")}
              </li>
              {phase.summary.failed > 0 && (
                <li className="font-mono tabular-nums text-red-600">
                  <bdi dir="ltr">{phase.summary.failed}</bdi> {t(locale, "pack.summaryFailed")}
                </li>
              )}
            </ul>
            <p className="mt-2 text-[11px] leading-relaxed text-slate-500">
              {t(locale, "pack.uninstallExplain")}
            </p>
            <div className="mt-3">
              <button
                type="button"
                onClick={() => {
                  setPhase({ kind: "idle" });
                }}
                className="h-10 w-full rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
              >
                {t(locale, "common.close")}
              </button>
            </div>
          </div>
        </div>
      )}

      {phase.kind === "done" && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          role="dialog"
          aria-modal="true"
          aria-label={t(locale, "pack.installing")}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
              setPhase({ kind: "idle" });
              flashNote(t(locale, "settings.saved"));
            }
          }}
        >
          <div className="w-full max-w-md rounded-xl border border-slate-200 bg-white p-4 shadow-2xl dark:border-slate-700 dark:bg-slate-900">
            <h2 className="text-sm font-black">{t(locale, "pack.activeTitle")}</h2>
            <p className="mt-2 font-mono text-xs font-bold tabular-nums">
              <bdi dir="ltr">
                {phase.created} / {phase.created + phase.skipped}
              </bdi>{" "}
              {t(locale, "pack.inCatalog")}
            </p>
            <div className="mt-3">
              <button
                type="button"
                onClick={() => {
                  setPhase({ kind: "idle" });
                  flashNote(t(locale, "settings.saved"));
                }}
                className="h-10 w-full rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400"
              >
                {t(locale, "common.close")}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
