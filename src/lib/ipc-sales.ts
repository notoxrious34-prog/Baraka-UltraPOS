/**
 * Sales bridge: stock reads, checkout, totals preview, drafts, conversions,
 * document search/detail, and returns.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type { ActiveCustomer, DocType, PaymentMethodCode, SaleReceipt } from "@/types/pos";
import { call } from "./ipc-kernel";

/** Read a stock level in base micros (`"0"` when absent). */
export async function ipcGetStock(
  sessionToken: string,
  productId: string,
  warehouseId: string,
): Promise<string> {
  interface StockWire {
    product_id: string;
    warehouse_id: string;
    base_unit_id: string;
    quantity_base_micro: string;
  }
  const wire = await call<StockWire>("stock_get", {
    request: {
      session_token: sessionToken,
      product_id: productId,
      warehouse_id: warehouseId,
    },
  });
  return wire.quantity_base_micro;
}

/** Manual stock correction payload (mirrors AdjustStockRequestDto). */
export interface StockAdjustPayload {
  session_token: string;
  product_id: string;
  warehouse_id: string;
  unit_id: string;
  /** Positive magnitude in selling-unit micros. */
  quantity_micro: string;
  /** Explicit direction; sign is never inferred. */
  direction: "INCREASE" | "DECREASE";
  /** Human reason (min 3 chars after trim, enforced server-side). */
  reason: string;
  /** Allow a negative resulting level (override permission required). */
  authorize_negative_stock: boolean;
}

/** Adjustment outcome (mirrors AdjustStockResponseDto). */
export interface StockAdjustResult {
  movement_id: string;
  new_quantity_base_micro: string;
}

/**
 * Apply one manual stock correction. Rejects with the backend wire code
 * verbatim (UNAUTHORIZED, INVALID_QUANTITY, ADJUSTMENT_REASON_REQUIRED,
 * STOCK_NEGATIVE_NOT_AUTHORIZED, STOCK_NEGATIVE_CONFIRMATION_REQUIRED, …).
 */
export async function adjustStock(payload: StockAdjustPayload): Promise<StockAdjustResult> {
  return call<StockAdjustResult>("stock_adjust", {
    request: { ...payload },
  });
}

export interface CheckoutLineWire {
  product_id: string;
  unit_id: string;
  quantity_micro: string;
  unit_price_centimes: string | null;
  line_discount_centimes: string;
}

export interface CheckoutPaymentWire {
  payment_method: PaymentMethodCode;
  amount_centimes: string;
}

export interface CheckoutRequestWire {
  command_id: string;
  session_token: string;
  warehouse_id: string;
  customer_id: string | null;
  document_type: DocType;
  /** Explicit sale-time receipt language (no silent default). */
  document_language: "AR" | "FR";
  lines: CheckoutLineWire[];
  global_discount_centimes: string;
  payments: CheckoutPaymentWire[];
  /**
   * Explicit in-session sale-on-shortage confirmation (TASK-BRK-033).
   * Privileged roles (ADMIN/GERANT) only; the backend ignores it otherwise.
   */
  allow_negative_stock: boolean;
  notes: string | null;
  /**
   * Manager PIN override unlocking below-cost lines for this sale only
   * (DSN-115). `null` locks every red line. The PIN modal (manager path)
   * fills this; the cashier path always sends `null`.
   */
  below_cost_override: { username: string; pin: string } | null;
  /**
   * Cart-opened moment (ISO, client-observed) pinning the fiscal year
   * (XS). `null` falls back to the commit moment.
   */
  cart_opened_at: string | null;
}

