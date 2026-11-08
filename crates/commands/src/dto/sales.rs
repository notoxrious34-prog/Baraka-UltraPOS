//! Sale DTOs: checkout, previews, drafts, stored documents.
use serde::{Deserialize, Serialize};

// Sale wire types -------------------------------------------------------
// Money travels as integer-centime strings, quantities as integer-micro
// strings, ids as UUID strings. No float ever crosses this boundary.

/// One cart line for checkout or draft parking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CartLineItemDto {
    /// Product id (UUID string).
    pub product_id: String,
    /// Selling unit id (UUID string).
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Explicit operator price (`Some` = override attempt).
    pub unit_price_centimes: Option<String>,
    /// Line discount PER UNIT in centimes (BRK-096: the F5 amount hits the
    /// unit price — `unit_price - discount` — never the line gross; the
    /// kernel derives the line total as `qty x discounted PU`).
    pub line_discount_centimes: String,
}

/// One tender line for checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentLineDto {
    /// Tender method (`CASH`, `CREDIT`, `CHEQUE`, `CARD`).
    pub payment_method: String,
    /// Tendered amount in centimes.
    pub amount_centimes: String,
}

/// Atomic checkout request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutSaleRequest {
    /// Client idempotency key (UUID string).
    pub command_id: String,
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Customer id for credit sales (UUID string).
    pub customer_id: Option<String>,
    /// Document kind (`TICKET`, `FACTURE`, `BL`).
    pub document_type: String,
    /// Sale-time receipt language (`AR` or `FR`, mandatory, no default).
    pub document_language: String,
    /// Cart lines.
    pub lines: Vec<CartLineItemDto>,
    /// Global discount in centimes.
    pub global_discount_centimes: String,
    /// Tender lines.
    pub payments: Vec<PaymentLineDto>,
    /// Explicit in-session sale-on-shortage confirmation (TASK-BRK-033).
    /// Privileged roles only; ignored otherwise. Defaults to false.
    #[serde(default)]
    pub allow_negative_stock: bool,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Atomic checkout response (stored receipt shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutSaleResponse {
    /// Stored document id.
    pub document_id: String,
    /// Gapless human number.
    pub document_number: String,
    /// HT subtotal after all discounts.
    pub subtotal_ht_centimes: String,
    /// Total TVA.
    pub tva_total_centimes: String,
    /// Total global discount allocated.
    pub discount_total_centimes: String,
    /// Timbre fiscal.
    pub timbre_centimes: String,
    /// Grand total.
    pub total_ttc_centimes: String,
    /// Total tendered.
    pub paid_amount_centimes: String,
    /// Change owed.
    pub change_amount_centimes: String,
    /// PENDING outbox print job id.
    pub print_job_id: String,
}

/// Read-only totals preview request (same cart shape as checkout).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewTotalsRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id (UUID string; shortage warnings read its levels).
    pub warehouse_id: String,
    /// Document kind (`TICKET`, `FACTURE`, `BL`).
    pub document_type: String,
    /// Cart lines.
    pub lines: Vec<CartLineItemDto>,
    /// Global discount in centimes.
    pub global_discount_centimes: String,
    /// Tender lines exactly as currently entered by the operator.
    pub payments: Vec<PaymentLineDto>,
    /// Customer id for tier pricing, if any (absent/null = retail guest).
    #[serde(default)]
    pub customer_id: Option<String>,
}

/// One leaf-level stock shortage (TASK-BRK-033).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockShortageWarningDto {
    /// Short physical product id (UUID string).
    pub product_id: String,
    /// Commercial designation (FR) for the confirmation banner.
    pub product_name: String,
    /// Stock-keeping reference.
    pub sku: String,
    /// On-hand base micros at preview time (advisory).
    pub available_qty_micro: String,
    /// Requested base micros aggregated over the cart.
    pub requested_qty_micro: String,
}

/// Margin Guard readout for one preview line (Pillar 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineMarginDto {
    /// Zero-based cart line index.
    pub line_index: usize,
    /// Integer-exact margin in basis points (`100 bps = 1.00%`).
    pub margin_basis_points: i64,
    /// True when the agreed unit price is strictly below cost.
    pub is_below_cost: bool,
}

/// Read-only totals preview response (commit-identical, all centime strings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewTotalsResponse {
    /// HT subtotal after all discounts.
    pub subtotal_ht_centimes: String,
    /// Total TVA.
    pub tva_total_centimes: String,
    /// Total global discount allocated.
    pub global_discount_centimes: String,
    /// Timbre fiscal.
    pub timbre_centimes: String,
    /// Grand total.
    pub total_ttc_centimes: String,
    /// Total tendered.
    pub paid_amount_centimes: String,
    /// Change owed.
    pub change_amount_centimes: String,
    /// Uncovered remainder (credit-gated at commit, reported here).
    pub remaining_due_centimes: String,
    /// Leaf-level shortages against the requested warehouse (advisory).
    #[serde(default)]
    pub stock_warnings: Vec<StockShortageWarningDto>,
    /// Per-line Margin Guard readouts in cart order (advisory).
    #[serde(default)]
    pub line_margins: Vec<LineMarginDto>,
}

