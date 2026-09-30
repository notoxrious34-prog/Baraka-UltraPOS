//! SQLite transactional-outbox print-job repository.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    PrintJobRecord, PrintJobRepository, PrintJobStatusRow, PRINT_MAX_ATTEMPTS,
};
use baraka_domain::sales::DocumentId;
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::persistence::transaction::run_transaction;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("print job store failure".to_string())
}

/// Receipt print-job outbox over a shared SQLite connection.
pub struct SqlitePrintJobRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqlitePrintJobRepository {
    /// Wrap the shared connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApplicationError> {
        self.db
            .lock()
            .map_err(|_| ApplicationError::Port("database lock poisoned".to_string()))
    }
}

impl PrintJobRepository for SqlitePrintJobRepository {
    fn enqueue_job(
        &self,
        document_id: DocumentId,
        job_type: &str,
        payload_json: &str,
    ) -> Result<String, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            enqueue_in_tx(tx, &document_id.to_string(), job_type, payload_json, &stamp)
        })
        .map_err(|_| store_failure())
    }

    fn claim_next_pending(&self) -> Result<Option<PrintJobRecord>, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            // Atomic claim: exactly one claimant wins the row.
            let claimed: Option<(String, String, String, i64)> = tx
                .query_row(
                    "SELECT id, document_id, job_type, attempt_count FROM print_jobs
                     WHERE status = 'PENDING' ORDER BY rowid LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .map(Some)
                .or_else(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    other => Err(other),
                })?;
            match claimed {
                None => Ok(None),
                Some((job_id, document_id, job_type, attempt_count)) => {
                    let changed = tx.execute(
                        "UPDATE print_jobs
                         SET status = 'PROCESSING', claimed_at = ?1, updated_at = ?1
                         WHERE id = ?2 AND status = 'PENDING'",
                        rusqlite::params![stamp, job_id],
                    )?;
                    if changed != 1 {
                        return Ok(None);
                    }
                    Ok(Some(PrintJobRecord {
                        job_id,
                        document_id: baraka_domain::sales::DocumentId::from(
                            Uuid::parse_str(&document_id).map_err(|_| {
                                rusqlite::Error::InvalidColumnName("document id".to_string())
                            })?,
                        ),
                        job_type,
                        attempt_count,
                    }))
                }
            }
        })
        .map_err(|_| store_failure())
    }

    fn mark_processing(&self, job_id: &str) -> Result<(), ApplicationError> {
        self.transition(job_id, "PROCESSING")
    }

    fn mark_done(&self, job_id: &str) -> Result<(), ApplicationError> {
        self.transition(job_id, "DONE")
    }

    fn mark_failed(&self, job_id: &str, reason: &str) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            // Terminal transition consumes the in-flight attempt so the stored
            // attempt_count equals the number of physical sends performed
            // (bounded by PRINT_MAX_ATTEMPTS), and tags duplicata history.
            tx.execute(
                "UPDATE print_jobs
                 SET status = 'FAILED', last_error = ?1, updated_at = ?2,
                     attempt_count = attempt_count + 1,
                     is_duplicate = CASE WHEN attempt_count + 1 > 0 THEN 1 ELSE is_duplicate END
                 WHERE id = ?3",
                rusqlite::params![reason, stamp, job_id],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn release_for_retry(&self, job_id: &str, last_error: &str) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            // Bounded retry: consume one attempt, watermark duplicata for
            // attempts > 0, and record the failure reason for observability.
            tx.execute(
                "UPDATE print_jobs
                 SET status = 'PENDING', claimed_at = NULL,
                     attempt_count = attempt_count + 1, is_duplicate = 1,
                     last_error = ?1, updated_at = ?2
                 WHERE id = ?3",
                rusqlite::params![last_error, stamp, job_id],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn release_unclaimed(&self, job_id: &str) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "UPDATE print_jobs
                 SET status = 'PENDING', claimed_at = NULL, updated_at = ?1
                 WHERE id = ?2",
                rusqlite::params![stamp, job_id],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn get_job_status(
        &self,
        document_id: DocumentId,
    ) -> Result<Option<PrintJobStatusRow>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT id, status, attempt_count FROM print_jobs
                 WHERE document_id = ?1 ORDER BY rowid DESC LIMIT 1",
                [document_id.to_string()],
                |row| {
                    Ok(PrintJobStatusRow {
                        job_id: row.get(0)?,
                        status_code: row.get(1)?,
                        attempt_count: row.get(2)?,
                    })
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn enqueue_reprint(&self, document_id: DocumentId) -> Result<String, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            enqueue_in_tx(tx, &document_id.to_string(), "DUPLICATA", "{}", &stamp)
        })
        .map_err(|_| store_failure())
    }

    fn reclaim_stale_processing(&self, timeout_secs: u64) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let cutoff = (Utc::now() - chrono::Duration::seconds(timeout_secs as i64))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let mut statement = guard
            .prepare(
                "UPDATE print_jobs
                 SET status = 'PENDING', claimed_at = NULL, updated_at = ?1
                 WHERE status = 'PROCESSING' AND claimed_at IS NOT NULL AND claimed_at < ?2",
            )
            .map_err(|_| store_failure())?;
        let changed = statement
            .execute(rusqlite::params![
                Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                cutoff
            ])
            .map_err(|_| store_failure())?;
        Ok(changed as u64)
    }

    fn reconcile_stale_pending(&self, stale_age_secs: u64) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let now = Utc::now();
        let cutoff = (now - chrono::Duration::seconds(stale_age_secs as i64))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let stamp = now.to_rfc3339_opts(SecondsFormat::Millis, true);
        let mut statement = guard
            .prepare(
                "UPDATE print_jobs
                 SET status = 'FAILED', last_error = 'ABANDONED_STALE_SESSION', updated_at = ?1
                 WHERE status = 'PENDING'
                   AND (attempt_count >= ?2 OR created_at < ?3)",
            )
            .map_err(|_| store_failure())?;
        let changed = statement
            .execute(rusqlite::params![stamp, PRINT_MAX_ATTEMPTS, cutoff])
            .map_err(|_| store_failure())?;
        Ok(changed as u64)
    }
}

impl SqlitePrintJobRepository {
    /// Shared terminal-state transition helper.
    fn transition(&self, job_id: &str, status: &str) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "UPDATE print_jobs SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![status, stamp, job_id],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }
}

/// Enqueue inside an explicit transaction handle (shared with checkout).
pub fn enqueue_in_tx(
    tx: &rusqlite::Transaction,
    document_id: &str,
    job_type: &str,
    payload_json: &str,
    stamp: &str,
) -> Result<String, crate::persistence::InfrastructureError> {
    let job_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO print_jobs
         (id, document_id, job_type, status, payload, payload_json,
          attempt_count, is_duplicate, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'PENDING', ?4, ?4, 0, 0, ?5, ?5)",
        rusqlite::params![job_id, document_id, job_type, payload_json, stamp],
    )?;
    Ok(job_id)
}
