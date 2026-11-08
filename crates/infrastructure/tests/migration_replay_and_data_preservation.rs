//! TASK-REL-054 drill: migration replay & data preservation 0001 -> 0019.
//!
//! A merchant upgrading Baraka UltraPOS must never lose a centime, a stock
//! unit, or a ledger entry. This suite proves the zero-data-loss invariant
//! on an isolated scratch database (never the live `%APPDATA%` database):
//!
//! * Stage 1 — simulate a legacy v13 database: apply embedded migrations
//!   0001..=0013 directly, then seed realistic business records (warehouse,
//!   admin user, categories, products, indebted customer, append-only
//!   ledger entry, stock level, one finalized BL with item + payment) and
//!   capture baseline aggregates.
//! * Stage 2 — run the official [`run_migrations`] upgrade path to 0019
//!   (0016 adds the FTS5 `products_fts` index with in-transaction backfill;
//!   0017 adds NULL-able `backups_log.file_size_bytes` + `trigger`;
//!   0018 adds NULL-able `document_items.discount_per_unit_centimes`;
//!   0019 adds NOT NULL `documents.template_version` DEFAULT 1 — every
//!   pre-DOC-04 row reads back as template v1, the geometry it printed).
//! * Stage 3 — assert byte-level parity: all 19 bookkeeping rows present
//!   with valid checksums, debt balance exactly 450,000 centimes, stock
//!   untouched, documents/ledger untouched, 0014 nullability + 0015 tier
//!   columns in their canonical shapes, plus FTS existence, consistency,
//!   legacy-row backfill proof, and 0019 template-version backfill proof.
//! * Stage 4 — downgrade-guard probe: inject a fake future migration row
//!   (version 9999) and prove the runner handles it deterministically
//!   without wiping or corrupting data.
//!
//! Documented deviations from the task text (code is the truth):
//! * Migration 0014 is `0014_make_print_jobs_document_id_nullable.sql`,
//!   not an FTS5 catalog search migration.
//! * Migration 0015 defaults `customers.customer_tier` to lowercase
//!   `'retail'` (domain canonical form, see `CustomerTier::as_str`), not
//!   `'RETAIL'`. Assertions use the canonical lowercase value.
//! * Migration 0016 is the FTS5 catalog search migration: it creates the
//!   `products_fts` table and backfills it. The FTS assertions below are
//!   written against this truth (TASK-BRK-082).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use baraka_infrastructure::adapters::sqlite_catalog_repository::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::{checksum_of, run_migrations, MIGRATIONS};
use rusqlite::Connection;

const STAMP: &str = "2026-03-14T10:00:00Z";
/// Legacy customer debt carried through the upgrade (4,500.00 DZD).
const LEGACY_DEBT_CENTIMES: i64 = 450_000;
/// 100 base units of product 1, fixed-point 10^4 micros per unit.
const LEGACY_STOCK_BASE_MICRO: i64 = 100 * 10_000;

struct ScratchDb {
    _dir: tempfile::TempDir,
    path: PathBuf,
    conn: Connection,
}

fn open_scratch(name: &str) -> ScratchDb {
    let dir = tempfile::tempdir().expect("create scratch tempdir");
    let path = dir.path().join(name);
    let conn = open_connection(&path).expect("open scratch connection");
    ScratchDb {
        _dir: dir,
        path,
        conn,
    }
}

/// Apply migrations 0001..=0013 exactly as the runner would (DDL body +
/// `schema_migrations` bookkeeping row), producing a faithful legacy v13
/// database. Migrations 0014..=0019 are deliberately withheld: they are the
/// upgrade under test.
fn apply_legacy_migrations(conn: &mut Connection) {
    for migration in MIGRATIONS.iter().filter(|m| m.version <= 13) {
        conn.execute_batch(migration.sql)
            .unwrap_or_else(|_| panic!("apply legacy migration {}", migration.version));
        conn.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                migration.version,
                migration.name,
                checksum_of(migration.sql),
                STAMP,
            ],
        )
        .unwrap_or_else(|_| panic!("record legacy migration {}", migration.version));
    }
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("legacy migration count");
    assert_eq!(
        count, 13,
        "legacy database must hold exactly migrations 1..=13"
    );
}

/// Canonical DEP-01 row is seeded by migration 0012; reuse it (never duplicate).
fn legacy_warehouse_id(conn: &Connection) -> String {
    conn.query_row(
        "SELECT id FROM warehouses WHERE code = 'DEP-01'",
        [],
        |row| row.get(0),
    )
    .expect("migration 0012 must have seeded DEP-01")
}

