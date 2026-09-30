//! Customer credit profile (domain view of the `customers` table).

use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;

/// Stable customer identifier (TEXT UUID, sovereign identity scheme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerId(pub Uuid);

impl CustomerId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CustomerId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for CustomerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CustomerId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Credit policy (`credit_policy` column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreditPolicy {
    /// No credit sales allowed.
    Disabled,
    /// Credit sales allowed up to a limit.
    Limited,
    /// Credit sales allowed without a limit.
    Unlimited,
}

impl CreditPolicy {
    /// Parse a stored policy code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "DISABLED" => Ok(Self::Disabled),
            "LIMITED" => Ok(Self::Limited),
            "UNLIMITED" => Ok(Self::Unlimited),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown credit policy: {other}"
            ))),
        }
    }

    /// Canonical storage code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Disabled => "DISABLED",
            Self::Limited => "LIMITED",
            Self::Unlimited => "UNLIMITED",
        }
    }
}

/// Credit state read for authorization decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditProfile {
    /// Customer under evaluation.
    pub customer_id: CustomerId,
    /// Active policy.
    pub policy: CreditPolicy,
    /// Limit in centimes (meaningful for `Limited`).
    pub credit_limit_centimes: i64,
    /// Current ledger balance in centimes.
    pub current_balance_centimes: i64,
    /// Inactive customers cannot buy on credit.
    pub is_active: bool,
}

/// Full customer entity (domain view of the `customers` table).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customer {
    /// Stable identifier.
    pub id: CustomerId,
    /// Display name (trimmed, 2..=100 chars).
    pub name: String,
    /// Optional unique phone (Algerian format when present).
    pub phone: Option<String>,
    /// Optional address.
    pub address: Option<String>,
    /// Credit policy.
    pub credit_policy: CreditPolicy,
    /// Limit in centimes (meaningful for `Limited`).
    pub credit_limit_centimes: i64,
    /// Active flag.
    pub is_active: bool,
}

/// Customer ledger entry direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CustomerLedgerEntryType {
    /// Amount owed by the customer (sale on credit).
    Debit,
    /// Amount paid back by the customer.
    Credit,
}

impl CustomerLedgerEntryType {
    /// Canonical storage code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Debit => "DEBIT",
            Self::Credit => "CREDIT",
        }
    }
}

/// Validate a customer display name (trimmed, 2..=100 chars).
pub fn validate_customer_name(raw: &str) -> Result<String, DomainError> {
    let trimmed = raw.trim();
    let count = trimmed.chars().count();
    if !(2..=100).contains(&count) {
        return Err(DomainError::InvalidCanonicalString(
            "customer name must be 2..=100 characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate an Algerian phone number.
///
/// Accepts mobile `0[5-7][0-9]{8}` (10 digits) and landline `0[2-4][0-9]{7}`
/// (9 digits). Returns the trimmed canonical value.
pub fn validate_customer_phone(raw: &str) -> Result<String, DomainError> {
    let trimmed = raw.trim();
    let digits_ok = trimmed.bytes().all(|byte| byte.is_ascii_digit());
    let mobile = trimmed.len() == 10
        && trimmed.starts_with('0')
        && matches!(trimmed.as_bytes().get(1), Some(b'5' | b'6' | b'7'))
        && digits_ok;
    let landline = trimmed.len() == 9
        && trimmed.starts_with('0')
        && matches!(trimmed.as_bytes().get(1), Some(b'2' | b'3' | b'4'))
        && digits_ok;
    if mobile || landline {
        Ok(trimmed.to_string())
    } else {
        Err(DomainError::InvalidCanonicalString(
            "INVALID_PHONE_FORMAT".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_boundaries() {
        assert!(validate_customer_name("Amine Benali").is_ok());
        assert!(validate_customer_name("  Amine  ").is_ok());
        assert!(validate_customer_name("A").is_err());
        assert!(validate_customer_name("   ").is_err());
        assert!(validate_customer_name(&"x".repeat(101)).is_err());
        assert_eq!(validate_customer_name("  Amine  ").expect("trim"), "Amine");
    }

    #[test]
    fn algerian_phone_shapes() {
        assert_eq!(
            validate_customer_phone("0550123456").expect("mobile"),
            "0550123456"
        );
        assert_eq!(
            validate_customer_phone("  023123456  ").expect("landline"),
            "023123456"
        );
        for bad in [
            "",
            "123",
            "058012345",
            "05501234567",
            "0550abc123",
            "+213550123456",
            "0150123456",
        ] {
            assert!(validate_customer_phone(bad).is_err(), "must reject {bad:?}");
        }
    }

    #[test]
    fn ledger_codes_round_trip() {
        assert_eq!(CustomerLedgerEntryType::Debit.code(), "DEBIT");
        assert_eq!(CustomerLedgerEntryType::Credit.code(), "CREDIT");
    }
}
