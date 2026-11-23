//! Signal-economy computer (TASK-BRK-116, Wave 5).
//!
//! Read-only cockpit feed: broken size runs, dead stock, demand velocity,
//! model profitability, credit aging. The use case fetches plain rows
//! through [`SignalRepository`], pairs them with the pure detectors in
//! `baraka_domain::analytics::signals`, and returns cockpit-ready alerts.
//! Thresholds default to the active [`VerticalProfile`]; the caller may
//! tighten them per store. No writes, no prices invented, no floats.

use std::collections::HashMap;

use baraka_domain::analytics::signals::{
    self, AgingBuckets, BrokenRunAlert, ModelMargin, SignalThresholds, VelocitySummary,
};
use baraka_domain::primitives::Money;
use baraka_domain::sales::vertical::VerticalProfile;

use crate::errors::ApplicationError;
use crate::ports::{
    CellDemandRow, CreditExposureRow, SessionContext, SignalRepository, VerticalProfilePort,
};

/// One broken size run for the cockpit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenRunSignal {
    /// Variant model id.
    pub model_id: String,
    /// Broken cells (zero stock, live siblings).
    pub cells: Vec<BrokenRunCellSignal>,
}

/// One broken cell for the cockpit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokenRunCellSignal {
    /// Size/color labels (`M / Noir`).
    pub axis1_value: String,
    pub axis2_value: String,
    /// Sibling demand proving the run is alive (base units).
    pub sibling_units_sold: u64,
}

/// One dead-stock line for the cockpit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadStockSignal {
    /// Product id.
    pub product_id: String,
    /// On-hand base micros.
    pub qty_on_hand_micro: i64,
    /// Days since the last movement.
    pub days_since_last_movement: u32,
}

/// One model's settled margin for the cockpit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMarginSignal {
    /// Variant model id.
    pub model_id: String,
    /// Settled revenue (centimes).
    pub revenue_centimes: i64,
    /// Settled COGS (centimes).
    pub cogs_centimes: i64,
    /// Margin in basis points (`None` = no revenue, no margin).
    pub margin_bps: Option<i64>,
}

/// Full cockpit feed (one computation, five signals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalsOutput {
    /// Active profile that supplied the thresholds.
    pub profile: VerticalProfile,
    /// Broken size runs, most starved first.
    pub broken_runs: Vec<BrokenRunSignal>,
    /// Dead stock, oldest first.
    pub dead_stock: Vec<DeadStockSignal>,
    /// Demand velocity over the window (`None` = no sales at all).
    pub velocity: Option<VelocitySummary>,
    /// Per-model margins, worst first (`None` margins sink last).
    pub model_margins: Vec<ModelMarginSignal>,
    /// Credit exposure by age.
    pub aging: AgingBuckets,
}

/// Compute the cockpit feed (BRK-116).
pub struct ComputeSignalsUseCase<'a, S, V>
where
    S: ?Sized,
    V: ?Sized,
{
    signals: &'a S,
    profiles: &'a V,
}

