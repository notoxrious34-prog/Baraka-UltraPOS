/**
 * Stock & Warehouses module root (TASK-DSN-029): warehouse selector,
 * alerts/history tabs, action bar, and warehouse creation. Manager-only
 * with defense-in-depth role gating; warehouse creation additionally
 * requires ADMIN (backend ManageWarehouses). Existing StockAdjustModal /
 * WarehouseTransferModal are driven through their sovereign stores and
 * mounted here exactly like PosShell (views are exclusive — no
 * double-mount). Refreshes lists when those modals close.
 */

import { useEffect, useRef, useState, type ReactElement } from "react";
import { t, translateBackendError } from "@/lib/i18n";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useSessionStore } from "@/stores/session-store";
import { useStockAdjustModalStore } from "@/stores/stock-adjust-modal-store";
import { useWarehouseTransferStore } from "@/stores/warehouse-transfer-store";
import {
  ipcListWarehouses,
  ipcLowStock,
  ipcStockMovementHistory,
} from "@/lib/ipc";
import type { WarehouseInfo } from "@/types/pos";
import type { LowStockRow, StockMovementRow } from "@/types/reports";
import { todayIso } from "@/features/reports/components/ReportDateRangePicker";
import { StockAdjustModal } from "@/components/pos/modals/StockAdjustModal";
import { WarehouseTransferModal } from "@/components/pos/modals/WarehouseTransferModal";
import { StockOverviewPane } from "@/features/stock/StockOverviewPane";
import { StockMovementHistoryPane } from "@/features/stock/StockMovementHistoryPane";
import { WarehouseCreateModal } from "@/features/stock/WarehouseCreateModal";
import type { MovementGroup } from "@/features/stock/stock-helpers";

type StockTab = "alerts" | "history";

/** Same manager rule as App.tsx isManagerRole (ADMIN or GERANT). */
function isManagerRole(role: string): boolean {
  return role === "GERANT" || role === "ADMIN";
}

