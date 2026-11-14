//! BRK-076: FTS5 migration 0016 apparatus tests.
//!
//! Step-0 gate, apply paths (fresh + upgrade), data preservation,
//! trigger behavior, consistency red-before-green, 80k backfill clock.

use baraka_infrastructure::persistence::connection::open_connection;

/// Step 0 (BRK-076): rusqlite bundled must ship FTS5, else STOP.
#[test]
fn step0_fts5_available_in_bundled_sqlite() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open_connection(&dir.path().join("step0.db")).expect("open");
    let enabled: i32 = conn
        .query_row(
            "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
            [],
            |row| row.get(0),
        )
        .expect("compileoption query");
    assert_eq!(enabled, 1, "FTS5 must be compiled in (got {enabled})");
    // Bundled SQLite is 3.46.0: `unicode61 "remove_diacritics N"` needs 3.51+,
    // so the FTS table uses plain `unicode61`. Equivalent by design: norms are
    // pre-stripped by `normalize_for_search` at write time (ADR-101: the FTS
    // tokenizer is deliberately dumb; unicode61 keeps ASCII case-folding).
    conn.execute_batch("CREATE VIRTUAL TABLE step0 USING fts5(x, tokenize='unicode61')")
        .expect("fts5 virtual table creation");
}
use std::sync::{Arc, Mutex};

use baraka_application::ports::SessionContext;
use baraka_application::ports::SessionToken;
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, NewUnitInput,
};
use baraka_domain::catalog::search_normalization::normalize_for_search;
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::migration::{checksum_of, run_migrations, MIGRATIONS};
use chrono::TimeDelta;

fn session_gerant() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role: Role::Gerant,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn base_unit(barcode: Option<String>) -> NewUnitInput {
    NewUnitInput {
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Piece".to_string(),
        factor_micro: 10_000,
        barcode,
        sale_price_centimes: 15_000,
        purchase_price_centimes: 12_000,
        wholesale_price_centimes: None,
        super_gros_price_centimes: None,
        is_base_unit: true,
        is_default_selling: true,
    }
}

/// Apply migrations 0001-0015 raw + bookkeeping: a genuine pre-16 database.
fn apply_pre16(conn: &mut rusqlite::Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY, name TEXT NOT NULL,
            checksum TEXT NOT NULL, applied_at TEXT NOT NULL) STRICT;",
    )
    .expect("bookkeeping table");
    for migration in MIGRATIONS.iter().filter(|m| m.version < 16) {
        conn.execute_batch(migration.sql)
            .unwrap_or_else(|_| panic!("apply v{}", migration.version));
        conn.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, ?2, ?3, '2026-01-01T00:00:00Z')",
            rusqlite::params![
                migration.version,
                migration.name,
                checksum_of(migration.sql)
            ],
        )
        .expect("record version");
    }
}

fn seed_two_products(db_path: &std::path::Path) {
    let mut conn = open_connection(db_path).expect("open");
    apply_pre16(&mut conn);
    // Genuine legacy rows: raw pre-16 SQL (no repo code — current writers
    // target the post-21 schema and must fail here by construction).
    for (sku, fr, ar, barcode) in [
        (
            "VIS-PLA-25",
            "Vis Placo 3.5x25",
            "برغي بلاكو",
            Some("6111250090011"),
        ),
        ("COU-PVC-32", "Coude PVC 32", "كود بلاستيك", None),
    ] {
        seed_legacy_product(&conn, sku, fr, ar, barcode);
    }
}

/// One pre-16 product + base unit, written with exactly the columns that
/// existed before migration 0016 (wholesale included via 0015, no
/// `search_norm_*`, no `super_gros_price_centimes`).
fn seed_legacy_product(
    conn: &rusqlite::Connection,
    sku: &str,
    name_fr: &str,
    name_ar: &str,
    barcode: Option<&str>,
) {
    let product_id = format!("legacy-{sku}");
    let unit_id = format!("legacy-{sku}-base");
    conn.execute(
        "INSERT INTO products
         (id, sku, barcode, name_ar, name_fr, category_id, is_kit, is_active,
          created_at, updated_at, revision, origin_device_id)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0, 1,
                 '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', 1, 'LOCAL')",
        rusqlite::params![product_id, sku, barcode, name_ar, name_fr],
    )
    .expect("legacy product");
    conn.execute(
        "INSERT INTO product_units
         (id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode,
          sale_price_centimes, purchase_price_centimes, wholesale_price_centimes,
          is_base_unit, is_default_selling)
         VALUES (?1, ?2, 'قطعة', 'Piece', 10000, ?3, 15000, 12000, NULL, 1, 1)",
        rusqlite::params![unit_id, product_id, barcode],
    )
    .expect("legacy unit");
}

