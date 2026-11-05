//! Report DTOs: read-only aggregates (sales, margins, credit, audit).
use serde::{Deserialize, Serialize};

// Reports wire types (TASK-BRK-012) ---------------------------------------
// Read-only aggregates. Money as centime strings, quantities as micro
// strings, ids as UUID strings, dates as YYYY-MM-DD. No float anywhere.

/// Shared date-range request shape for every ranged report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDateRangeRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
}

/// One daily sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailySalesSummaryDto {
    /// Calendar day (`YYYY-MM-DD`).
    pub day: String,
    /// Grand total in centimes.
    pub total_ttc_centimes: String,
    /// HT subtotal in centimes.
    pub total_ht_centimes: String,
    /// TVA total in centimes.
    pub total_tva_centimes: String,
    /// Committed document count.
    pub document_count: String,
    /// Mean basket in centimes.
    pub average_basket_centimes: String,
}

/// One per-product sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesByProductRowDto {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// French display name.
    pub name_fr: String,
    /// Quantity sold in base micros.
    pub quantity_sold_base_micro: String,
    /// Revenue in centimes.
    pub revenue_ttc_centimes: String,
    /// Contributing line count.
    pub line_count: String,
}

/// One per-category sales aggregate row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesByCategoryRowDto {
    /// Category id (`None` = uncategorized products).
    pub category_id: Option<String>,
    /// Category code (`None` = uncategorized).
    pub category_code: Option<String>,
    /// French display name (`None` = uncategorized).
    pub name_fr: Option<String>,
    /// Revenue in centimes.
    pub revenue_ttc_centimes: String,
    /// Contributing line count.
    pub line_count: String,
}

/// One per-product gross-margin row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrossMarginRowDto {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// Revenue HT in centimes.
    pub revenue_ht_centimes: String,
    /// Cost of goods sold in centimes.
    pub cost_total_centimes: String,
    /// Margin in centimes (revenue_ht - cost_total).
    pub margin_centimes: String,
    /// Margin percentage in basis points (margin * 10000 / revenue_ht,
    /// `"0"` when revenue is zero). Display-only, computed server-side.
    pub margin_percent_bp: String,
    /// True when any contributing line had a zero unit cost.
    pub has_untracked_cost: bool,
}

/// One credit-outstanding row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditOutstandingRowDto {
    /// Customer id.
    pub customer_id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Policy code.
    pub credit_policy: String,
    /// Limit in centimes (`None` for UNLIMITED).
    pub credit_limit_centimes: Option<String>,
    /// Current balance in centimes (> 0).
    pub current_balance_centimes: String,
    /// Limit consumed in basis points (`None` for UNLIMITED).
    pub limit_used_bp: Option<String>,
}

/// One stock-movement history row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovementRowDto {
    /// Movement id.
    pub id: String,
    /// Movement type code.
    pub movement_type: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Product id.
    pub product_id: String,
    /// Quantity in base micros (signed delta).
    pub quantity_base_micro: String,
    /// Reference document type, if any.
    pub reference_type: Option<String>,
    /// Reference document id, if any.
    pub reference_id: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator id.
    pub created_by: String,
}

/// `reports.stockMovementHistory` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovementHistoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional warehouse filter.
    pub warehouse_id: Option<String>,
    /// Optional product filter.
    pub product_id: Option<String>,
    /// Optional movement-type filter.
    pub movement_type: Option<String>,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
    /// Page size (default 50, max 200).
    pub limit: Option<u32>,
    /// Row offset.
    pub offset: Option<u32>,
}

/// One cash-session history row (drill down via cash.getSessionReport).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionHistoryRowDto {
    /// Session id.
    pub id: String,
    /// Terminal id.
    pub terminal_id: String,
    /// Status code.
    pub status: String,
    /// Opening instant (RFC3339).
    pub opened_at: String,
    /// Closing instant, if closed.
    pub closed_at: Option<String>,
    /// Opening float in centimes.
    pub opening_float_centimes: String,
    /// Frozen expected total, if computed.
    pub system_expected_centimes: Option<String>,
    /// Counted amount, if supplied.
    pub actual_counted_centimes: Option<String>,
    /// Variance, if computed.
    pub variance_centimes: Option<String>,
}

/// `reports.cashSessionsHistory` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionsHistoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional terminal filter.
    pub terminal_id: Option<String>,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
}

/// One user-activity (audit) row (generic payload, never decoded per type).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserActivityRowDto {
    /// Monotonic sequence number.
    pub sequence_no: String,
    /// Capture instant (RFC3339).
    pub timestamp: String,
    /// Acting operator id.
    pub user_id: String,
    /// Action code.
    pub action: String,
    /// Entity type.
    pub entity_type: String,
    /// Entity id.
    pub entity_id: String,
    /// Raw previous-value JSON, if any.
    pub old_value_json: Option<String>,
    /// Raw new-value JSON, if any.
    pub new_value_json: Option<String>,
}

/// `reports.userActivity` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserActivityRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional operator filter.
    pub user_id: Option<String>,
    /// Optional action filter.
    pub action: Option<String>,
    /// Inclusive start day (`YYYY-MM-DD`).
    pub date_from: String,
    /// Inclusive end day (`YYYY-MM-DD`).
    pub date_to: String,
    /// Page size (default 50, max 200).
    pub limit: Option<u32>,
    /// Row offset.
    pub offset: Option<u32>,
}

/// One low-stock row (ADR-037: quantity_base_micro <= 0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowStockRowDto {
    /// Product id.
    pub product_id: String,
    /// Product SKU.
    pub sku: String,
    /// French display name.
    pub name_fr: String,
    /// Warehouse id.
    pub warehouse_id: String,
    /// Warehouse code.
    pub warehouse_code: String,
    /// On-hand quantity in base micros (<= 0).
    pub quantity_base_micro: String,
}

/// `reports.lowStock` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowStockRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Optional warehouse filter (`None` covers all warehouses).
    pub warehouse_id: Option<String>,
}
