//! Document DTOs: conversions, search, returns and credit notes.
use super::*;
use serde::{Deserialize, Serialize};

/// DEVIS-to-BL conversion request (ADR-040, TASK-BRK-014A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertDevisToBlRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key (UUID v4 string).
    pub command_id: String,
    /// Source DEVIS document id (UUID string).
    pub source_document_id: String,
}

/// DEVIS-to-BL conversion response (stored result shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertDevisToBlResponse {
    /// New BL document id.
    pub new_document_id: String,
    /// New BL gapless human number.
    pub new_document_number: String,
    /// New BL status code (`FINALIZED`).
    pub new_document_status: String,
    /// Copied grand total in centimes.
    pub total_ttc_centimes: String,
    /// Copied line count.
    pub items_count: u32,
    /// Per-line negative-stock override notices (empty when stock sufficed).
    pub warnings: Vec<String>,
}

/// BL-to-FACTURE financial-closing request (ADR-040/041/046, TASK-BRK-038).
///
/// Tender shape reuses the checkout tender DTO verbatim (same method codes,
/// same positive-amount contract); settlement must equal the source total
/// exactly — this closing carries no change concept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertBlToFactureRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key (UUID v4 string).
    pub command_id: String,
    /// Source BL document id (UUID string).
    pub source_document_id: String,
    /// Customer-facing receipt language for the FACTURE (`AR` or `FR`).
    pub document_language: String,
    /// Tender lines covering the source total exactly.
    pub payments: Vec<PaymentLineDto>,
}

/// BL-to-FACTURE financial-closing response (stored result shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertBlToFactureResponse {
    /// New FACTURE document id.
    pub new_document_id: String,
    /// New FACTURE gapless human number.
    pub new_document_number: String,
    /// New FACTURE status code (`FINALIZED`).
    pub new_document_status: String,
    /// Settled grand total in centimes (source copy).
    pub total_ttc_centimes: String,
    /// Copied line count.
    pub items_count: u32,
    /// Customer balance after settlement, when credit was used.
    pub customer_new_balance_centimes: Option<String>,
}

/// Read-only document search request (TASK-BRK-042, conversion lookup).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSearchRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Document kind filter (`DEVIS`, `BL`, `FACTURE`, `TICKET`).
    pub document_type: Option<String>,
    /// Lifecycle status filter (`FINALIZED`, `CONVERTED`, `DRAFT`, …).
    pub status: Option<String>,
    /// Partial match against human number or customer name.
    pub query: Option<String>,
    /// Max rows (default 20, clamped to 1..=100).
    pub limit: Option<u32>,
}

/// Read-only document summary row (all money as canonical strings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSummaryDto {
    /// Document id (UUID string).
    pub id: String,
    /// Gapless human number.
    pub document_number: String,
    /// Document kind code.
    pub document_type: String,
    /// Lifecycle status code.
    pub status: String,
    /// Owning customer id, if any.
    pub party_id: Option<String>,
    /// Owning customer name, if any.
    pub party_name: Option<String>,
    /// Grand total in centimes.
    pub total_ttc_centimes: String,
    /// Stored line count.
    pub items_count: u32,
    /// Creation instant (RFC3339).
    pub created_at: String,
}

/// Read-only document search response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSearchResponse {
    /// Matching summaries, newest first.
    pub documents: Vec<DocumentSummaryDto>,
}

/// One returnable source line (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnableLineDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned product id (UUID string).
    pub product_id: String,
    /// Product display name (FR).
    pub product_name: String,
    /// Stock-keeping reference.
    pub sku: String,
    /// Selling unit id (UUID string).
    pub unit_id: String,
    /// Selling unit display name (FR).
    pub unit_name: String,
    /// Originally sold magnitude in selling-unit micros.
    pub original_qty_micro: String,
    /// Cumulatively returned magnitude in selling-unit micros.
    pub already_returned_qty_micro: String,
    /// Still-returnable magnitude in selling-unit micros.
    pub remaining_qty_micro: String,
    /// Agreed unit price in centimes.
    pub unit_price_cents: String,
    /// Net unit price after pro-rated discount, in centimes.
    pub net_unit_price_cents: String,
    /// Line TVA rate in micros.
    pub tva_rate: String,
}

/// Returnable source document summary (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnableDocumentSummaryDto {
    /// Source document id (UUID string).
    pub source_document_id: String,
    /// Gapless human number.
    pub source_document_number: String,
    /// Document kind code.
    pub document_type: String,
    /// Owning party kind (`CUSTOMER`, or `None` for walk-in sales).
    pub party_type: Option<String>,
    /// Owning customer id, if any.
    pub party_id: Option<String>,
    /// Owning customer name, if any.
    pub party_name: Option<String>,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Source subtotal HT in centimes.
    pub total_ht_cents: String,
    /// Source grand total in centimes.
    pub total_ttc_cents: String,
    /// Convertible now (FINALIZED returnable kind with returnable stock).
    pub is_eligible_for_return: bool,
    /// Per-line returnable state.
    pub items: Vec<ReturnableLineDto>,
}

