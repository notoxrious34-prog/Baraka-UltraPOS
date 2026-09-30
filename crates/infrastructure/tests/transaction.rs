//! Transaction boundary tests: commit, rollback, retry discipline.

mod common;

use std::cell::Cell;

use baraka_infrastructure::persistence::transaction::run_transaction;
use baraka_infrastructure::InfrastructureError;
use rusqlite::{Error as RusqliteError, ErrorCode};

/// Fabricate a transient SQLITE_BUSY failure without real lock contention.
fn busy_error() -> InfrastructureError {
    InfrastructureError::from(RusqliteError::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseBusy,
            extended_code: 5,
        },
        Some("simulated SQLITE_BUSY".to_string()),
    ))
}

#[test]
fn commits_on_success() {
    let mut db = common::migrated_file_db();
    run_transaction(&mut db.conn, |tx| {
        tx.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('shop', 'Baraka', '2026-01-01T00:00:00Z')",
            [],
        )?;
        Ok(())
    })
    .expect("commit");
    let value: String = db
        .conn
        .query_row("SELECT value FROM settings WHERE key = 'shop'", [], |row| {
            row.get(0)
        })
        .expect("row persisted");
    assert_eq!(value, "Baraka");
}

#[test]
fn business_error_rolls_back_without_retry() {
    let mut db = common::migrated_file_db();
    let baseline: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
        .expect("baseline");
    let attempts = Cell::new(0u32);
    let result: Result<(), InfrastructureError> = run_transaction(&mut db.conn, |tx| {
        attempts.set(attempts.get() + 1);
        tx.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('shop', 'Baraka', '2026-01-01T00:00:00Z')",
            [],
        )?;
        Err(InfrastructureError::Config("boom".to_string()))
    });
    assert!(matches!(result, Err(InfrastructureError::Config(_))));
    assert_eq!(attempts.get(), 1, "business errors must never retry");
    let rows: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
        .expect("count");
    assert_eq!(rows, baseline, "partial work must roll back");
}

#[test]
fn constraint_violation_is_never_retried() {
    let mut db = common::migrated_file_db();
    db.conn
        .execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('shop', 'Baraka', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed");
    let attempts = Cell::new(0u32);
    let result: Result<(), InfrastructureError> = run_transaction(&mut db.conn, |tx| {
        attempts.set(attempts.get() + 1);
        tx.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('shop', 'Dup', '2026-01-01T00:00:00Z')",
            [],
        )?;
        Ok(())
    });
    assert!(
        matches!(result, Err(InfrastructureError::Sqlite(_))),
        "expected constraint error, got: {result:?}"
    );
    assert_eq!(attempts.get(), 1, "constraint errors must never retry");
}

#[test]
fn transient_busy_retries_then_succeeds() {
    let mut db = common::migrated_file_db();
    let attempts = Cell::new(0u32);
    run_transaction(&mut db.conn, |tx| {
        attempts.set(attempts.get() + 1);
        if attempts.get() < 3 {
            return Err(busy_error());
        }
        tx.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('shop', 'Baraka', '2026-01-01T00:00:00Z')",
            [],
        )?;
        Ok(())
    })
    .expect("eventual success");
    assert_eq!(attempts.get(), 3);
}

#[test]
fn transient_busy_exhausted_reports_timeout() {
    let mut db = common::migrated_file_db();
    let attempts = Cell::new(0u32);
    let result: Result<(), InfrastructureError> = run_transaction(&mut db.conn, |_| {
        attempts.set(attempts.get() + 1);
        Err(busy_error())
    });
    assert!(
        matches!(result, Err(InfrastructureError::BusyTimeout(3))),
        "expected BusyTimeout(3), got: {result:?}"
    );
    assert_eq!(attempts.get(), 3);
}
