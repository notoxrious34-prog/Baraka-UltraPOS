-- Migration 0017: Record real backup size + trigger on backups_log (TASK-BRK-094)
--
-- Additive and replay-safe: two NULL-able columns, no backfill. Legacy rows
-- read back as unknown size (UI contract: "—", never 0) and manual trigger.
-- A version rollback simply ignores the new columns (old SELECT lists are
-- explicit). No index needed (lookup key stays backup_id).

ALTER TABLE backups_log ADD COLUMN file_size_bytes INTEGER NULL;

ALTER TABLE backups_log ADD COLUMN trigger TEXT NULL;