/** Commit a sale atomically; rejects with a backend wire code on failure. */
export async function ipcCheckout(request: CheckoutRequestWire): Promise<SaleReceipt> {
  interface ReceiptWire {
    document_id: string;
    document_number: string;
    subtotal_ht_centimes: string;
    tva_total_centimes: string;
    discount_total_centimes: string;
    timbre_centimes: string;
    total_ttc_centimes: string;
    paid_amount_centimes: string;
    change_amount_centimes: string;
    print_job_id: string;
  }
  const wire = await call<ReceiptWire>("sales_checkout", { request });
  return {
    document_id: wire.document_id,
    document_number: wire.document_number,
    subtotal_ht_centimes: wire.subtotal_ht_centimes,
    tva_total_centimes: wire.tva_total_centimes,
    discount_total_centimes: wire.discount_total_centimes,
    timbre_centimes: wire.timbre_centimes,
    total_ttc_centimes: wire.total_ttc_centimes,
    paid_amount_centimes: wire.paid_amount_centimes,
    change_amount_centimes: wire.change_amount_centimes,
    print_job_id: wire.print_job_id,
  };
}

/** One cart line to explode on the prep sheet (mirrors PrepSheetLineDto). */
export interface PrepSheetLineWire {
  product_id: string;
  unit_id: string;
  quantity_micro: string;
}

/** One whole-rung checklist share (mirrors ColisageShareDto). Counts are integers, NOT money. */
export interface PrepSheetShareWire {
  unit_id: string;
  unit_name_fr: string;
  unit_name_ar: string;
  factor_micro: number;
  count: number;
}

/** One exploded checklist line — NO price field exists by design. */
export interface PrepSheetLineResult {
  product_id: string;
  sku: string;
  name_ar: string;
  name_fr: string;
  quantity_base_micro: string;
  shares: PrepSheetShareWire[];
}

/** Build the priceless warehouse checklist (Bon de Préparation, BRK-126). */
export async function ipcBuildPrepSheet(
  sessionToken: string,
  lines: PrepSheetLineWire[],
): Promise<PrepSheetLineResult[]> {
  return call<PrepSheetLineResult[]>("sales_build_prep_sheet", {
    request: { session_token: sessionToken, lines },
  });
}

/** One leaf-level stock shortage (mirrors StockShortageWarningDto). */
export interface StockShortageWarningWire {
  product_id: string;
  product_name: string;
  sku: string;
  available_qty_micro: string;
  requested_qty_micro: string;
}

/** Margin Guard readout for one preview line (mirrors LineMarginDto). */
export interface LineMarginWire {
  line_index: number;
  margin_basis_points: number;
  is_below_cost: boolean;
}

/** Commit-identical totals preview (mirrors PreviewTotalsResponse). */
export interface PreviewTotalsWire {
  subtotal_ht_centimes: string;
  tva_total_centimes: string;
  global_discount_centimes: string;
  timbre_centimes: string;
  total_ttc_centimes: string;
  paid_amount_centimes: string;
  change_amount_centimes: string;
  remaining_due_centimes: string;
  /** Leaf-level shortages against the requested warehouse (advisory). */
  stock_warnings: StockShortageWarningWire[];
  /** Per-line Margin Guard readouts in cart order (advisory). */
  line_margins: LineMarginWire[];
}

/** Totals preview request body (same cart shape as checkout). */
export interface PreviewTotalsRequestWire {
  warehouse_id: string;
  document_type: DocType;
  lines: CheckoutLineWire[];
  global_discount_centimes: string;
  payments: CheckoutPaymentWire[];
  /** Sale customer id for tier pricing (`null` = retail guest). */
  customer_id: string | null;
}

/**
 * Read-only, commit-identical totals preview (TASK-BRK-031).
 * Prices the cart through the same Rust kernel as `sales_checkout`, so
 * the previewed totals can never diverge from the committed ones. The
 * checkout modal renders ONLY these values — zero client-side totals
 * arithmetic exists in the checkout flow.
 */
export async function ipcPreviewTotals(
  sessionToken: string,
  request: PreviewTotalsRequestWire,
): Promise<PreviewTotalsWire> {
  return call<PreviewTotalsWire>("sales_preview_totals", {
    request: { session_token: sessionToken, ...request },
  });
}

