/**
 * Stock adjustment modal (TASK-BRK-049): physical inventory
 * reconciliation over the existing `stock_adjust` command.
 *
 * DELTA mode (signed variation) or NEW_PHYSICAL_COUNT mode (shelf count
 * minus recorded stock). Product picker over catalog_search, warehouse
 * dropdown, BigInt-micro preview banner, reason radio set, single-flight
 * dispatch with success view. All quantities are canonical micro strings
 * parsed to BigInt (zero floats). Backend errors surface through
 * translateBackendError; the backend is final.
 */

import { useEffect, useState, type ReactElement } from "react";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import {
  parseDecimalToMicro,
  searchAdjustProducts,
  useStockAdjustModalStore,
  type StockAdjustReason,
} from "@/stores/stock-adjust-modal-store";
import { t, translateBackendError } from "@/lib/i18n";
import { formatQty } from "@/lib/money";
import { ipcGetStock } from "@/lib/ipc";
import { ModalShell } from "@/components/pos/modals/ModalShell";
import type { CatalogProduct } from "@/types/pos";

function fmtQty(micro: bigint): string {
  try {
    return formatQty(micro.toString());
  } catch {
    return "…";
  }
}

function parseMicro(value: string): bigint {
  const trimmed = value.trim();
  if (!/^-?\d+$/.test(trimmed)) {
    return 0n;
  }
  try {
    return BigInt(trimmed);
  } catch {
    return 0n;
  }
}

function reasonKey(reason: StockAdjustReason): Parameters<typeof t>[1] {
  switch (reason) {
    case "INVENTAIRE":
      return "stockAdjust.reasonInventaire";
    case "CASSE":
      return "stockAdjust.reasonCasse";
    case "PERTE":
      return "stockAdjust.reasonPerte";
    case "SURPLUS":
      return "stockAdjust.reasonSurplus";
    case "AUTRE":
      return "stockAdjust.reasonAutre";
  }
}

