//! Supplier and purchase DTOs: suppliers, orders, receipts, invoices.
use serde::{Deserialize, Serialize};

// Supplier / purchase wire types (TASK-BRK-011) ---------------------------
// Money travels as integer-centime strings; quantities as integer-micro
// strings; ids as UUID strings. No float ever crosses this boundary.

/// Stored supplier view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierDto {
    /// Supplier id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Address, if registered.
    pub address: Option<String>,
    /// NIF (tax registration), if registered.
    pub nif: Option<String>,
    /// RC (registre de commerce), if registered.
    pub rc: Option<String>,
    /// Notes, if any.
    pub notes: Option<String>,
    /// Active flag.
    pub is_active: bool,
    /// Cached balance in centimes (owed to supplier).
    pub current_balance_centimes: String,
    /// Terminal that created the record (ADR-028).
    pub origin_device_id: String,
    /// Row creation instant (RFC3339).
    pub created_at: String,
    /// Row update instant (RFC3339).
    pub updated_at: String,
}

/// `suppliers.create` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSupplierRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Display name (mandatory).
    pub name: String,
    /// Optional phone.
    pub phone: Option<String>,
    /// Optional address.
    pub address: Option<String>,
    /// Optional NIF.
    pub nif: Option<String>,
    /// Optional RC.
    pub rc: Option<String>,
    /// Optional notes.
    pub notes: Option<String>,
}

/// `suppliers.update` request (only `Some` fields are written).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSupplierRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Replacement display name.
    pub name: Option<String>,
    /// Replacement phone (`Some(None)` clears it).
    pub phone: Option<Option<String>>,
    /// Replacement address (`Some(None)` clears it).
    pub address: Option<Option<String>>,
    /// Replacement NIF (`Some(None)` clears it).
    pub nif: Option<Option<String>>,
    /// Replacement RC (`Some(None)` clears it).
    pub rc: Option<Option<String>>,
    /// Replacement notes (`Some(None)` clears it).
    pub notes: Option<Option<String>>,
    /// Replacement active flag.
    pub is_active: Option<bool>,
}

/// `suppliers.get` / `suppliers.getStatement` / `purchases.getDocument` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierIdRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Target id (supplier or document, per command).
    pub id: String,
}

/// `suppliers.search` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSuppliersRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Prefix/substring query (blank returns no rows).
    pub query: String,
    /// Max rows (clamped 1..=100).
    pub limit: Option<u32>,
}

/// `suppliers.search` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSuppliersResponseDto {
    /// Matching suppliers ordered by name.
    pub suppliers: Vec<SupplierDto>,
}

/// One supplier ledger entry with its server-side running balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierLedgerEntryDto {
    /// Entry id.
    pub id: String,
    /// `DEBIT` or `CREDIT`.
    pub entry_type: String,
    /// Positive magnitude in centimes.
    pub amount_centimes: String,
    /// `PURCHASE_INVOICE` or `PAYMENT`.
    pub reference_type: String,
    /// Producing document/entry id.
    pub reference_id: String,
    /// Optional description.
    pub description: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator id.
    pub created_by: String,
    /// Running balance after this entry, in centimes.
    pub balance_after_centimes: String,
}

/// `suppliers.getStatement` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierStatementDto {
    /// Ledger rows in ledger order.
    pub entries: Vec<SupplierLedgerEntryDto>,
    /// Cached supplier balance for cross-verification.
    pub current_balance_centimes: String,
}

/// One purchase line as submitted (BC/BR/FF payloads).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseLineRequestDto {
    /// Product id.
    pub product_id: String,
    /// Handling unit id.
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Estimated or actual unit cost in centimes.
    pub unit_cost_centimes: String,
}

/// One stored purchase line view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseOrderLineDto {
    /// Stored item id.
    pub id: String,
    /// Product id.
    pub product_id: String,
    /// Handling unit id.
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Quantity in base-unit micros.
    pub quantity_base_micro: String,
    /// Unit cost in centimes.
    pub unit_cost_centimes: String,
    /// Line total in centimes.
    pub line_total_centimes: String,
}

/// Shared purchase-document response shape for BC/BR/FF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseOrderDto {
    /// Document id.
    pub id: String,
    /// Gapless human number.
    pub number: String,
    /// Document type code (`BC`, `BR`, `FF`).
    pub doc_type: String,
    /// Status code.
    pub status: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Predecessor document id, if any.
    pub source_document_id: Option<String>,
    /// Document total in centimes.
    pub total_centimes: String,
    /// Ordered lines.
    pub lines: Vec<PurchaseOrderLineDto>,
    /// Creation instant (RFC3339).
    pub created_at: String,
    /// Creator operator id.
    pub created_by: String,
}

