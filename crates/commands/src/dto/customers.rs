//! Customer DTOs: creation, search, credit payments, statements.
use serde::{Deserialize, Serialize};

// Customer wire types ---------------------------------------------------
// Ids are UUID strings per the sovereign identity scheme; money travels
// as integer-centime strings.

/// Customer creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewCustomerRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Display name (2..=100 chars).
    pub name: String,
    /// Optional unique phone (Algerian format).
    pub phone: Option<String>,
    /// Optional address.
    pub address: Option<String>,
    /// Policy code (`DISABLED`, `LIMITED`, `UNLIMITED`).
    pub credit_policy: String,
    /// Limit in centimes (required when policy is LIMITED).
    pub credit_limit_centimes: Option<String>,
    /// Pricing tier code (`retail`, `artisan`, `wholesale`; defaults to `retail`).
    pub customer_tier: Option<String>,
}

/// Customer update request (only `Some` fields are written).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCustomerRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
    /// Replacement display name.
    pub name: Option<String>,
    /// Replacement phone (`Some(None)` clears it).
    pub phone: Option<Option<String>>,
    /// Replacement address (`Some(None)` clears it).
    pub address: Option<Option<String>>,
    /// Replacement policy code.
    pub credit_policy: Option<String>,
    /// Replacement limit in centimes.
    pub credit_limit_centimes: Option<String>,
    /// Replacement pricing tier code.
    pub customer_tier: Option<String>,
    /// Replacement active flag.
    pub is_active: Option<bool>,
}

/// Customer list row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerSummaryDto {
    /// Customer id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Policy code.
    pub credit_policy: String,
    /// Pricing tier code.
    pub customer_tier: String,
}

/// Full customer view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerDetailDto {
    /// Customer id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Phone, if registered.
    pub phone: Option<String>,
    /// Address, if registered.
    pub address: Option<String>,
    /// Policy code.
    pub credit_policy: String,
    /// Limit in centimes (`None` unless LIMITED).
    pub credit_limit_centimes: Option<String>,
    /// Pricing tier code.
    pub customer_tier: String,
    /// Current ledger balance in centimes.
    pub current_balance_centimes: String,
    /// Active flag.
    pub is_active: bool,
}

/// Customer search request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCustomersRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Prefix/multilingual query (blank returns no rows).
    pub query: String,
    /// Max rows (clamped to 1..=100 server-side).
    pub limit: Option<u32>,
}

/// Customer search response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCustomersResponse {
    /// Matching customers ordered by name.
    pub customers: Vec<CustomerSummaryDto>,
}

/// Single-customer fetch request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetCustomerRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
}

/// Credit payment recording request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordCreditPaymentRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
    /// Payment amount in centimes (must be positive).
    pub amount_centimes: String,
    /// Optional payment reference.
    pub reference: Option<String>,
    /// Allow the balance to go negative (GERANT/ADMIN only).
    pub allow_overpayment: bool,
}

/// Credit payment outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordCreditPaymentResponse {
    /// Stored ledger entry id.
    pub ledger_entry_id: String,
    /// Balance after the payment, in centimes.
    pub new_balance_centimes: String,
}

/// Statement retrieval request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStatementRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Customer id (UUID string).
    pub customer_id: String,
    /// Optional inclusive lower bound (RFC3339).
    pub from_date: Option<String>,
    /// Optional inclusive upper bound (RFC3339).
    pub to_date: Option<String>,
}

/// One statement line with its server-side running balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatementEntryDto {
    /// `DEBIT` or `CREDIT`.
    pub entry_type: String,
    /// Entry amount in centimes.
    pub amount_centimes: String,
    /// Referencing document kind, if any.
    pub reference_type: Option<String>,
    /// Referencing document id, if any.
    pub reference_id: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Running balance after this entry, in centimes.
    pub balance_after_centimes: String,
}

/// Statement response (ledger order).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStatementResponse {
    /// Statement lines in ledger order.
    pub entries: Vec<StatementEntryDto>,
}
