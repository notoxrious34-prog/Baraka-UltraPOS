/**
 * Warehouse transfer cockpit (TASK-BRK-051, realigned TASK-DSN-080):
 * inter-warehouse transfers plus warehouse creation / depot management.
 *
 * Transfer tab: endpoint selectors with a same-warehouse guard, product
 * picker over catalog_search, per-line BigInt-micro quantities, one atomic
 * OUT/IN pair per line server-side. Create tab: code/name form gated to
 * MANAGE_WAREHOUSES. Transfer execution gated to TRANSFER_STOCK; both
 * render Disabled-with-Guidance when unauthorized. All money/quantities
 * are canonical strings; backend errors surface through
 * translateBackendError; the backend is final.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import {
  parseTransferQtyInput,
  searchTransferProducts,
  useWarehouseTransferStore,
  type WarehouseTransferTab,
} from "@/stores/warehouse-transfer-store";
import { t, translateBackendError } from "@/lib/i18n";
import { createDebouncedSearchRunner } from "@/lib/debounced-search";
import { formatQty } from "@/lib/money";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import { SafeDeleteModal } from "@/components/common/SafeDeleteModal";
import type { CatalogProduct, Locale } from "@/types/pos";
import type { WarehouseItem } from "@/lib/ipc";

function fmtQty(micro: bigint): string {
  try {
    return formatQty(micro.toString());
  } catch {
    return "…";
  }
}

/** Canonical primary-depot code: strictly immutable (ADR-092). */
const PRINCIPAL_DEPOT_CODE = "DEP-01";

/**
 * Depot management tab (TASK-DSN-051, MANAGE_WAREHOUSES only): inline
 * rename, active toggle, and dual-track deletion with DEP-01 lock badges.
 */