/// Draft parking request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDraftSaleRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id (UUID string).
    pub warehouse_id: String,
    /// Customer id, if known (UUID string).
    pub customer_id: Option<String>,
    /// Sale-time receipt language (`AR` or `FR`, mandatory, no default).
    pub document_language: String,
    /// Cart lines.
    pub lines: Vec<CartLineItemDto>,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Draft parking response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSaleResponse {
    /// Parked draft id.
    pub draft_id: String,
    /// Capture instant (RFC3339).
    pub created_at: String,
}

/// Stored document header view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDto {
    /// Document id.
    pub document_id: String,
    /// Human number.
    pub number: String,
    /// Document kind code.
    pub doc_type: String,
    /// Lifecycle status code.
    pub status: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Subtotal HT.
    pub subtotal_ht_centimes: String,
    /// Total global discount allocated (DOC-05: the engine `Remise totale`
    /// row — the sale wire omitted it, so no OS printout could match).
    pub global_discount_centimes: String,
    /// Total TVA.
    pub tva_total_centimes: String,
    /// Grand total.
    pub total_ttc_centimes: String,
    /// Total tendered.
    pub paid_amount_centimes: String,
    /// Change owed.
    pub change_amount_centimes: String,
    /// Capture instant (RFC3339 millis, stored verbatim — DOC-05: the OS
    /// printout must show the document instant, never the print instant).
    pub created_at: String,
    /// Acting operator id (stored verbatim — DOC-05: engine cashier row).
    pub created_by: String,
    /// Derived settlement state (`PAID`, `PARTIAL`, `UNPAID`).
    pub payment_status: String,
    /// Pinned layout version (DOC-04): the OS printout picks v1 or v2 from
    /// the stored document, never from the current binary.
    pub template_version: u32,
    /// Resolved operator login name (DOC-07) — the OS printout shows this,
    /// never the raw `created_by` id (orphans keep the verbatim id).
    pub created_by_name: String,
    /// Resolved depot name (DOC-07) — same orphan contract as above.
    pub warehouse_name: String,
    /// Wall-clock instant `DD/MM/YYYY HH:MM` (DOC-07) — the OS printout
    /// shows this, never the raw RFC3339 `created_at`.
    pub created_at_display: String,
}

/// Stored document line view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentItemDto {
    /// Product id.
    pub product_id: String,
    /// Selling unit id.
    pub unit_id: String,
    /// Quantity in selling-unit micros.
    pub quantity_micro: String,
    /// Unit price in centimes.
    pub unit_price_centimes: String,
    /// Stored per-unit discount in centimes (DSN-097; None = legacy line
    /// written before migration 0018 - legacy display keeps the raw unit price).
    pub discount_per_unit_centimes: Option<String>,
    /// Discounted unit price in centimes (exact unit_price minus per_unit at the
    /// edge; None = legacy line, display falls back to the raw unit price).
    pub discounted_unit_price_centimes: Option<String>,
    /// Line TVA.
    pub line_tva_centimes: String,
    /// Line TTC.
    pub line_ttc_centimes: String,
    /// Commercial designation (FR); absent when the product row is gone.
    pub product_name_fr: Option<String>,
    /// Commercial designation (AR); absent when the product row is gone.
    pub product_name_ar: Option<String>,
    /// Stock-keeping reference; absent when the product row is gone.
    pub sku: Option<String>,
    /// Selling-unit label (FR); absent when the unit row is gone.
    pub unit_name_fr: Option<String>,
    /// Selling-unit label (AR); absent when the unit row is gone.
    pub unit_name_ar: Option<String>,
    /// True when the product row is missing (frontend shows SKU-or-ID +
    /// archived marker, key TBD by DSN).
    pub product_missing: bool,
}

/// Stored payment row view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentDto {
    /// Tender method code.
    pub method_code: String,
    /// Tendered amount in centimes.
    pub amount_centimes: String,
}

/// Full sale view response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSaleResponse {
    /// Document header.
    pub document: DocumentDto,
    /// Item snapshots.
    pub items: Vec<DocumentItemDto>,
    /// Payment rows.
    pub payments: Vec<PaymentDto>,
    /// Latest print-job status (`NONE` when absent).
    pub print_status: String,
}
