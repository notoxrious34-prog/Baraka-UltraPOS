//! Embedded, zero-dependency migration runner.
//!
//! Migration scripts live in `crates/infrastructure/migrations/` and are
//! compiled into the binary via `include_str!`. Applied versions are tracked
//! in `schema_migrations` with a SHA-256 checksum; a checksum mismatch on an
//! already-applied version aborts with an integrity error.

use rusqlite::Connection;

use super::transaction::run_transaction;
use super::InfrastructureError;

/// Baseline migration descriptor.
pub struct EmbeddedMigration {
    /// Monotonic schema version.
    pub version: i64,
    /// File name of the embedded script.
    pub name: &'static str,
    /// Embedded SQL body.
    pub sql: &'static str,
}

/// All migrations known to this binary, in apply order.
pub const MIGRATIONS: &[EmbeddedMigration] = &[
    EmbeddedMigration {
        version: 1,
        name: "0001_baseline_schema.sql",
        sql: include_str!("../../migrations/0001_baseline_schema.sql"),
    },
    EmbeddedMigration {
        version: 2,
        name: "0002_auth_lockout.sql",
        sql: include_str!("../../migrations/0002_auth_lockout.sql"),
    },
    EmbeddedMigration {
        version: 3,
        name: "0003_stock_engine_schema.sql",
        sql: include_str!("../../migrations/0003_stock_engine_schema.sql"),
    },
    EmbeddedMigration {
        version: 4,
        name: "0004_sales_engine_schema.sql",
        sql: include_str!("../../migrations/0004_sales_engine_schema.sql"),
    },
    EmbeddedMigration {
        version: 5,
        name: "0005_customer_module_indexes.sql",
        sql: include_str!("../../migrations/0005_customer_module_indexes.sql"),
    },
    EmbeddedMigration {
        version: 6,
        name: "0006_print_worker_columns.sql",
        sql: include_str!("../../migrations/0006_print_worker_columns.sql"),
    },
    EmbeddedMigration {
        version: 7,
        name: "0007_document_language.sql",
        sql: include_str!("../../migrations/0007_document_language.sql"),
    },
    EmbeddedMigration {
        version: 8,
        name: "0008_cash_session_engine_schema.sql",
        sql: include_str!("../../migrations/0008_cash_session_engine_schema.sql"),
    },
    EmbeddedMigration {
        version: 9,
        name: "0009_supplier_purchase_engine_schema.sql",
        sql: include_str!("../../migrations/0009_supplier_purchase_engine_schema.sql"),
    },
    EmbeddedMigration {
        version: 10,
        name: "0010_reporting_indexes.sql",
        sql: include_str!("../../migrations/0010_reporting_indexes.sql"),
    },
    EmbeddedMigration {
        version: 11,
        name: "0011_add_converted_document_status.sql",
        sql: include_str!("../../migrations/0011_add_converted_document_status.sql"),
    },
    EmbeddedMigration {
        version: 12,
        name: "0012_seed_default_warehouse.sql",
        sql: include_str!("../../migrations/0012_seed_default_warehouse.sql"),
    },
    EmbeddedMigration {
        version: 13,
        name: "0013_add_refund_out_movement_type.sql",
        sql: include_str!("../../migrations/0013_add_refund_out_movement_type.sql"),
    },
    EmbeddedMigration {
        version: 14,
        name: "0014_make_print_jobs_document_id_nullable.sql",
        sql: include_str!("../../migrations/0014_make_print_jobs_document_id_nullable.sql"),
    },
    EmbeddedMigration {
        version: 15,
        name: "0015_pricing_tiers.sql",
        sql: include_str!("../../migrations/0015_pricing_tiers.sql"),
    },
];

/// SHA-256 hex digest of `input` (pure Rust, no extra dependency).
pub fn checksum_of(input: &str) -> String {
    sha256_hex(input.as_bytes())
}

/// Apply every pending embedded migration, in version order.
///
/// Each migration runs inside [`run_transaction`]: the DDL body and the
/// `schema_migrations` bookkeeping row commit atomically.
pub fn run_migrations(conn: &mut Connection) -> Result<(), InfrastructureError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            checksum TEXT NOT NULL,
            applied_at TEXT NOT NULL
        ) STRICT;",
    )?;
    for migration in MIGRATIONS {
        let checksum = checksum_of(migration.sql);
        let recorded: Option<String> = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        match recorded {
            Some(previous) if previous == checksum => continue,
            Some(previous) => {
                return Err(InfrastructureError::ChecksumMismatch {
                    version: migration.version,
                    expected: previous,
                    found: checksum,
                });
            }
            None => {}
        }
        let applied_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(conn, |tx| {
            tx.execute_batch(migration.sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name, checksum, applied_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![migration.version, migration.name, checksum, applied_at],
            )?;
            Ok(())
        })?;
    }
    Ok(())
}

/// SHA-256 core lives in the shared streaming module; this alias keeps
/// migration checksums byte-identical to the audited implementation.
fn sha256_hex(input: &[u8]) -> String {
    super::sha256::sha256_hex(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_reference_vectors() {
        assert_eq!(
            checksum_of(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            checksum_of("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn checksum_is_deterministic_hex() {
        let first = checksum_of("baraka-ultrapos-baseline");
        let second = checksum_of("baraka-ultrapos-baseline");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
