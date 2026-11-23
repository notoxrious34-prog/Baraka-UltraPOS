//! `ComputeSignalsUseCase` tests over in-memory port fakes (TASK-BRK-116).
//!
//! No SQLite here: the fake [`SignalRepository`] returns deterministic rows
//! so these tests pin the use-case grouping (siblings per model), the gap
//! filling (missing days sell nothing), the override plumbing, and the
//! profile defaulting — the SQLite reads ship separately.

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CellDemandRow, CreditExposureRow, DailyUnitsRow, ModelEconomicsRow, ProductRecencyRow,
    SessionContext, SessionToken, SignalRepository, VerticalProfilePort,
};
use baraka_application::use_cases::signals::ComputeSignalsUseCase;
use baraka_domain::analytics::signals::SignalThresholds;
use baraka_domain::role::Role;
use baraka_domain::sales::vertical::VerticalProfile;
use baraka_domain::user::UserId;

fn session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role: Role::Admin,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    }
}

struct FakeSignals {
    cells: Vec<CellDemandRow>,
    recency: Vec<ProductRecencyRow>,
    economics: Vec<ModelEconomicsRow>,
    exposures: Vec<CreditExposureRow>,
    days: Vec<DailyUnitsRow>,
}

impl SignalRepository for FakeSignals {
    fn cell_demand(&self, _window_days: u32) -> Result<Vec<CellDemandRow>, ApplicationError> {
        Ok(self.cells.clone())
    }

    fn product_recency(&self) -> Result<Vec<ProductRecencyRow>, ApplicationError> {
        Ok(self.recency.clone())
    }

    fn model_economics(
        &self,
        _window_days: u32,
    ) -> Result<Vec<ModelEconomicsRow>, ApplicationError> {
        Ok(self.economics.clone())
    }

    fn credit_exposure_entries(
        &self,
        _limit: usize,
    ) -> Result<Vec<CreditExposureRow>, ApplicationError> {
        Ok(self.exposures.clone())
    }

    fn daily_units(
        &self,
        _date_from: &str,
        _date_to: &str,
    ) -> Result<Vec<DailyUnitsRow>, ApplicationError> {
        Ok(self.days.clone())
    }
}

struct FakeProfiles;

impl VerticalProfilePort for FakeProfiles {
    fn load_profile(&self) -> Result<VerticalProfile, ApplicationError> {
        Ok(VerticalProfile::ApparelWholesaleRetail)
    }

    fn save_profile(
        &self,
        _profile: &baraka_domain::sales::VerticalProfile,
        _actor: &str,
    ) -> Result<(), ApplicationError> {
        Ok(())
    }
}

fn harness() -> (FakeSignals, FakeProfiles) {
    let today = chrono::Utc::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    (
        FakeSignals {
            cells: vec![
                CellDemandRow {
                    model_id: "M1".to_string(),
                    cell_index: 0,
                    axis1_value: "M".to_string(),
                    axis2_value: "Noir".to_string(),
                    product_id: "P-M".to_string(),
                    qty_on_hand_micro: 0,
                    units_sold_window: 2,
                },
                CellDemandRow {
                    model_id: "M1".to_string(),
                    cell_index: 1,
                    axis1_value: "L".to_string(),
                    axis2_value: "Noir".to_string(),
                    product_id: "P-L".to_string(),
                    qty_on_hand_micro: 50_000,
                    units_sold_window: 20,
                },
            ],
            recency: vec![ProductRecencyRow {
                product_id: "P-OLD".to_string(),
                qty_on_hand_micro: 100_000,
                days_since_last_movement: Some(90),
            }],
            economics: vec![ModelEconomicsRow {
                model_id: "M1".to_string(),
                revenue_centimes: 600,
                cogs_centimes: 500,
            }],
            exposures: vec![CreditExposureRow {
                customer_id: "C1".to_string(),
                balance_centimes: 100_000,
                age_days: 10,
            }],
            days: vec![DailyUnitsRow {
                day: today,
                base_units: 4,
            }],
        },
        FakeProfiles,
    )
}

#[test]
fn compute_groups_siblings_per_model_and_flags_the_broken_cell() {
    let (repo, profiles) = harness();
    let use_case = ComputeSignalsUseCase::new(&repo, &profiles);
    let out = use_case.compute(&session(), 30, None).expect("compute");
    assert_eq!(out.profile, VerticalProfile::ApparelWholesaleRetail);
    // Sibling demand for M = 22 total - 2 own = 20 >= textile 12 -> broken.
    assert_eq!(out.broken_runs.len(), 1);
    assert_eq!(out.broken_runs[0].model_id, "M1");
    assert_eq!(out.broken_runs[0].cells.len(), 1);
    assert_eq!(out.broken_runs[0].cells[0].axis1_value, "M");
    assert_eq!(out.broken_runs[0].cells[0].sibling_units_sold, 20);
}

#[test]
fn compute_surfaces_dead_stock_margin_and_aging() {
    let (repo, profiles) = harness();
    let use_case = ComputeSignalsUseCase::new(&repo, &profiles);
    let out = use_case.compute(&session(), 30, None).expect("compute");
    assert_eq!(out.dead_stock.len(), 1);
    assert_eq!(out.dead_stock[0].product_id, "P-OLD");
    assert_eq!(out.model_margins.len(), 1);
    assert_eq!(out.model_margins[0].margin_bps, Some(1_667));
    assert_eq!(
        out.aging.bucket_0_30,
        baraka_domain::primitives::Money::from_centimes(100_000)
    );
    // Gaps fill with zeros: 30-day window, one selling day.
    let velocity = out.velocity.expect("velocity");
    assert_eq!(velocity.units_per_day, 0);
}

#[test]
fn compute_honours_a_full_threshold_override() {
    let (repo, profiles) = harness();
    let use_case = ComputeSignalsUseCase::new(&repo, &profiles);
    let mut strict = SignalThresholds::for_profile(VerticalProfile::HardwareGeneral);
    strict.broken_run_min_sibling_units = 1_000_000;
    let out = use_case
        .compute(&session(), 30, Some(strict))
        .expect("compute");
    assert!(out.broken_runs.is_empty());
}