/** Park the current cart as an editable draft. Returns the draft id. */
export async function ipcCreateDraft(
  sessionToken: string,
  warehouseId: string,
  customer: ActiveCustomer,
  lines: CheckoutLineWire[],
  notes: string | null,
  documentLanguage: "AR" | "FR",
): Promise<string> {
  interface DraftWire {
    draft_id: string;
    created_at: string;
  }
  const wire = await call<DraftWire>("sales_create_draft", {
    request: {
      session_token: sessionToken,
      warehouse_id: warehouseId,
      customer_id: customer.kind === "named" ? customer.id : null,
      document_language: documentLanguage,
      lines,
      notes,
    },
  });
  return wire.draft_id;
}

/** Discard a parked draft (DRAFT -> CANCELLED). */
export async function ipcDiscardDraft(
  sessionToken: string,
  draftId: string,
): Promise<void> {
  return call("sales_discard_draft", {
    request: {
      session_token: sessionToken,
      draft_id: draftId,
    },
  });
}

/** DEVIS-to-BL conversion request (mirrors ConvertDevisToBlRequest). */
export interface ConvertDevisToBlWireRequest {
  session_token: string;
  command_id: string;
  source_document_id: string;
}

/** DEVIS-to-BL conversion response (mirrors ConvertDevisToBlResponse). */
export interface ConvertDevisToBlWireResponse {
  new_document_id: string;
  new_document_number: string;
  new_document_status: string;
  total_ttc_centimes: string;
  items_count: number;
  warnings: string[];
}

/**
 * Convert a FINALIZED DEVIS into a FINALIZED BL (stock deducted, source
 * sealed CONVERTED). Idempotent via command_id. Rejects with the backend
 * wire code verbatim (SOURCE_DOCUMENT_NOT_FOUND,
 * INVALID_CONVERSION_SOURCE_TYPE/STATUS, DOCUMENT_ALREADY_CONVERTED,
 * EMPTY_SOURCE_DOCUMENT, INSUFFICIENT_PAYMENT family, UNAUTHORIZED, …).
 */
export async function convertDevisToBl(
  request: ConvertDevisToBlWireRequest,
): Promise<ConvertDevisToBlWireResponse> {
  return call<ConvertDevisToBlWireResponse>("convert_devis_to_bl", {
    request: { ...request },
  });
}

/** BL-to-FACTURE closing tender line (mirrors PaymentLineDto). */
export interface ConvertBlToFactureTenderWire {
  payment_method: "CASH" | "CREDIT" | "CHEQUE" | "CARD";
  amount_centimes: string;
}

/** BL-to-FACTURE closing request (mirrors ConvertBlToFactureRequest). */
export interface ConvertBlToFactureWireRequest {
  session_token: string;
  command_id: string;
  source_document_id: string;
  document_language: "AR" | "FR";
  payments: ConvertBlToFactureTenderWire[];
}

/** BL-to-FACTURE closing response (mirrors ConvertBlToFactureResponse). */
export interface ConvertBlToFactureWireResponse {
  new_document_id: string;
  new_document_number: string;
  new_document_status: string;
  total_ttc_centimes: string;
  items_count: number;
  customer_new_balance_centimes: string | null;
}

/**
 * Close a FINALIZED BL into a FINALIZED FACTURE with exact tender
 * settlement (no change concept; zero stock writes). Idempotent via
 * command_id. Rejects with the backend wire code verbatim
 * (INSUFFICIENT_PAYMENT on tender mismatch, EMPTY_CART on empty tenders,
 * CREDIT_LIMIT_EXCEEDED, NO_OPEN_CASH_SESSION_FOR_TERMINAL,
 * DOCUMENT_ALREADY_CONVERTED, UNAUTHORIZED, …).
 */
export async function convertBlToFacture(
  request: ConvertBlToFactureWireRequest,
): Promise<ConvertBlToFactureWireResponse> {
  return call<ConvertBlToFactureWireResponse>("convert_bl_to_facture", {
    request: { ...request },
  });
}

/** Document search request (mirrors DocumentSearchRequest). */
export interface DocumentSearchWireRequest {
  session_token: string;
  document_type: string | null;
  status: string | null;
  query: string | null;
  limit: number | null;
}

