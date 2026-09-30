/**
 * Supplier & purchase wire types (TASK-BRK-011).
 *
 * Money travels as integer-centime strings; quantities as integer-micro
 * strings; ids as UUID strings. The `number` type never carries money here.
 */

/** Supplier master-data view. */
export interface SupplierView {
  id: string;
  name: string;
  phone: string | null;
  address: string | null;
  nif: string | null;
  rc: string | null;
  notes: string | null;
  isActive: boolean;
  /** Owed-to-supplier cached balance (centimes string, may be negative). */
  currentBalanceCentimes: string;
  /** Terminal that created the record (ADR-028). */
  originDeviceId: string;
  createdAt: string;
  updatedAt: string;
}

/** Creation payload for the supplier form modal. */
export interface SupplierFormInput {
  name: string;
  phone: string | null;
  address: string | null;
  nif: string | null;
  rc: string | null;
  notes: string | null;
}

/** Sparse update payload (`undefined` keeps, `null` clears). */
export interface SupplierUpdateInput {
  name?: string;
  phone?: string | null;
  address?: string | null;
  nif?: string | null;
  rc?: string | null;
  notes?: string | null;
  isActive?: boolean;
}

/** One supplier ledger entry with its server-side running balance. */
export interface SupplierLedgerEntry {
  id: string;
  entryType: string;
  amountCentimes: string;
  referenceType: string;
  referenceId: string;
  description: string | null;
  createdAt: string;
  createdBy: string;
  balanceAfterCentimes: string;
}

/** Statement response (ledger rows + cached balance mirror). */
export interface SupplierStatement {
  entries: SupplierLedgerEntry[];
  currentBalanceCentimes: string;
}

/** One purchase line as submitted (all integer strings). */
export interface PurchaseLineInput {
  productId: string;
  unitId: string;
  quantityMicro: string;
  unitCostCentimes: string;
}

/** One stored purchase line view. */
export interface PurchaseDocumentLine {
  id: string;
  productId: string;
  unitId: string;
  quantityMicro: string;
  quantityBaseMicro: string;
  unitCostCentimes: string;
  lineTotalCentimes: string;
}

/** Purchase document view (BC / BR / FF). */
export interface PurchaseDocumentView {
  id: string;
  number: string;
  docType: string;
  status: string;
  supplierId: string;
  warehouseId: string;
  sourceDocumentId: string | null;
  totalCentimes: string;
  lines: PurchaseDocumentLine[];
  createdAt: string;
  createdBy: string;
}

/** Payment outcome (ADR-030: cash movement id only for CASH). */
export interface SupplierPaymentOutcome {
  ledgerEntryId: string;
  newBalanceCentimes: string;
  cashMovementId: string | null;
}