/// `purchases.receiveGoods` response (BR document).
pub type GoodsReceiptDto = PurchaseOrderDto;

/// `purchases.recordInvoice` response (FF document).
pub type SupplierInvoiceDto = PurchaseOrderDto;

/// `purchases.createOrder` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePurchaseOrderRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Receiving warehouse id.
    pub warehouse_id: String,
    /// Ordered lines.
    pub lines: Vec<PurchaseLineRequestDto>,
}

/// `purchases.updateOrder` request (DRAFT only; replaces the line set).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePurchaseOrderRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// BC document id.
    pub document_id: String,
    /// Supplier id (must own the BC).
    pub supplier_id: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Replacement line set.
    pub lines: Vec<PurchaseLineRequestDto>,
}

/// `purchases.receiveGoods` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiveGoodsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Source BC id for a receipt against an order (None = standalone).
    pub source_bc_id: Option<String>,
    /// Supplier id.
    pub supplier_id: String,
    /// Receiving warehouse id.
    pub warehouse_id: String,
    /// Received lines.
    pub lines: Vec<PurchaseLineRequestDto>,
}

/// `purchases.previewReceiveDeviations` request (DSN-109, proposal 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewReceiveDeviationsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Receipt lines under cost review.
    pub lines: Vec<ReceiveCostLineRequestDto>,
    /// Flag threshold in basis points (e.g. 1000 = 10%).
    pub threshold_bps: i64,
}

/// One receipt line under cost review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiveCostLineRequestDto {
    /// Product id.
    pub product_id: String,
    /// Handling unit id.
    pub unit_id: String,
    /// Quantity in unit micros.
    pub quantity_micro: String,
    /// Proposed unit cost in centimes.
    pub unit_cost_centimes: String,
}

/// Advisory deviation verdict for one receipt line (never blocks).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiveDeviationDto {
    /// Product id.
    pub product_id: String,
    /// Last posted receipt cost, if any history exists.
    pub last_cost_centimes: Option<String>,
    /// 90-day average receipt cost, if any history exists.
    pub average90_cost_centimes: Option<String>,
    /// Signed drift vs last in basis points, if a last cost exists.
    pub delta_vs_last_bps: Option<i64>,
    /// Signed drift vs the 90-day average in basis points, if it exists.
    pub delta_vs_avg_bps: Option<i64>,
    /// True only when the cost beats both references by more than the
    /// threshold. No history never flags.
    pub flagged: bool,
}

/// `purchases.receiveMatrixGoods` request (DSN-109): a size run lands
/// through the model's default selling units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiveMatrixGoodsRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Receiving warehouse id.
    pub warehouse_id: String,
    /// Variant model id the cells belong to.
    pub model_id: String,
    /// Source BC id for a receipt against an order (None = standalone).
    pub source_bc_id: Option<String>,
    /// Non-empty cell set.
    pub cells: Vec<MatrixReceiveCellRequestDto>,
}

/// One size cell: quantity in default-unit micros, cost per that unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixReceiveCellRequestDto {
    /// First-axis value (e.g. `"M"`).
    pub axis1: String,
    /// Second-axis value (e.g. `"Rouge"`).
    pub axis2: String,
    /// Quantity in default-selling-unit micros.
    pub quantity_micro: String,
    /// Unit cost in centimes per default unit.
    pub unit_cost_centimes: String,
}

/// `purchases.recordInvoice` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSupplierInvoiceRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Source BR id (None = standalone combined receipt+invoice, ADR-029).
    pub source_br_id: Option<String>,
    /// Supplier id.
    pub supplier_id: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Invoice lines (must match the BR exactly when sourced).
    pub lines: Vec<PurchaseLineRequestDto>,
    /// Financial total in centimes (the ledger DEBIT amount).
    pub invoice_total_centimes: String,
}

/// `purchases.recordPayment` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSupplierPaymentRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
    /// Supplier id.
    pub supplier_id: String,
    /// Payment amount in centimes (must be positive).
    pub amount_centimes: String,
    /// `CASH` (drawer outflow, session required) or `OTHER` (ADR-030).
    pub payment_method: String,
    /// Optional description.
    pub description: Option<String>,
}

/// `purchases.recordPayment` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierPaymentOutcomeDto {
    /// Stored supplier_ledger entry id.
    pub ledger_entry_id: String,
    /// Supplier cached balance after the payment, in centimes.
    pub new_balance_centimes: String,
    /// Drawer movement id (CASH payments only, ADR-030).
    pub cash_movement_id: Option<String>,
}