/** Document summary row (mirrors DocumentSummaryDto). */
export interface DocumentSummaryWireDto {
  id: string;
  document_number: string;
  document_type: string;
  status: string;
  party_id: string | null;
  party_name: string | null;
  total_ttc_centimes: string;
  items_count: number;
  created_at: string;
}

/** Document search response (mirrors DocumentSearchResponse). */
export interface DocumentSearchWireResponse {
  documents: DocumentSummaryWireDto[];
}

/**
 * Search document headers by kind/status with partial human-number or
 * customer-name match (newest first, server-clamped limit). Rejects with
 * the backend wire code verbatim (UNAUTHORIZED, INVALID_INPUT, …).
 */
export async function searchDocuments(
  request: DocumentSearchWireRequest,
): Promise<DocumentSearchWireResponse> {
  return call<DocumentSearchWireResponse>("documents_search", {
    request: { ...request },
  });
}

/** Full document view (mirrors GetSaleResponse from sales_get). */
export interface DocumentDetailsWireResponse {
  document: {
    document_id: string;
    number: string;
    doc_type: string;
    status: string;
    warehouse_id: string;
    subtotal_ht_centimes: string;
    /** Stored global discount (DOC-05: engine `Remise totale` row). */
    global_discount_centimes: string;
    tva_total_centimes: string;
    total_ttc_centimes: string;
    paid_amount_centimes: string;
    change_amount_centimes: string;
    /**
     * Frozen wholesale credit snapshot (BRK-108 engine columns, DSN-103 print).
     * `null` = no balance movement (cash-only) or legacy row — surfaces must
     * render the equation block absent, never zero.
     */
    credit_solde_before_centimes: string | null;
    /** Balance after this document (`null` = absent, see above). */
    credit_solde_after_centimes: string | null;
    /** Signed movement (`+` credit sale, `-` AVOIR refund; `null` = absent). */
    credit_delta_centimes: string | null;
    /** Capture instant, stored verbatim (DOC-05). */
    created_at: string;
    /** Acting operator id, stored verbatim (DOC-05 engine cashier row). */
    created_by: string;
    payment_status: string;
    /** Pinned layout version (DOC-04): v1 frozen, v2 DOC-07 redesign. */
    template_version: number;
    /** Resolved operator login name (DOC-07) — shown, never the raw id. */
    created_by_name: string;
    /** Resolved depot name (DOC-07) — same orphan contract. */
    warehouse_name: string;
    /** Wall-clock instant `DD/MM/YYYY HH:MM` (DOC-07) — shown as-is. */
    created_at_display: string;
  };
  items: Array<{
    product_id: string;
    unit_id: string;
    quantity_micro: string;
    unit_price_centimes: string;
    /** Stored per-unit discount (`null` = legacy line, pre-0018). */
    discount_per_unit_centimes: string | null;
    /** Exact `unit_price - per_unit` (`null` = legacy line: show raw PU). */
    discounted_unit_price_centimes: string | null;
    line_tva_centimes: string;
    line_ttc_centimes: string;
    /** Commercial designation (FR); null when the product row is gone. */
    product_name_fr: string | null;
    /** Commercial designation (AR); null when the product row is gone. */
    product_name_ar: string | null;
    /** Stock-keeping reference; null when the product row is gone. */
    sku: string | null;
    /** Selling-unit label (FR); null when the unit row is gone. */
    unit_name_fr: string | null;
    /** Selling-unit label (AR); null when the unit row is gone. */
    unit_name_ar: string | null;
    /** True when the product row is missing (archived marker). */
    product_missing: boolean;
    /**
     * Whole-rung Colisage breakdown, largest rung first (BRK-109).
     * Empty = no whole-rung expression (show base quantity);
     * null = detail predates BRK-109 (absence, never zero).
     */
    colisage: Array<{
      unit_id: string;
      unit_name_fr: string;
      unit_name_ar: string;
      factor_micro: number;
      count: number;
    }> | null;
  }>;
  payments: Array<{
    method_code: string;
    amount_centimes: string;
  }>;
  print_status: string;
}

