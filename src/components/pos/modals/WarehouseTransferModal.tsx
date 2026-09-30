/**
 * Warehouse transfer cockpit (TASK-BRK-051): inter-warehouse transfers
 * plus ADMIN-only warehouse creation.
 *
 * Transfer tab: endpoint selectors with a same-warehouse guard, product
 * picker over catalog_search, per-line BigInt-micro quantities, one atomic
 * OUT/IN pair per line server-side. Create tab: code/name form gated to
 * ADMIN (ManageWarehouses). All money/quantities are canonical strings;
 * backend errors surface through translateBackendError; the backend is
 * final.
 */

import { useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import {
  isAdminRole,
  isManagerRole,
  parseTransferQtyInput,
  searchTransferProducts,
  useWarehouseTransferStore,
  type WarehouseTransferTab,
} from "@/stores/warehouse-transfer-store";
import { t, translateBackendError } from "@/lib/i18n";
import { formatQty } from "@/lib/money";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { CatalogProduct } from "@/types/pos";

function fmtQty(micro: bigint): string {
  try {
    return formatQty(micro.toString());
  } catch {
    return "…";
  }
}

export function WarehouseTransferModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const s = useWarehouseTransferStore();
  const [pickerQuery, setPickerQuery] = useState("");
  const [pickerResults, setPickerResults] = useState<CatalogProduct[]>([]);
  const [pickerBusy, setPickerBusy] = useState(false);

  const role = session !== null ? session.role : "";
  const manager = isManagerRole(role);
  const admin = isAdminRole(role);
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

  if (!manager) {
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

  const runPickerSearch = (): void => {
    if (pickerQuery.trim() === "") {
      setPickerResults([]);
      return;
    }
    setPickerBusy(true);
    searchTransferProducts(pickerQuery)
      .then((products) => {
        setPickerResults(products);
      })
      .catch(() => {
        setPickerResults([]);
      })
      .finally(() => {
        setPickerBusy(false);
      });
  };

  const tabButton = (tab: WarehouseTransferTab, labelKey: Parameters<typeof t>[1]): ReactElement => (
    <button
      key={tab}
      type="button"
      disabled={s.isSubmitting || (tab === "create_warehouse" && !admin)}
      title={tab === "create_warehouse" && !admin ? t(locale, "adminOnlyTab") : undefined}
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
                    runPickerSearch();
                  }
                }}
                placeholder={t(locale, "transferProductSearch")}
                autoComplete="off"
                disabled={s.isSubmitting}
                className="h-11 min-w-0 flex-1 rounded-lg border border-slate-300 bg-white px-3 text-sm font-semibold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
              <button
                type="button"
                onClick={runPickerSearch}
                disabled={s.isSubmitting || pickerBusy}
                className="h-11 shrink-0 rounded-lg bg-primary-600 px-3 text-sm font-black text-white disabled:opacity-50"
              >
                {t(locale, "addItemToTransfer")}
              </button>
            </div>
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
                  void s.submitTransfer();
                }}
                disabled={
                  s.isSubmitting ||
                  sameEndpoints ||
                  s.sourceWarehouseId === null ||
                  s.destinationWarehouseId === null ||
                  !s.transferItems.some((line) => line.quantityMicro > 0n)
                }
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
              >
                {s.isSubmitting
                  ? t(locale, "transferSubmitting")
                  : t(locale, "executeTransfer")}
              </button>
            </div>
          </>
        ) : (
          <>
            {!admin && (
              <p className="rounded-md bg-amber-50 px-3 py-2 text-sm font-bold text-amber-800 dark:bg-amber-900/30 dark:text-amber-200">
                {t(locale, "adminOnlyTab")}
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
                disabled={s.isSubmitting || !admin}
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
                disabled={s.isSubmitting || !admin}
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
                  void s.submitCreateWarehouse();
                }}
                disabled={
                  s.isSubmitting ||
                  !admin ||
                  s.newWarehouseCode.trim() === "" ||
                  s.newWarehouseName.trim() === ""
                }
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
              >
                {s.isSubmitting
                  ? t(locale, "transferSubmitting")
                  : t(locale, "createWarehouseBtn")}
              </button>
            </div>
          </>
        )}
      </div>
    </ModalShell>
  );
}
