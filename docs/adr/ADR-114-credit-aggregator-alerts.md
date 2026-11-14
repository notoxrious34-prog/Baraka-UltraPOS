# ADR-114: Customer Credit Aggregator & Limit Alerts

Status: Accepted (TASK-BRK-110, Wave 2).

## Context

BRK-108 froze per-document snapshots, but the book had no whole-picture:
no total outstanding, no at-risk list, and limit breaches surfaced only as
a hard block at checkout (`CreditLimitExceeded`). Managers needed a live
exposure strip; cashiers needed a pre-emptive dossier banner.

## Decision

- Domain classifier (`customer.rs`): `assess_credit_risk` + `CreditRisk`
  (`OverLimit` / `NearLimit` / `indebted` / `Clear`), `i128`-safe, with
  `CREDIT_NEAR_LIMIT_PERCENT = 80`. Pure, unit-pinned.
- Read-model aggregator, no migration: `get_credit_overview(top_n)` on
  `CustomerRepository` reads cached balances in one light pass (the ledger
  stays the truth; cached balances are its maintained mirror). Use case
  clamps `top_n` to 1..=100; command `customers_credit_overview` ships
  canonical money strings and u64 counts.
- Frontend: overview strip in `CustomersModule` (total / debtors /
  over-limit / near-limit; silent collapse on fetch failure) and a
  dossier banner (`assessDetailRisk` BigInt mirror; fires only on
  `LIMITED` + parseable figures, never on garbage).
- Deliberately out of scope: per-row risk badges in the POS select modal
  (directory summaries carry no balance; checkout already hard-blocks
  with a translated `CREDIT_LIMIT_EXCEDED`, so no new IPC was justified).

## Consequences

- One read path for exposure; no new tables, no backfill.
- Collapse rules on both layers: missing figures never render as alarms.