impl<'a, S, V> ComputeSignalsUseCase<'a, S, V>
where
    S: SignalRepository + ?Sized,
    V: VerticalProfilePort + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(signals: &'a S, profiles: &'a V) -> Self {
        Self { signals, profiles }
    }

    /// Compute every signal over `window_days` of settled truth.
    ///
    /// `thresholds_override` replaces the profile defaults wholesale when
    /// `Some` (the cockpit sends one complete set, never a patch).
    pub fn compute(
        &self,
        _session: &SessionContext,
        window_days: u32,
        thresholds_override: Option<SignalThresholds>,
    ) -> Result<SignalsOutput, ApplicationError> {
        let window_days = window_days.clamp(1, 365);
        let profile = self.profiles.load_profile()?;
        let thresholds =
            thresholds_override.unwrap_or_else(|| SignalThresholds::for_profile(profile));

        Ok(SignalsOutput {
            profile,
            broken_runs: self.broken_runs(window_days, &thresholds)?,
            dead_stock: self.dead_stock(&thresholds)?,
            velocity: self.velocity(window_days, &thresholds)?,
            model_margins: self.model_margins(window_days)?,
            aging: self.aging(&thresholds)?,
        })
    }

    fn broken_runs(
        &self,
        window_days: u32,
        thresholds: &SignalThresholds,
    ) -> Result<Vec<BrokenRunSignal>, ApplicationError> {
        let rows = self.signals.cell_demand(window_days)?;
        // Group cells per model preserving the repository order (already
        // sorted by model then axis sorts); sibling demand = model total
        // minus the cell's own sales.
        let mut models: Vec<(String, Vec<CellDemandRow>)> = Vec::new();
        for row in rows {
            match models.last_mut() {
                Some((id, cells)) if *id == row.model_id => cells.push(row),
                _ => models.push((row.model_id.clone(), vec![row])),
            }
        }
        let mut out = Vec::new();
        for (model_id, cells) in models {
            let total: u64 = cells.iter().map(|c| c.units_sold_window).sum();
            let observations: Vec<signals::RunCellObservation> = cells
                .iter()
                .map(|c| signals::RunCellObservation {
                    qty_on_hand_micro: c.qty_on_hand_micro,
                    sibling_units_sold: total.saturating_sub(c.units_sold_window),
                })
                .collect();
            let alerts: Vec<BrokenRunAlert> =
                signals::detect_broken_runs(&observations, thresholds);
            if alerts.is_empty() {
                continue;
            }
            let mut flagged: Vec<BrokenRunCellSignal> = alerts
                .into_iter()
                .map(|a| {
                    let cell = &cells[a.cell_index];
                    BrokenRunCellSignal {
                        axis1_value: cell.axis1_value.clone(),
                        axis2_value: cell.axis2_value.clone(),
                        sibling_units_sold: a.sibling_units_sold,
                    }
                })
                .collect();
            flagged.sort_by_key(|c| std::cmp::Reverse(c.sibling_units_sold));
            out.push(BrokenRunSignal {
                model_id,
                cells: flagged,
            });
        }
        out.sort_by_key(|m| {
            std::cmp::Reverse(m.cells.iter().map(|c| c.sibling_units_sold).sum::<u64>())
        });
        Ok(out)
    }

    fn dead_stock(
        &self,
        thresholds: &SignalThresholds,
    ) -> Result<Vec<DeadStockSignal>, ApplicationError> {
        let rows = self.signals.product_recency()?;
        let observations: Vec<signals::StockObservation> = rows
            .iter()
            .map(|r| signals::StockObservation {
                qty_on_hand_micro: r.qty_on_hand_micro,
                days_since_last_movement: r.days_since_last_movement.unwrap_or(u32::MAX),
            })
            .collect();
        let mut out: Vec<DeadStockSignal> = signals::detect_dead_stock(&observations, thresholds)
            .into_iter()
            .map(|a| {
                let row = &rows[a.item_index];
                DeadStockSignal {
                    product_id: row.product_id.clone(),
                    qty_on_hand_micro: row.qty_on_hand_micro,
                    days_since_last_movement: a.days_since_last_movement,
                }
            })
            .collect();
        out.sort_by_key(|d| std::cmp::Reverse(d.days_since_last_movement));
        Ok(out)
    }

    fn velocity(
        &self,
        window_days: u32,
        thresholds: &SignalThresholds,
    ) -> Result<Option<VelocitySummary>, ApplicationError> {
        let to = chrono::Utc::now().date_naive();
        let from = to - chrono::Duration::days(i64::from(window_days) - 1);
        let rows = self.signals.daily_units(
            &from.format("%Y-%m-%d").to_string(),
            &to.format("%Y-%m-%d").to_string(),
        )?;
        // Fill calendar gaps with zeros: missing days sold nothing, and the
        // trend halves must span the real window, not the selling days.
        let mut by_day: HashMap<&str, u64> = HashMap::new();
        for row in &rows {
            by_day.insert(row.day.as_str(), row.base_units);
        }
        let mut days = Vec::with_capacity(window_days as usize);
        let mut cursor = from;
        while cursor <= to {
            let key = cursor.format("%Y-%m-%d").to_string();
            days.push(*by_day.get(key.as_str()).unwrap_or(&0));
            cursor += chrono::Duration::days(1);
        }
        Ok(signals::summarise_velocity(&days, thresholds))
    }

    fn model_margins(&self, window_days: u32) -> Result<Vec<ModelMarginSignal>, ApplicationError> {
        let rows = self.signals.model_economics(window_days)?;
        let mut out: Vec<ModelMarginSignal> = rows
            .into_iter()
            .map(|r| {
                let margin: ModelMargin = signals::model_margin(
                    Money::from_centimes(r.revenue_centimes),
                    Money::from_centimes(r.cogs_centimes),
                );
                ModelMarginSignal {
                    model_id: r.model_id,
                    revenue_centimes: r.revenue_centimes,
                    cogs_centimes: r.cogs_centimes,
                    margin_bps: margin.margin_bps,
                }
            })
            .collect();
        // Worst margin first; models without revenue sink last.
        out.sort_by_key(|m| m.margin_bps.unwrap_or(i64::MAX));
        Ok(out)
    }

    fn aging(&self, thresholds: &SignalThresholds) -> Result<AgingBuckets, ApplicationError> {
        let rows = self.signals.credit_exposure_entries(10_000)?;
        let exposures: Vec<signals::CreditExposure> = rows
            .into_iter()
            .map(|r: CreditExposureRow| signals::CreditExposure {
                balance: Money::from_centimes(r.balance_centimes),
                age_days: r.age_days,
            })
            .collect();
        Ok(signals::bucket_aging(&exposures, thresholds))
    }
}