/// 0016 applies cleanly on a fresh database with bookkeeping.
#[test]
fn fresh_apply_registers_v16_with_checksum() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("fresh.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let (name, checksum): (String, String) = conn
        .query_row(
            "SELECT name, checksum FROM schema_migrations WHERE version = 16",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("v16 bookkeeping");
    assert_eq!(name, "0016_fts_search_index.sql");
    let expected = checksum_of(
        MIGRATIONS
            .iter()
            .find(|migration| migration.version == 16)
            .expect("v16 embedded")
            .sql,
    );
    assert_eq!(checksum, expected);
}

/// 0016+0017+0018+0019+0020+0021+0022 apply on an upgraded database (0001-0015 pre-applied raw).
#[test]
fn upgrade_path_applies_only_pending() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("up.db")).expect("open");
    apply_pre16(&mut conn);
    run_migrations(&mut conn).expect("upgrade to 22");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 22);
    let fts_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM products_fts", [], |row| row.get(0))
        .expect("fts readable");
    assert_eq!(fts_count, 0);
    let backup_cols: Vec<String> = conn
        .prepare("PRAGMA table_info(backups_log)")
        .expect("pragma")
        .query_map([], |row| row.get(1))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("cols");
    assert!(backup_cols.contains(&"file_size_bytes".to_string()));
    assert!(backup_cols.contains(&"trigger".to_string()));
}

/// Data preservation on a migrated COPY: counts equal, spot norms exact.
#[test]
fn data_preservation_backfills_norms_and_aliases() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("pre.db");
    seed_two_products(&db_path);
    let mut conn = open_connection(&db_path).expect("reopen");
    run_migrations(&mut conn).expect("migrate to 16");
    let (products, units): (i64, i64) = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM products),
                    (SELECT COUNT(*) FROM product_units)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("counts");
    assert_eq!((products, units), (2, 2));
    let (norm_fr, norm_ar): (String, String) = conn
        .query_row(
            "SELECT search_norm_fr, search_norm_ar FROM products WHERE sku = 'VIS-PLA-25'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("norms");
    assert_eq!(norm_fr, normalize_for_search("Vis Placo 3.5x25"));
    assert_eq!(norm_ar, normalize_for_search("برغي بلاكو"));
    // Alias seed: token "vis" expands to the Arabic phonetic aliases.
    let aliases: String = conn
        .query_row(
            "SELECT group_concat(alias_norm, ' ') FROM product_aliases
             WHERE product_id = (SELECT id FROM products WHERE sku = 'VIS-PLA-25')",
            [],
            |row| row.get(0),
        )
        .expect("aliases");
    assert!(aliases.contains("فيس"), "alias فيس seeded, got: {aliases}");
    // FTS mirror is queryable through MATCH (read-only probe).
    let fts_hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM products_fts WHERE products_fts MATCH 'vis*'",
            [],
            |row| row.get(0),
        )
        .expect("fts match");
    assert_eq!(fts_hits, 1);
    let ar_hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM products_fts WHERE products_fts MATCH 'فيس'",
            [],
            |row| row.get(0),
        )
        .expect("fts match ar");
    assert_eq!(ar_hits, 1);
}

/// RED (regression-law demo): pre-backfill state fails consistency.
#[test]
fn consistency_fails_on_pre_backfill_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("red.db");
    seed_two_products(&db_path);
    let mut conn = open_connection(&db_path).expect("reopen");
    run_migrations(&mut conn).expect("migrate to 16");
    // Simulate the pre-backfill state: mirror emptied, relational rows kept.
    conn.execute_batch("DELETE FROM products_fts;")
        .expect("empty fts");
    let repo = SqliteCatalogRepository::new(Arc::new(Mutex::new(conn)));
    let result = repo.verify_fts_consistency();
    assert!(result.is_err(), "pre-backfill state MUST fail, got Ok");
    let message = format!("{:?}", result.expect_err("must fail"));
    assert!(
        message.contains("row-count mismatch"),
        "unexpected: {message}"
    );
}

