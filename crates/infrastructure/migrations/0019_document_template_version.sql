-- Migration 0019: Document print-template version (DOC-04)
--
-- Additive and replay-safe: one NOT NULL column with a v1 default, so
-- every pre-DOC-04 document reads back as template v1 (the geometry it
-- was printed with). A reprint pins the stored version instead of the
-- current renderer — old documents keep old bytes forever.
-- A version rollback simply ignores the new column (all SELECT and INSERT
-- lists elsewhere are explicit). No index needed (lookup key stays
-- document_id).

ALTER TABLE documents ADD COLUMN template_version INTEGER NOT NULL DEFAULT 1;
