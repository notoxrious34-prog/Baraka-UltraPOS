//! Pure stock-movement policy kernel shared by every engine caller.
//!
//! Layering note: SQL execution lives in infrastructure (only it may name
//! `rusqlite` types), so the atomic read-modify-write runs there. This
//! module owns the sign table, the negative-stock policy decision, and the
//! new-quantity computation, so the SQLite engine, tests, and the future
//! Sale engine all share one audited decision core.

use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{apply_delta, StockMovementType};

use crate::errors::ApplicationError;

/// Outcome of the negative-stock policy check for a computed level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegativeStockPolicy {
    /// Proceed normally (level stays non-negative).
    Allowed,
    /// Proceed with the authorized override (audit row mandatory).
    AllowedWithOverride,
    /// Reject with `InsufficientStock`.
    Rejected,
}

/// Decide the negative-stock path for a computed `new_quantity`.
///
/// Transfers always reject: physical stock must exist at the source.
/// Every other movement rejects unless `authorize_negative_stock` was
/// explicitly granted by a permission-checked caller.
pub fn decide_negative_policy(
    movement_type: StockMovementType,
    new_quantity_base_micro: i64,
    authorize_negative_stock: bool,
) -> NegativeStockPolicy {
    if new_quantity_base_micro >= 0 {
        return NegativeStockPolicy::Allowed;
    }
    match movement_type {
        StockMovementType::TransferOut => NegativeStockPolicy::Rejected,
        _ if authorize_negative_stock => NegativeStockPolicy::AllowedWithOverride,
        _ => NegativeStockPolicy::Rejected,
    }
}

/// Resolve the effective authorization flag for an adjustment use case.
///
/// - Without `AdjustStock`: `Unauthorized` always.
/// - `requested_authorize` without `NegativeStockOverride`: `Unauthorized`
///   (the override permission is checked exactly when authorization is
///   requested, never on the positive path).
/// - Otherwise the caller's flag passes through untouched; the engine
///   rejects negative outcomes itself when the flag is `false`.
pub fn resolve_adjust_authorization(
    has_adjust: bool,
    has_override: bool,
    requested_authorize: bool,
) -> Result<bool, ApplicationError> {
    if !has_adjust {
        return Err(ApplicationError::Unauthorized);
    }
    if requested_authorize && !has_override {
        return Err(ApplicationError::Unauthorized);
    }
    Ok(requested_authorize)
}

/// Compute the post-movement level, or fail on fixed-point overflow.
pub fn compute_new_quantity(
    current_base_micro: i64,
    movement_type: StockMovementType,
    quantity_base_micro: i64,
) -> Result<i64, DomainError> {
    apply_delta(
        current_base_micro,
        movement_type.signed_delta(quantity_base_micro),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_matrix_covers_all_types_and_paths() {
        use NegativeStockPolicy::{Allowed, AllowedWithOverride, Rejected};
        // Non-negative results always proceed.
        for movement in [
            StockMovementType::SaleOut,
            StockMovementType::ReturnIn,
            StockMovementType::PurchaseIn,
            StockMovementType::AdjustmentPositive,
            StockMovementType::AdjustmentNegative,
            StockMovementType::TransferOut,
            StockMovementType::TransferIn,
        ] {
            assert_eq!(decide_negative_policy(movement, 0, false), Allowed);
            assert_eq!(decide_negative_policy(movement, 100, false), Allowed);
        }
        // Negative results: transfers refuse even when authorized.
        assert_eq!(
            decide_negative_policy(StockMovementType::TransferOut, -1, true),
            Rejected
        );
        assert_eq!(
            decide_negative_policy(StockMovementType::TransferOut, -1, false),
            Rejected
        );
        // Other movements honor the explicit authorization flag.
        assert_eq!(
            decide_negative_policy(StockMovementType::SaleOut, -1, true),
            AllowedWithOverride
        );
        assert_eq!(
            decide_negative_policy(StockMovementType::SaleOut, -1, false),
            Rejected
        );
        assert_eq!(
            decide_negative_policy(StockMovementType::AdjustmentNegative, -5, true),
            AllowedWithOverride
        );
    }

    #[test]
    fn new_quantity_guards_overflow() {
        assert_eq!(
            compute_new_quantity(1_000, StockMovementType::PurchaseIn, 500),
            Ok(1_500)
        );
        assert_eq!(
            compute_new_quantity(i64::MAX, StockMovementType::PurchaseIn, 1),
            Err(DomainError::MoneyOverflow)
        );
    }

    #[test]
    fn adjust_authorization_branches() {
        // No AdjustStock: always unauthorized.
        assert!(matches!(
            resolve_adjust_authorization(false, true, false),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            resolve_adjust_authorization(false, false, true),
            Err(ApplicationError::Unauthorized)
        ));
        // Flag passes through when no override is requested.
        assert!(matches!(
            resolve_adjust_authorization(true, false, false),
            Ok(false)
        ));
        // Requested override without the permission: rejected.
        assert!(matches!(
            resolve_adjust_authorization(true, false, true),
            Err(ApplicationError::Unauthorized)
        ));
        // Requested override with the permission: honored.
        assert!(matches!(
            resolve_adjust_authorization(true, true, true),
            Ok(true)
        ));
        assert!(matches!(
            resolve_adjust_authorization(true, true, false),
            Ok(false)
        ));
    }
}
