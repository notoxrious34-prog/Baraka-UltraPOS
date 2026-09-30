//! Payment methods, tender lines, and derived payment status.

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use crate::primitives::Money;

/// Tender method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentMethod {
    /// Cash notes and coins.
    Cash,
    /// On-account customer credit.
    Credit,
    /// Bank cheque.
    Cheque,
    /// Bank card.
    Card,
}

impl PaymentMethod {
    /// Canonical storage/wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Cash => "CASH",
            Self::Credit => "CREDIT",
            Self::Cheque => "CHEQUE",
            Self::Card => "CARD",
        }
    }

    /// Parse a wire/storage code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "CASH" => Ok(Self::Cash),
            "CREDIT" => Ok(Self::Credit),
            "CHEQUE" => Ok(Self::Cheque),
            "CARD" => Ok(Self::Card),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown payment method: {other}"
            ))),
        }
    }

    /// True only for cash tender (the sole change-eligible method).
    #[must_use]
    pub const fn is_cash(self) -> bool {
        matches!(self, Self::Cash)
    }
}

/// One tender line of a payment split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaymentLine {
    /// Tender method.
    pub method: PaymentMethod,
    /// Tendered amount (must be positive).
    pub amount: Money,
}

/// Derived settlement state of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Fully tendered.
    Paid,
    /// Partially tendered with a credit line covering the rest.
    Partial,
    /// Nothing tendered yet.
    Unpaid,
}

impl PaymentStatus {
    /// Derive from tendered vs total (`has_credit` gates partial acceptance).
    #[must_use]
    pub const fn derive(paid_centimes: i64, total_ttc_centimes: i64, has_credit: bool) -> Self {
        if paid_centimes >= total_ttc_centimes {
            Self::Paid
        } else if paid_centimes <= 0 && !has_credit {
            Self::Unpaid
        } else if has_credit {
            Self::Partial
        } else {
            Self::Unpaid
        }
    }
}
