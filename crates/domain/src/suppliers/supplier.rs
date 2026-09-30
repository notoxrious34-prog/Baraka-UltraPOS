//! Supplier aggregate and purchase-order lifecycle status.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;
use crate::primitives::Money;

/// Stable supplier identifier (TEXT UUID storage scheme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SupplierId(pub Uuid);

impl SupplierId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SupplierId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for SupplierId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SupplierId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Purchase-order (BC) lifecycle status. BR/FF documents have no separate
/// draft status: they are committed immediately (their `documents.status`
/// follows the existing FINALIZED sealed semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PurchaseOrderStatus {
    /// Editable intent; zero stock/ledger effect.
    Draft,
    /// Some ordered quantity has been received.
    PartiallyReceived,
    /// Every ordered line is fully received.
    FullyReceived,
    /// Cancelled while still open; terminal.
    Cancelled,
}

impl PurchaseOrderStatus {
    /// Canonical storage TEXT value (documents.status).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::PartiallyReceived => "PARTIALLY_RECEIVED",
            Self::FullyReceived => "FULLY_RECEIVED",
            Self::Cancelled => "CANCELLED",
        }
    }

    /// Parse a stored status code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "DRAFT" => Ok(Self::Draft),
            "PARTIALLY_RECEIVED" => Ok(Self::PartiallyReceived),
            "FULLY_RECEIVED" => Ok(Self::FullyReceived),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown purchase order status: {other}"
            ))),
        }
    }

    /// Whether this status still accepts goods receipts against the BC.
    #[must_use]
    pub const fn open_for_receipt(self) -> bool {
        matches!(self, Self::Draft | Self::PartiallyReceived)
    }
}

/// Supplier master data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Supplier {
    /// Stable identifier.
    pub id: SupplierId,
    /// Display name (trimmed, 2..=100 chars).
    pub name: String,
    /// Optional phone.
    pub phone: Option<String>,
    /// Optional address.
    pub address: Option<String>,
    /// Optional NIF (tax registration).
    pub nif: Option<String>,
    /// Optional RC (registre de commerce).
    pub rc: Option<String>,
    /// Optional notes.
    pub notes: Option<String>,
    /// Active flag (inactive suppliers reject purchases and payments).
    pub is_active: bool,
    /// Cached balance (cents owed to the supplier). Derived-truth mirror.
    pub current_balance: Money,
}

/// Validate a supplier display name (identical contract to customers).
pub fn validate_supplier_name(raw: &str) -> Result<String, DomainError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DomainError::SupplierNameRequired);
    }
    let chars = trimmed.chars().count();
    if !(2..=100).contains(&chars) {
        return Err(DomainError::InvalidCanonicalString(
            "supplier name must be 2..=100 characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate an optional supplier text field (trim; blank -> None; bounded).
pub fn validate_supplier_optional_text(
    field: &str,
    raw: Option<&str>,
    max_chars: usize,
) -> Result<Option<String>, DomainError> {
    match raw {
        None => Ok(None),
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.chars().count() > max_chars {
                return Err(DomainError::InvalidCanonicalString(format!(
                    "supplier {field} must be at most {max_chars} characters"
                )));
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purchase_status_codes_round_trip() {
        for status in [
            PurchaseOrderStatus::Draft,
            PurchaseOrderStatus::PartiallyReceived,
            PurchaseOrderStatus::FullyReceived,
            PurchaseOrderStatus::Cancelled,
        ] {
            assert_eq!(PurchaseOrderStatus::from_code(status.code()), Ok(status));
        }
        assert_eq!(
            PurchaseOrderStatus::from_code("FINALIZED"),
            Err(DomainError::InvalidCanonicalString(
                "unknown purchase order status: FINALIZED".to_string()
            ))
        );
        assert!(PurchaseOrderStatus::Draft.open_for_receipt());
        assert!(PurchaseOrderStatus::PartiallyReceived.open_for_receipt());
        assert!(!PurchaseOrderStatus::FullyReceived.open_for_receipt());
        assert!(!PurchaseOrderStatus::Cancelled.open_for_receipt());
    }

    #[test]
    fn name_validation_matches_customer_contract() {
        assert_eq!(
            validate_supplier_name("  Ferraille SARL  ").expect("ok"),
            "Ferraille SARL"
        );
        assert!(matches!(
            validate_supplier_name("   "),
            Err(DomainError::SupplierNameRequired)
        ));
        assert!(validate_supplier_name("F").is_err());
        assert!(validate_supplier_name(&"N".repeat(101)).is_err());
    }

    #[test]
    fn optional_fields_trim_and_blank_to_none() {
        assert_eq!(
            validate_supplier_optional_text("phone", Some("  0550  "), 20).expect("phone"),
            Some("0550".to_string())
        );
        assert_eq!(
            validate_supplier_optional_text("nif", Some("   "), 20).expect("blank"),
            None
        );
        assert!(validate_supplier_optional_text("rc", Some(&"x".repeat(21)), 20).is_err());
    }
}
