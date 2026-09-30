//! Connection factory tests: mandatory PRAGMAs and FK enforcement.

mod common;

use baraka_infrastructure::persistence::connection::{open_connection, open_in_memory};

#[test]
fn file_connection_enforces_mandatory_pragmas() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pragma.db");
    let conn = open_connection(&path).expect("open");

    let journal: String = conn
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .expect("journal_mode");
    assert_eq!(journal.to_lowercase(), "wal");

    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous;", [], |row| row.get(0))
        .expect("synchronous");
    assert_eq!(synchronous, 2, "synchronous must be FULL (2)");

    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
        .expect("foreign_keys");
    assert_eq!(foreign_keys, 1);

    let busy_timeout: i64 = conn
        .query_row("PRAGMA busy_timeout;", [], |row| row.get(0))
        .expect("busy_timeout");
    assert_eq!(busy_timeout, 5000);
}

#[test]
fn foreign_keys_are_really_enforced() {
    let db = common::migrated_file_db();
    db.conn
        .execute(
            "INSERT INTO users (id, username, password_hash, role, created_at, updated_at)
             VALUES ('u1', 'caissier1', 'hash', 'CASHIER', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed user");
    // Unknown customer must violate the foreign key (would silently
    // succeed if `PRAGMA foreign_keys = ON` were missing).
    let error = db
        .conn
        .execute(
            "INSERT INTO customer_ledger
             (id, sequence_no, customer_id, entry_type, amount_centimes,
              running_balance_centimes, created_by, created_at)
             VALUES ('cl1', 1, 'ghost', 'DEBT', 1000, 1000, 'u1', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect_err("ghost customer must fail");
    assert!(
        format!("{error:?}").contains("FOREIGN KEY"),
        "unexpected error: {error:?}"
    );
}

#[test]
fn in_memory_opens_for_pure_schema_tests() {
    let conn = open_in_memory().expect("open in-memory");
    conn.execute_batch("CREATE TABLE probe (id TEXT PRIMARY KEY) STRICT;")
        .expect("ddl on memory db");
}
