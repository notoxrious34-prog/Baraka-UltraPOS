//! Supplier ledger entry types (append-only double-entry mirror).

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use crate::primitives::Money;
use crate::suppliers::supplier::SupplierId;

/// DEBIT increases what we owe the supplier; CREDIT reduces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LedgerEntryType {
    /// Purchase invoice raised against us.
    Debit,
    /// Payment we made to the supplier.
    Credit,
}

impl LedgerEntryType {
    /// Canonical storage TEXT value.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Debit => "DEBIT",
            Self::Credit => "CREDIT",
        }
    }

    /// Parse a stored code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "DEBIT" => Ok(Self::Debit),
            "CREDIT" => Ok(Self::Credit),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown supplier ledger entry type: {other}"
            ))),
        }
    }

    /// Signed balance contribution (`owed_to_supplier` perspective).
    #[must_use]
    pub const fn balance_sign(self) -> i8 {
        match self {
            Self::Debit => 1,
            Self::Credit => -1,
        }
    }
}

/// What produced the ledger entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LedgerReferenceType {
    /// Supplier invoice document (FF).
    PurchaseInvoice,
    /// Outbound payment (cash or bank).
    Payment,
}

impl LedgerReferenceType {
    /// Canonical storage TEXT value.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PurchaseInvoice => "PURCHASE_INVOICE",
            Self::Payment => "PAYMENT",
        }
    }

    /// Parse a stored code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "PURCHASE_INVOICE" => Ok(Self::PurchaseInvoice),
            "PAYMENT" => Ok(Self::Payment),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown supplier ledger reference type: {other}"
            ))),
        }
    }
}

/// One append-only ledger row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplierLedgerEntry {
    /// Stable entry id (TEXT UUID).
    pub id: String,
    /// Owning supplier.
    pub supplier_id: SupplierId,
    /// Entry direction.
    pub entry_type: LedgerEntryType,
    /// Positive magnitude only (CHECK amount_centimes > 0).
    pub amount: Money,
    /// Producing document class.
    pub reference_type: LedgerReferenceType,
    /// Producing document/entry id.
    pub reference_id: String,
    /// Optional description.
    pub description: Option<String>,
    /// Capture instant (RFC3339).
    pub created_at: String,
    /// Acting operator id.
    pub created_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for entry in [LedgerEntryType::Debit, LedgerEntryType::Credit] {
            assert_eq!(LedgerEntryType::from_code(entry.code()), Ok(entry));
        }
        for reference in [
            LedgerReferenceType::PurchaseInvoice,
            LedgerReferenceType::Payment,
        ] {
            assert_eq!(
                LedgerReferenceType::from_code(reference.code()),
                Ok(reference)
            );
        }
        assert!(LedgerEntryType::from_code("VOID").is_err());
        assert!(LedgerReferenceType::from_code("SALE").is_err());
    }

    #[test]
    fn signs_are_opposite() {
        assert_eq!(
            LedgerEntryType::Debit.balance_sign() + LedgerEntryType::Credit.balance_sign(),
            0
        );
    }
}