/// One returned line payload (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnItemPayloadDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned magnitude in selling-unit micros.
    pub quantity_micro: String,
    /// Operator reason, if any.
    pub reason: Option<String>,
}

/// Refund settlement selector (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RefundDestinationDto {
    /// Drawer payout from an OPEN session.
    Cash { cash_session_id: String },
    /// Debt reduction for a named customer.
    CustomerCredit { customer_id: String },
    /// Logistics-only return, no financial leg.
    NoRefund,
}

/// Return execution request (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReturnRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Acting operator id (UUID string; permission gated by the session role).
    pub operator_id: String,
    /// Returned-against source document id (UUID string).
    pub source_document_id: String,
    /// Terminal identifier (must match the edge-resolved terminal).
    pub terminal_id: String,
    /// Returned lines.
    pub items: Vec<ReturnItemPayloadDto>,
    /// Financial settlement.
    pub refund: RefundDestinationDto,
    /// Client idempotency key (UUID string).
    pub idempotency_key: String,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Return execution response (TASK-BRK-047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReturnResponseDto {
    /// New AVOIR document id (UUID string).
    pub return_document_id: String,
    /// New AVOIR gapless human number.
    pub return_document_number: String,
    /// Source document id (UUID string).
    pub source_document_id: String,
    /// Source gapless human number.
    pub source_document_number: String,
    /// Refunded HT in centimes.
    pub total_ht_cents: String,
    /// Refunded TVA in centimes.
    pub total_tva_cents: String,
    /// Refunded TTC in centimes.
    pub total_ttc_cents: String,
    /// Settlement kind (`CASH`, `CUSTOMER_CREDIT`, `NO_REFUND`).
    pub refund_type: String,
    /// Returned line count.
    pub lines_count: usize,
}

/// One returnable source line, eligibility view (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnableItemDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned product id (UUID string).
    pub product_id: String,
    /// Product display name (FR, id fallback).
    pub product_name: String,
    /// Product-level barcode, if any.
    pub barcode: Option<String>,
    /// Selling unit id (UUID string).
    pub unit_id: String,
    /// Selling unit display name (FR, id fallback).
    pub unit_name: String,
    /// Agreed unit price in centimes.
    pub unit_price_centimes: String,
    /// Stored per-unit discount in centimes (DSN-097; None = legacy line).
    pub discount_per_unit_centimes: Option<String>,
    /// Discounted unit price in centimes (exact edge transcription; None =
    /// legacy line, display falls back to the raw unit price).
    pub discounted_unit_price_centimes: Option<String>,
    /// Originally sold magnitude in selling-unit micros.
    pub original_quantity_micro: String,
    /// Cumulatively returned magnitude in selling-unit micros.
    pub already_returned_micro: String,
    /// Still-returnable magnitude in selling-unit micros.
    pub remaining_allowable_micro: String,
    /// Line TVA rate in micros.
    pub tva_rate: u32,
    /// Original line discount in centimes.
    pub discount_centimes: String,
}

/// Return context for one source document (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnContextResponseDto {
    /// Source document id (UUID string).
    pub source_document_id: String,
    /// Gapless human number.
    pub document_number: String,
    /// Document kind code.
    pub document_type: String,
    /// Owning customer id, if any.
    pub party_id: Option<String>,
    /// Owning customer name, if any.
    pub party_name: Option<String>,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Creation instant (RFC3339).
    pub created_at: String,
    /// Convertible now (FINALIZED returnable kind with remaining stock).
    pub is_eligible: bool,
    /// Machine-readable reason when ineligible (`NOT_FINALIZED`,
    /// `NON_RETURNABLE_TYPE`, `FULLY_RETURNED`).
    pub ineligibility_reason: Option<String>,
    /// Per-line returnable state.
    pub items: Vec<ReturnableItemDto>,
}

/// One returned line payload, simplified facade (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnItemInputDto {
    /// Source line id (UUID string).
    pub source_item_id: String,
    /// Returned magnitude in selling-unit micros.
    pub quantity_micro: String,
}

/// Simplified single-shot return request (TASK-BRK-046 re-issue).
///
/// Unlike the explicit `ProcessReturnRequestDto`, settlement travels as a
/// plain method code and session/terminal/idempotency resolve at the edge:
/// operator and terminal come from the session and TerminalContext, and a
/// fresh idempotency key is minted per call (callers needing retry-safety
/// use `returns_process` with an explicit key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentsProcessReturnRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Returned-against source document id (UUID string).
    pub source_document_id: String,
    /// Settlement selector (`CASH`, `CUSTOMER_CREDIT`, `NO_REFUND`).
    pub refund_method: String,
    /// Returned lines.
    pub items: Vec<ReturnItemInputDto>,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Simplified single-shot return response (TASK-BRK-046 re-issue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentsProcessReturnResponseDto {
    /// New AVOIR document id (UUID string).
    pub return_document_id: String,
    /// New AVOIR gapless human number.
    pub return_document_number: String,
    /// Refunded TTC in centimes.
    pub total_refund_ttc_centimes: String,
    /// Settlement kind as requested (`CASH`, `CUSTOMER_CREDIT`, `NO_REFUND`).
    pub refund_method: String,
}
