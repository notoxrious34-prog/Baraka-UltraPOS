//! Derived stock levels (read cache over the movement ledger).
//!
//! `stock_levels` MUST always reconcile to the signed sum of
//! `stock_movements` for the same (product, warehouse) pair.

use serde::{Deserialize, Serialize};

use crate::catalog::ProductId;
use crate::errors::DomainError;
use crate::inventory::WarehouseId;

/// Cached on-hand quantity, in base-unit micros (may be negative only when
/// an authorized override explicitly allowed it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockLevel {
    /// Physical product (never a kit header).
    pub product_id: ProductId,
    /// Warehouse holding the stock.
    pub warehouse_id: WarehouseId,
    /// On-hand quantity in base micros.
    pub quantity_base_micro: i64,
}

/// Pure level transition: `current + signed_delta` with overflow guard.
///
/// `signed_delta` comes from
/// [`StockMovementType::signed_delta`](crate::inventory::StockMovementType::signed_delta).
/// Returns [`DomainError::MoneyOverflow`] when the result leaves `i64`
/// (the canonical fixed-point overflow signal for all quantity math).
pub fn apply_delta(current_base_micro: i64, signed_delta: i128) -> Result<i64, DomainError> {
    let next = i128::from(current_base_micro)
        .checked_add(signed_delta)
        .ok_or(DomainError::MoneyOverflow)?;
    i64::try_from(next).map_err(|_| DomainError::MoneyOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::StockMovementType;

    #[test]
    fn all_seven_types_apply_correct_signs() {
        let table = [
            (StockMovementType::SaleOut, 900),
            (StockMovementType::ReturnIn, 1_100),
            (StockMovementType::PurchaseIn, 1_100),
            (StockMovementType::AdjustmentPositive, 1_100),
            (StockMovementType::AdjustmentNegative, 900),
            (StockMovementType::TransferOut, 900),
            (StockMovementType::TransferIn, 1_100),
        ];
        for (movement, expected) in table {
            let next = apply_delta(1_000, movement.signed_delta(100)).expect("apply");
            assert_eq!(next, expected, "{}", movement.code());
        }
    }

    #[test]
    fn overflow_is_rejected() {
        assert_eq!(apply_delta(i64::MAX, 1), Err(DomainError::MoneyOverflow));
        assert_eq!(apply_delta(i64::MIN, -1), Err(DomainError::MoneyOverflow));
        assert_eq!(apply_delta(0, 0), Ok(0));
        assert_eq!(apply_delta(-50, 50), Ok(0));
    }
}
