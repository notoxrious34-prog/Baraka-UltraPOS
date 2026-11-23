//! Signal analytics kernel (TASK-BRK-116, Wave 5).
//!
//! Pure math over plain input rows: zero I/O, zero floats, zero profile
//! branches. Profiles only supply [`SignalThresholds`] defaults through
//! [`SignalThresholds::for_profile`]; every detector below takes the
//! thresholds as a parameter, so core math never asks which trade runs
//! the counter (ADR-110).
//!
//! All ratios are integer basis points (1/100 of a percent, 10_000 = 100%)
//! with `ROUND_HALF_UP` on `i128` intermediates, mirroring the pricing
//! kernel. Quantities ride fixed-point micro units (`10^4`).

use crate::primitives::Money;
use crate::sales::vertical::VerticalProfile;

/// Tunable knobs for every signal detector. Profiles pick the defaults;
/// the counter manager may tighten them per store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalThresholds {
    /// A zero-stock cell counts as a broken run only when its siblings
    /// sold at least this many base units in the lookback window.
    pub broken_run_min_sibling_units: u64,
    /// Lookback window (days) for sibling demand and velocity.
    pub demand_lookback_days: u32,
    /// Stock with no movement for this long is dead.
    pub dead_stock_days: u32,
    /// Dead-stock flag needs at least this much base quantity on hand.
    pub dead_stock_min_qty_micro: i64,
    /// Velocity below this many base units/day is "slow".
    pub slow_units_per_day: u64,
    /// Credit bucket boundaries (days).
    pub aging_bucket_days: [u32; 3],
}

impl SignalThresholds {
    /// Textile turns fast in small sizes: short patience, twitchy runs.
    /// Hardware turns slow in bulky units: long patience, calm runs.
    /// Hybrid takes the stricter (textile) side so nothing rots unseen.
    #[must_use]
    pub const fn for_profile(profile: VerticalProfile) -> Self {
        match profile {
            VerticalProfile::ApparelWholesaleRetail | VerticalProfile::HybridEnterprise => Self {
                broken_run_min_sibling_units: 12,
                demand_lookback_days: 30,
                dead_stock_days: 60,
                dead_stock_min_qty_micro: 10_000,
                slow_units_per_day: 1,
                aging_bucket_days: [30, 60, 90],
            },
            VerticalProfile::HardwareGeneral => Self {
                broken_run_min_sibling_units: 3,
                demand_lookback_days: 90,
                dead_stock_days: 180,
                dead_stock_min_qty_micro: 10_000,
                slow_units_per_day: 1,
                aging_bucket_days: [30, 60, 90],
            },
        }
    }
}

/// One matrix cell observed for a broken size run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCellObservation {
    /// Base quantity currently on hand (micro units).
    pub qty_on_hand_micro: i64,
    /// Base units the SIBLING cells sold inside the lookback window.
    pub sibling_units_sold: u64,
}

/// Zero-stock cell inside a model that still sells around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenRunAlert {
    /// Index of the broken cell inside the caller's ordering.
    pub cell_index: usize,
    /// Sibling demand that proves the run is alive.
    pub sibling_units_sold: u64,
}

/// Flag every zero-stock cell whose siblings prove demand.
///
/// Pure: no model identity needed, the caller maps indexes back.
#[must_use]
pub fn detect_broken_runs(
    cells: &[RunCellObservation],
    thresholds: &SignalThresholds,
) -> Vec<BrokenRunAlert> {
    cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| {
            cell.qty_on_hand_micro <= 0
                && cell.sibling_units_sold >= thresholds.broken_run_min_sibling_units
        })
        .map(|(index, cell)| BrokenRunAlert {
            cell_index: index,
            sibling_units_sold: cell.sibling_units_sold,
        })
        .collect()
}

/// One stocked product observed for dead-stock radar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockObservation {
    /// Base quantity currently on hand (micro units).
    pub qty_on_hand_micro: i64,
    /// Days since the last stock movement (in or out).
    pub days_since_last_movement: u32,
}

/// Stocked product with no movement past the patience threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadStockAlert {
    /// Index of the dead item inside the caller's ordering.
    pub item_index: usize,
    /// Days since the last movement.
    pub days_since_last_movement: u32,
}

