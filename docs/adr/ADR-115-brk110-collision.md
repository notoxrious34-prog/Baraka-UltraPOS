# ADR-115: BRK-110 Number Collision — Decision Pending

Status: Open (collision recorded, no resolution applied).

## Fact

- Shipped: credit aggregator + limit alerts under `TASK-BRK-110`
  (commits `244aaa7`, `a56dac3`; `ADR-114`; ~10 `BRK-110` code references).
- Ratified roadmap (`ROADMAP-v2.0-DUAL-VERTICAL.md`, Wave 2) already
  assigns `TASK-BRK-110` to vertical profiles + `Super-Gros` tiers.
- `BRK-111..116` are all roadmap-reserved; no code uses them yet.

## Rule applied

Per constitution Pillar 3: the conflict is recorded, nothing is
renamed or rewritten unilaterally (no history rewrite, no roadmap edit
without owner sign-off). The owner was asked; no answer yet.

## Options on the table

1. Aggregator keeps `BRK-110`; profiles move to `BRK-117` (roadmap
   one-line amendment + this ADR updated to Accepted).
2. Aggregator renamed to `BRK-117` (~10 code/test references);
   profiles keep `BRK-110`.
3. Independent `BRK-111` (last-price memory) proceeds first; numbering
   resolved in parallel.

## Consequence until resolved

Wave 2 profiles work (`DSN-105` depends on it) is blocked on the
numbering decision. Collision-free `BRK-111` may proceed independently.