/**
 * Fetch a full document view by UUID via the existing sales_get command.
 * Note: sales_get takes bare scalar params (no nested request object);
 * the session token is injected at the top level by the bridge edge.
 */
export async function getDocumentDetails(
  documentId: string,
): Promise<DocumentDetailsWireResponse> {
  return call<DocumentDetailsWireResponse>("sales_get", {
    document_id: documentId,
  });
}

/** One returnable source line (mirrors ReturnableLineDto). */
export interface ReturnableLine {
  source_item_id: string;
  product_id: string;
  product_name: string;
  sku: string;
  unit_id: string;
  unit_name: string;
  original_qty_micro: string;
  already_returned_qty_micro: string;
  remaining_qty_micro: string;
  unit_price_cents: string;
  net_unit_price_cents: string;
  tva_rate: string;
}

/** Returnable source summary (mirrors ReturnableDocumentSummaryDto). */
export interface ReturnableDocumentSummary {
  source_document_id: string;
  source_document_number: string;
  document_type: string;
  party_type: string | null;
  party_id: string | null;
  party_name: string | null;
  warehouse_id: string;
  total_ht_cents: string;
  total_ttc_cents: string;
  is_eligible_for_return: boolean;
  items: ReturnableLine[];
}

/**
 * Fetch the returnable state of one source document by UUID.
 * Note: like sales_get, this command takes bare scalar params.
 */
export async function getReturnableDocumentSummary(
  sourceDocumentId: string,
): Promise<ReturnableDocumentSummary> {
  return call<ReturnableDocumentSummary>("returns_get_source_summary", {
    source_document_id: sourceDocumentId,
  });
}

/** One returned line payload (mirrors ReturnItemPayloadDto). */
export interface ReturnItemInput {
  source_item_id: string;
  quantity_micro: string;
  reason: string | null;
}

/** Refund settlement selector (mirrors RefundDestinationDto). */
export type RefundDestination =
  | { type: "Cash"; payload: { cash_session_id: string } }
  | { type: "CustomerCredit"; payload: { customer_id: string } }
  | { type: "NoRefund" };

/** Return execution payload (mirrors ProcessReturnRequestDto). */
export interface ProcessReturnPayload {
  session_token: string;
  operator_id: string;
  source_document_id: string;
  terminal_id: string;
  items: ReturnItemInput[];
  refund: RefundDestination;
  idempotency_key: string;
  notes: string | null;
}

/** Return execution result (mirrors ProcessReturnResponseDto). */
export interface ProcessReturnResult {
  return_document_id: string;
  return_document_number: string;
  source_document_id: string;
  source_document_number: string;
  total_ht_cents: string;
  total_tva_cents: string;
  total_ttc_cents: string;
  refund_type: string;
  lines_count: number;
}

/**
 * Validate and atomically persist a merchandise return. Rejects with the
 * backend wire code verbatim (RETURN_QUANTITY_EXCEEDED,
 * CREDIT_CUSTOMER_REQUIRED, NO_OPEN_CASH_SESSION_FOR_TERMINAL,
 * UNAUTHORIZED, …).
 */
export async function processReturn(
  payload: ProcessReturnPayload,
): Promise<ProcessReturnResult> {
  return call<ProcessReturnResult>("returns_process", {
    request: { ...payload },
  });
}

/** One returnable source line, eligibility view (mirrors ReturnableItemDto). */
export interface ReturnableItem {
  source_item_id: string;
  product_id: string;
  product_name: string;
  barcode: string | null;
  unit_id: string;
  unit_name: string;
  unit_price_centimes: string;
  /** Stored per-unit discount (`null` = legacy line, pre-0018). */
  discount_per_unit_centimes: string | null;
  /** Exact `unit_price - per_unit` (`null` = legacy line: show raw PU). */
  discounted_unit_price_centimes: string | null;
  original_quantity_micro: string;
  already_returned_micro: string;
  remaining_allowable_micro: string;
  tva_rate: number;
  discount_centimes: string;
}

