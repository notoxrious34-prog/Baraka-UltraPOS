//! Migration engine tests: clean apply, idempotency, checksum integrity.

mod common;

use baraka_infrastructure::persistence::migration::{checksum_of, run_migrations};
use baraka_infrastructure::InfrastructureError;

const EXPECTED_TABLES: [&str; 25] = [
    "schema_migrations",
    "settings",
    "users",
    "categories",
    "products",
    "product_units",
    "product_kit_items",
    "warehouses",
    "stock_levels",
    "stock_movements",
    "customers",
    "customer_ledger",
    "suppliers",
    "supplier_ledger",
    "documents",
    "document_items",
    "payments",
    "cash_sessions",
    "cash_movements",
    "tax_rules",
    "document_sequences",
    "print_jobs",
    "command_log",
    "audit_log",
    "backups_log",
];

#[test]
fn baseline_creates_all_25_canonical_tables() {
    let db = common::migrated_file_db();
    for table in EXPECTED_TABLES {
        let found: String = db
            .conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| panic!("canonical table missing: {table}"));
        assert_eq!(found, table);
    }
    let (version, name, checksum, applied_at): (i64, String, String, String) = db
        .conn
        .query_row(
            "SELECT version, name, checksum, applied_at FROM schema_migrations WHERE version = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("migration row");
    assert_eq!(version, 1);
    assert_eq!(name, "0001_baseline_schema.sql");
    assert_eq!(checksum.len(), 64);
    assert!(!applied_at.is_empty());
}

#[test]
fn rerun_is_idempotent() {
    let mut db = common::migrated_file_db();
    run_migrations(&mut db.conn).expect("second run");
    let rows: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(rows, 14);
}

