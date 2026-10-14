//! Atomic catalog batch-import engine tests (TASK-BRK-067).
//!
//! End-to-end coverage of `ImportProductsUseCase` over the real SQLite
//! repository: product + base-unit creation, DEP-01 opening-stock
//! provisioning, duplicate-SKU skip/error policies, deterministic SKU
//! generation, and row validation.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{SessionContext, SessionToken};
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, ImportBatchInput, ImportBatchReport,
    ImportDuplicatePolicy, ImportProductRow, ImportProductsUseCase, NewUnitInput,
};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use rusqlite::Connection;
use uuid::Uuid;

const GERANT_UUID: &str = "33333333-3333-3333-3333-333333333333";
const TEST_TERMINAL: &str = "TEST-TERMINAL-FIX";
const DEP01_ID: &str = "01950000-0000-7000-8000-000000000001";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<Connection>>,
    repo: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("import.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'gerant1', 'hash', 'GERANT', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![GERANT_UUID],
    )
    .expect("seed user");
    let db = Arc::new(Mutex::new(conn));
    let repo = SqliteCatalogRepository::new(Arc::clone(&db));
    Harness {
        _dir: dir,
        db,
        repo,
    }
}

fn gerant() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(GERANT_UUID).expect("uuid")),
        role: Role::Gerant,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn row(sku: Option<&str>, name_fr: &str, sale_price_centimes: i64) -> ImportProductRow {
    ImportProductRow {
        sku: sku.map(str::to_string),
        barcode: None,
        name_fr: name_fr.to_string(),
        name_ar: None,
        category_id: None,
        base_unit_name_fr: None,
        base_unit_name_ar: None,
        purchase_price_centimes: Some(10_000),
        sale_price_centimes,
        initial_quantity_micro: None,
    }
}

fn run_batch(
    harness: &Harness,
    rows: Vec<ImportProductRow>,
    on_duplicate: ImportDuplicatePolicy,
) -> ImportBatchReport {
    ImportProductsUseCase::new(&harness.repo)
        .execute(
            &gerant(),
            ImportBatchInput {
                rows,
                warehouse_code: None,
                on_duplicate,
                origin_device_id: TEST_TERMINAL.to_string(),
            },
        )
        .expect("import")
}

fn count(harness: &Harness, sql: &str) -> i64 {
    harness
        .db
        .lock()
        .expect("lock")
        .query_row(sql, [], |row| row.get(0))
        .expect("count")
}

fn seed_product(harness: &Harness, sku: &str) {
    CreateProductUseCase::new(&harness.repo, &harness.repo)
        .execute(
            &gerant(),
            CreateProductInput {
                sku: sku.to_string(),
                barcode: None,
                name_ar: "موجود".to_string(),
                name_fr: "Produit Existant".to_string(),
                category_id: None,
                is_kit: false,
                units: vec![NewUnitInput {
                    unit_name_ar: "قطعة".to_string(),
                    unit_name_fr: "Pièce".to_string(),
                    factor_micro: 10_000,
                    barcode: None,
                    sale_price_centimes: 20_000,
                    purchase_price_centimes: 15_000,
                    is_base_unit: true,
                    is_default_selling: true,
                }],
            },
            TEST_TERMINAL,
        )
        .expect("seed product");
}

#[test]
fn import_batch_creates_products_and_base_units() {
    let harness = setup();
    let rows = vec![
        row(Some("IMP-001"), "Vis Placo 25", 15_000),
        row(Some("imp-002"), "Coude PVC 32", 8_500),
        row(Some("IMP-003"), "Câble 2.5mm", 32_000),
        row(Some("IMP-004"), "Robinet 1/2", 45_000),
        row(Some("IMP-005"), "Boulon M8", 2_500),
    ];
    let report = run_batch(&harness, rows, ImportDuplicatePolicy::Skip);
    assert_eq!(report.total_rows, 5);
    assert_eq!(report.imported_count, 5);
    assert_eq!(report.skipped_count, 0);
    assert!(report.errors.is_empty());
    assert_eq!(count(&harness, "SELECT COUNT(*) FROM products"), 5);
    assert_eq!(count(&harness, "SELECT COUNT(*) FROM product_units"), 5);
    // Lowercase SKU is uppercased; base unit carries hardware defaults.
    let (sku, unit_fr, unit_ar, factor, base, def): (String, String, String, i64, i64, i64) =
        harness
            .db
            .lock()
            .expect("lock")
            .query_row(
                "SELECT p.sku, u.unit_name_fr, u.unit_name_ar, u.factor_micro,
                    u.is_base_unit, u.is_default_selling
             FROM products p JOIN product_units u ON u.product_id = p.id
             WHERE p.name_fr = 'Coude PVC 32'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("unit row");
    assert_eq!(sku, "IMP-002");
    assert_eq!(unit_fr, "Pièce");
    assert_eq!(unit_ar, "قطعة");
    assert_eq!(factor, 10_000);
    assert_eq!(base, 1);
    assert_eq!(def, 1);
}

#[test]
fn import_batch_provisions_initial_stock_in_dep01() {
    let harness = setup();
    let mut first = row(Some("STK-001"), "Vis Placo 25", 15_000);
    first.initial_quantity_micro = Some(500_000);
    let report = run_batch(&harness, vec![first], ImportDuplicatePolicy::Skip);
    assert_eq!(report.imported_count, 1);
    let product_id: String = harness
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT id FROM products WHERE sku = 'STK-001'", [], |row| {
            row.get(0)
        })
        .expect("product id");
    let (warehouse_id, quantity): (String, i64) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT warehouse_id, quantity_base_micro FROM stock_levels WHERE product_id = ?1",
            rusqlite::params![product_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("stock level");
    assert_eq!(warehouse_id, DEP01_ID);
    assert_eq!(quantity, 500_000);
    let (movement_type, created_by, movement_quantity): (String, String, i64) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT movement_type, created_by, quantity_base_micro
             FROM stock_movements WHERE product_id = ?1",
            rusqlite::params![product_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("movement");
    assert_eq!(movement_type, "ADJUSTMENT_POSITIVE");
    assert_eq!(created_by, GERANT_UUID);
    assert_eq!(movement_quantity, 500_000);
}

#[test]
fn import_batch_skips_duplicate_skus_gracefully() {
    let harness = setup();
    seed_product(&harness, "DUP-001");
    let rows = vec![
        row(Some("NEW-001"), "Vis Placo 25", 15_000),
        row(Some("DUP-001"), "Doublon Robinet", 45_000),
        row(Some("NEW-002"), "Coude PVC 32", 8_500),
    ];
    let report = run_batch(&harness, rows, ImportDuplicatePolicy::Skip);
    assert_eq!(report.total_rows, 3);
    assert_eq!(report.imported_count, 2);
    assert_eq!(report.skipped_count, 1);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].row_index, 1);
    assert_eq!(report.errors[0].sku.as_deref(), Some("DUP-001"));
    assert!(report.errors[0].error.contains("skipped"));
    assert_eq!(count(&harness, "SELECT COUNT(*) FROM products"), 3);
}

