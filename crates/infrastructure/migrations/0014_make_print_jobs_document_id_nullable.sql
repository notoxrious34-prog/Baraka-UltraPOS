-- Migration 0014: Make print_jobs.document_id nullable for non-sale print jobs (debt receipts, Z-reports, test prints)

CREATE TABLE print_jobs_dg_tmp (
    id TEXT PRIMARY KEY,
    document_id TEXT REFERENCES documents(id),
    status TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    is_duplicate INTEGER NOT NULL DEFAULT 0,
    payload_json TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    job_type TEXT,
    payload TEXT,
    last_error TEXT,
    claimed_at TEXT
) STRICT;

INSERT INTO print_jobs_dg_tmp (
    id, document_id, status, attempt_count, is_duplicate,
    payload_json, created_at, updated_at,
    job_type, payload, last_error, claimed_at
)
SELECT
    id, document_id, status, attempt_count, is_duplicate,
    payload_json, created_at, updated_at,
    job_type, payload, last_error, claimed_at
FROM print_jobs;

DROP TABLE print_jobs;

ALTER TABLE print_jobs_dg_tmp RENAME TO print_jobs;

CREATE INDEX IF NOT EXISTS idx_print_jobs_document ON print_jobs(document_id);
