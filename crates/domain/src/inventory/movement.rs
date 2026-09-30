//! Stock movements (append-only) and their sign semantics.
//!
//! Quantities are stored as POSITIVE magnitudes only; direction derives
//! exclusively from `StockMovementType` (see `sign`).

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::catalog::{ProductId, UnitId};
use crate::errors::DomainError;
use crate::inventory::WarehouseId;
use crate::primitives::QtyMicro;
use crate::user::UserId;

/// All movement types; TEXT values match the storage CHECK constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StockMovementType {
    /// Physical sale consumption (negative delta).
    SaleOut,
    /// Customer return restock (positive delta).
    ReturnIn,
    /// Supplier purchase restock (positive delta).
    PurchaseIn,
    /// Manual upward correction (positive delta).
    AdjustmentPositive,
    /// Manual downward correction (negative delta).
    AdjustmentNegative,
    /// Transfer leg leaving the source warehouse (negative delta).
    TransferOut,
    /// Transfer leg entering the destination warehouse (positive delta).
    TransferIn,
}

impl StockMovementType {
    /// Canonical storage TEXT value.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SaleOut => "SALE_OUT",
            Self::ReturnIn => "RETURN_IN",
            Self::PurchaseIn => "PURCHASE_IN",
            Self::AdjustmentPositive => "ADJUSTMENT_POSITIVE",
            Self::AdjustmentNegative => "ADJUSTMENT_NEGATIVE",
            Self::TransferOut => "TRANSFER_OUT",
            Self::TransferIn => "TRANSFER_IN",
        }
    }

    /// Direction of the applied delta: +1 grows stock, -1 shrinks it.
    #[must_use]
    pub const fn sign(self) -> i128 {
        match self {
            Self::ReturnIn | Self::PurchaseIn | Self::AdjustmentPositive | Self::TransferIn => 1,
            Self::SaleOut | Self::AdjustmentNegative | Self::TransferOut => -1,
        }
    }

    /// Signed delta for a positive magnitude, in base micros.
    #[must_use]
    pub fn signed_delta(self, magnitude_base_micro: i64) -> i128 {
        self.sign() * i128::from(magnitude_base_micro)
    }

    /// Parse a stored TEXT value.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "SALE_OUT" => Ok(Self::SaleOut),
            "RETURN_IN" => Ok(Self::ReturnIn),
            "PURCHASE_IN" => Ok(Self::PurchaseIn),
            "ADJUSTMENT_POSITIVE" => Ok(Self::AdjustmentPositive),
            "ADJUSTMENT_NEGATIVE" => Ok(Self::AdjustmentNegative),
            "TRANSFER_OUT" => Ok(Self::TransferOut),
            "TRANSFER_IN" => Ok(Self::TransferIn),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown movement type: {other}"
            ))),
        }
    }
}

impl Display for StockMovementType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.code())
    }
}

impl FromStr for StockMovementType {
    type Err = DomainError;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        Self::from_code(code)
    }
}

/// Business document a movement references, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceType {
    /// Point-of-sale document (future TASK-BRK-006).
    Sale,
    /// Return document (future engine).
    Return,
    /// Purchase document (future engine).
    Purchase,
    /// Warehouse-to-warehouse transfer pair.
    Transfer,
}

impl ReferenceType {
    /// Canonical storage TEXT value.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Sale => "SALE",
            Self::Return => "RETURN",
            Self::Purchase => "PURCHASE",
            Self::Transfer => "TRANSFER",
        }
    }

    /// Parse a stored TEXT value.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "SALE" => Ok(Self::Sale),
            "RETURN" => Ok(Self::Return),
            "PURCHASE" => Ok(Self::Purchase),
            "TRANSFER" => Ok(Self::Transfer),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown reference type: {other}"
            ))),
        }
    }
}

/// One immutable stock ledger line (magnitude-only quantities).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockMovement {
    /// Stable identifier (TEXT UUID).
    pub id: Uuid,
    /// Physical product moved (never a kit header).
    pub product_id: ProductId,
    /// Warehouse where the movement lands.
    pub warehouse_id: WarehouseId,
    /// Direction + business meaning.
    pub movement_type: StockMovementType,
    /// Selling unit the operator handled.
    pub unit_id: UnitId,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: QtyMicro,
    /// Positive magnitude in base-unit micros.
    pub quantity_base_micro: i64,
    /// Referenced document kind, if any.
    pub reference_type: Option<ReferenceType>,
    /// Referenced document id, if any.
    pub reference_id: Option<String>,
    /// Shared id of a transfer OUT/IN pair, if any.
    pub transfer_batch_id: Option<String>,
    /// True only when an authorized negative-stock path was taken.
    pub authorized_negative_stock: bool,
    /// Human reason (mandatory for adjustments).
    pub reason: Option<String>,
    /// Capture instant.
    pub created_at: DateTime<Utc>,
    /// Acting operator.
    pub created_by: UserId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_and_signs() {
        let table = [
            (StockMovementType::SaleOut, "SALE_OUT", -1),
            (StockMovementType::ReturnIn, "RETURN_IN", 1),
            (StockMovementType::PurchaseIn, "PURCHASE_IN", 1),
            (
                StockMovementType::AdjustmentPositive,
                "ADJUSTMENT_POSITIVE",
                1,
            ),
            (
                StockMovementType::AdjustmentNegative,
                "ADJUSTMENT_NEGATIVE",
                -1,
            ),
            (StockMovementType::TransferOut, "TRANSFER_OUT", -1),
            (StockMovementType::TransferIn, "TRANSFER_IN", 1),
        ];
        for (movement, code, sign) in table {
            assert_eq!(movement.code(), code);
            assert_eq!(movement.to_string(), code);
            assert_eq!(movement.sign(), sign);
            assert_eq!(StockMovementType::from_code(code), Ok(movement));
            assert_eq!(movement.signed_delta(1_000), sign * 1_000);
        }
        assert!(StockMovementType::from_code("NOPE").is_err());
        assert_eq!(
            ReferenceType::from_code("TRANSFER"),
            Ok(ReferenceType::Transfer)
        );
    }
}