/// RED-2: a stale FTS row (corrupted norm) fails consistency.
/// Seeds 65 products so the every-64th stride provably samples rowid 64.
#[test]
fn consistency_fails_on_stale_norm() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("stale.db");
    seed_two_products(&db_path);
    {
        // Fillers are legacy rows too: the stride sampler needs 65 rows
        // present BEFORE the 0016 upgrade.
        let conn = open_connection(&db_path).expect("open");
        for index in 0..63 {
            seed_legacy_product(
                &conn,
                &format!("STALE-{index:02}"),
                &format!("Article Test {index}"),
                &format!("صنف {index}"),
                None,
            );
        }
    }
    let mut conn = open_connection(&db_path).expect("reopen");
    run_migrations(&mut conn).expect("migrate to 16");
    // Corrupt every FTS row directly (bypasses triggers, like a torn write).
    // NOTE: the sampler checks every 64th row, so on a 2-product database a
    // targeted corruption could hide in the unsampled gap — corrupting all
    // rows proves detection independent of stride alignment.
    conn.execute("UPDATE products_fts SET norm_fr = 'zzz-stale'", [])
        .expect("corrupt");
    let repo = SqliteCatalogRepository::new(Arc::new(Mutex::new(conn)));
    assert!(repo.verify_fts_consistency().is_err());
}

/// GREEN: migrated database passes consistency.
#[test]
fn consistency_passes_after_migration() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("green.db");
    seed_two_products(&db_path);
    let mut conn = open_connection(&db_path).expect("reopen");
    run_migrations(&mut conn).expect("migrate to 16");
    let repo = SqliteCatalogRepository::new(Arc::new(Mutex::new(conn)));
    repo.verify_fts_consistency().expect("consistent");
}

fn raw_exec(db_path: &std::path::Path, sql: &str) {
    let conn = open_connection(db_path).expect("open");
    conn.execute_batch(sql).expect("raw exec");
}

fn fts_count(db_path: &std::path::Path, match_query: &str) -> i64 {
    let conn = open_connection(db_path).expect("open");
    conn.query_row(
        "SELECT COUNT(*) FROM products_fts WHERE products_fts MATCH ?1",
        [match_query],
        |row| row.get(0),
    )
    .expect("fts match")
}

fn check_consistent(db_path: &std::path::Path) -> Result<(), String> {
    let conn = open_connection(db_path).expect("open");
    let repo = SqliteCatalogRepository::new(Arc::new(Mutex::new(conn)));
    repo.verify_fts_consistency()
        .map_err(|error| format!("{error:?}"))
}

/// Triggers keep the mirror synchronous on every write path.
#[test]
fn triggers_mirror_inserts_updates_deletes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("trg.db");
    {
        let mut conn = open_connection(&db_path).expect("open");
        run_migrations(&mut conn).expect("migrate");
    }
    {
        let conn = open_connection(&db_path).expect("open");
        let repo = SqliteCatalogRepository::new(Arc::new(Mutex::new(conn)));
        CreateProductUseCase::new(&repo, &repo)
            .execute(
                &session_gerant(),
                CreateProductInput {
                    sku: "TRG-1".to_string(),
                    barcode: None,
                    name_ar: "قفل فولاذ".to_string(),
                    name_fr: "Cadenas Acier 40mm".to_string(),
                    category_id: None,
                    is_kit: false,
                    units: vec![base_unit(Some("4001".to_string()))],
                },
                "FTS-PROBE",
            )
            .expect("seed");
    }
    // Insert trigger mirrored the row.
    assert_eq!(fts_count(&db_path, "cadenas*"), 1);
    assert_eq!(fts_count(&db_path, "4001"), 1);
    // Unit barcode change refreshes norm_barcode.
    raw_exec(
        &db_path,
        "UPDATE product_units SET barcode = '4002' WHERE barcode = '4001'",
    );
    let conn = open_connection(&db_path).expect("open");
    let barcode_norm: String = conn
        .query_row(
            "SELECT norm_barcode FROM products_fts WHERE norm_fr LIKE '%cadenas%'",
            [],
            |row| row.get(0),
        )
        .expect("barcode norm");
    assert!(
        barcode_norm.contains("4002"),
        "unit change mirrored: {barcode_norm}"
    );
    assert!(
        !barcode_norm.contains("4001"),
        "stale barcode gone: {barcode_norm}"
    );
    drop(conn);
    // Product rename refreshes norms.
    raw_exec(
        &db_path,
        "UPDATE products SET name_fr = 'Serrure Porte' WHERE sku = 'TRG-1'",
    );
    assert_eq!(fts_count(&db_path, "serrure*"), 1);
    assert_eq!(fts_count(&db_path, "cadenas*"), 0);
    // Alias delete refreshes norm_aliases.
    raw_exec(
        &db_path,
        "DELETE FROM product_aliases WHERE product_id = (SELECT id FROM products WHERE sku = 'TRG-1')",
    );
    // Product delete removes the FTS row (units first: FK RESTRICT, same
    // order as the repository delete path).
    raw_exec(
        &db_path,
        "DELETE FROM product_units WHERE product_id = (SELECT id FROM products WHERE sku = 'TRG-1');
         DELETE FROM products WHERE sku = 'TRG-1';",
    );
    assert_eq!(fts_count(&db_path, "serrure*"), 0);
    check_consistent(&db_path).expect("consistent after writes");
}