export function StockModule(): ReactElement {
  const locale = usePosUiStore((state) => state.locale);
  const activeModal = usePosUiStore((state) => state.activeModal);
  const openPosModal = usePosUiStore((state) => state.openModal);
  const session = useSessionStore((state) => state.session);
  const openAdjustStore = useStockAdjustModalStore((state) => state.openModal);
  const openTransferStore = useWarehouseTransferStore((state) => state.openModal);

  const [tab, setTab] = useState<StockTab>("alerts");
  const [warehouses, setWarehouses] = useState<WarehouseInfo[]>([]);
  const [warehouseId, setWarehouseId] = useState<string | null>(null);
  const [lowRows, setLowRows] = useState<LowStockRow[]>([]);
  const [moveRows, setMoveRows] = useState<StockMovementRow[]>([]);
  const [group, setGroup] = useState<MovementGroup>("ALL");
  const [loading, setLoading] = useState(false);
  const [listError, setListError] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [refreshTick, setRefreshTick] = useState(0);

  const authorized = session !== null && isManagerRole(session.role);
  const token = authorized && session !== null ? session.token : null;
  const isAdmin = session !== null && session.role === "ADMIN";

  useEffect(() => {
    if (token === null) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    setListError(null);
    const today = todayIso();
    Promise.all([
      ipcListWarehouses(),
      ipcLowStock(token, warehouseId),
      ipcStockMovementHistory(token, {
        dateFrom: today,
        dateTo: today,
        warehouseId,
        productId: null,
        movementType: null,
        limit: "100",
        offset: "0",
      }),
    ])
      .then(([listed, low, moves]) => {
        if (!cancelled) {
          setWarehouses(listed);
          setLowRows(low);
          setMoveRows(moves);
          setLoading(false);
        }
      })
      .catch((failure: unknown) => {
        if (!cancelled) {
          setListError(typeof failure === "string" ? failure : "INTERNAL_ERROR");
          setLowRows([]);
          setMoveRows([]);
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, warehouseId, refreshTick]);

  // Refresh after adjust/transfer modals close (stock changed underneath).
  // Fires only on a real close transition, never on mount.
  const prevModal = useRef(activeModal);
  useEffect(() => {
    const wasStockModal =
      prevModal.current === "stock-adjust" || prevModal.current === "warehouse-transfer";
    prevModal.current = activeModal;
    if (wasStockModal && activeModal === "none") {
      setRefreshTick((tick) => tick + 1);
    }
  }, [activeModal]);

  if (!authorized || token === null) {
    return (
      <section className="flex h-full flex-col gap-3 overflow-y-auto p-4">
        <p className="rounded-md bg-red-50 px-3 py-2 text-sm font-bold text-red-700 dark:bg-red-900/30 dark:text-red-200">
          {translateBackendError("UNAUTHORIZED", locale)}
        </p>
      </section>
    );
  }

  const warehouseCodeOf = (id: string): string =>
    warehouses.find((warehouse) => warehouse.id === id)?.code ?? id.slice(0, 8);

  const openAdjust = (row?: LowStockRow): void => {
    openAdjustStore(
      row === undefined
        ? undefined
        : {
            warehouseId: row.warehouseId,
            productId: row.productId,
            productName: row.nameFr,
            currentStockMicro: row.quantityBaseMicro,
          },
    );
    openPosModal("stock-adjust");
  };

  const openTransfer = (): void => {
    void openTransferStore();
    openPosModal("warehouse-transfer");
  };

  return (
    <div className="flex h-full flex-col overflow-hidden bg-slate-50 dark:bg-slate-900">
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-slate-200 p-3 dark:border-slate-800">
        <h1 className="me-2 text-sm font-black">{t(locale, "stock.title")}</h1>
        <select
          value={warehouseId ?? ""}
          onChange={(event) => {
            setWarehouseId(event.target.value === "" ? null : event.target.value);
          }}
          className="h-10 rounded-lg border border-slate-300 bg-white px-2 text-xs font-bold outline-none dark:border-slate-700 dark:bg-slate-800"
          aria-label={t(locale, "stock.colWarehouse")}
        >
          <option value="">{t(locale, "stock.allWarehouses")}</option>
          {warehouses.map((warehouse) => (
            <option key={warehouse.id} value={warehouse.id}>
              {warehouse.code} · {warehouse.name}
            </option>
          ))}
        </select>
        <div className="flex gap-1">
          <button
            type="button"
            onClick={() => {
              setTab("alerts");
            }}
            className={`h-10 rounded-lg px-3 text-xs font-bold ${
              tab === "alerts"
                ? "bg-primary-900 text-white dark:bg-primary-700"
                : "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200"
            }`}
          >
            {t(locale, "stock.tabAlerts")}
          </button>
          <button
            type="button"
            onClick={() => {
              setTab("history");
            }}
            className={`h-10 rounded-lg px-3 text-xs font-bold ${
              tab === "history"
                ? "bg-primary-900 text-white dark:bg-primary-700"
                : "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200"
            }`}
          >
            {t(locale, "stock.tabHistory")}
          </button>
        </div>
        <div className="ms-auto flex flex-wrap gap-1">
          <button
            type="button"
            onClick={() => {
              openAdjust();
            }}
            className="h-10 rounded-lg bg-slate-200 px-3 text-xs font-bold dark:bg-slate-700"
          >
            {t(locale, "stock.adjust")}
          </button>
          <button
            type="button"
            onClick={openTransfer}
            className="h-10 rounded-lg bg-slate-200 px-3 text-xs font-bold dark:bg-slate-700"
          >
            {t(locale, "stock.transfer")}
          </button>
          {isAdmin && (
            <button
              type="button"
              onClick={() => {
                setCreateOpen(true);
              }}
              className="h-10 rounded-lg bg-accent-500 px-3 text-xs font-black text-slate-950"
            >
              {t(locale, "stock.newWarehouse")}
            </button>
          )}
        </div>
      </div>
      {listError !== null && (
        <p className="shrink-0 px-4 pt-2 text-xs font-bold text-red-600">
          {translateBackendError(listError, locale)}
        </p>
      )}
      {tab === "alerts" ? (
        <StockOverviewPane
          locale={locale}
          warehouseCount={warehouses.length}
          rows={lowRows}
          loading={loading}
          onAdjust={openAdjust}
        />
      ) : (
        <StockMovementHistoryPane
          locale={locale}
          rows={moveRows}
          loading={loading}
          group={group}
          onGroup={setGroup}
          warehouseCodeOf={warehouseCodeOf}
        />
      )}
      {createOpen && (
        <WarehouseCreateModal
          locale={locale}
          token={token}
          onClose={() => {
            setCreateOpen(false);
          }}
          onSaved={() => {
            setCreateOpen(false);
            setRefreshTick((tick) => tick + 1);
          }}
        />
      )}
      {activeModal === "stock-adjust" && <StockAdjustModal />}
      {activeModal === "warehouse-transfer" && <WarehouseTransferModal />}
    </div>
  );
}
