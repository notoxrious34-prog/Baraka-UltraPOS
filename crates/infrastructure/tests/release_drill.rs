//! v1.0.0 data-preservation drill verification (upgrade-preservation leg).
//!
//! `#[ignore]` drill test, parameterized by environment variable:
//!
//! ```sh
//! BARAKA_DRILL_DB="D:/path/to/copy/baraka.db" \
//!   CARGO_BUILD_JOBS=2 cargo test -p baraka-infrastructure \
//!   --test release_drill -- --ignored --nocapture
//! ```
//!
//! The test opens the copied database READ-ONLY (no migration runs, no
//! write of any kind) and asserts the preservation contract:
//!   1. `schema_migrations` holds exactly versions 1..=14 (migration
//!      version still 14 — no migration was applied by the release binary).
//!   2. Every recorded checksum matches the checksum recomputed over the
//!      embedded migration SQL (no tamper, no drift).
//!   3. `PRAGMA foreign_key_check` is clean.
//!   4. Core tables (`users`, `products`) are readable.
//!   5. If `BARAKA_DRILL_SKU` is set, that product row is present.
//!
//! Schema-version tracking note: this codebase tracks schema state in the
//! `schema_migrations` table, NOT in `PRAGMA user_version` (no
//! `user_version` reference exists under `crates/`). The drill contract
//! below follows the actual mechanism.

use rusqlite::{Connection, OpenFlags};

use baraka_infrastructure::persistence::migration::{checksum_of, MIGRATIONS};

const EXPECTED_MIGRATION_VERSION: i64 = 14;

fn drill_db_path() -> std::path::PathBuf {
    let raw = std::env::var("BARAKA_DRILL_DB")
        .expect("BARAKA_DRILL_DB must point at the copied baraka.db");
    let path = std::path::PathBuf::from(raw);
    assert!(path.is_file(), "drill copy missing: {}", path.display());
    path
}

/// Upgrade-preservation contract over a read-only copy of the app database.
#[ignore]
#[test]
fn drill_upgrade_preservation_contract() {
    let path = drill_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open drill copy read-only");

    // (1) Exactly versions 1..=14 — migration version still 14.
    let (count, max_version): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("schema_migrations readable");
    assert_eq!(count, EXPECTED_MIGRATION_VERSION, "migration row count");
    assert_eq!(
        max_version, EXPECTED_MIGRATION_VERSION,
        "migration max version"
    );
    println!("DRILL schema_migrations count={count} max={max_version}");

    // (2) Stored checksums match the embedded migration SQL byte-for-byte.
    let mut checked = 0;
    for migration in MIGRATIONS {
        let stored: String = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| panic!("migration v{} missing", migration.version));
        let recomputed = checksum_of(migration.sql);
        assert_eq!(stored, recomputed, "checksum v{}", migration.version);
        checked += 1;
    }
    println!("DRILL checksums verified rows={checked}");

    // (3) No dangling references.
    let violations: Vec<String> = conn
        .prepare("PRAGMA foreign_key_check")
        .expect("fk pragma")
        .query_map([], |row| {
            Ok(format!(
                "table={} rowid={} ref={} fkid={}",
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .expect("fk rows")
        .collect::<Result<_, _>>()
        .expect("fk collect");
    assert!(violations.is_empty(), "fk violations: {violations:?}");
    println!("DRILL foreign_key_check clean");

    // (4) Core tables readable (bootstrap + catalog evidence).
    let users: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .expect("users readable");
    let products: i64 = conn
        .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
        .expect("products readable");
    println!("DRILL users={users} products={products}");

    // (5) Optional known-row pin.
    if let Ok(sku) = std::env::var("BARAKA_DRILL_SKU") {
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products WHERE sku = ?1",
                [&sku],
                |row| row.get(0),
            )
            .expect("sku lookup");
        assert_eq!(found, 1, "seeded sku {sku} present exactly once");
        println!("DRILL sku={sku} present");
    }

    println!("DRILL upgrade-preservation contract HELD");
}
