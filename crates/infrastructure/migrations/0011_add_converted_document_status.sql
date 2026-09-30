-- Baraka UltraPOS migration 0011: DEVIS-to-BL conversion support (TASK-BRK-014A).
--
-- Reconciliation against the task's table-recreation assumption (verified
-- against 0001_baseline_schema.sql before writing):
-- - documents.status is plain `TEXT NOT NULL` with NO CHECK constraint, and
--   documents.source_document_id already exists as
--   `TEXT REFERENCES documents(id)`. There is therefore nothing to widen:
--   the 'CONVERTED' status value and source linkage are accepted by the
--   current schema unchanged. A create-copy-drop-rename rebuild is NOT
--   performed: it would churn every row, index, trigger, and foreign key
--   for zero semantic benefit (same conclusion as migration 0009's
--   investigation points 1 and 2, which likewise skipped rebuilds).
-- - The one genuinely missing piece is an index on
--   documents(source_document_id): conversion idempotency checks, source
--   fetches, and double-conversion guards all filter on this column, and
--   no index covers it today. Added below, non-destructively.
-- - One REAL conflict was discovered during implementation (not assumed by
--   the task): trigger `prevent_documents_update_finalized` (0001) aborts
--   EVERY update of a FINALIZED row, which would forbid the specified
--   Finalized -> Converted seal itself. The trigger is therefore recreated
--   here with one surgical exception: exactly the transition
--   OLD.status = 'FINALIZED' AND NEW.status = 'CONVERTED' is permitted.
--   Every other mutation of a sealed row stays forbidden with the identical
--   abort message. Dropping/recreating a trigger touches no data.
-- Never DROP TABLE or rebuild: existing data is preserved byte-for-byte.

CREATE INDEX IF NOT EXISTS idx_documents_source_document
    ON documents(source_document_id) WHERE source_document_id IS NOT NULL;

DROP TRIGGER IF EXISTS prevent_documents_update_finalized;
CREATE TRIGGER prevent_documents_update_finalized
BEFORE UPDATE ON documents
BEGIN
    SELECT CASE
        WHEN OLD.status IN ('COMPLETED', 'FINALIZED', 'POSTED')
         AND NOT (OLD.status = 'FINALIZED' AND NEW.status = 'CONVERTED')
        THEN RAISE(ABORT, 'MUTATION_FORBIDDEN: finalized document is immutable')
    END;
END;
