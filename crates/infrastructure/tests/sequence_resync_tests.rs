//! Post-restore sequence auto-heal unit tests (owner decision: BRK-124 automatic).
//!
//! `resync_sequences_from_documents` raises `document_sequences` counters
//! to the maximum stored document number per `(type, fiscal_year)` —
//! monotonic only, drafts and legacy numbers skipped, restore never fails.

use baraka_infrastructure::adapters::sqlite_document_sequence_repository::resync_sequences_from_documents;

fn setup() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().expect("memory");
    conn.execute_batch(
        "CREATE TABLE documents (
            type TEXT NOT NULL,
            fiscal_year INTEGER NOT NULL,
            number TEXT NOT NULL
        );
        CREATE TABLE document_sequences (
            type TEXT NOT NULL,
            fiscal_year INTEGER NOT NULL,
            last_sequence INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(type, fiscal_year)
        );",
    )
    .expect("tables");
    conn
}

fn seed_doc(conn: &rusqlite::Connection, doc_type: &str, year: i32, number: &str) {
    conn.execute(
        "INSERT INTO documents (type, fiscal_year, number) VALUES (?1, ?2, ?3)",
        rusqlite::params![doc_type, year, number],
    )
    .expect("seed doc");
}

fn seed_seq(conn: &rusqlite::Connection, doc_type: &str, year: i32, last: i64) {
    conn.execute(
        "INSERT INTO document_sequences (type, fiscal_year, last_sequence) VALUES (?1, ?2, ?3)",
        rusqlite::params![doc_type, year, last],
    )
    .expect("seed seq");
}

fn last_sequence(conn: &rusqlite::Connection, doc_type: &str, year: i32) -> Option<i64> {
    conn.query_row(
        "SELECT last_sequence FROM document_sequences WHERE type = ?1 AND fiscal_year = ?2",
        rusqlite::params![doc_type, year],
        |row| row.get(0),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
    .expect("read seq")
}

fn resync(conn: &mut rusqlite::Connection) {
    let tx = conn.transaction().expect("tx");
    resync_sequences_from_documents(&tx).expect("resync");
    tx.commit().expect("commit");
}

#[test]
fn resync_raises_lagging_counter_to_max_stored_number() {
    let mut conn = setup();
    seed_doc(&conn, "TICKET", 2024, "TICK-2024-000007");
    seed_seq(&conn, "TICKET", 2024, 3);
    resync(&mut conn);
    assert_eq!(last_sequence(&conn, "TICKET", 2024), Some(7));
}

#[test]
fn resync_never_lowers_ahead_counter() {
    let mut conn = setup();
    seed_doc(&conn, "TICKET", 2026, "TICK-2026-000002");
    seed_seq(&conn, "TICKET", 2026, 9);
    resync(&mut conn);
    assert_eq!(last_sequence(&conn, "TICKET", 2026), Some(9));
}

#[test]
fn resync_creates_missing_counter_row() {
    let mut conn = setup();
    seed_doc(&conn, "BL", 2026, "BL-2026-000004");
    resync(&mut conn);
    assert_eq!(last_sequence(&conn, "BL", 2026), Some(4));
}

#[test]
fn resync_skips_drafts_and_legacy_numbers() {
    let mut conn = setup();
    seed_doc(&conn, "DRAFT", 2026, "DRAFT-AB12CD34");
    seed_doc(&conn, "TICKET", 2026, "old-format-no-seq");
    seed_doc(&conn, "TICKET", 2026, "TICK-2026-000000");
    resync(&mut conn);
    assert_eq!(last_sequence(&conn, "DRAFT", 2026), None);
    assert_eq!(last_sequence(&conn, "TICKET", 2026), None);
}