/// 80k backfill wall clock (measurement only, #[ignore] like the perf probe).
/// Seeds genuine legacy rows on a pre-16 database, then times the v16
/// upgrade (DDL + norm backfill + alias seeding, one transaction).
#[test]
#[ignore = "slow: seeds 80k products; run explicitly with --ignored --nocapture"]
fn backfill_80k_wall_clock() {
    use std::time::Instant;
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("backfill80k.db");
    {
        let mut conn = open_connection(&db_path).expect("open");
        apply_pre16(&mut conn);
    }
    const COUNT: usize = 80_000;
    const FR: &[&str] = &[
        "Vis Placo",
        "Boulon Hex",
        "Robinet Laiton",
        "Cadenas Acier",
        "Cable Cuivre",
        "Marteau Manche",
        "Serrure Porte",
        "Tuyau PVC",
    ];
    const AR: &[&str] = &[
        "برغي بلاكو",
        "صامولة حديد",
        "حنفية نحاس",
        "قفل فولاذ",
        "كابل نحاس",
        "مطرقة يد",
        "قفل باب",
        "انبوب بلاستيك",
    ];
    {
        let mut conn = open_connection(&db_path).expect("open");
        let tx = conn.transaction().expect("tx");
        {
            let mut insert_product = tx
                .prepare(
                    "INSERT INTO products
                     (id, sku, barcode, name_ar, name_fr, category_id, is_kit, is_active,
                      created_at, updated_at, revision, origin_device_id)
                     VALUES (?1, ?2, NULL, ?3, ?4, NULL, 0, 1,
                             '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z', 1, 'LOCAL')",
                )
                .expect("prepare product");
            let mut insert_unit = tx
                .prepare(
                    "INSERT INTO product_units
                     (id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode,
                      sale_price_centimes, purchase_price_centimes, wholesale_price_centimes,
                      is_base_unit, is_default_selling)
                     VALUES (?1, ?2, 'قطعة', 'Piece', 10000, ?3, 15000, 12000, NULL, 1, 1)",
                )
                .expect("prepare unit");
            for index in 0..COUNT {
                let barcode = if index % 5 < 2 {
                    Some(format!("80K{index:08}"))
                } else {
                    None
                };
                let product_id = format!("bulk-{index:05}");
                insert_product
                    .execute(rusqlite::params![
                        product_id,
                        format!("BF-{index:05}"),
                        format!("{} رقم {index}", AR[index % AR.len()]),
                        format!("{} n°{index}", FR[index % FR.len()]),
                    ])
                    .unwrap_or_else(|_| panic!("seed product {index}"));
                insert_unit
                    .execute(rusqlite::params![
                        format!("bulk-{index:05}-base"),
                        product_id,
                        barcode,
                    ])
                    .unwrap_or_else(|_| panic!("seed unit {index}"));
            }
        }
        tx.commit().expect("commit legacy bulk");
    }
    let mut conn = open_connection(&db_path).expect("reopen");
    let started = Instant::now();
    run_migrations(&mut conn).expect("migrate to 16");
    let elapsed = started.elapsed();
    println!("BACKFILL 80k upgrade (DDL + norms + aliases, one tx): {elapsed:?}");
    let (products, fts, aliases): (i64, i64, i64) = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM products),
                    (SELECT COUNT(*) FROM products_fts),
                    (SELECT COUNT(*) FROM product_aliases)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("counts");
    println!("BACKFILL counts: products={products} fts={fts} aliases={aliases}");
    assert_eq!(products, COUNT as i64);
    assert_eq!(fts, COUNT as i64);
    drop(conn);
    check_consistent(&db_path).expect("consistent after 80k backfill");
}
