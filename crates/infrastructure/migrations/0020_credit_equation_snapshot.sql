-- Migration 0020: Wholesale credit-equation snapshot (TASK-BRK-108, ADR-110).
--
-- WHY: a finalized credit document must be its own printed witness. The
-- ledger remains the single truth (append-only, cached-balance invariant
-- untouched); these three columns freeze the balance movement so a reprint
-- years later shows the exact equation the customer signed:
--     Ancien Solde + Montant Document - Versement = Nouveau Solde   (charge)
--     Ancien Solde - Montant Document + Versement = Nouveau Solde   (avoir)
-- `credit_delta_centimes` is the SIGNED balance movement applied by this
-- document (+credit on a credit sale, -credit-refund on an AVOIR), so any
-- surface can reconstruct the equation with the document's own ttc/paid.
--
-- Additive and replay-safe: three NULL-able columns, no backfill, no index.
-- Legacy rows (pre-BRK-108) read back as NULL = "no snapshot recorded", which
-- the presentation layer must render as absent, never as zero. A version
-- rollback simply ignores the columns (all SELECT/INSERT lists elsewhere are
-- explicit column by column).
--
-- Invariants enforced by the writing use cases (inside the same
-- BEGIN IMMEDIATE sale transaction, before COMMIT):
--   solde_after = solde_before + credit_delta            (i128 checked)
--   credit_delta = ttc - versement                       (charge documents)
--   credit_delta = -(ttc - versement)                    (AVOIR documents)

ALTER TABLE documents ADD COLUMN credit_solde_before_centimes INTEGER NULL;

ALTER TABLE documents ADD COLUMN credit_solde_after_centimes INTEGER NULL;

ALTER TABLE documents ADD COLUMN credit_delta_centimes INTEGER NULL;
