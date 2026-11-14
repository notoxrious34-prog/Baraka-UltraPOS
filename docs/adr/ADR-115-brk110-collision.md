# ADR-115: BRK-110 Number Collision — Resolved

Status: Accepted (owner decision 2026-11-14: option 1).

## Fact

- Shipped: credit aggregator + limit alerts under `TASK-BRK-110`
  (commits `244aaa7`, `a56dac3`; `ADR-114`; ~10 `BRK-110` code references).
- Ratified roadmap (Wave 2) had assigned `TASK-BRK-110` to vertical
  profiles + `Super-Gros` tiers; `BRK-111..116` are all reserved.

## Decision (option 1 — minimal churn)

The shipped aggregator keeps `BRK-110`. The profiles item moves to
`TASK-BRK-117` (verified free: only cited here as an option).
`ROADMAP-v2.0-DUAL-VERTICAL.md` Wave 2 amended accordingly; `DSN-105`
now keys off `BRK-117`.

## Consequence

No history rewrite, no code rename. Profiles + Super-Gros execute as
`BRK-117`; `BRK-111` (last-price memory) proceeds independently.
