//! Cash movement primitives: signed amount semantics per type.
//!
//! Amounts persist as signed centimes; the sign is dictated exclusively
//! by [`MovementType`]. All money is [`Money`]; no raw integers for money.

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use crate::primitives::Money;

/// All cash movement types; TEXT values match the storage CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MovementType {
    /// Opening float row (>= 0, first row of every session).
    OpeningFloat,
    /// Net cash tendered minus change from a sale (> 0).
    SaleCashIn,
    /// Cash-collected credit payment (> 0, ADR-027).
    CreditPaymentIn,
    /// Drawer expense (< 0 stored).
    ExpenseOut,
    /// Manual cash injection (> 0).
    ManualIn,
    /// Manual cash removal (< 0 stored).
    ManualOut,
    /// Outbound supplier payment collected in cash (< 0 stored, ADR-030).
    SupplierPaymentOut,
    /// Customer return refund paid out of the drawer (< 0 stored).
    /// Written only by the return engine against an OPEN session; the SQL
    /// CHECK widening for this code ships with the SQLite return adapter.
    RefundOut,
}

impl MovementType {
    /// Canonical storage TEXT value.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::OpeningFloat => "OPENING_FLOAT",
            Self::SaleCashIn => "SALE_CASH_IN",
            Self::CreditPaymentIn => "CREDIT_PAYMENT_IN",
            Self::ExpenseOut => "EXPENSE_OUT",
            Self::ManualIn => "MANUAL_IN",
            Self::ManualOut => "MANUAL_OUT",
            Self::SupplierPaymentOut => "SUPPLIER_PAYMENT_OUT",
            Self::RefundOut => "REFUND_OUT",
        }
    }

    /// Parse a stored movement type code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "OPENING_FLOAT" => Ok(Self::OpeningFloat),
            "SALE_CASH_IN" => Ok(Self::SaleCashIn),
            "CREDIT_PAYMENT_IN" => Ok(Self::CreditPaymentIn),
            "EXPENSE_OUT" => Ok(Self::ExpenseOut),
            "MANUAL_IN" => Ok(Self::ManualIn),
            "MANUAL_OUT" => Ok(Self::ManualOut),
            "SUPPLIER_PAYMENT_OUT" => Ok(Self::SupplierPaymentOut),
            "REFUND_OUT" => Ok(Self::RefundOut),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown cash movement type: {other}"
            ))),
        }
    }

    /// Whether the stored amount must be strictly positive.
    #[must_use]
    pub const fn requires_positive(self) -> bool {
        match self {
            Self::SaleCashIn | Self::CreditPaymentIn | Self::ManualIn => true,
            Self::OpeningFloat
            | Self::ExpenseOut
            | Self::ManualOut
            | Self::SupplierPaymentOut
            | Self::RefundOut => false,
        }
    }

    /// Whether the stored amount must be strictly negative.
    #[must_use]
    pub const fn requires_negative(self) -> bool {
        match self {
            Self::ExpenseOut | Self::ManualOut | Self::SupplierPaymentOut | Self::RefundOut => true,
            Self::OpeningFloat | Self::SaleCashIn | Self::CreditPaymentIn | Self::ManualIn => false,
        }
    }
}

/// One cash drawer movement row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashMovement {
    /// Stable identifier (TEXT UUID).
    pub id: String,
    /// Owning session.
    pub cash_session_id: String,
    /// Business meaning (drives the sign invariant).
    pub movement_type: MovementType,
    /// Signed amount (sign must agree with `movement_type`).
    pub amount: Money,
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
}

/// Validate a stored signed amount against its movement type.
///
/// Mirrors the SQL CHECK introduced in migration 0008 and widened by
/// migration 0009 (SUPPLIER_PAYMENT_OUT) so violations surface as typed
/// domain errors before reaching storage. `RefundOut` is validated here
/// ahead of its CHECK widening, which ships with the SQLite return
/// adapter (writes flow only through the return engine).
pub fn validate_movement_amount(
    movement_type: MovementType,
    amount_centimes: i64,
) -> Result<Money, DomainError> {
    let valid = match movement_type {
        MovementType::OpeningFloat => amount_centimes >= 0,
        MovementType::SaleCashIn | MovementType::CreditPaymentIn | MovementType::ManualIn => {
            amount_centimes > 0
        }
        MovementType::ExpenseOut
        | MovementType::ManualOut
        | MovementType::SupplierPaymentOut
        | MovementType::RefundOut => amount_centimes < 0,
    };
    if valid {
        Ok(Money::from_centimes(amount_centimes))
    } else {
        Err(DomainError::InvalidCanonicalString(format!(
            "invalid amount {amount_centimes} for {}",
            movement_type.code()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_codes_round_trip() {
        for kind in [
            MovementType::OpeningFloat,
            MovementType::SaleCashIn,
            MovementType::CreditPaymentIn,
            MovementType::ExpenseOut,
            MovementType::ManualIn,
            MovementType::ManualOut,
            MovementType::SupplierPaymentOut,
            MovementType::RefundOut,
        ] {
            assert_eq!(MovementType::from_code(kind.code()), Ok(kind));
        }
        assert!(MovementType::from_code("CASH_IN").is_err());
    }

    #[test]
    fn sign_invariants_match_sql_check() {
        assert!(validate_movement_amount(MovementType::OpeningFloat, 0).is_ok());
        assert!(validate_movement_amount(MovementType::OpeningFloat, -1).is_err());
        assert!(validate_movement_amount(MovementType::SaleCashIn, 1).is_ok());
        assert!(validate_movement_amount(MovementType::SaleCashIn, 0).is_err());
        assert!(validate_movement_amount(MovementType::CreditPaymentIn, 100).is_ok());
        assert!(validate_movement_amount(MovementType::CreditPaymentIn, 0).is_err());
        assert!(validate_movement_amount(MovementType::ExpenseOut, -1).is_ok());
        assert!(validate_movement_amount(MovementType::ExpenseOut, 0).is_err());
        assert!(validate_movement_amount(MovementType::ExpenseOut, 1).is_err());
        assert!(validate_movement_amount(MovementType::ManualIn, 5).is_ok());
        assert!(validate_movement_amount(MovementType::ManualIn, 0).is_err());
        assert!(validate_movement_amount(MovementType::ManualOut, -5).is_ok());
        assert!(validate_movement_amount(MovementType::ManualOut, 0).is_err());
        assert!(validate_movement_amount(MovementType::SupplierPaymentOut, -500).is_ok());
        assert!(validate_movement_amount(MovementType::SupplierPaymentOut, 0).is_err());
        assert!(validate_movement_amount(MovementType::SupplierPaymentOut, 1).is_err());
        assert!(validate_movement_amount(MovementType::RefundOut, -500).is_ok());
        assert!(validate_movement_amount(MovementType::RefundOut, 0).is_err());
        assert!(validate_movement_amount(MovementType::RefundOut, 1).is_err());
    }
}