#[test]
fn import_batch_auto_generates_missing_skus() {
    let harness = setup();
    let rows = vec![
        row(None, "Vis Placo 25", 15_000),
        row(Some(""), "Coude PVC 32", 8_500),
    ];
    let report = run_batch(&harness, rows, ImportDuplicatePolicy::Skip);
    assert_eq!(report.imported_count, 2);
    assert!(report.errors.is_empty());
    let skus: Vec<String> = harness
        .db
        .lock()
        .expect("lock")
        .prepare("SELECT sku FROM products ORDER BY sku")
        .expect("prepare")
        .query_map([], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("skus");
    assert_eq!(skus.len(), 2);
    for sku in &skus {
        assert!(!sku.is_empty());
        assert_eq!(*sku, sku.to_uppercase());
        assert!(sku.starts_with("PRD-"));
    }
    assert_ne!(skus[0], skus[1]);
}

#[test]
fn import_batch_rejects_negative_prices() {
    let harness = setup();
    let mut bad_price = row(Some("NEG-001"), "Vis Placo 25", -500);
    bad_price.purchase_price_centimes = None;
    let mut blank_name = row(Some("BLK-001"), "   ", 1_000);
    blank_name.name_fr = "   ".to_string();
    let report = run_batch(
        &harness,
        vec![
            bad_price,
            blank_name,
            row(Some("OK-001"), "Coude PVC 32", 8_500),
        ],
        ImportDuplicatePolicy::Skip,
    );
    assert_eq!(report.total_rows, 3);
    assert_eq!(report.imported_count, 1);
    assert_eq!(report.skipped_count, 0);
    assert_eq!(report.errors.len(), 2);
    assert_eq!(report.errors[0].row_index, 0);
    assert_eq!(report.errors[0].error, "NEGATIVE_SALE_PRICE");
    assert_eq!(report.errors[1].row_index, 1);
    assert_eq!(report.errors[1].error, "BLANK_NAME_FR");
    assert_eq!(count(&harness, "SELECT COUNT(*) FROM products"), 1);
}

#[test]
fn import_batch_error_policy_aborts_on_duplicate() {
    let harness = setup();
    seed_product(&harness, "DUP-001");
    let result = ImportProductsUseCase::new(&harness.repo).execute(
        &gerant(),
        ImportBatchInput {
            rows: vec![
                row(Some("NEW-001"), "Vis Placo 25", 15_000),
                row(Some("DUP-001"), "Doublon Robinet", 45_000),
            ],
            warehouse_code: None,
            on_duplicate: ImportDuplicatePolicy::Error,
            origin_device_id: TEST_TERMINAL.to_string(),
        },
    );
    let error = result.expect_err("duplicate aborts");
    assert!(error.to_string().contains("DUP-001"));
    // Full rollback: not even the valid first row landed.
    assert_eq!(count(&harness, "SELECT COUNT(*) FROM products"), 1);
}

#[test]
fn import_requires_manage_permission() {
    let harness = setup();
    let vendeur = SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(GERANT_UUID).expect("uuid")),
        role: Role::Vendeur,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    };
    let result = ImportProductsUseCase::new(&harness.repo).execute(
        &vendeur,
        ImportBatchInput {
            rows: vec![row(Some("X-001"), "Vis", 1_000)],
            warehouse_code: None,
            on_duplicate: ImportDuplicatePolicy::Skip,
            origin_device_id: TEST_TERMINAL.to_string(),
        },
    );
    assert!(matches!(
        result,
        Err(baraka_application::errors::ApplicationError::Unauthorized)
    ));
}
