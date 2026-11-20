/**
 * Supplier & purchase bridge (TASK-BRK-011).
 * Backend command mapping (sovereign contract):
 *   suppliers_create / suppliers_update / suppliers_get / suppliers_search /
 *   suppliers_get_statement / purchases_create_order / purchases_update_order /
 *   purchases_receive_goods / purchases_preview_receive_deviations /
 *   purchases_receive_matrix_goods / purchases_record_invoice /
 *   purchases_record_payment / purchases_get_document - exact backend names.
 * All money crosses as centime strings; all quantities as micro strings.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type {
  PurchaseDocumentView,
  PurchaseLineInput,
  SupplierFormInput,
  SupplierLedgerEntry,
  SupplierPaymentOutcome,
  SupplierStatement,
  SupplierUpdateInput,
  SupplierView,
} from "@/types/supplier";
import { call, type DeleteEntityResult } from "./ipc-kernel";

interface SupplierWire {
  id: string;
  name: string;
  phone: string | null;
  address: string | null;
  nif: string | null;
  rc: string | null;
  notes: string | null;
  is_active: boolean;
  current_balance_centimes: string;
  origin_device_id: string;
  created_at: string;
  updated_at: string;
}

interface PurchaseLineWire {
  id: string;
  product_id: string;
  unit_id: string;
  quantity_micro: string;
  quantity_base_micro: string;
  unit_cost_centimes: string;
  line_total_centimes: string;
}

interface PurchaseDocumentWire {
  id: string;
  number: string;
  doc_type: string;
  status: string;
  supplier_id: string;
  warehouse_id: string;
  source_document_id: string | null;
  total_centimes: string;
  lines: PurchaseLineWire[];
  created_at: string;
  created_by: string;
}

interface SupplierLedgerEntryWire {
  id: string;
  entry_type: string;
  amount_centimes: string;
  reference_type: string;
  reference_id: string;
  description: string | null;
  created_at: string;
  created_by: string;
  balance_after_centimes: string;
}

interface SupplierStatementWire {
  entries: SupplierLedgerEntryWire[];
  current_balance_centimes: string;
}

interface SupplierPaymentWire {
  ledger_entry_id: string;
  new_balance_centimes: string;
  cash_movement_id: string | null;
}

function toSupplier(wire: SupplierWire): SupplierView {
  return {
    id: wire.id,
    name: wire.name,
    phone: wire.phone,
    address: wire.address,
    nif: wire.nif,
    rc: wire.rc,
    notes: wire.notes,
    isActive: wire.is_active,
    currentBalanceCentimes: wire.current_balance_centimes,
    originDeviceId: wire.origin_device_id,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

function toPurchaseLine(wire: PurchaseLineWire) {
  return {
    id: wire.id,
    productId: wire.product_id,
    unitId: wire.unit_id,
    quantityMicro: wire.quantity_micro,
    quantityBaseMicro: wire.quantity_base_micro,
    unitCostCentimes: wire.unit_cost_centimes,
    lineTotalCentimes: wire.line_total_centimes,
  };
}

function toPurchaseDocument(wire: PurchaseDocumentWire): PurchaseDocumentView {
  return {
    id: wire.id,
    number: wire.number,
    docType: wire.doc_type,
    status: wire.status,
    supplierId: wire.supplier_id,
    warehouseId: wire.warehouse_id,
    sourceDocumentId: wire.source_document_id,
    totalCentimes: wire.total_centimes,
    lines: wire.lines.map(toPurchaseLine),
    createdAt: wire.created_at,
    createdBy: wire.created_by,
  };
}

function toLedgerEntry(wire: SupplierLedgerEntryWire): SupplierLedgerEntry {
  return {
    id: wire.id,
    entryType: wire.entry_type,
    amountCentimes: wire.amount_centimes,
    referenceType: wire.reference_type,
    referenceId: wire.reference_id,
    description: wire.description,
    createdAt: wire.created_at,
    createdBy: wire.created_by,
    balanceAfterCentimes: wire.balance_after_centimes,
  };
}

function purchaseLineWire(line: PurchaseLineInput) {
  return {
    product_id: line.productId,
    unit_id: line.unitId,
    quantity_micro: line.quantityMicro,
    unit_cost_centimes: line.unitCostCentimes,
  };
}

/** Register a supplier (Gerant/Admin). */
export async function ipcCreateSupplier(
  sessionToken: string,
  input: SupplierFormInput,
): Promise<SupplierView> {
  const wire = await call<SupplierWire>("suppliers_create", {
    request: {
      session_token: sessionToken,
      name: input.name,
      phone: input.phone,
      address: input.address,
      nif: input.nif,
      rc: input.rc,
      notes: input.notes,
    },
  });
  return toSupplier(wire);
}