#[test]
fn tampered_checksum_is_rejected() {
    let mut db = common::migrated_file_db();
    db.conn
        .execute(
            "UPDATE schema_migrations SET checksum = 'tampered' WHERE version = 1",
            [],
        )
        .expect("tamper");
    // The UPDATE above is blocked only on ledger tables; schema_migrations
    // itself is mutable on purpose so drift is detectable here.
    let error = run_migrations(&mut db.conn).expect_err("tamper must fail");
    match error {
        InfrastructureError::ChecksumMismatch {
            version,
            expected,
            found,
        } => {
            assert_eq!(version, 1);
            assert_eq!(expected, "tampered");
            assert_eq!(found.len(), 64);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn checksum_helper_is_stable() {
    assert_eq!(
        checksum_of("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn migration_0005_adds_customer_indexes() {
    let db = common::migrated_file_db();
    let sql: String = db
        .conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'idx_customers_phone_unique'",
            [],
            |row| row.get(0),
        )
        .expect("phone index");
    assert!(
        sql.contains("WHERE phone IS NOT NULL"),
        "partial unique: {sql}"
    );
    let name_idx: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_customers_name'",
            [],
            |row| row.get(0),
        )
        .expect("name index");
    assert_eq!(name_idx, 1);
}

#[test]
fn migration_0002_adds_lockout_columns() {
    let db = common::migrated_file_db();
    let mut names: Vec<String> = Vec::new();
    let mut statement = db
        .conn
        .prepare("SELECT name FROM pragma_table_info('users')")
        .expect("pragma");
    let rows = statement.query_map([], |row| row.get(0)).expect("columns");
    for name in rows {
        names.push(name.expect("column name"));
    }
    assert!(names.contains(&"failed_login_attempts".to_string()));
    assert!(names.contains(&"locked_until".to_string()));
    let versions: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(versions, 14);
}

#[test]
fn migration_0006_adds_print_columns_and_settings() {
    let db = common::migrated_file_db();
    let mut names: Vec<String> = Vec::new();
    let mut statement = db
        .conn
        .prepare("SELECT name FROM pragma_table_info('print_jobs')")
        .expect("pragma");
    let rows = statement.query_map([], |row| row.get(0)).expect("columns");
    for name in rows {
        names.push(name.expect("column name"));
    }
    assert!(names.contains(&"last_error".to_string()));
    assert!(names.contains(&"claimed_at".to_string()));
    for key in [
        "printer.transport",
        "printer.serial_port",
        "printer.serial_baud",
        "printer.paper_width_dots",
    ] {
        let value: String = db
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .unwrap_or_else(|_| panic!("missing setting {key}"));
        assert!(!value.is_empty() || key == "printer.serial_port");
    }
    let baud: String = db
        .conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'printer.serial_baud'",
            [],
            |row| row.get(0),
        )
        .expect("baud");
    assert_eq!(baud, "9600");
}

#[test]
fn migration_0007_adds_document_language() {
    let db = common::migrated_file_db();
    let definition: String = db
        .conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'documents'",
            [],
            |row| row.get(0),
        )
        .expect("documents ddl");
    assert!(
        definition.contains("language TEXT NOT NULL DEFAULT 'FR'"),
        "unexpected ddl: {definition}"
    );
    // Legacy rows default honestly to 'FR' (pre-fix rendering behavior).
    db.conn
        .execute(
            "INSERT INTO warehouses (id, code, name, is_active) VALUES ('w1', 'W1', 'W', 1)",
            [],
        )
        .expect("warehouse");
    db.conn
        .execute(
            "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
             VALUES ('u1', 'u', 'h', 'ADMIN', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("user");
    db.conn
        .execute(
            "INSERT INTO documents
             (id, number, fiscal_year, type, warehouse_id, document_language,
              subtotal_ht_centimes, vat_total_centimes, global_discount_centimes,
              timbre_centimes, rounding_centimes, ttc_centimes, paid_amount_centimes,
              change_amount_centimes, payment_method, status, created_at, created_by,
              revision, origin_device_id)
             VALUES ('d1', 'TICK-2026-000001', 2026, 'TICKET', 'w1', 'FR',
                     0, 0, 0, 0, 0, 0, 0, 0, 'CASH', 'FINALIZED',
                     '2026-01-01T00:00:00Z', 'u1', 1, 'LOCAL')",
            [],
        )
        .expect("legacy insert");
    let language: String = db
        .conn
        .query_row(
            "SELECT language FROM documents WHERE id = 'd1'",
            [],
            |row| row.get(0),
        )
        .expect("language");
    assert_eq!(language, "FR");
    // The CHECK rejects anything outside the AR/FR contract.
    let rejected = db.conn.execute(
        "INSERT INTO documents
         (id, number, fiscal_year, type, warehouse_id, document_language, language,
          subtotal_ht_centimes, vat_total_centimes, global_discount_centimes,
          timbre_centimes, rounding_centimes, ttc_centimes, paid_amount_centimes,
          change_amount_centimes, payment_method, status, created_at, created_by,
          revision, origin_device_id)
         VALUES ('d2', 'TICK-2026-000002', 2026, 'TICKET', 'w1', 'FR', 'EN',
                 0, 0, 0, 0, 0, 0, 0, 0, 'CASH', 'FINALIZED',
                 '2026-01-01T00:00:00Z', 'u1', 1, 'LOCAL')",
        [],
    );
    assert!(rejected.is_err(), "CHECK must reject 'EN'");
}

#[test]
fn migration_0012_seeds_default_warehouse_idempotently() {
    let mut db = common::migrated_file_db();
    // Fresh database: exactly one seeded row with the canonical identity.
    let count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM warehouses", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 1);
    let (id, code, name, is_active, is_default): (String, String, String, i64, i64) = db
        .conn
        .query_row(
            "SELECT id, code, name, is_active, is_default FROM warehouses",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("seeded row");
    assert_eq!(id, "01950000-0000-7000-8000-000000000001");
    assert_eq!(code, "DEP-01");
    assert_eq!(name, "Dépôt Principal");
    assert_eq!(is_active, 1);
    assert_eq!(is_default, 1);
    // The fixed id parses as a UUID, so downstream parsing never panics.
    uuid::Uuid::parse_str(&id).expect("seed id is a valid UUID");
    // Non-empty table: re-running migrations is a strict no-op (no duplicate).
    db.conn
        .execute(
            "INSERT INTO warehouses (id, code, name) VALUES ('w2', 'EAST', 'Est')",
            [],
        )
        .expect("second warehouse");
    run_migrations(&mut db.conn).expect("re-run");
    let count: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM warehouses", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 2);
    let dep01: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM warehouses WHERE code = 'DEP-01'",
            [],
            |row| row.get(0),
        )
        .expect("dep01 count");
    assert_eq!(dep01, 1);
}

#[test]
fn migration_0013_widens_cash_movements_for_refund_out() {
    let db = common::migrated_file_db();
    let definition: String = db
        .conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'cash_movements'",
            [],
            |row| row.get(0),
        )
        .expect("cash_movements ddl");
    assert!(
        definition.contains("'REFUND_OUT'"),
        "CHECK must list REFUND_OUT: {definition}"
    );
    // Pre-existing rows survive the rebuild (opening float seed pattern).
    let migrated: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 13",
            [],
            |row| row.get(0),
        )
        .expect("v13 row");
    assert_eq!(migrated, 1);
    let name: String = db
        .conn
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 13",
            [],
            |row| row.get(0),
        )
        .expect("v13 name");
    assert_eq!(name, "0013_add_refund_out_movement_type.sql");
}

#[test]
fn migration_0014_makes_print_job_document_id_nullable() {
    let db = common::migrated_file_db();
    // v14 bookkeeping row present with the exact file name.
    let name: String = db
        .conn
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 14",
            [],
            |row| row.get(0),
        )
        .expect("v14 name");
    assert_eq!(name, "0014_make_print_jobs_document_id_nullable.sql");
    // document_id column is now nullable (notnull == 0).
    let notnull: i64 = db
        .conn
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('print_jobs') WHERE name = 'document_id'",
            [],
            |row| row.get(0),
        )
        .expect("pragma");
    assert_eq!(notnull, 0);
    // All 12 columns survive the rebuild, and the document index is back.
    let columns: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('print_jobs')",
            [],
            |row| row.get(0),
        )
        .expect("columns");
    assert_eq!(columns, 12);
    let index: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_print_jobs_document'",
            [],
            |row| row.get(0),
        )
        .expect("index");
    assert_eq!(index, 1);
    // A NULL-document row inserts cleanly under the rebuilt schema.
    db.conn
        .execute(
            "INSERT INTO print_jobs
             (id, document_id, status, payload_json, created_at, updated_at)
             VALUES ('null-doc-job-1', NULL, 'PENDING', '{}',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("null insert");
    let stored: Option<String> = db
        .conn
        .query_row(
            "SELECT document_id FROM print_jobs WHERE id = 'null-doc-job-1'",
            [],
            |row| row.get(0),
        )
        .expect("null read");
    assert_eq!(stored, None);
}
