//! Newer-database startup guard (TASK-BRK-104, ADR-106).
//!
//! A database last written by a NEWER binary (downgrade attempt) must be
//! REFUSED at startup with an explicit error — never opened silently.
//! `run_migrations` itself stays tolerant (pinned by
//! `downgrade_guard_probe_future_version_is_deterministic`); the guard is
//! a separate pure probe the startup path calls after migrations succeed.

mod common;

use baraka_infrastructure::persistence::migration::{
    probe_newer_database, run_migrations, supported_schema_version, MIGRATIONS,
};
use baraka_infrastructure::persistence::InfrastructureError;

#[test]
fn supported_version_matches_embedded_migrations() {
    assert_eq!(
        supported_schema_version(),
        MIGRATIONS.len() as i64,
        "guard horizon must track the embedded list exactly"
    );
    assert!(
        supported_schema_version() >= 18,
        "baseline covers migrations 0001..=0018"
    );
}

#[test]
fn fresh_migrated_db_passes_probe() {
    let db = common::migrated_file_db();
    probe_newer_database(&db.conn).expect("current-version db must pass");
}

#[test]
fn future_version_row_is_refused_with_exact_versions() {
    let mut db = common::migrated_file_db();
    // Simulate a database last written by a newer binary.
    db.conn
        .execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (9999, '9999_future_schema.sql', 'probe_hash', '2026-11-05T00:00:00Z')",
            [],
        )
        .expect("inject future migration row");

    // Migrations stay tolerant (pinned behavior) but the probe refuses.
    run_migrations(&mut db.conn).expect("runner stays tolerant");
    let error = probe_newer_database(&db.conn).expect_err("future version must be refused");
    match error {
        InfrastructureError::DatabaseNewerThanBinary { found, supported } => {
            assert_eq!(found, 9999, "probe reports the recorded future version");
            assert_eq!(
                supported,
                MIGRATIONS.len() as i64,
                "probe reports this binary horizon"
            );
        }
        other => panic!("wrong error variant: {other}"),
    }
}