/** Sparse supplier update (`null` clears a field; omitted keeps it). */
export async function ipcUpdateSupplier(
  sessionToken: string,
  supplierId: string,
  input: SupplierUpdateInput,
): Promise<SupplierView> {
  const wire = await call<SupplierWire>("suppliers_update", {
    request: {
      session_token: sessionToken,
      supplier_id: supplierId,
      name: input.name,
      phone: input.phone,
      address: input.address,
      nif: input.nif,
      rc: input.rc,
      notes: input.notes,
      is_active: input.isActive,
    },
  });
  return toSupplier(wire);
}

/** Full supplier view by id. */
export async function ipcGetSupplier(
  sessionToken: string,
  supplierId: string,
): Promise<SupplierView> {
  const wire = await call<SupplierWire>("suppliers_get", {
    request: { session_token: sessionToken, id: supplierId },
  });
  return toSupplier(wire);
}

/** Dual-track safe supplier deletion: purge when unreferenced, deactivate on history. */
export async function ipcDeleteSupplier(
  sessionToken: string,
  supplierId: string,
  reason: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("suppliers_delete", {
    request: { session_token: sessionToken, id: supplierId, reason },
  });
}

/** Supplier directory search (blank query returns no rows). */
export async function ipcSearchSuppliers(
  sessionToken: string,
  query: string,
  limit: number,
): Promise<SupplierView[]> {
  interface SearchWire {
    suppliers: SupplierWire[];
  }
  const wire = await call<SearchWire>("suppliers_search", {
    request: { session_token: sessionToken, query, limit },
  });
  return wire.suppliers.map(toSupplier);
}

/** List suppliers (active only). */
export async function ipcListSuppliers(sessionToken: string): Promise<SupplierView[]> {
  interface ListWire {
    suppliers: SupplierWire[];
  }
  const wire = await call<ListWire>("suppliers_list", {
    session_token: sessionToken,
  });
  return wire.suppliers.map(toSupplier);
}

/** Supplier statement with server-side running balances. */
export async function ipcGetSupplierStatement(
  sessionToken: string,
  supplierId: string,
): Promise<SupplierStatement> {
  const wire = await call<SupplierStatementWire>("suppliers_get_statement", {
    request: { session_token: sessionToken, id: supplierId },
  });
  return {
    entries: wire.entries.map(toLedgerEntry),
    currentBalanceCentimes: wire.current_balance_centimes,
  };
}

/** Create a DRAFT purchase order (BC). */
export async function ipcCreatePurchaseOrder(
  sessionToken: string,
  commandId: string,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_create_order", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
    },
  });
  return toPurchaseDocument(wire);
}

/** Replace the line set of a DRAFT purchase order. */
export async function ipcUpdatePurchaseOrder(
  sessionToken: string,
  commandId: string,
  documentId: string,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_update_order", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      document_id: documentId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
    },
  });
  return toPurchaseDocument(wire);
}

/** Receive goods (BR); optional source BC id, null for standalone. */
export async function ipcReceiveGoods(
  sessionToken: string,
  commandId: string,
  sourceBcId: string | null,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_receive_goods", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      source_bc_id: sourceBcId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
    },
  });
  return toPurchaseDocument(wire);
}

