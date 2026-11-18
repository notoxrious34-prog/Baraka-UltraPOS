/**
 * Inventory bridge: depots, transfers, stock presence.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type { WarehouseInfo } from "@/types/pos";
import { call, type DeleteEntityResult } from "./ipc-kernel";

/** Warehouse row (mirrors WarehouseDto; no is_default on the wire). */
export interface WarehouseItem {
  id: string;
  code: string;
  name: string;
  is_active: boolean;
}

/** Warehouse creation payload (mirrors CreateWarehouseRequestDto). */
export interface CreateWarehousePayload {
  session_token: string;
  code: string;
  name: string;
}

/** Single-line transfer payload (mirrors TransferStockRequestDto). */
export interface StockTransferPayload {
  session_token: string;
  source_warehouse_id: string;
  destination_warehouse_id: string;
  product_id: string;
  unit_id: string;
  quantity_micro: string;
  reason: string | null;
}

/** Transfer outcome (mirrors TransferStockResponseDto). */
export interface StockTransferResult {
  transfer_batch_id: string;
  out_movement_id: string;
  in_movement_id: string;
  source_new_quantity_base_micro: string;
  destination_new_quantity_base_micro: string;
}

/**
 * List active warehouses ordered by code. Rejects with the backend wire
 * code verbatim.
 */
export async function listWarehouses(sessionToken: string): Promise<WarehouseItem[]> {
  interface WarehousesWire {
    warehouses: WarehouseItem[];
  }
  const wire = await call<WarehousesWire>("warehouses_list", {
    session_token: sessionToken,
  });
  return wire.warehouses;
}

/**
 * Create a warehouse (ADMIN-only server-side via ManageWarehouses).
 * Resolves the stored row. Rejects verbatim (UNAUTHORIZED,
 * WAREHOUSE_CODE_EXISTS, …).
 */
export async function createWarehouse(payload: CreateWarehousePayload): Promise<WarehouseItem> {
  return call<WarehouseItem>("warehouses_create", {
    request: { ...payload },
  });
}

/**
 * Move stock as one atomic OUT/IN pair (Gerant/Admin via TransferStock).
 * Resolves movement ids and both new levels. Rejects verbatim
 * (TRANSFER_SAME_WAREHOUSE, WAREHOUSE_INACTIVE, STOCK_INSUFFICIENT, …).
 */
export async function transferStock(payload: StockTransferPayload): Promise<void> {
  await call<unknown>("stock_transfer", {
    request: { ...payload },
  });
}

/** List active warehouses (token injected by the wrapper). */
export async function ipcListWarehouses(): Promise<WarehouseInfo[]> {
  interface WarehousesWire {
    warehouses: Array<{ id: string; code: string; name: string; is_active: boolean }>;
  }
  const wire = await call<WarehousesWire>("warehouses_list", {});
  return wire.warehouses.map((warehouse) => ({
    id: warehouse.id,
    code: warehouse.code,
    name: warehouse.name,
    isActive: warehouse.is_active,
  }));
}

/** Create a warehouse (ADMIN only); resolves the stored view. */
export async function ipcCreateWarehouse(
  sessionToken: string,
  code: string,
  name: string,
): Promise<WarehouseInfo> {
  interface WarehouseWire {
    id: string;
    code: string;
    name: string;
    is_active: boolean;
  }
  const wire = await call<WarehouseWire>("warehouses_create", {
    request: {
      session_token: sessionToken,
      code: code.trim().toUpperCase(),
      name: name.trim(),
    },
  });
  return {
    id: wire.id,
    code: wire.code,
    name: wire.name,
    isActive: wire.is_active,
  };
}

/** Sparse warehouse update payload (ADR-092): rename and/or (de)activate. */
export interface WarehouseUpdateInput {
  name?: string;
  isActive?: boolean;
}

/** Sparse warehouse update (ADMIN only server-side; DEP-01 deactivation rejected). */
export async function ipcUpdateWarehouse(
  sessionToken: string,
  warehouseId: string,
  input: WarehouseUpdateInput,
): Promise<WarehouseInfo> {
  interface WarehouseWire {
    id: string;
    code: string;
    name: string;
    is_active: boolean;
  }
  const wire = await call<WarehouseWire>("warehouses_update", {
    request: {
      session_token: sessionToken,
      id: warehouseId,
      name: input.name,
      is_active: input.isActive,
    },
  });
  return {
    id: wire.id,
    code: wire.code,
    name: wire.name,
    isActive: wire.is_active,
  };
}

/** Dual-track safe warehouse deletion: purge when unreferenced, deactivate on history. */
export async function ipcDeleteWarehouse(
  sessionToken: string,
  warehouseId: string,
  reason: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("warehouses_delete", {
    request: { session_token: sessionToken, id: warehouseId, reason },
  });
}

/** Depot stock-presence row: non-zero level count for one depot. */
export interface WarehouseStockPresence {
  warehouseId: string;
  nonzeroLevels: number;
}

/**
 * Read-only stock presence for every active depot (BRK-095): one row per
 * depot, code-ordered, zero-count depots included. Same ManageWarehouses
 * gate as the depot deletion path; performs zero writes. The delete
 * pre-check wire — the UI derives `stocked` as `nonzeroLevels > 0`.
 */
export async function ipcWarehouseStockPresence(
  sessionToken: string,
): Promise<WarehouseStockPresence[]> {
  interface PresenceWire {
    presence: Array<{ warehouse_id: string; nonzero_levels: number }>;
  }
  const wire = await call<PresenceWire>("warehouses_stock_presence", {
    session_token: sessionToken,
  });
  return wire.presence.map((row) => ({
    warehouseId: row.warehouse_id,
    nonzeroLevels: row.nonzero_levels,
  }));
}
