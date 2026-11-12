# ADR-112: Read-Time Colisage Breakdown on Document Lines

Status: ACCEPTED.
Date: 2026-11-12.
Authority: TASK-BRK-109 (Wave 1 of `ROADMAP-v2.0-DUAL-VERTICAL.md`, ADR-110).

## Context

A document line stores its quantity in base micros; merchants think in
selling rungs (`Pièce`, `Boîte ×3`, `Demi-Dz ×6`, `Douzaine ×12`,
`Balle/Carton`). Printing or displaying the raw base quantity forces the
cashier to do ladder arithmetic by hand — exactly the wholesale friction
Wave 1 must kill. The ladder already exists (`product_units.factor_micro`,
immutable base rung at 10_000); no schema was missing, only the engine.

Two design facts, both pinned by tests:

1. `documents.paid_amount_centimes` counts every tender (BRK-108 probe), and
   symmetrically a line's `quantity_base_micro` counts every micro — the
   breakdown must therefore be computed from the STORED line quantity, never
   re-derived from prices.
2. Fractional hardware quantities (12.5 m) have no whole-rung expression.
   Refusing them (empty vector) is honest; rounding them would print a lie.

## Decision

1. Domain `sales::colisage` (`crates/domain/src/sales/colisage.rs`) is the
   only arithmetic authority: `breakdown(base_micros, ladder)` expresses any
   non-negative base quantity as whole rungs, largest first, in `i128` with
   explicit overflow errors. Only nonzero rungs are emitted; a nonzero
   remainder (sub-rung fraction, missing base rung) is refused with
   `InvalidQuantity`, never silently rounded.
2. Read-time, NO migration: `DocumentRepository::get_items_colisage`
   (ports) → `SqliteDocumentRepository` loads each line's live unit ladder
   and breaks the stored base quantity down. Ladder gaps degrade to an empty
   vector — the read never fails, mirroring the orphaned-name policy.
3. `SaleDetail.items_colisage` (application) → `DocumentItemDto.colisage`
   (commands, `Option<Vec<ColisageShareDto>>`, `None` = pre-BRK-109 absence,
   same absence-not-zero rule as the credit snapshot) → `src/lib/ipc.ts`
   type. Display integers ride as JSON numbers; money still rides as strings.
4. Fractional lines (12.5 m) yield `[]`; DSN-104/DSN-103 surfaces show the
   base quantity with its unit label instead.

## Consequences

- Reprints stay exact: the breakdown derives from immutable line snapshots
  plus the live ladder (labels may improve; counts cannot drift).
- Zero write-path cost: checkout/conversion/returns untouched; no trigger,
  no backfill, no version gate.
- Cost: one extra units query per document read (same order as lines);
  acceptable for a detail view, never used in list/search paths.
- Printing (`DSN-103/104`) must render shares largest-first and fall back to
   the base quantity on `[]`/`null` — this ADR is the citation for that rule.