fn seed_legacy_business_data(conn: &Connection) {
    let warehouse_id = legacy_warehouse_id(conn);

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('u-admin', 'admin', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$hash', 'ADMIN', 1, ?1, ?1)",
        [STAMP],
    )
    .expect("seed admin user");

    for (id, code, name_ar, name_fr) in [
        ("cat-01", "QUI", "خردوات", "Quincaillerie"),
        ("cat-02", "ELE", "كهرباء", "Électricité"),
    ] {
        conn.execute(
            "INSERT INTO categories (id, code, name_ar, name_fr, is_active)
             VALUES (?1, ?2, ?3, ?4, 1)",
            [id, code, name_ar, name_fr],
        )
        .expect("seed category");
    }

    // Three products, each with one base selling unit (factor 10^4 = 1.0).
    for (pid, sku, name_fr, sale, purchase, category) in [
        ("p-01", "VIS-060", "Vis bois 6x60", 2_500, 1_800, "cat-01"),
        ("p-02", "MAR-001", "Marteau 500g", 85_000, 62_000, "cat-01"),
        ("p-03", "CAB-25", "Câble 2.5mm (m)", 12_000, 9_500, "cat-02"),
    ] {
        conn.execute(
            "INSERT INTO products
             (id, sku, name_ar, name_fr, category_id, created_at, updated_at, revision, origin_device_id)
             VALUES (?1, ?2, 'صنف', ?3, ?4, ?5, ?5, 1, 'LOCAL')",
            rusqlite::params![pid, sku, name_fr, category, STAMP],
        )
        .expect("seed product");
        let unit_id = format!("{pid}-base");
        conn.execute(
            "INSERT INTO product_units
             (id, product_id, unit_name_ar, unit_name_fr, factor_micro,
              sale_price_centimes, purchase_price_centimes, is_base_unit, is_default_selling)
             VALUES (?1, ?2, 'قطعة', 'Pièce', 10000, ?3, ?4, 1, 1)",
            rusqlite::params![unit_id, pid, sale, purchase],
        )
        .expect("seed base unit");
    }

    // Two customers; c-01 carries the legacy opening debt of 450,000 centimes.
    for (id, code, name, balance) in [
        ("c-01", "CL-0001", "Ahmed Benali", LEGACY_DEBT_CENTIMES),
        ("c-02", "CL-0002", "Sara Mansouri", 0_i64),
    ] {
        conn.execute(
            "INSERT INTO customers
             (id, code, name, phone, credit_policy, credit_limit_centimes,
              current_balance_centimes, created_at, updated_at, revision, origin_device_id)
             VALUES (?1, ?2, ?3, NULL, 'LIMITED', 1000000, ?4, ?5, ?5, 1, 'LOCAL')",
            rusqlite::params![id, code, name, balance, STAMP],
        )
        .expect("seed customer");
    }
    conn.execute(
        "INSERT INTO customer_ledger
         (id, sequence_no, customer_id, entry_type, amount_centimes,
          running_balance_centimes, document_type, document_id, created_by, created_at)
         VALUES ('cl-01', 1, 'c-01', 'DEBIT', ?1, ?1, 'OPENING_BALANCE', NULL, 'u-admin', ?2)",
        rusqlite::params![LEGACY_DEBT_CENTIMES, STAMP],
    )
    .expect("seed opening debt ledger entry");

    // 100 base units of product 1 in DEP-01, with the matching IN movement.
    conn.execute(
        "INSERT INTO stock_levels (warehouse_id, product_id, quantity_micro, quantity_base_micro)
         VALUES (?1, 'p-01', ?2, ?2)",
        rusqlite::params![warehouse_id, LEGACY_STOCK_BASE_MICRO],
    )
    .expect("seed stock level");
    conn.execute(
        "INSERT INTO stock_movements
         (id, sequence_no, warehouse_id, product_id, movement_type,
          quantity_micro, quantity_base_micro, created_by, created_at)
         VALUES ('sm-01', 1, ?1, 'p-01', 'IN', ?2, ?2, 'u-admin', ?3)",
        rusqlite::params![warehouse_id, LEGACY_STOCK_BASE_MICRO, STAMP],
    )
    .expect("seed stock movement");

    // One finalized BL (cash sale to c-02): 2 units of p-01, fully paid.
    conn.execute(
        "INSERT INTO documents
         (id, number, fiscal_year, type, party_type, party_id, warehouse_id,
          document_language, subtotal_ht_centimes, vat_total_centimes,
          global_discount_centimes, timbre_centimes, rounding_centimes,
          ttc_centimes, paid_amount_centimes, change_amount_centimes,
          payment_method, status, created_at, created_by, revision, origin_device_id)
         VALUES ('d-bl-01', 'BL-2026-000001', 2026, 'BL', 'CUSTOMER', 'c-02', ?1,
                 'FR', 5000, 950, 0, 0, 0, 5950, 5950, 0,
                 'ESPECES', 'FINALIZED', ?2, 'u-admin', 1, 'LOCAL')",
        rusqlite::params![warehouse_id, STAMP],
    )
    .expect("seed finalized BL");
    conn.execute(
        "INSERT INTO document_items
         (id, document_id, product_id, unit_id, quantity_micro, quantity_base_micro,
          unit_price_centimes, discount_centimes, tva_rate_micro,
          line_ht_centimes, line_tva_centimes, line_ttc_centimes,
          cost_price_centimes, sort_order)
         VALUES ('di-01', 'd-bl-01', 'p-01', 'p-01-base', 20000, 20000,
                 2500, 0, 190000, 5000, 950, 5950, 1800, 0)",
        [],
    )
    .expect("seed BL item");
    conn.execute(
        "INSERT INTO payments
         (id, sequence_no, document_id, customer_id, amount_centimes,
          payment_method, cash_session_id, created_by, created_at)
         VALUES ('pay-01', 1, 'd-bl-01', NULL, 5950, 'ESPECES', NULL, 'u-admin', ?1)",
        [STAMP],
    )
    .expect("seed BL payment");
}

