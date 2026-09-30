//! Cash session aggregate: lifecycle OPEN -> CLOSING -> CLOSED.
//!
//! All centimes fields are [`Money`]; raw `i64` never leaves this module
//! for money. Quantity-style integers (revision) stay plain `i64`.

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use crate::primitives::Money;

/// Lifecycle status of a cash session (TEXT values match storage CHECK).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CashSessionStatus {
    /// Accepting sales, credit payments, and expenses.
    Open,
    /// Counting in progress; sales are rejected identically to CLOSED.
    Closing,
    /// Immutable; no field may ever change again (trigger-enforced).
    Closed,
}

impl CashSessionStatus {
    /// Canonical storage TEXT value.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Open => "OPEN",
            Self::Closing => "CLOSING",
            Self::Closed => "CLOSED",
        }
    }

    /// Parse a stored status code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "OPEN" => Ok(Self::Open),
            "CLOSING" => Ok(Self::Closing),
            "CLOSED" => Ok(Self::Closed),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown cash session status: {other}"
            ))),
        }
    }
}

/// Cash session aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashSession {
    /// Stable identifier (TEXT UUID).
    pub id: String,
    /// Selling warehouse.
    pub warehouse_id: String,
    /// Terminal (== device identifier, never client-supplied).
    pub terminal_id: String,
    /// Lifecycle status.
    pub status: CashSessionStatus,
    /// Operator who opened the session.
    pub opened_by: String,
    /// Opening instant (RFC3339).
    pub opened_at: String,
    /// Opening float (must be >= 0).
    pub opening_float: Money,
    /// Operator who closed, if closed.
    pub closed_by: Option<String>,
    /// Closing instant, if closed.
    pub closed_at: Option<String>,
    /// Physically counted amount, if supplied at confirm.
    pub actual_counted: Option<Money>,
    /// Frozen expected total (populated at BeginClosing).
    pub system_expected: Option<Money>,
    /// Counted minus expected (populated at ConfirmClosing).
    pub variance: Option<Money>,
    /// Optional closing notes.
    pub closing_notes: Option<String>,
    /// Optimistic concurrency revision.
    pub revision: i64,
    /// Origin device (== terminal identifier for this system).
    pub origin_device_id: String,
    /// Row creation instant (RFC3339).
    pub created_at: String,
    /// Row update instant (RFC3339).
    pub updated_at: String,
}

/// Validate an opening float (must be >= 0).
pub fn validate_opening_float(centimes: i64) -> Result<Money, DomainError> {
    if centimes < 0 {
        return Err(DomainError::CashSessionInvalidOpeningFloat);
    }
    Ok(Money::from_centimes(centimes))
}

/// Validate a counted amount at confirm (must be >= 0).
pub fn validate_counted_amount(centimes: i64) -> Result<Money, DomainError> {
    if centimes < 0 {
        return Err(DomainError::CashSessionInvalidCountedAmount);
    }
    Ok(Money::from_centimes(centimes))
}

/// Compute the closing variance with pure integer arithmetic.
///
/// `variance = actual_counted - system_expected`. Both inputs are raw
/// centimes; overflow maps to [`DomainError::MoneyOverflow`].
pub fn compute_variance(
    actual_counted_centimes: i64,
    system_expected_centimes: i64,
) -> Result<Money, DomainError> {
    actual_counted_centimes
        .checked_sub(system_expected_centimes)
        .map(Money::from_centimes)
        .ok_or(DomainError::MoneyOverflow)
}

/// Validate an expense description (non-empty after trim, max 255 chars).
pub fn validate_expense_description(description: &str) -> Result<String, DomainError> {
    let trimmed = description.trim();
    if trimmed.is_empty() {
        return Err(DomainError::CashExpenseDescriptionRequired);
    }
    if trimmed.chars().count() > 255 {
        return Err(DomainError::CashExpenseDescriptionRequired);
    }
    Ok(trimmed.to_string())
}

/// Validate an expense amount (must be > 0).
pub fn validate_expense_amount(centimes: i64) -> Result<Money, DomainError> {
    if centimes <= 0 {
        return Err(DomainError::CashExpenseInvalidAmount);
    }
    Ok(Money::from_centimes(centimes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes_round_trip() {
        for status in [
            CashSessionStatus::Open,
            CashSessionStatus::Closing,
            CashSessionStatus::Closed,
        ] {
            assert_eq!(CashSessionStatus::from_code(status.code()), Ok(status));
        }
        assert!(CashSessionStatus::from_code("AJAR").is_err());
    }

    #[test]
    fn opening_float_rejects_negative_accepts_zero() {
        assert!(validate_opening_float(-1).is_err());
        assert_eq!(
            validate_opening_float(0).expect("zero"),
            Money::from_centimes(0)
        );
        assert_eq!(
            validate_opening_float(50_000).expect("positive"),
            Money::from_centimes(50_000)
        );
    }

    #[test]
    fn counted_amount_rejects_negative() {
        assert!(validate_counted_amount(-1).is_err());
        assert_eq!(
            validate_counted_amount(0).expect("zero"),
            Money::from_centimes(0)
        );
    }

    #[test]
    fn variance_is_pure_integer_difference() {
        assert_eq!(
            compute_variance(100_000, 99_500).expect("variance"),
            Money::from_centimes(500)
        );
        assert_eq!(
            compute_variance(90_000, 100_000).expect("negative"),
            Money::from_centimes(-10_000)
        );
        assert_eq!(
            compute_variance(0, 0).expect("zero"),
            Money::from_centimes(0)
        );
    }

    #[test]
    fn variance_symmetric_across_wide_range() {
        for expected in [0i64, 1, 99_500, 1_000_000, 999_999_999] {
            for actual in [0i64, 1, 50_000, 100_000, 12_345_678] {
                let forward = compute_variance(actual, expected).expect("forward");
                let backward = compute_variance(expected, actual).expect("backward");
                assert_eq!(
                    forward.centimes(),
                    -backward.centimes(),
                    "asymmetric actual={actual} expected={expected}"
                );
                assert_eq!(forward.centimes(), actual - expected);
            }
        }
    }

    #[test]
    fn expense_validation_paths() {
        assert!(validate_expense_amount(0).is_err());
        assert!(validate_expense_amount(-5).is_err());
        assert!(validate_expense_description("   ").is_err());
        assert_eq!(
            validate_expense_description("  Café  ").expect("trim"),
            "Café"
        );
        let long = "x".repeat(256);
        assert!(validate_expense_description(&long).is_err());
        assert!(validate_expense_description(&"y".repeat(255)).is_ok());
    }
}
