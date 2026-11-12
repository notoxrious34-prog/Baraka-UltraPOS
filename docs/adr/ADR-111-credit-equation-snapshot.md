# ADR-111: Immutable Wholesale Credit-Equation Snapshot on Finalized Documents

Status: ACCEPTED.
Date: 2026-11-12.
Authority: TASK-BRK-108 (Wave 1 of `ROADMAP-v2.0-DUAL-VERTICAL.md`, ADR-110 Pillar 6.3).

## Context

The ledger was already the single truth: `customer_ledger` is append-only,
`customers.current_balance_centimes` is a verified cache, and
`SUM(DEBIT) − SUM(CREDIT) == cached balance` is enforced at backup/restore and
inside the closing path. What no document could do was **testify to its own
balance movement**. Reprinting a June credit ticket in December showed today's
figures or a reconstructed guess — never the equation the customer signed:

```text
Ancien Solde + Montant Document − Versement = Nouveau Solde
```

TASK-AUD-049-R1 proved there was no `Ancien Solde` string anywhere in `crates/`
or `src/`, so the printed credit proof was a real Wave-1 gap, not a nicety.

Two facts discovered while designing the snapshot (both now pinned by tests):

1. `documents.paid_amount_centimes` counts **every** tender, the CREDIT leg
   included. For a 200.00 sale paid 50.00 cash + 150.00 credit, the stored
   paid column is 200.00, not 50.00. The printed `Versement` therefore cannot
   be the paid column; it is the **non-credit portion** `Montant − |delta|`.
2. Finalized documents are immutable by trigger, so the snapshot must be
   written in the **same INSERT** — a post-insert `UPDATE` is impossible by
   design, which conveniently forces atomicity.

## Decision

1. Migration `0020_credit_equation_snapshot.sql` (append-only, strictly
   additive) adds three NULL-able columns to `documents`:
   `credit_solde_before_centimes`, `credit_solde_after_centimes`,
   `credit_delta_centimes`. No backfill and no index; pre-BRK-108 rows read
   back NULL, which surfaces must render as **absent, never zero**.
2. Domain `CreditSnapshot` (`crates/domain/src/sales/credit_equation.rs`) is the
   only arithmetic authority: `freeze(before, delta)` computes
   `after = before + delta` in `i128` with an explicit overflow error;
   `versement_for(montant)` returns `montant − |delta|` (one expression covers
   both directions); `verifies(montant, versement)` proves the printed form —
   `Ancien + Montant − Versement` for a charge, `Ancien − Montant + Versement`
   for an AVOIR (delta sign decides).
3. Writers freeze the snapshot inside the existing atomic transaction, before
   any ledger write, using the pre-write cached balance:
   - checkout (`sqlite_checkout_repository.rs`, step 2b) — credit sales;
   - BL→FACTURE closing (`sqlite_conversion_repository.rs`) — credit settlement;
   - returns (`sqlite_return_repository.rs`) — AVOIR, `delta = −credit refund`.
   Cash-only documents write NULL (no movement, no lie).
4. Read path exposes the triple through `Document` → `document_details` DTO as
   canonical strings (`None` stays `null`), so DSN-103 can print from the frozen
   snapshot without recomputing a centime.

## Consequences

- A reprint is a witness, not a recomputation: later payments cannot rewrite
  history, and `0019` template pinning already guarantees the geometry.
- Rollback-safe: every writer lists columns explicitly, so a binary without
  BRK-108 ignores the columns (pinned by the migration test's legacy-INSERT
  case).
- Cost: four write sites must stay in sync; the domain type is the single
  arithmetic place, and `sale_preview_commit_parity` still owns total parity.
- Printing (`DSN-103`) must use `versement_for`, never the paid column — this
  ADR is the authoritative citation for that rule.