struct Baseline {
    debt_balance: i64,
    ledger_net: i64,
    stock_micro: i64,
    documents: i64,
    document_items: i64,
    payments: i64,
    customers: i64,
    products: i64,
    movements: i64,
    bl_ttc: i64,
    unit_prices: Vec<(String, i64, i64)>,
}

fn capture_baseline(conn: &Connection) -> Baseline {
    let debt_balance: i64 = conn
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = 'c-01'",
            [],
            |row| row.get(0),
        )
        .expect("baseline debt");
    let ledger_net: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(CASE WHEN entry_type = 'DEBIT' THEN amount_centimes
                                     ELSE -amount_centimes END), 0)
             FROM customer_ledger WHERE customer_id = 'c-01'",
            [],
            |row| row.get(0),
        )
        .expect("baseline ledger net");
    let stock_micro: i64 = conn
        .query_row(
            "SELECT quantity_micro FROM stock_levels WHERE product_id = 'p-01'",
            [],
            |row| row.get(0),
        )
        .expect("baseline stock");
    let count = |table: &str| -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap_or_else(|_| panic!("baseline count {table}"))
    };
    let bl_ttc: i64 = conn
        .query_row(
            "SELECT ttc_centimes FROM documents WHERE id = 'd-bl-01'",
            [],
            |row| row.get(0),
        )
        .expect("baseline BL total");
    let mut stmt = conn
        .prepare("SELECT id, sale_price_centimes, purchase_price_centimes FROM product_units ORDER BY id")
        .expect("baseline units query");
    let unit_prices = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("baseline units")
        .map(|row| row.expect("unit row"))
        .collect();

    // Stage-1 sanity: the seeded business facts must hold before the upgrade.
    assert_eq!(debt_balance, LEGACY_DEBT_CENTIMES, "baseline debt");
    assert_eq!(ledger_net, LEGACY_DEBT_CENTIMES, "baseline ledger net");
    assert_eq!(stock_micro, LEGACY_STOCK_BASE_MICRO, "baseline stock");
    assert_eq!(bl_ttc, 5_950, "baseline BL total");

    Baseline {
        debt_balance,
        ledger_net,
        stock_micro,
        documents: count("documents"),
        document_items: count("document_items"),
        payments: count("payments"),
        customers: count("customers"),
        products: count("products"),
        movements: count("stock_movements"),
        bl_ttc,
        unit_prices,
    }
}

/// Build a legacy v13 database, seed it, and upgrade it to 0019 via the
/// official runner. Shared by the replay test and the downgrade probe so
/// each test owns an isolated scratch file.
fn build_upgraded_db(name: &str) -> (ScratchDb, Baseline) {
    let mut db = open_scratch(name);
    apply_legacy_migrations(&mut db.conn);
    seed_legacy_business_data(&db.conn);
    let baseline = capture_baseline(&db.conn);
    assert_eq!(baseline.documents, 1, "exactly one seeded document");
    assert_eq!(baseline.customers, 2, "exactly two seeded customers");
    run_migrations(&mut db.conn).expect("official upgrade 0013 -> 0019");
    (db, baseline)
}

