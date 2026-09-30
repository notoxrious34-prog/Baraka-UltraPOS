-- Baraka UltraPOS migration 0004: sale engine alignment (additive only).
--
-- Reconciliation against 0001 (see completion report):
-- - document_sequences: kept (type, fiscal_year, last_sequence) as-is; the
--   §1 target shape is an alternative naming of identical semantics.
--   Rebuilding would destroy allocated counters. No change.
-- - command_log: kept; created_by added (nullable for old rows).
-- - print_jobs: kept; job_type + payload added (payload_json stays NOT NULL
--   and is written with the same JSON). Status CHECK enforced in code.
-- - documents: kept; notes added (nullable).
-- - document_items, payments, customer_ledger, customers: already complete.
-- - Finalized UPDATE triggers exist in 0001; document_items DELETE exists.
--   The one genuinely missing piece is a documents DELETE guard, added here.
-- Never DROP TABLE or rebuild: existing data is preserved.

ALTER TABLE command_log ADD COLUMN created_by TEXT;

ALTER TABLE print_jobs ADD COLUMN job_type TEXT;
ALTER TABLE print_jobs ADD COLUMN payload TEXT;

ALTER TABLE documents ADD COLUMN notes TEXT;

CREATE TRIGGER IF NOT EXISTS prevent_documents_delete_finalized
BEFORE DELETE ON documents
BEGIN
    SELECT CASE
        WHEN OLD.status IN ('COMPLETED', 'FINALIZED', 'POSTED')
        THEN RAISE(ABORT, 'MUTATION_FORBIDDEN: finalized document is immutable')
    END;
END;
