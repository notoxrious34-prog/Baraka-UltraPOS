-- 0026: two-till sync identity (TASK-BRK-124, ADR-123 subset).
--
-- - `documents.origin_number`: the human number the document carried on its
--   ORIGIN till. NULL for locally created documents. Set exactly once at
--   sync import; local finalization paths never write it (they allocate
--   from the local sequence and keep NULL).
-- - Partial unique index: the idempotency key of an import is
--   (origin_device_id, origin_number, type). Local rows (origin_number IS
--   NULL) are excluded, so local numbering is untouched. Re-importing the
--   same package aborts on this index — fail closed, never duplicate.
-- - `sync_outbox`: append-only capture ledger reserved for automatic
--   per-write export cursors (ADR-123 section 3). v1 export derives from
--   committed documents, so the table ships empty and guarded; no
--   write-path hook touches the atomic till paths.
ALTER TABLE documents ADD COLUMN origin_number TEXT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_sync_identity
    ON documents(origin_device_id, origin_number, type)
    WHERE origin_number IS NOT NULL;
CREATE TABLE IF NOT EXISTS sync_outbox (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    at TEXT NOT NULL
) STRICT;
CREATE TRIGGER prevent_sync_outbox_update
BEFORE UPDATE ON sync_outbox
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: sync_outbox is strictly append-only');
END;
CREATE TRIGGER prevent_sync_outbox_delete
BEFORE DELETE ON sync_outbox
BEGIN
    SELECT RAISE(ABORT, 'MUTATION_FORBIDDEN: sync_outbox is strictly append-only');
END;