fn assert_parity(conn: &Connection, baseline: &Baseline) {
    // All 19 migrations recorded, checksums non-empty and byte-identical to
    // the embedded SQL (a changed byte would abort the runner on real DBs).
    let (count, max_version): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), MAX(version) FROM schema_migrations WHERE version BETWEEN 1 AND 19",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("migration bookkeeping readable");
    assert_eq!(count, 19, "all 19 migrations recorded");
    assert_eq!(max_version, 19, "schema at version 19");
    for migration in MIGRATIONS.iter() {
        let recorded: String = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| panic!("migration {} not recorded", migration.version));
        assert!(
            !recorded.is_empty(),
            "migration {} checksum non-empty",
            migration.version
        );
        assert_eq!(
            recorded,
            checksum_of(migration.sql),
            "migration {} checksum byte-identical",
            migration.version
        );
    }

    // Financial parity down to the centime.
    let debt: i64 = conn
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = 'c-01'",
            [],
            |row| row.get(0),
        )
        .expect("debt readable after upgrade");
    assert_eq!(
        debt, baseline.debt_balance,
        "customer debt preserved exactly"
    );
    assert_eq!(debt, LEGACY_DEBT_CENTIMES, "debt still 450,000 centimes");
    let ledger_net: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(CASE WHEN entry_type = 'DEBIT' THEN amount_centimes
                                     ELSE -amount_centimes END), 0)
             FROM customer_ledger WHERE customer_id = 'c-01'",
            [],
            |row| row.get(0),
        )
        .expect("ledger net readable");
    assert_eq!(ledger_net, baseline.ledger_net, "ledger net preserved");
    let stock: i64 = conn
        .query_row(
            "SELECT quantity_micro FROM stock_levels WHERE product_id = 'p-01'",
            [],
            |row| row.get(0),
        )
        .expect("stock readable");
    assert_eq!(stock, baseline.stock_micro, "stock count preserved");
    assert_eq!(stock, LEGACY_STOCK_BASE_MICRO, "still 100 base units");
    let bl_ttc: i64 = conn
        .query_row(
            "SELECT ttc_centimes FROM documents WHERE id = 'd-bl-01'",
            [],
            |row| row.get(0),
        )
        .expect("BL readable");
    assert_eq!(bl_ttc, baseline.bl_ttc, "BL total preserved");
    // DOC-04: the pre-migration BL (seeded without the column) upgrades
    // to template v1 — the geometry it was printed with — never NULL.
    let bl_template: i64 = conn
        .query_row(
            "SELECT template_version FROM documents WHERE id = 'd-bl-01'",
            [],
            |row| row.get(0),
        )
        .expect("BL template version readable");
    assert_eq!(bl_template, 1, "legacy document pins template v1");

    // Row-count parity: nothing gained, nothing lost.
    for (table, expected) in [
        ("documents", baseline.documents),
        ("document_items", baseline.document_items),
        ("payments", baseline.payments),
        ("customers", baseline.customers),
        ("products", baseline.products),
        ("stock_movements", baseline.movements),
    ] {
        let actual: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|_| panic!("count {table}"));
        assert_eq!(actual, expected, "row-count parity for {table}");
    }

    // Migration 0015 shapes: legacy customers default to canonical
    // lowercase 'retail'; legacy units keep NULL wholesale price while
    // sale/purchase prices are untouched.
    for customer in ["c-01", "c-02"] {
        let tier: String = conn
            .query_row(
                "SELECT customer_tier FROM customers WHERE id = ?1",
                [customer],
                |row| row.get(0),
            )
            .expect("tier readable");
        assert_eq!(
            tier, "retail",
            "legacy customer {customer} defaults to retail tier"
        );
    }
    let mut stmt = conn
        .prepare("SELECT id, sale_price_centimes, purchase_price_centimes FROM product_units ORDER BY id")
        .expect("units query");
    let current: Vec<(String, i64, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("units")
        .map(|row| row.expect("unit row"))
        .collect();
    assert_eq!(
        current, baseline.unit_prices,
        "sale/purchase prices untouched by 0015"
    );
    let null_wholesale: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM product_units WHERE wholesale_price_centimes IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("wholesale null count");
    assert_eq!(null_wholesale, 3, "legacy units keep NULL wholesale price");

    // Migration 0014 shape: print_jobs.document_id is now nullable.
    let notnull: i64 = conn
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('print_jobs') WHERE name = 'document_id'",
            [],
            |row| row.get(0),
        )
        .expect("print_jobs pragma");
    assert_eq!(notnull, 0, "0014 made print_jobs.document_id nullable");

    // Migration 0018 shape: the per-unit column exists and every legacy
    // line reads back NULL — legacy display keeps the transcribed lump
    // discount, legacy returns keep the lump pro-rating (BRK-096).
    let per_unit_col: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('document_items')
             WHERE name = 'discount_per_unit_centimes'",
            [],
            |row| row.get(0),
        )
        .expect("document_items pragma");
    assert_eq!(per_unit_col, 1, "0018 added discount_per_unit_centimes");
    let legacy_null: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM document_items
             WHERE discount_per_unit_centimes IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("legacy per-unit null count");
    assert_eq!(
        legacy_null, baseline.document_items,
        "legacy lines all read back NULL per-unit"
    );

    // Migration 0016 shape: the FTS5 index exists, is consistent with the
    // base tables, and legacy rows were backfilled with non-empty norms.
    let fts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'products_fts'",
            [],
            |row| row.get(0),
        )
        .expect("fts presence check");
    assert_eq!(fts, 1, "0016 created the products_fts table");
    let mar_norm: (String, String) = conn
        .query_row(
            "SELECT search_norm_fr, search_norm_ar FROM products WHERE sku = 'MAR-001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("legacy product norms readable");
    assert!(
        !mar_norm.0.is_empty(),
        "legacy MAR-001 backfilled with non-empty search_norm_fr"
    );
    assert!(
        !mar_norm.1.is_empty(),
        "legacy MAR-001 backfilled with non-empty search_norm_ar"
    );
    let by_sku: String = conn
        .query_row(
            "SELECT name_fr FROM products WHERE sku = 'MAR-001'",
            [],
            |row| row.get(0),
        )
        .expect("catalog lookup");
    assert_eq!(by_sku, "Marteau 500g");

    // Structural integrity: foreign keys clean after both rebuilds.
    let violations: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("fk check");
    assert_eq!(violations, 0, "foreign_key_check clean after upgrade");
}