/** Receive a matrix size run through default units (DSN-109); one atomic BR. */
export async function ipcReceiveMatrixGoods(
  sessionToken: string,
  commandId: string,
  sourceBcId: string | null,
  supplierId: string,
  warehouseId: string,
  modelId: string,
  cells: MatrixReceiveCellInput[],
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_receive_matrix_goods", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      source_bc_id: sourceBcId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      model_id: modelId,
      cells: cells.map((cell) => ({
        axis1: cell.axis1,
        axis2: cell.axis2,
        quantity_micro: cell.quantityMicro,
        unit_cost_centimes: cell.unitCostCentimes,
      })),
    },
  });
  return toPurchaseDocument(wire);
}

/** One size cell under receipt (quantities as micro strings, cost as centime string). */
export interface MatrixReceiveCellInput {
  axis1: string;
  axis2: string;
  quantityMicro: string;
  unitCostCentimes: string;
}

/** One receipt line under cost review. */
export interface ReceiveCostLineInput {
  productId: string;
  unitId: string;
  quantityMicro: string;
  unitCostCentimes: string;
}

/** Advisory deviation verdict for one receipt line (never blocks). */
export interface ReceiveDeviation {
  productId: string;
  lastCostCentimes: string | null;
  average90CostCentimes: string | null;
  deltaVsLastBps: number | null;
  deltaVsAvgBps: number | null;
  flagged: boolean;
}

interface ReceiveDeviationWire {
  product_id: string;
  last_cost_centimes: string | null;
  average90_cost_centimes: string | null;
  delta_vs_last_bps: number | null;
  delta_vs_avg_bps: number | null;
  flagged: boolean;
}

/** Advisory cost review (DSN-109, proposal 7); read-only, Gerant/Admin only. */
export async function ipcPreviewReceiveDeviations(
  sessionToken: string,
  lines: ReceiveCostLineInput[],
  thresholdBps: number,
): Promise<ReceiveDeviation[]> {
  const wire = await call<ReceiveDeviationWire[]>("purchases_preview_receive_deviations", {
    request: {
      session_token: sessionToken,
      lines: lines.map((line) => ({
        product_id: line.productId,
        unit_id: line.unitId,
        quantity_micro: line.quantityMicro,
        unit_cost_centimes: line.unitCostCentimes,
      })),
      threshold_bps: thresholdBps,
    },
  });
  return wire.map((verdict) => ({
    productId: verdict.product_id,
    lastCostCentimes: verdict.last_cost_centimes,
    average90CostCentimes: verdict.average90_cost_centimes,
    deltaVsLastBps: verdict.delta_vs_last_bps,
    deltaVsAvgBps: verdict.delta_vs_avg_bps,
    flagged: verdict.flagged,
  }));
}

/** Record a supplier invoice (FF); optional source BR (ADR-029). */
export async function ipcRecordSupplierInvoice(
  sessionToken: string,
  commandId: string,
  sourceBrId: string | null,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
  invoiceTotalCentimes: string,
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_record_invoice", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      source_br_id: sourceBrId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
      invoice_total_centimes: invoiceTotalCentimes,
    },
  });
  return toPurchaseDocument(wire);
}

/** Record a supplier payment (CASH or OTHER, ADR-030). */
export async function ipcRecordSupplierPayment(
  sessionToken: string,
  commandId: string,
  supplierId: string,
  amountCentimes: string,
  paymentMethod: "CASH" | "OTHER",
  description: string | null,
): Promise<SupplierPaymentOutcome> {
  const wire = await call<SupplierPaymentWire>("purchases_record_payment", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      supplier_id: supplierId,
      amount_centimes: amountCentimes,
      payment_method: paymentMethod,
      description,
    },
  });
  return {
    ledgerEntryId: wire.ledger_entry_id,
    newBalanceCentimes: wire.new_balance_centimes,
    cashMovementId: wire.cash_movement_id,
  };
}

/** Load any purchase document (BC/BR/FF) by id. */
export async function ipcGetPurchaseDocument(
  sessionToken: string,
  documentId: string,
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_get_document", {
    request: { session_token: sessionToken, id: documentId },
  });
  return toPurchaseDocument(wire);
}
