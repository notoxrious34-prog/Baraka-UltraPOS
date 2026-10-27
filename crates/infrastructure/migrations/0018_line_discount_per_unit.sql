-- Migration 0018: Per-unit line discount storage (TASK-BRK-096)
--
-- Additive and replay-safe: one NULL-able column, no backfill. Legacy rows
-- read back as NULL (contract: legacy display keeps the transcribed lump
-- discount; legacy returns keep the lump pro-rating). New checkout, draft,
-- conversion-copy, and return rows store the exact per-unit value (no
-- division anywhere in the new math).
-- A version rollback simply ignores the new column (all SELECT and INSERT
-- lists elsewhere are explicit). No index needed (lookup key stays
-- document_id).

ALTER TABLE document_items ADD COLUMN discount_per_unit_centimes INTEGER NULL;