#[test]
fn migration_replay_preserves_legacy_business_data() {
    let (db, baseline) = build_upgraded_db("replay.db");
    assert_parity(&db.conn, &baseline);

    // The upgraded file also reopens cleanly through a fresh connection
    // (WAL checkpoint + pragma path), proving the artifact is durable.
    let reopened = open_connection(&db.path).expect("reopen upgraded db");
    assert_parity(&reopened, &baseline);

    // The FTS index is consistent on the UPGRADED drill DB (legacy-seeded
    // rows included): row counts match and sampled norms recompute.
    let consistency_conn = open_connection(&db.path).expect("consistency connection");
    let repo = SqliteCatalogRepository::new(Arc::new(Mutex::new(consistency_conn)));
    repo.verify_fts_consistency()
        .expect("fts consistent on upgraded drill db");
}

#[test]
fn downgrade_guard_probe_future_version_is_deterministic() {
    let (mut db, baseline) = build_upgraded_db("downgrade_probe.db");

    // Simulate a database last written by a newer binary: a future
    // bookkeeping row the embedded runner does not know.
    db.conn
        .execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (9999, '9999_future_schema.sql', 'fake_sha256_hash_for_probe', ?1)",
            [STAMP],
        )
        .expect("inject future migration row");

    // OBSERVED BEHAVIOR (pinned here): the runner iterates only its embedded
    // MIGRATIONS list and verifies only those checksums, so unknown future
    // rows are IGNORED and the call returns Ok — it neither applies,
    // validates, nor removes the future row, and it never wipes data. There
    // is no hard startup block; forward-compatibility is by ignore-list.
    run_migrations(&mut db.conn).expect("runner must tolerate unknown future row");
    let max_version: i64 = db
        .conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("max version");
    assert_eq!(max_version, 9999, "future row left untouched");
    let future_name: String = db
        .conn
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 9999",
            [],
            |row| row.get(0),
        )
        .expect("future row still present");
    assert_eq!(future_name, "9999_future_schema.sql");

    // Data is byte-intact after the probe: full parity still holds
    // (assert_parity scopes the bookkeeping check to versions 1..=19,
    // so the synthetic future row coexists without tripping it).
    assert_parity(&db.conn, &baseline);

    // Re-running is idempotent: still Ok, still 20 rows (19 known + future).
    run_migrations(&mut db.conn).expect("second run after probe");
    let total: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("total rows");
    assert_eq!(total, 20, "19 known + 1 future row, no duplication");
}