export function StockAdjustModal(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const closeModal = usePosUiStore((state) => state.closeModal);
  const session = useSessionStore((state) => state.session);
  const s = useStockAdjustModalStore();
  const [pickerQuery, setPickerQuery] = useState("");
  const [pickerResults, setPickerResults] = useState<CatalogProduct[]>([]);
  const [pickerBusy, setPickerBusy] = useState(false);
  const [pickedMeta, setPickedMeta] = useState<{ sku: string; barcode: string | null } | null>(
    null,
  );

  const delta = parseMicro(s.computedDeltaMicro);
  const newStock = parseMicro(s.computedNewStockMicro);
  const canSubmit =
    !s.isSubmitting &&
    s.productId !== null &&
    s.unitId !== null &&
    s.selectedWarehouseId !== null &&
    delta !== 0n;

  // Capture-phase keyboard guard: global contract closes on Escape —
  // mid-dispatch that would unmount this controller, so Escape is
  // suppressed while submitting; when idle, F12 confirms and Escape
  // closes (CheckoutModal precedent).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (s.isSubmitting) {
        if (event.key === "Escape" || event.key === "Enter" || event.key === "F12") {
          event.preventDefault();
          event.stopImmediatePropagation();
        }
        return;
      }
      if (event.key === "F12") {
        event.preventDefault();
        void s.submitAdjustment();
      } else if (event.key === "Escape") {
        event.preventDefault();
        s.closeModal();
        closeModal();
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => {
      window.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [s.isSubmitting, s, closeModal]);

  const close = (): void => {
    if (s.isSubmitting) {
      return;
    }
    s.closeModal();
    closeModal();
  };

  const changeProduct = (): void => {
    if (s.isSubmitting) {
      return;
    }
    useStockAdjustModalStore.setState({
      productId: null,
      productName: null,
      unitId: null,
      unitName: null,
      currentStockMicro: "0",
      computedDeltaMicro: "0",
      computedNewStockMicro: "0",
      errorMessage: null,
      successResult: null,
    });
    setPickedMeta(null);
    setPickerResults([]);
  };

  const runPickerSearch = (): void => {
    if (pickerQuery.trim() === "" || session === null) {
      setPickerResults([]);
      return;
    }
    setPickerBusy(true);
    searchAdjustProducts(pickerQuery)
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

  const pickProduct = (product: CatalogProduct): void => {
    const unit =
      product.units.find((candidate) => candidate.is_default_selling) ??
      product.units.find((candidate) => candidate.is_base_unit) ??
      product.units[0];
    if (unit === undefined) {
      return;
    }
    const token = session !== null ? session.token : null;
    const warehouseId = s.selectedWarehouseId;
    useStockAdjustModalStore.setState({
      productId: product.product_id,
      productName: locale === "ar-DZ" ? product.name_ar : product.name_fr,
      unitId: unit.unit_id,
      unitName: unit.unit_name_fr,
      errorMessage: null,
      successResult: null,
    });
    setPickedMeta({ sku: product.sku, barcode: product.barcode });
    if (token !== null && warehouseId !== null) {
      ipcGetStock(token, product.product_id, warehouseId)
        .then((level) => {
          useStockAdjustModalStore.setState((state) => ({
            currentStockMicro: level,
            computedDeltaMicro:
              state.inputMode === "DELTA"
                ? parseDecimalToMicro(state.rawInputValue).toString()
                : (
                    parseDecimalToMicro(state.rawInputValue) - parseMicro(level)
                  ).toString(),
            computedNewStockMicro:
              state.inputMode === "DELTA"
                ? (parseMicro(level) + parseDecimalToMicro(state.rawInputValue)).toString()
                : parseDecimalToMicro(state.rawInputValue).toString(),
          }));
        })
        .catch(() => {});
    }
    setPickerResults([]);
    setPickerQuery("");
  };

  return (
    <ModalShell title={t(locale, "stockAdjust.title")} onClose={close}>
      <div className="flex max-h-[70vh] flex-col gap-3 overflow-y-auto">
        {s.successResult !== null ? (
          <div className="flex flex-col gap-2">
            <p className="rounded-lg bg-emerald-50 px-3 py-3 text-center text-base font-black text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200">
              {t(locale, "stockAdjust.successTitle")} —{" "}
              <span className="font-mono">{s.successResult.movementId.slice(0, 8)}</span>
            </p>
            <div className="flex items-center justify-between rounded-lg bg-slate-100 px-3 py-2 font-mono text-sm dark:bg-slate-800">
              <span>
                {t(locale, "stockAdjust.previousStock")}:{" "}
                {fmtQty(s.successResult.previousMicro)}
              </span>
              <span aria-hidden="true">→</span>
              <span className="font-black">
                {t(locale, "stockAdjust.newStock")}: {fmtQty(s.successResult.newMicro)}
              </span>
            </div>
            <button
              type="button"
              onClick={close}
              className="h-11 rounded-lg bg-slate-200 text-sm font-black hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-100"
            >
              {t(locale, "stockAdjust.close")}
            </button>
          </div>
        ) : (
          <>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">
                {t(locale, "stockAdjust.warehouse")}
              </span>
              <select
                value={s.selectedWarehouseId ?? ""}
                disabled={s.isSubmitting}
                onChange={(event) => {
                  s.setWarehouseId(event.target.value);
                }}
                className="h-11 w-full rounded-lg border border-slate-300 bg-white px-2 text-sm font-bold outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              >
                <option value="">{t(locale, "stockAdjust.selectWarehouse")}</option>
                {s.warehouses.map((warehouse) => (
                  <option key={warehouse.id} value={warehouse.id}>
                    {warehouse.code} — {warehouse.name}
                  </option>
                ))}
              </select>
            </label>

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
                placeholder={t(locale, "stockAdjust.searchProduct")}
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
                {t(locale, "returns.search")}
              </button>
            </div>
            {pickerResults.length > 0 && (
              <div className="max-h-60 space-y-1.5 overflow-y-auto rounded-lg border border-slate-200 bg-slate-50/50 p-2 dark:border-slate-700 dark:bg-slate-900/50">
                {pickerResults.map((product) => (
                  <div
                    key={product.product_id}
                    className="flex items-center justify-between gap-3 rounded-lg border border-slate-200 bg-white px-3 py-2 dark:border-slate-700 dark:bg-slate-800"
                  >
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-extrabold">
                        {locale === "ar-DZ" ? product.name_ar : product.name_fr}
                      </span>
                      <span className="mt-1 flex flex-wrap items-center gap-1.5">
                        <span className="rounded bg-slate-100 px-2 py-0.5 font-mono text-xs dark:bg-slate-700">
                          {product.sku}
                        </span>
                        {product.barcode !== null && (
                          <span className="rounded bg-slate-100 px-2 py-0.5 font-mono text-xs opacity-70 dark:bg-slate-700">
                            <bdi dir="ltr">{product.barcode}</bdi>
                          </span>
                        )}
                      </span>
                    </span>
                    <button
                      type="button"
                      disabled={s.isSubmitting}
                      onClick={() => {
                        pickProduct(product);
                      }}
                      className="h-8 shrink-0 rounded bg-primary-600 px-3 text-xs font-bold text-white hover:bg-primary-700 disabled:opacity-50"
                    >
                      {t(locale, "stockAdjust.selectProductAction")}
                    </button>
                  </div>
                ))}
              </div>
            )}

            {s.productId === null ? (
              <p className="rounded-lg border border-slate-200 bg-slate-100/70 p-3 text-center text-xs font-medium text-slate-500 dark:border-slate-700 dark:bg-slate-800/70 dark:text-slate-400">
                {t(locale, "stockAdjust.pleaseSelectGuidance")}
              </p>
            ) : (
              <div className="space-y-1 rounded-lg border-2 border-emerald-500 bg-emerald-50/50 p-3 dark:bg-emerald-950/20">
                <p className="text-xs font-bold text-emerald-700 dark:text-emerald-400">
                  {"✓ "}
                  {t(locale, "stockAdjust.selectedProductBadge")}
                </p>
                <p className="truncate text-sm font-extrabold">{s.productName}</p>
                <p className="font-mono text-xs opacity-70">
                  {pickedMeta !== null ? pickedMeta.sku : ""}
                </p>
                <p className="font-mono text-xs opacity-70">
                  {t(locale, "stockAdjust.currentStock")}: {fmtQty(parseMicro(s.currentStockMicro))}{" "}
                  {s.unitName ?? ""}
                </p>
                <button
                  type="button"
                  onClick={changeProduct}
                  disabled={s.isSubmitting}
                  className="text-xs font-semibold text-slate-600 underline disabled:opacity-50 dark:text-slate-300"
                >
                  {t(locale, "stockAdjust.changeProduct")}
                </button>
              </div>
            )}

            <div className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">
                {t(locale, "stockAdjust.inputMode")}
              </span>
              <div className="grid grid-cols-2 gap-2">
                <button
                  key="DELTA"
                  type="button"
                  disabled={s.isSubmitting}
                  onClick={() => {
                    s.setInputMode("DELTA");
                  }}
                  className={`rounded-lg px-2 py-2 text-xs font-black disabled:opacity-50 ${
                    s.inputMode === "DELTA"
                      ? "bg-primary-600 text-white"
                      : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                  }`}
                >
                  {t(locale, "stockAdjust.modeDelta")}
                </button>
                <button
                  key="NEW_PHYSICAL_COUNT"
                  type="button"
                  disabled={s.isSubmitting}
                  onClick={() => {
                    s.setInputMode("NEW_PHYSICAL_COUNT");
                  }}
                  className={`rounded-lg px-2 py-2 text-xs font-black disabled:opacity-50 ${
                    s.inputMode === "NEW_PHYSICAL_COUNT"
                      ? "bg-primary-600 text-white"
                      : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                  }`}
                >
                  {t(locale, "stockAdjust.modePhysicalCount")}
                </button>
              </div>
            </div>

            <label className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">
                {s.inputMode === "DELTA"
                  ? t(locale, "stockAdjust.deltaQty")
                  : t(locale, "stockAdjust.physicalQty")}
              </span>
              <input
                value={s.rawInputValue}
                onChange={(event) => {
                  s.setInputValue(event.target.value);
                }}
                inputMode="decimal"
                autoComplete="off"
                dir="ltr"
                disabled={s.isSubmitting}
                className="h-11 rounded-lg border border-slate-300 bg-white px-3 font-mono text-base font-black outline-none focus:border-primary-600 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800"
              />
            </label>

            <div
              className={`flex items-center justify-between rounded-lg px-3 py-2 text-sm font-black ${
                delta === 0n
                  ? "bg-slate-100 text-slate-500 dark:bg-slate-800"
                  : delta > 0n
                    ? "bg-emerald-50 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-200"
                    : "bg-amber-50 text-amber-800 dark:bg-amber-900/30 dark:text-amber-200"
              }`}
            >
              <span>
                {delta === 0n
                  ? t(locale, "stockAdjust.validation.zeroDelta")
                  : delta > 0n
                    ? `${t(locale, "stockAdjust.surplusBadge")} (+${fmtQty(delta)})`
                    : `${t(locale, "stockAdjust.lossBadge")} (${fmtQty(delta)})`}
              </span>
              <span className="font-mono">
                {fmtQty(parseMicro(s.currentStockMicro))} → {fmtQty(newStock)}
              </span>
            </div>
            {newStock < 0n && (
              <p className="text-[11px] font-bold text-amber-700 dark:text-amber-300">
                {t(locale, "stockAdjust.validation.negativeStockWarning")}
              </p>
            )}

            <div className="flex flex-col gap-1">
              <span className="text-xs font-bold text-slate-500">
                {t(locale, "stockAdjust.reason")}
              </span>
              <div className="flex flex-wrap gap-1">
                {(
                  [
                    "INVENTAIRE",
                    "CASSE",
                    "PERTE",
                    "SURPLUS",
                    "AUTRE",
                  ] as const
                ).map((option) => (
                  <button
                    key={option}
                    type="button"
                    disabled={s.isSubmitting}
                    onClick={() => {
                      s.setReason(option);
                    }}
                    className={`rounded-md px-2 py-1 text-xs font-black disabled:opacity-50 ${
                      s.reason === option
                        ? "bg-primary-600 text-white"
                        : "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-700 dark:text-slate-300"
                    }`}
                  >
                    {t(locale, reasonKey(option))}
                  </button>
                ))}
              </div>
            </div>

            <input
              value={s.notes}
              onChange={(event) => {
                s.setNotes(event.target.value);
              }}
              placeholder={t(locale, "stockAdjust.notes")}
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
                {t(locale, "stockAdjust.cancel")}
              </button>
              <button
                type="button"
                onClick={() => {
                  void s.submitAdjustment();
                }}
                disabled={!canSubmit}
                className="h-11 flex-[2] rounded-lg bg-accent-500 text-sm font-black text-slate-950 hover:bg-amber-400 disabled:opacity-40"
              >
                {s.isSubmitting
                  ? t(locale, "stockAdjust.submitting")
                  : t(locale, "stockAdjust.confirm")}
              </button>
            </div>
          </>
        )}
      </div>
    </ModalShell>
  );
}
