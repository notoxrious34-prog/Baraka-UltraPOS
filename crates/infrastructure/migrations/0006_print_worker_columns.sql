-- Baraka UltraPOS migration 0006: print worker columns + printer settings.
--
-- Reconciliation against 0001/0004 (see completion report):
-- - print_jobs already carries id, document_id, job_type (0004), status,
--   payload, payload_json, attempt_count (0001), is_duplicate,
--   created_at, updated_at. Genuinely missing: last_error, claimed_at.
-- - Append-only triggers cover every entry type identically. No change.
-- - Settings rows seed Serial-only defaults (USB is out of scope).
-- Never DROP TABLE or rebuild: existing data is preserved.

ALTER TABLE print_jobs ADD COLUMN last_error TEXT;
ALTER TABLE print_jobs ADD COLUMN claimed_at TEXT;

INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
    ('printer.transport', 'SERIAL', '2026-01-01T00:00:00Z'),
    ('printer.serial_port', '', '2026-01-01T00:00:00Z'),
    ('printer.serial_baud', '9600', '2026-01-01T00:00:00Z'),
    ('printer.paper_width_dots', '576', '2026-01-01T00:00:00Z');