/// Flag stocked products the counter forgot.
#[must_use]
pub fn detect_dead_stock(
    items: &[StockObservation],
    thresholds: &SignalThresholds,
) -> Vec<DeadStockAlert> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.qty_on_hand_micro >= thresholds.dead_stock_min_qty_micro
                && item.days_since_last_movement >= thresholds.dead_stock_days
        })
        .map(|(index, item)| DeadStockAlert {
            item_index: index,
            days_since_last_movement: item.days_since_last_movement,
        })
        .collect()
}

/// Demand velocity over the lookback window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VelocitySummary {
    /// Base units/day, whole units (floor).
    pub units_per_day: u64,
    /// `true` when below the profile's slow line.
    pub is_slow: bool,
    /// Trend in basis points: recent half vs older half
    /// (`None` when the older half is empty — no baseline, no lie).
    pub trend_bps: Option<i64>,
}

/// Summarise daily base-unit sales; `None` on empty input (no data, no signal).
#[must_use]
pub fn summarise_velocity(
    daily_base_units: &[u64],
    thresholds: &SignalThresholds,
) -> Option<VelocitySummary> {
    if daily_base_units.is_empty() {
        return None;
    }
    let total: u128 = daily_base_units.iter().map(|&d| d as u128).sum();
    let days = daily_base_units.len() as u128;
    let units_per_day = (total / days) as u64;
    let half = daily_base_units.len() / 2;
    let trend_bps = if half == 0 {
        None
    } else {
        let older: u128 = daily_base_units[..half].iter().map(|&d| d as u128).sum();
        let recent: u128 = daily_base_units[half..].iter().map(|&d| d as u128).sum();
        if older == 0 {
            None
        } else {
            let delta = recent as i128 - older as i128;
            Some(((delta * 10_000) / older as i128) as i64)
        }
    };
    Some(VelocitySummary {
        units_per_day,
        is_slow: units_per_day < thresholds.slow_units_per_day,
        trend_bps,
    })
}

/// Credit exposure split by age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgingBuckets {
    pub bucket_0_30: Money,
    pub bucket_31_60: Money,
    pub bucket_61_90: Money,
    pub bucket_90_plus: Money,
}

impl Default for AgingBuckets {
    fn default() -> Self {
        Self {
            bucket_0_30: Money::zero(),
            bucket_31_60: Money::zero(),
            bucket_61_90: Money::zero(),
            bucket_90_plus: Money::zero(),
        }
    }
}

impl AgingBuckets {
    /// Total exposure across all buckets.
    #[must_use]
    pub fn total(self) -> Money {
        self.bucket_0_30
            .checked_add(self.bucket_31_60)
            .and_then(|s| s.checked_add(self.bucket_61_90))
            .and_then(|s| s.checked_add(self.bucket_90_plus))
            .expect("aging total overflows i64 centimes")
    }
}

/// One open credit balance with its age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreditExposure {
    pub balance: Money,
    pub age_days: u32,
}

/// Bucket open balances by age. Negative balances (advance payments)
/// never inflate exposure: they land in the freshest bucket as-is so the
/// total stays exact while aging stays honest.
#[must_use]
pub fn bucket_aging(exposures: &[CreditExposure], thresholds: &SignalThresholds) -> AgingBuckets {
    let [b1, b2, b3] = thresholds.aging_bucket_days;
    let mut out = AgingBuckets::default();
    for exposure in exposures {
        let slot = if exposure.age_days <= b1 {
            &mut out.bucket_0_30
        } else if exposure.age_days <= b2 {
            &mut out.bucket_31_60
        } else if exposure.age_days <= b3 {
            &mut out.bucket_61_90
        } else {
            &mut out.bucket_90_plus
        };
        *slot = slot
            .checked_add(exposure.balance)
            .expect("aging bucket overflows i64 centimes");
    }
    out
}

/// Model profitability from settled totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelMargin {
    /// Margin in basis points (`None` when revenue is zero — no sales, no margin).
    pub margin_bps: Option<i64>,
}

