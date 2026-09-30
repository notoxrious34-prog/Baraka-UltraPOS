//! Resilient transaction boundary (`BEGIN IMMEDIATE` + bounded retry).
//!
//! Retries happen ONLY for transient `SQLITE_BUSY` / `SQLITE_LOCKED`
//! states. Constraint violations, foreign-key failures, and every other
//! business error roll back immediately and are never retried.

use std::thread;
use std::time::Duration;

use rusqlite::{Connection, Error as RusqliteError, ErrorCode, Transaction, TransactionBehavior};

use super::InfrastructureError;

/// Maximum transaction attempts (1 initial + 2 retries).
pub const MAX_ATTEMPTS: u32 = 3;

/// Progressive backoff between attempts (milliseconds).
pub const BACKOFF_MILLIS: [u64; 3] = [10, 25, 50];

/// True only for transient busy/locked SQLite states.
fn is_transient(error: &InfrastructureError) -> bool {
    matches!(
        error,
        InfrastructureError::Sqlite(RusqliteError::SqliteFailure(inner, _))
            if inner.code == ErrorCode::DatabaseBusy
                || inner.code == ErrorCode::DatabaseLocked
    )
}

/// Execute `f` inside a `BEGIN IMMEDIATE` transaction.
///
/// Commits on success. On error the transaction is rolled back (via drop)
/// and the error is returned; transient busy/locked errors are retried with
/// exponential backoff up to [`MAX_ATTEMPTS`] total attempts.
pub fn run_transaction<T, F>(conn: &mut Connection, f: F) -> Result<T, InfrastructureError>
where
    F: Fn(&Transaction) -> Result<T, InfrastructureError>,
{
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let tx = match conn.transaction_with_behavior(TransactionBehavior::Immediate) {
            Ok(tx) => tx,
            Err(begin_error) => {
                let wrapped = InfrastructureError::from(begin_error);
                if is_transient(&wrapped) && attempt < MAX_ATTEMPTS {
                    backoff(attempt);
                    continue;
                }
                if is_transient(&wrapped) {
                    return Err(InfrastructureError::BusyTimeout(attempt));
                }
                return Err(wrapped);
            }
        };
        match f(&tx) {
            Ok(value) => match tx.commit() {
                Ok(()) => return Ok(value),
                Err(commit_error) => {
                    let wrapped = InfrastructureError::from(commit_error);
                    if is_transient(&wrapped) && attempt < MAX_ATTEMPTS {
                        backoff(attempt);
                        continue;
                    }
                    if is_transient(&wrapped) {
                        return Err(InfrastructureError::BusyTimeout(attempt));
                    }
                    return Err(wrapped);
                }
            },
            Err(business_error) => {
                drop(tx); // Rolls back: never retry business errors.
                if is_transient(&business_error) && attempt < MAX_ATTEMPTS {
                    backoff(attempt);
                    continue;
                }
                if is_transient(&business_error) {
                    return Err(InfrastructureError::BusyTimeout(attempt));
                }
                return Err(business_error);
            }
        }
    }
}

fn backoff(failed_attempt: u32) {
    let index = (failed_attempt.saturating_sub(1) as usize).min(BACKOFF_MILLIS.len() - 1);
    thread::sleep(Duration::from_millis(BACKOFF_MILLIS[index]));
}