/** Return context for one source document (mirrors ReturnContextResponseDto). */
export interface ReturnContextResponse {
  source_document_id: string;
  document_number: string;
  document_type: string;
  party_id: string | null;
  party_name: string | null;
  warehouse_id: string;
  created_at: string;
  is_eligible: boolean;
  ineligibility_reason: string | null;
  items: ReturnableItem[];
}

/**
 * Fetch the return context of one source document by UUID (eligibility,
 * remaining ceilings, catalog labels). Like sales_get, this command takes
 * bare scalar params.
 */
export async function getDocumentReturnContext(
  sourceDocumentId: string,
): Promise<ReturnContextResponse> {
  return call<ReturnContextResponse>("documents_get_return_context", {
    source_document_id: sourceDocumentId,
  });
}

/** Simplified single-shot return payload (mirrors DocumentsProcessReturnRequestDto). */
export interface ProcessReturnRequest {
  session_token: string;
  source_document_id: string;
  refund_method: "CASH" | "CUSTOMER_CREDIT" | "NO_REFUND";
  items: Array<{
    source_item_id: string;
    quantity_micro: string;
  }>;
  notes: string | null;
}

/** Simplified single-shot return result (mirrors DocumentsProcessReturnResponseDto). */
export interface ProcessReturnResponse {
  return_document_id: string;
  return_document_number: string;
  total_refund_ttc_centimes: string;
  refund_method: string;
}

/**
 * Validate and atomically persist a merchandise return through the
 * simplified facade (operator/terminal/idempotency resolve at the edge).
 * Rejects with the dedicated return taxonomy verbatim
 * (RETURN_QUANTITY_EXCEEDED, INVALID_SOURCE_DOCUMENT_STATE,
 * NO_ACTIVE_CASH_SESSION, MISSING_CUSTOMER_FOR_CREDIT_REFUND,
 * UNAUTHORIZED, EMPTY_RETURN, …).
 */
export async function processSaleReturn(
  payload: ProcessReturnRequest,
): Promise<ProcessReturnResponse> {
  return call<ProcessReturnResponse>("documents_process_return", {
    request: { ...payload },
  });
}

/** One alternate size/color sold inside an exchange (mirrors ExchangeNewItemDto). */
export interface ExchangeNewItem {
  product_id: string;
  unit_id: string;
  quantity_micro: string;
}

/** Single-shot exchange payload (mirrors DocumentsProcessExchangeRequestDto). */
export interface ProcessExchangeRequest {
  session_token: string;
  source_document_id: string;
  return_items: Array<{
    source_item_id: string;
    quantity_micro: string;
  }>;
  new_items: ExchangeNewItem[];
  refund_method: "CASH" | "CUSTOMER_CREDIT" | "NO_REFUND";
  below_cost_override: { manager_username: string; manager_pin: string } | null;
  notes: string | null;
}

/** Single-shot exchange result (mirrors DocumentsProcessExchangeResponseDto). */
export interface ProcessExchangeResponse {
  return_document_id: string;
  return_document_number: string;
  sale_document_id: string;
  sale_document_number: string;
  return_ttc_centimes: string;
  sale_ttc_centimes: string;
  net_centimes: string;
}

/**
 * Validate and atomically persist a size/color exchange through the
 * simplified facade (BRK-125, Échange Express). Return legs reuse historic
 * prices, alternates replay last-price memory, the net settles exactly.
 * Rejects with the dedicated return taxonomy verbatim
 * (RETURN_QUANTITY_EXCEEDED, INVALID_SOURCE_DOCUMENT_STATE,
 * NO_ACTIVE_CASH_SESSION, MISSING_CUSTOMER_FOR_CREDIT_REFUND,
 * EXCHANGE_PRICE_UNKNOWN, EXCHANGE_REFUND_REQUIRED, …).
 */
export async function processSaleExchange(
  payload: ProcessExchangeRequest,
): Promise<ProcessExchangeResponse> {
  return call<ProcessExchangeResponse>("documents_process_exchange", {
    request: { ...payload },
  });
}
