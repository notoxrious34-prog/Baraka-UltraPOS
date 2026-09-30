//! Crash-matrix scenario table, payload builders, and JSON status helpers.
//!
//! Scenario ids are stable kebab strings (`cash-kp3`, `credit-kp5`, …) and
//! double as the deterministic `command_id` seed: every attempt of one
//! scenario reuses the same command id, which is exactly what the
//! exactly-once retry proof requires.

use std::path::{Path, PathBuf};

/// Cash or credit variant of the single-line sale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaleKind {
    Cash,
    Credit,
}

/// Semantic kill points (ADR-063 §2). `Kp2` is the documented collapse:
///
/// the idempotency claim lives INSIDE the sale transaction (command_log
/// INSERT is step 9 of 9 in `SqliteCheckoutRepository::checkout_atomic`),
/// so there is no separate claim commit to kill after. KP-2 therefore
/// targets the first in-transaction write (document_sequences).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillPoint {
    Kp1PreBegin,
    Kp2FirstWrite,
    Kp3Documents,
    Kp4Stock,
    Kp5Ledger,
    Kp6PrintJob,
    Kp7aCommit,
    Kp7bPostResponse,
}

/// One matrix row: scenario id, sale kind, kill point.
#[derive(Debug, Clone, Copy)]
pub struct Scenario {
    /// Stable id, e.g. `credit-kp5`.
    pub id: &'static str,
    pub kind: SaleKind,
    pub point: KillPoint,
}

const fn row(kind: SaleKind, point: KillPoint, id: &'static str) -> Scenario {
    Scenario { id, kind, point }
}

/// The §5 matrix: 7 cash + 8 credit scenarios. Kit expansion, mixed
/// payment, returns, and non-sale flows are out of scope (own suites).
pub const SCENARIOS: &[Scenario] = &[
    row(SaleKind::Cash, KillPoint::Kp1PreBegin, "cash-kp1"),
    row(SaleKind::Cash, KillPoint::Kp2FirstWrite, "cash-kp2"),
    row(SaleKind::Cash, KillPoint::Kp3Documents, "cash-kp3"),
    row(SaleKind::Cash, KillPoint::Kp4Stock, "cash-kp4"),
    row(SaleKind::Cash, KillPoint::Kp6PrintJob, "cash-kp6"),
    row(SaleKind::Cash, KillPoint::Kp7aCommit, "cash-kp7a"),
    row(SaleKind::Cash, KillPoint::Kp7bPostResponse, "cash-kp7b"),
    row(SaleKind::Credit, KillPoint::Kp1PreBegin, "credit-kp1"),
    row(SaleKind::Credit, KillPoint::Kp2FirstWrite, "credit-kp2"),
    row(SaleKind::Credit, KillPoint::Kp3Documents, "credit-kp3"),
    row(SaleKind::Credit, KillPoint::Kp4Stock, "credit-kp4"),
    row(SaleKind::Credit, KillPoint::Kp5Ledger, "credit-kp5"),
    row(SaleKind::Credit, KillPoint::Kp6PrintJob, "credit-kp6"),
    row(SaleKind::Credit, KillPoint::Kp7aCommit, "credit-kp7a"),
    row(SaleKind::Credit, KillPoint::Kp7bPostResponse, "credit-kp7b"),
];

/// Deterministic command id: every attempt of a scenario shares it.
#[must_use]
pub fn command_id_for(scenario_id: &str) -> String {
    format!("crash-{scenario_id}")
}

/// Fixed payload economics, shared by driver and verifier:
/// one unit at 10 000 centimes, factor 10 000 micro per unit.
pub const UNIT_PRICE_CENTIMES: i64 = 10_000;
pub const FACTOR_MICRO: i64 = 10_000;
/// Sale quantity in whole units (one leaf, one line).
pub const QTY_UNITS: i64 = 1;
/// Base-micros moved by the fixed payload.
pub const QTY_BASE_MICRO: i64 = QTY_UNITS * FACTOR_MICRO;
/// Timbre fiscal on cash tickets (pricing kernel: ceil(1 %/100) stepped,
/// clamped to [500, 250000]): base 10 000 → stepped 100 → clamped 500.
/// Cash tender must therefore be 10 500 exact (change 0).
pub const TIMBRE_CENTIMES: i64 = 500;
/// Cash ticket total for the fixed payload (no VAT baseline).
pub const CASH_TTC_CENTIMES: i64 = UNIT_PRICE_CENTIMES + TIMBRE_CENTIMES;
/// Credit ticket total (no cash tendered → no timbre).
pub const CREDIT_TTC_CENTIMES: i64 = UNIT_PRICE_CENTIMES;
/// Seeded on-hand base micros (covers every scenario sale with margin).
pub const SEED_LEVEL_MICRO: i64 = 1_000_000;

/// Database file name inside a scenario temp dir.
#[must_use]
pub fn db_path(dir: &Path) -> PathBuf {
    dir.join("crash.db")
}

/// Minimal CLI scan: returns the value following `--flag`, if present.
/// Unknown args (cargo harness flags, …) are ignored gracefully.
#[must_use]
pub fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|pair| {
        if pair[0] == flag {
            Some(pair[1].clone())
        } else {
            None
        }
    })
}

/// Emit the single machine-readable JSON status line (normal exits only;
/// abort exits produce no output by design).
pub fn emit_status_line(outcome: &str, document_number: &str) {
    println!("{{\"outcome\":\"{outcome}\",\"document_number\":\"{document_number}\"}}");
}

/// Parse a child status line into (outcome, document_number).
#[must_use]
pub fn parse_status_line(line: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let outcome = value.get("outcome")?.as_str()?.to_string();
    let number = value
        .get("document_number")?
        .as_str()
        .unwrap_or("")
        .to_string();
    Some((outcome, number))
}
