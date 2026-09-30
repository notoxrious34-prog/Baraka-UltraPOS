//! Shared helpers for infrastructure integration tests (file-backed DBs).

use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use rusqlite::Connection;

/// A migrated temporary file database; the directory guard must stay alive.
pub struct TestDb {
    /// Keeps the temp directory (and the DB file) alive for the test.
    pub _dir: tempfile::TempDir,
    /// Open, migrated connection under test.
    pub conn: Connection,
}

/// Create a fresh migrated file-backed database.
pub fn migrated_file_db() -> TestDb {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("test.db");
    let mut conn = open_connection(&path).expect("open connection");
    run_migrations(&mut conn).expect("run migrations");
    TestDb { _dir: dir, conn }
}