/// `ROUND_HALF_UP` margin on `i128` intermediates; `None` on zero revenue.
#[must_use]
pub fn model_margin(revenue: Money, cogs: Money) -> ModelMargin {
    if revenue.centimes() <= 0 {
        return ModelMargin { margin_bps: None };
    }
    let profit = revenue.centimes() as i128 - cogs.centimes() as i128;
    let raw = profit * 10_000;
    let denom = revenue.centimes() as i128;
    // ROUND_HALF_UP: bias the remainder away from zero.
    let rounded = if raw >= 0 {
        (raw + denom / 2) / denom
    } else {
        (raw - denom / 2) / denom
    };
    ModelMargin {
        margin_bps: Some(rounded as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sales::vertical::VerticalProfile;

    fn textile() -> SignalThresholds {
        SignalThresholds::for_profile(VerticalProfile::ApparelWholesaleRetail)
    }

    #[test]
    fn broken_run_flags_zero_cell_with_live_siblings() {
        let cells = vec![
            RunCellObservation {
                qty_on_hand_micro: 50_000,
                sibling_units_sold: 0,
            },
            RunCellObservation {
                qty_on_hand_micro: 0,
                sibling_units_sold: 30,
            },
            RunCellObservation {
                qty_on_hand_micro: 0,
                sibling_units_sold: 2,
            },
        ];
        let alerts = detect_broken_runs(&cells, &textile());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].cell_index, 1);
        assert_eq!(alerts[0].sibling_units_sold, 30);
    }

    #[test]
    fn dead_stock_flags_forgotten_shelf() {
        let items = vec![
            StockObservation {
                qty_on_hand_micro: 100_000,
                days_since_last_movement: 90,
            },
            StockObservation {
                qty_on_hand_micro: 100_000,
                days_since_last_movement: 10,
            },
            StockObservation {
                qty_on_hand_micro: 0,
                days_since_last_movement: 400,
            },
        ];
        let alerts = detect_dead_stock(&items, &textile());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].item_index, 0);
    }

    #[test]
    fn velocity_empty_is_no_signal_and_trend_is_honest() {
        assert_eq!(summarise_velocity(&[], &textile()), None);
        // Older half empty -> no baseline, no lie.
        let flat = vec![0, 0, 5, 5];
        assert_eq!(
            summarise_velocity(&flat, &textile())
                .expect("velocity")
                .trend_bps,
            None
        );
        // Recent double vs older -> +10000 bps.
        let rising = vec![2, 2, 4, 4];
        let summary = summarise_velocity(&rising, &textile()).expect("velocity");
        assert_eq!(summary.units_per_day, 3);
        assert_eq!(summary.trend_bps, Some(10_000));
        assert!(!summary.is_slow);
    }

    #[test]
    fn aging_buckets_are_exact_and_total() {
        let exposures = vec![
            CreditExposure {
                balance: Money::from_centimes(100_000),
                age_days: 10,
            },
            CreditExposure {
                balance: Money::from_centimes(50_000),
                age_days: 45,
            },
            CreditExposure {
                balance: Money::from_centimes(25_000),
                age_days: 75,
            },
            CreditExposure {
                balance: Money::from_centimes(10_000),
                age_days: 200,
            },
        ];
        let buckets = bucket_aging(&exposures, &textile());
        assert_eq!(buckets.bucket_0_30, Money::from_centimes(100_000));
        assert_eq!(buckets.bucket_31_60, Money::from_centimes(50_000));
        assert_eq!(buckets.bucket_61_90, Money::from_centimes(25_000));
        assert_eq!(buckets.bucket_90_plus, Money::from_centimes(10_000));
        assert_eq!(buckets.total(), Money::from_centimes(185_000));
    }

    #[test]
    fn margin_rounds_half_up_and_refuses_zero_revenue() {
        assert_eq!(
            model_margin(Money::from_centimes(0), Money::from_centimes(0)).margin_bps,
            None
        );
        // 1/6 margin = 1666.66… -> 1667 half-up.
        assert_eq!(
            model_margin(Money::from_centimes(600), Money::from_centimes(500)).margin_bps,
            Some(1_667)
        );
        // Exact third: 200/300 -> 6666.66 -> 6667.
        assert_eq!(
            model_margin(Money::from_centimes(300), Money::from_centimes(100)).margin_bps,
            Some(6_667)
        );
    }

    #[test]
    fn hardware_is_more_patient_than_textile() {
        let hardware = SignalThresholds::for_profile(VerticalProfile::HardwareGeneral);
        assert!(hardware.dead_stock_days > textile().dead_stock_days);
        assert!(hardware.demand_lookback_days > textile().demand_lookback_days);
    }
}