function ManageDepotsTab({ canManage }: { canManage: boolean }): ReactElement {
  const locale: Locale = usePosUiStore((state) => state.locale);
  const s = useWarehouseTransferStore();
  const [drafts, setDrafts] = useState<Record<string, { name: string; isActive: boolean }>>({});
  const [deleting, setDeleting] = useState<WarehouseItem | null>(null);
  const [note, setNote] = useState<string | null>(null);

  useEffect(() => {
    const seeded: Record<string, { name: string; isActive: boolean }> = {};
    for (const warehouse of s.warehouses) {
      seeded[warehouse.id] = { name: warehouse.name, isActive: warehouse.is_active };
    }
    setDrafts(seeded);
  }, [s.warehouses]);

  if (!canManage) {
    return (
      <p className="rounded-md bg-amber-50 px-3 py-2 text-sm font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
        {t(locale, "stock.warehouseCreateReservedGuidance")}
      </p>
    );
  }

  const draftOf = (warehouse: WarehouseItem): { name: string; isActive: boolean } =>
    drafts[warehouse.id] ?? { name: warehouse.name, isActive: warehouse.is_active };

  const save = (warehouse: WarehouseItem): void => {
    const draft = draftOf(warehouse);
    const name = draft.name.trim();
    if (name === "" || s.isSubmitting) {
      return;
    }
    const input: { name?: string; isActive?: boolean } = {};
    if (name !== warehouse.name) {
      input.name = name;
    }
    if (draft.isActive !== warehouse.is_active) {
      input.isActive = draft.isActive;
    }
    if (Object.keys(input).length === 0) {
      return;
    }
    void s.submitUpdateWarehouse(warehouse.id, input).then((ok) => {
      if (ok) {
        setNote(t(locale, "settings.saved"));
        window.setTimeout(() => {
          setNote(null);
        }, 8000);
      }
    });
  };

  return (
    <>
      {s.isLoadingWarehouses && (
        <p className="text-center text-xs font-bold text-slate-500">{t(locale, "cash.loading")}</p>
      )}
      <div className="flex flex-col gap-2">
        {s.warehouses.map((warehouse) => {
          const locked = warehouse.code === PRINCIPAL_DEPOT_CODE;
          const draft = draftOf(warehouse);
          return (
            <div
              key={warehouse.id}
              className="flex flex-wrap items-center gap-2 rounded-lg border border-slate-200 p-2 dark:border-slate-700"
            >
              <bdi
                dir="ltr"
                className="rounded-md bg-slate-100 px-2 py-0.5 font-mono text-xs font-black tabular-nums text-slate-600 dark:bg-slate-800 dark:text-slate-300"
              >
                {warehouse.code}
              </bdi>
              {locked && (
                <span className="inline-flex items-center gap-1 rounded-full bg-slate-200 px-2 py-0.5 text-[10px] font-black text-slate-600 dark:bg-slate-700 dark:text-slate-300">
                  <svg
                    width="11"
                    height="11"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <rect x="4" y="11" width="16" height="10" rx="2" />
                    <path d="M8 11V7a4 4 0 0 1 8 0v4" />
                  </svg>
                  {t(locale, "wh.locked")}
                </span>
              )}
              <input
                value={draft.name}
                disabled={locked || s.isSubmitting}
                onChange={(event) => {
                  const name = event.target.value;
                  setDrafts((prev) => ({
                    ...prev,
                    [warehouse.id]: { name, isActive: draft.isActive },
                  }));
                }}
                aria-label={t(locale, "wh.rename")}
                autoComplete="off"
                className="h-9 min-w-0 flex-1 rounded-md border border-slate-300 bg-white px-2 text-sm font-semibold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
              <label className="flex shrink-0 cursor-pointer items-center gap-1 text-xs font-bold">
                <input
                  type="checkbox"
                  checked={draft.isActive}
                  disabled={locked || s.isSubmitting}
                  onChange={(event) => {
                    const isActive = event.target.checked;
                    setDrafts((prev) => ({
                      ...prev,
                      [warehouse.id]: { name: draft.name, isActive },
                    }));
                  }}
                  className="h-4 w-4 accent-emerald-600"
                />
                <span>{t(locale, "wh.active")}</span>
              </label>
              {!locked && (
                <>
                  <button
                    type="button"
                    disabled={s.isSubmitting}
                    onClick={() => {
                      save(warehouse);
                    }}
                    className="h-9 shrink-0 rounded-md bg-slate-200 px-3 text-xs font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:hover:bg-slate-600"
                  >
                    {t(locale, "sup.save")}
                  </button>
                  <button
                    type="button"
                    disabled={s.isSubmitting}
                    onClick={() => {
                      setDeleting(warehouse);
                    }}
                    aria-label={`${t(locale, "del.delete")} — ${warehouse.code}`}
                    title={t(locale, "del.delete")}
                    className="h-9 shrink-0 rounded-md bg-red-50 px-3 text-xs font-black text-red-700 hover:bg-red-100 disabled:opacity-50 dark:bg-red-900/30 dark:text-red-200 dark:hover:bg-red-900/50"
                  >
                    {t(locale, "del.delete")}
                  </button>
                </>
              )}
            </div>
          );
        })}
      </div>
      {s.errorMessage !== null && (
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError(s.errorMessage, locale)}
        </p>
      )}
      {note !== null && (
        <p role="status" className="text-center text-xs font-black text-emerald-700 dark:text-emerald-300">
          {note}
        </p>
      )}
      {deleting !== null && (
        <SafeDeleteModal
          isOpen
          titleKey="del.warehouseTitle"
          entityName={deleting.name}
          entityCode={deleting.code}
          isBlocked={false}
          onClose={() => {
            setDeleting(null);
          }}
          onConfirm={() =>
            s.submitDeleteWarehouse(deleting.id).then((mode) => {
              if (mode === null) {
                throw new Error(s.errorMessage ?? "INTERNAL_ERROR");
              }
              return { id: deleting.id, mode, message: mode };
            })
          }
          onSuccess={(result) => {
            setDeleting(null);
            setNote(t(locale, result.mode === "hard_deleted" ? "del.deleted" : "del.archived"));
            window.setTimeout(() => {
              setNote(null);
            }, 8000);
          }}
        />
      )}
    </>
  );
}

