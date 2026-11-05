//! Cash-session DTOs: sessions, movements, closing, reports.
use serde::{Deserialize, Serialize};

// Cash session wire types -------------------------------------------------
// Money travels as integer-centime strings; ids are UUID strings;
// timestamps are ISO8601 strings. No float ever crosses this boundary.

/// Stored cash session view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionDto {
    /// Session id.
    pub id: String,
    /// Selling warehouse id.
    pub warehouse_id: String,
    /// Terminal identifier.
    pub terminal_id: String,
    /// Status code (`OPEN`, `CLOSING`, `CLOSED`).
    pub status: String,
    /// Operator who opened the session.
    pub opened_by: String,
    /// Opening operator username.
    pub opened_by_username: String,
    /// Opening instant (RFC3339).
    pub opened_at: String,
    /// Opening float in centimes.
    pub opening_float_centimes: String,
    /// Operator who closed, if closed.
    pub closed_by: Option<String>,
    /// Closing operator username, if closed.
    pub closed_by_username: Option<String>,
    /// Closing instant, if closed.
    pub closed_at: Option<String>,
    /// Physically counted amount, if supplied.
    pub actual_counted_centimes: Option<String>,
    /// Frozen expected total.
    pub system_expected_centimes: Option<String>,
    /// Counted minus expected.
    pub variance_centimes: Option<String>,
    /// Optional closing notes.
    pub closing_notes: Option<String>,
    /// Optimistic concurrency revision.
    pub revision: String,
    /// Row creation instant (RFC3339).
    pub created_at: String,
    /// Row update instant (RFC3339).
    pub updated_at: String,
    /// Non-blocking post-close automatic-backup warning code, if any
    /// (TASK-BRK-092; the close itself already committed).
    pub backup_warning: Option<String>,
}

/// Stored cash movement view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovementDto {
    /// Movement id.
    pub id: String,
    /// Owning session.
    pub cash_session_id: String,
    /// Movement type code.
    pub movement_type: String,
    /// Signed amount in centimes.
    pub amount_centimes: String,
    /// Referencing business kind, if any.
    pub reference_type: Option<String>,
    /// Referencing business id, if any.
    pub reference_id: Option<String>,
    /// Human description, if any.
    pub description: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator.
    pub created_by: String,
    /// Acting operator username.
    pub created_by_username: String,
}

/// Totals grouped by movement type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovementTotalDto {
    /// Movement type code.
    pub movement_type: String,
    /// Signed total in centimes.
    pub total_centimes: String,
    /// Row count for this type.
    pub count: String,
}

/// Session report (X-Report interim or Z-Report final).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSessionReportDto {
    /// Session header.
    pub session: CashSessionDto,
    /// Movements in creation order.
    pub movements: Vec<CashMovementDto>,
    /// Totals grouped by movement type.
    pub totals_by_type: Vec<CashMovementTotalDto>,
    /// True when status == CLOSED.
    pub is_final: bool,
}

/// `cash.open` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCashSessionRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Selling warehouse id.
    pub warehouse_id: String,
    /// Opening float in centimes.
    pub opening_float_centimes: String,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.recordExpense` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordExpenseRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Expense amount in centimes (must be positive).
    pub amount_centimes: String,
    /// Mandatory description.
    pub description: String,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.confirmClosing` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmClosingRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Session to close.
    pub session_id: String,
    /// Physically counted amount in centimes.
    pub actual_counted_centimes: String,
    /// Optional closing notes.
    pub closing_notes: Option<String>,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.beginClosing` / `cash.cancelClosing` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashClosingCommandDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Client idempotency key.
    pub command_id: String,
}

/// `cash.getSessionReport` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetSessionReportRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Session id.
    pub session_id: String,
}
