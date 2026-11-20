//! Supplier price-deviation preview (DSN-109, proposal 7).
//!
//! Margin protection at source: each receipt line is compared against the
//! last purchase price and the 90-day average. A line is flagged only when
//! its unit cost exceeds BOTH references by more than the threshold. The
//! preview is advisory — it never blocks the atomic receipt.

use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Permission;

use crate::errors::ApplicationError;
use crate::ports::{CostReference, PurchaseRepository, SessionContext};

/// One receipt line under cost review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveCostLineInput {
    /// Product purchased.
    pub product_id: ProductId,
    /// Handling unit purchased in.
    pub unit_id: UnitId,
    /// Quantity in unit-micros (validated downstream, not here).
    pub quantity_micro: i64,
    /// Proposed unit cost in centimes.
    pub unit_cost_centimes: i64,
}

/// Advisory deviation verdict for one receipt line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveDeviation {
    /// Product reviewed.
    pub product_id: ProductId,
    /// Last posted receipt cost, if any history exists.
    pub last_cost_centimes: Option<i64>,
    /// 90-day average receipt cost, if any history exists.
    pub average90_cost_centimes: Option<i64>,
    /// Signed drift vs last in basis points, if a last cost exists.
    pub delta_vs_last_bps: Option<i64>,
    /// Signed drift vs the 90-day average in basis points, if it exists.
    pub delta_vs_avg_bps: Option<i64>,
    /// True only when the cost beats both references by more than the
    /// threshold. No history never flags.
    pub flagged: bool,
}

/// Signed drift in basis points: `(cost - reference) * 10_000 / reference`.
///
/// Returns `None` for a missing or non-positive reference (never divide by
/// zero, never treat "no history" as zero).
fn delta_bps(cost_centimes: i64, reference: Option<i64>) -> Option<i64> {
    let reference = reference?;
    if reference <= 0 {
        return None;
    }
    let drift = i128::from(cost_centimes) - i128::from(reference);
    Some((drift * 10_000 / i128::from(reference)) as i64)
}

/// Local gate: `PurchaseReceive` holders only (Gerant/Admin) — costs are
/// manager-only data.
fn require(session: &SessionContext, permission: Permission) -> Result<(), ApplicationError> {
    if session.role.has_permission(permission) {
        Ok(())
    } else {
        Err(ApplicationError::Unauthorized)
    }
}

/// Preview supplier price deviations without touching stock or ledgers.
pub struct PreviewReceiveDeviationsUseCase<'a, R>
where
    R: ?Sized,
{
    purchases: &'a R,
}

impl<'a, R> PreviewReceiveDeviationsUseCase<'a, R>
where
    R: PurchaseRepository + ?Sized,
{
    /// Borrow the purchase repository for cost history.
    #[must_use]
    pub const fn new(purchases: &'a R) -> Self {
        Self { purchases }
    }

    /// Compare each line against its cost references.
    ///
    /// Costs stay manager-only: requires `PurchaseReceive`.
    pub fn execute(
        &self,
        session: &SessionContext,
        lines: &[ReceiveCostLineInput],
        threshold_bps: i64,
    ) -> Result<Vec<ReceiveDeviation>, ApplicationError> {
        require(session, Permission::PurchaseReceive)?;
        if threshold_bps < 0 {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("threshold_bps must be >= 0".to_string()),
            ));
        }
        let mut out = Vec::with_capacity(lines.len());
        for line in lines {
            let CostReference {
                last_cost_centimes,
                average90_cost_centimes,
                ..
            } = self.purchases.purchase_cost_reference(line.product_id)?;
            let delta_last = delta_bps(line.unit_cost_centimes, last_cost_centimes);
            let delta_avg = delta_bps(line.unit_cost_centimes, average90_cost_centimes);
            let flagged = matches!(
                (delta_last, delta_avg),
                (Some(last), Some(avg)) if last > threshold_bps && avg > threshold_bps
            );
            out.push(ReceiveDeviation {
                product_id: line.product_id,
                last_cost_centimes,
                average90_cost_centimes,
                delta_vs_last_bps: delta_last,
                delta_vs_avg_bps: delta_avg,
                flagged,
            });
        }
        Ok(out)
    }
}