export function WarehouseTransferModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const s = useWarehouseTransferStore();
  const [pickerQuery, setPickerQuery] = useState("");
  const [pickerResults, setPickerResults] = useState<CatalogProduct[]>([]);
  const [pickerBusy, setPickerBusy] = useState(false);
  // Instant debounced picker search (TASK-DSN-067): 175ms, ticket-guarded.
  const pickerSearchRef = useRef<ReturnType<typeof createDebouncedSearchRunner> | null>(null);
  if (pickerSearchRef.current === null) {
    const runner = createDebouncedSearchRunner({
      delayMs: 175,
      onSearch: (firedQuery, ticket) => {
        setPickerBusy(true);
        searchTransferProducts(firedQuery)
          .then((products) => {
            if (runner.isLatest(ticket)) {
              setPickerResults(products);
            }
          })
          .catch(() => {
            if (runner.isLatest(ticket)) {
              setPickerResults([]);
            }
          })
          .finally(() => {
            if (runner.isLatest(ticket)) {
              setPickerBusy(false);
            }
          });
      },
    });
    pickerSearchRef.current = runner;
  }
  useEffect(() => {
    const runner = pickerSearchRef.current;
    if (runner === null) {
      return;
    }
    runner.search(pickerQuery);
    if (pickerQuery.trim() === "") {
      setPickerResults([]);
    }
  }, [pickerQuery]);
  useEffect(
    () => () => {
      pickerSearchRef.current?.dispose();
    },
    [],
  );

  const hasPermission = useSessionStore((state) => state.hasPermission);
  const canTransfer =
    session !== null && hasPermission("TRANSFER_STOCK");
  const canManageWarehouses =
    session !== null && hasPermission("MANAGE_WAREHOUSES");
  const sameEndpoints =
    s.sourceWarehouseId !== null &&
    s.sourceWarehouseId === s.destinationWarehouseId;

  // Capture-phase Escape guard while dispatching (CheckoutModal precedent).
  useEffect(() => {
    if (!s.isSubmitting) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape" || event.key === "Enter" || event.key === "F12") {
        event.preventDefault();
        event.stopImmediatePropagation();
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [s.isSubmitting]);

  if (session === null) {
    return (
      <ModalShell title={t(locale, "warehousesTitle")} onClose={closeModal}>
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError("UNAUTHORIZED", locale)}
        </p>
      </ModalShell>
    );
  }

  const close = (): void => {
    if (s.isSubmitting) {
      return;
    }
    s.closeModal();
    closeModal();
  };

  const flushPickerSearch = (): void => {
    const runner = pickerSearchRef.current;
    if (runner === null) {
      return;
    }
    runner.search(pickerQuery);
    runner.flush();
  };

  const tabButton = (tab: WarehouseTransferTab, labelKey: Parameters<typeof t>[1]): ReactElement => (
    <button
      key={tab}
      type="button"
      disabled={s.isSubmitting || (tab === "create_warehouse" && !canManageWarehouses)}
      title={
        tab === "create_warehouse" && !canManageWarehouses
          ? t(locale, "stock.warehouseCreateReservedGuidance")
          : undefined
      }
      onClick={() => {
        s.setActiveTab(tab);
      }}
      className={`flex-1 rounded-lg px-2 py-2 text-xs font-black disabled:opacity-50 ${
        s.activeTab === tab
          ? "bg-primary-600 text-white"
          : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
      }`}
    >
      {t(locale, labelKey)}
    </button>
  );

  const warehouseOptions = (value: string | null, onChange: (id: string) => void): ReactElement => (
    <select
      value={value ?? ""}
      disabled={s.isSubmitting}
      onChange={(event) => {
        onChange(event.target.value);
      }}
      className="h-11 w-full rounded-lg border border-slate-300 bg-white px-2 text-sm font-bold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
    >
      <option value="">—</option>
      {s.warehouses.map((warehouse) => (
        <option key={warehouse.id} value={warehouse.id}>
          {warehouse.code} — {warehouse.name}
        </option>
      ))}
    </select>
  );

  return (
    <ModalShell title={t(locale, "warehousesTitle")} onClose={close}>
      <div className="flex max-h-[70vh] flex-col gap-3 overflow-y-auto">
        <div className="flex gap-2">
          {tabButton("transfer", "tabTransfer")}
          {tabButton("create_warehouse", "tabCreateWarehouse")}
          {tabButton("manage", "wh.manage")}
        </div>

        {s.successMessage !== null && (
          <p className="rounded-lg bg-emerald-50 px-3 py-2 text-center text-sm font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
            {t(locale, s.successMessage as Parameters<typeof t>[1])}
          </p>
        )}

        {s.activeTab === "transfer" ? (
          <>
            <div className="grid grid-cols-[1fr_auto_1fr] items-end gap-2">
              <label className="flex flex-col gap-1">
                <span className="text-xs font-bold text-slate-500">
                  {t(locale, "sourceWarehouse")}
                </span>
                {warehouseOptions(s.sourceWarehouseId, (id) => {
                  s.setSourceWarehouse(id);
                })}
                <span className="text-[11px] text-slate-500">{t(locale, "selectSourceHelp")}</span>
              </label>
              <span aria-hidden="true" className="pb-8 text-xl font-black text-slate-400">
                ⇄
              </span>
              <label className="flex flex-col gap-1">
                <span className="text-xs font-bold text-slate-500">
                  {t(locale, "destinationWarehouse")}
                </span>
                {warehouseOptions(s.destinationWarehouseId, (id) => {
                  s.setDestinationWarehouse(id);
                })}
                <span className="text-[11px] text-slate-500">{t(locale, "selectDestHelp")}</span>
              </label>
            </div>
            {sameEndpoints && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {t(locale, "sameWarehouseError")}
              </p>
            )}

            <div className="flex gap-2">
              <input
                value={pickerQuery}
                onChange={(event) => {
                  setPickerQuery(event.target.value);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    flushPickerSearch();
                  } else if (event.key === "Escape") {
                    pickerSearchRef.current?.reset();
                    setPickerQuery("");
                    setPickerResults([]);
                  }
                }}
                placeholder={t(locale, "transferProductSearch")}
                autoComplete="off"
                disabled={s.isSubmitting}
                className="h-11 min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
            </div>
            {pickerBusy && pickerQuery.trim() !== "" && (
              <p className="text-center text-xs font-bold text-ink-muted">
                {t(locale, "stock.searching")}
              </p>
            )}
            {pickerResults.length > 0 && (
              <div className="flex max-h-40 flex-col gap-1 overflow-y-auto">
                {pickerResults.map((product) => {
                  const unit =
                    product.units.find((candidate) => candidate.is_default_selling) ??
                    product.units[0];
                  return (
                    <div
                      key={product.product_id}
                      className="flex items-center justify-between gap-2 rounded-md bg-slate-100 px-2 py-1.5 dark:bg-slate-800"
                    >
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-extrabold">
                          {locale === "ar-DZ" ? product.name_ar : product.name_fr}
                        </span>
                        <span className="block font-mono text-[11px] opacity-60">
                          {product.sku} · {unit?.unit_name_fr ?? "—"}
                        </span>
                      </span>
                      <button
                        type="button"
                        disabled={s.isSubmitting || unit === undefined}
                        onClick={() => {
                          if (unit === undefined) {
                            return;
                          }
                          s.addTransferItem({
                            productId: product.product_id,
                            productName:
                              locale === "ar-DZ" ? product.name_ar : product.name_fr,
                            sku: product.sku,
                            unitId: unit.unit_id,
                            unitName: unit.unit_name_fr,
                          });
                        }}
                        className="shrink-0 rounded-md bg-accent-500 px-2 py-1 text-xs font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
                      >
                        +
                      </button>
                    </div>
                  );
                })}
              </div>
            )}

            {s.transferItems.length === 0 ? (
              <p className="text-center text-xs font-bold text-slate-500">
                {t(locale, "noItemsInTransfer")}
              </p>
            ) : (
              <div className="flex flex-col gap-1">
                {s.transferItems.map((line) => (
                  <div
                    key={line.productId}
                    className="flex items-center gap-2 rounded-md bg-slate-100 px-2 py-1.5 dark:bg-slate-800"
                  >
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-extrabold">
                        {line.productName}
                      </span>
                      <span className="block font-mono text-[11px] opacity-60">
                        {line.sku} · {line.unitName}
                      </span>
                    </span>
                    <label className="flex shrink-0 items-center gap-1">
                      <span className="text-[11px] font-bold text-slate-500">
                        {t(locale, "transferQty")}
                      </span>
                      <input
                        value={fmtQty(line.quantityMicro)}
                        onChange={(event) => {
                          s.updateTransferItemQty(
                            line.productId,
                            parseTransferQtyInput(event.target.value),
                          );
                        }}
                        inputMode="decimal"
                        autoComplete="off"
                        dir="ltr"
                        disabled={s.isSubmitting}
                        className="h-9 w-24 rounded-md border border-slate-300 bg-white px-2 font-mono text-sm font-black outline-none focus:border-primary-600 disabled:opacity-40 dark:border-slate-700 dark:bg-slate-900"
                      />
                    </label>
                    <button
                      type="button"
                      disabled={s.isSubmitting}
                      title={t(locale, "transferRemoveItem")}
                      aria-label={`${t(locale, "transferRemoveItem")} — ${line.sku}`}
                      onClick={() => {
                        s.removeTransferItem(line.productId);
                      }}
                      className="shrink-0 rounded-md bg-slate-200 px-2 py-1 text-xs font-black hover:bg-red-200 disabled:opacity-40 dark:bg-slate-700"
                    >
                      ✕
                    </button>
                  </div>
                ))}
              </div>
            )}

            <input
              value={s.transferNotes}
              onChange={(event) => {
                s.setTransferNotes(event.target.value);
              }}
              placeholder={t(locale, "transferNotesPlaceholder")}
              autoComplete="off"
              disabled={s.isSubmitting}
              className="h-10 w-full rounded-lg border border-slate-300 bg-white px-3 text-sm outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
            />

            {s.errorMessage !== null && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {translateBackendError(s.errorMessage, locale)}
              </p>
            )}

            <div className="flex gap-2">
              <button
                type="button"
                onClick={close}
                disabled={s.isSubmitting}
                className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
              >
                {t(locale, "transferCancel")}
              </button>
              <button
                type="button"
                onClick={() => {
                  if (!canTransfer) {
                    return;
                  }
                  void s.submitTransfer();
                }}
                disabled={
                  s.isSubmitting ||
                  !canTransfer ||
                  sameEndpoints ||
                  s.sourceWarehouseId === null ||
                  s.destinationWarehouseId === null ||
                  !s.transferItems.some((line) => line.quantityMicro > 0n)
                }
                title={
                  canTransfer ? undefined : t(locale, "stock.transferReservedGuidance")
                }
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
              >
                {s.isSubmitting
                  ? t(locale, "transferSubmitting")
                  : t(locale, "executeTransfer")}
              </button>
            </div>
          </>
        ) : s.activeTab === "create_warehouse" ? (
          <>
            {!canManageWarehouses && (
              <p className="rounded-md bg-amber-50 px-3 py-2 text-sm font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
                {t(locale, "stock.warehouseCreateReservedGuidance")}
              </p>
            )}
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">{t(locale, "warehouseCode")}</span>
              <input
                value={s.newWarehouseCode}
                onChange={(event) => {
                  s.setNewWarehouseCode(event.target.value);
                }}
                autoComplete="off"
                dir="ltr"
                disabled={s.isSubmitting || !canManageWarehouses}
                className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-sm font-black uppercase outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">{t(locale, "warehouseName")}</span>
              <input
                value={s.newWarehouseName}
                onChange={(event) => {
                  s.setNewWarehouseName(event.target.value);
                }}
                autoComplete="off"
                disabled={s.isSubmitting || !canManageWarehouses}
                className="h-11 rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
            </label>
            {s.errorMessage !== null && (
              <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
                {translateBackendError(s.errorMessage, locale)}
              </p>
            )}
            <div className="flex gap-2">
              <button
                type="button"
                onClick={close}
                disabled={s.isSubmitting}
                className="h-11 flex-1 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 disabled:opacity-50 dark:bg-slate-700 dark:text-slate-100"
              >
                {t(locale, "transferCancel")}
              </button>
              <button
                type="button"
                onClick={() => {
                  if (!canManageWarehouses) {
                    return;
                  }
                  void s.submitCreateWarehouse();
                }}
                disabled={
                  s.isSubmitting ||
                  !canManageWarehouses ||
                  s.newWarehouseCode.trim() === "" ||
                  s.newWarehouseName.trim() === ""
                }
                title={
                  canManageWarehouses
                    ? undefined
                    : t(locale, "stock.warehouseCreateReservedGuidance")
                }
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
              >
                {s.isSubmitting
                  ? t(locale, "transferSubmitting")
                  : t(locale, "createWarehouseBtn")}
              </button>
            </div>
          </>
        ) : (
          <ManageDepotsTab canManage={canManageWarehouses} />
        )}
      </div>
    </ModalShell>
  );
}
