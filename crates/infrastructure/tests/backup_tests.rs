//! SQLite backup engine integration tests (TASK-BRK-051, ADR-010).
//!
//! File-backed temporary databases: every test opens a live database,
//! seeds representative rows, snapshots it through the online backup API,
//! and asserts consistency, tamper detection, and rotation — all against
//! real files on disk.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{BackupRepository, NewCustomer, SessionContext, SessionToken};
use baraka_application::use_cases::backup::{
    CreateBackupUseCase, ListBackupsUseCase, VerifyBackupIntegrityUseCase,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::CreditPolicy;
use baraka_domain::sales::CustomerTier;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteBackupManager, SqliteCatalogRepository, SqliteCustomerRepository,
    SqliteDocumentRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

use baraka_application::ports::{
    CustomerRepository, DocumentRepository, ProductRepository, WarehouseRepository,
};

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const TEST_TERMINAL: &str = "TEST-TERMINAL-BKP";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    backups: SqliteBackupManager,
    catalog: SqliteCatalogRepository,
    warehouses: SqliteWarehouseRepository,
    documents: SqliteDocumentRepository,
    customers: SqliteCustomerRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let live_path = dir.path().join("live.db");
    let mut conn = open_connection(&live_path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('77777777-7777-7777-7777-777777777777', 'gerant1', 'hash', 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        backups: SqliteBackupManager::new(Arc::clone(&shared), live_path),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
    }
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn backup_dir(harness: &Harness) -> std::path::PathBuf {
    harness._dir.path().join("backups")
}

/// Seed one warehouse, one product with stock, and one customer with a
/// ledger DEBIT row — representative retail state.
fn seed_retail(harness: &Harness) -> (WarehouseId, ProductId, UnitId) {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("MAIN").expect("code"),
        name: "Main".to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, TEST_TERMINAL)
        .expect("warehouse");
    let product_id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id: product_id,
                sku: Sku::new("BK1").expect("sku"),
                barcode: None,
                name_ar: "منتج".to_string(),
                name_fr: "Produit".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: unit_id,
                product_id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: Some(Barcode::new("611125BK1001").expect("barcode")),
                sale_price: Money::from_centimes(10_000),
                purchase_price: Money::from_centimes(8_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("product");
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO stock_levels (warehouse_id, product_id, quantity_micro, quantity_base_micro)
             VALUES (?1, ?2, 100000, 100000)",
            rusqlite::params![warehouse.id.to_string(), product_id.to_string()],
        )
        .expect("stock level");
    let customer = CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Gerant),
            NewCustomer {
                name: "Débiteur".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
                customer_tier: CustomerTier::Retail,
            },
            TEST_TERMINAL,
        )
        .expect("customer");
    harness
        .customers
        .record_credit_debit(
            customer,
            20_000,
            20_000,
            baraka_domain::sales::DocumentId::new(),
            UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        )
        .expect("debit");
    (warehouse.id, product_id, unit_id)
}

/// Seed one finalized sale document with a single line (for orphan tests).
fn seed_document_with_line(
    harness: &Harness,
    warehouse: WarehouseId,
    product: ProductId,
    unit: UnitId,
) -> uuid::Uuid {
    use baraka_domain::sales::{Document, DocumentId, DocumentItem, DocumentLanguage};
    use baraka_domain::sales::{DocumentStatus, DocumentType};

    let id = DocumentId::new();
    let line_id = Uuid::new_v4();
    harness
        .documents
        .finalize_document(
            &Document {
                id,
                language: DocumentLanguage::Fr,
                number: "TICK-2026-000001".to_string(),
                fiscal_year: 2026,
                doc_type: DocumentType::Ticket,
                status: DocumentStatus::Finalized,
                warehouse_id: warehouse,
                party_id: None,
                subtotal_ht_centimes: 10_000,
                vat_total_centimes: 0,
                global_discount_centimes: 0,
                timbre_centimes: 0,
                ttc_centimes: 10_000,
                paid_amount_centimes: 10_000,
                change_amount_centimes: 0,
                payment_method: "CASH".to_string(),
                notes: None,
                created_at: chrono::Utc::now(),
                created_by: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
            },
            &[DocumentItem {
                id: line_id,
                document_id: id,
                product_id: product,
                unit_id: unit,
                quantity_micro: 10_000,
                quantity_base_micro: 10_000,
                unit_price_centimes: 10_000,
                discount_centimes: 0,
                discount_per_unit_centimes: None,
                tva_rate_micro: 0,
                line_ht_centimes: 10_000,
                line_tva_centimes: 0,
                line_ttc_centimes: 10_000,
                cost_price_centimes: 8_000,
                sort_order: 0,
            }],
            &[],
            TEST_TERMINAL,
        )
        .expect("seed document");
    line_id
}

fn row_count(conn: &rusqlite::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .expect("count")
}

fn create(harness: &Harness, retention: usize) -> baraka_domain::backup::BackupMetadata {
    CreateBackupUseCase::new(&harness.backups)
        .execute(
            &session_with(Role::Gerant),
            baraka_domain::backup::BackupTriggerType::Manual,
            &backup_dir(harness),
            retention,
            TEST_TERMINAL,
            APP_VERSION,
        )
        .expect("backup")
}

fn is_code(error: &ApplicationError, code: &str) -> bool {
    matches!(
        error,
        ApplicationError::Validation(DomainError::InvalidCanonicalString(actual))
            if actual == code
    )
}

#[test]
fn live_backup_creates_verified_logged_snapshot() {
    let harness = setup();
    seed_retail(&harness);
    let meta = create(&harness, 30);
    assert_eq!(meta.status, baraka_domain::backup::BackupStatus::Success);
    assert!(meta.file_path.ends_with(&meta.file_name));
    assert!(meta.file_name.starts_with("baraka_pos_backup_"));
    assert_eq!(
        meta.file_size_bytes,
        Some(std::fs::metadata(&meta.file_path).expect("stat").len())
    );
    assert_eq!(meta.sha256_checksum.len(), 64);
    assert!(meta.error_message.is_none());
    assert_eq!(
        meta.schema_version as usize,
        baraka_infrastructure::persistence::migration::MIGRATIONS.len()
    );
    assert!(std::path::Path::new(&meta.file_path).is_file());
    // Audit row mirrors the metadata; the list use case rebuilds it.
    let listed = ListBackupsUseCase::new(&harness.backups)
        .execute(&session_with(Role::Gerant), &backup_dir(&harness), 10)
        .expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].file_name, meta.file_name);
    assert_eq!(listed[0].id.to_string(), meta.id.to_string());
    let rows = harness.backups.list_backups(10).expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].backup_id, meta.file_name);
    assert_eq!(rows[0].database_hash, meta.sha256_checksum);
    assert_eq!(rows[0].status, "SUCCESS");
    assert_eq!(rows[0].source_device_id, TEST_TERMINAL);
}

#[test]
fn backup_opens_independently_with_identical_row_counts() {
    let harness = setup();
    seed_retail(&harness);
    let meta = create(&harness, 30);
    let snapshot = rusqlite::Connection::open(&meta.file_path).expect("open snapshot");
    let live = harness.db.lock().expect("lock");
    for table in [
        "users",
        "warehouses",
        "products",
        "product_units",
        "stock_levels",
        "customers",
        "customer_ledger",
        "documents",
    ] {
        assert_eq!(
            row_count(&snapshot, table),
            row_count(&live, table),
            "row drift in {table}"
        );
    }
}

#[test]
fn sha256_matches_exact_file_contents() {
    use baraka_infrastructure::persistence::sha256::sha256_hex;

    let harness = setup();
    seed_retail(&harness);
    let meta = create(&harness, 30);
    let bytes = std::fs::read(&meta.file_path).expect("read snapshot");
    assert_eq!(sha256_hex(&bytes), meta.sha256_checksum);
    assert_eq!(
        harness
            .backups
            .sha256_of(std::path::Path::new(&meta.file_path))
            .expect("hash"),
        meta.sha256_checksum
    );
}

#[test]
fn retention_prunes_oldest_snapshots_and_marks_rows() {
    let harness = setup();
    seed_retail(&harness);
    let dir = backup_dir(&harness);
    let mut names = Vec::new();
    for _ in 0..4 {
        names.push(create(&harness, 2).file_name);
    }
    assert_eq!(names.len(), 4);
    // Exactly the two newest SUCCESS rows survive on disk and in state.
    let rows = harness.backups.list_backups(10).expect("list");
    let live: Vec<_> = rows.iter().filter(|row| row.status == "SUCCESS").collect();
    let pruned: Vec<_> = rows.iter().filter(|row| row.status == "PRUNED").collect();
    assert_eq!(live.len(), 2);
    assert_eq!(pruned.len(), 2);
    for row in &live {
        assert!(dir.join(&row.file_name).is_file());
    }
    for row in &pruned {
        assert!(!dir.join(&row.file_name).exists());
    }
}

#[test]
fn tampered_snapshot_fails_checksum_verification() {
    let harness = setup();
    seed_retail(&harness);
    let meta = create(&harness, 30);
    // Flip one byte deep inside the file.
    let mut bytes = std::fs::read(&meta.file_path).expect("read");
    let pivot = bytes.len() / 2;
    bytes[pivot] ^= 0xFF;
    std::fs::write(&meta.file_path, &bytes).expect("tamper");
    let error = VerifyBackupIntegrityUseCase::new(&harness.backups)
        .execute(
            &session_with(Role::Gerant),
            &backup_dir(&harness),
            &meta.file_name,
        )
        .expect_err("tamper must fail");
    assert!(
        is_code(&error, "BACKUP_CHECKSUM_MISMATCH"),
        "unexpected: {error:?}"
    );
}

#[test]
fn foreign_key_violations_mark_backup_corrupted() {
    let harness = setup();
    let (warehouse, product, unit) = seed_retail(&harness);
    seed_document_with_line(&harness, warehouse, product, unit);
    let meta = create(&harness, 30);
    // Break referential integrity inside the COPY: delete a product unit
    // still referenced by a document line (FK enforcement off), then
    // re-point the log hash at the doctored bytes so verification reaches
    // the pragma stage instead of the checksum stage. Opened via the
    // sanctioned opener so v16 FTS triggers (baraka_search_norm) can fire.
    let conn = open_connection(std::path::Path::new(&meta.file_path)).expect("open");
    conn.execute_batch("PRAGMA foreign_keys=OFF;")
        .expect("pragma");
    conn.execute(
        "DELETE FROM product_units WHERE id = (SELECT unit_id FROM document_items LIMIT 1)",
        [],
    )
    .expect("orphan");
    drop(conn);
    let fresh_hash = harness
        .backups
        .sha256_of(std::path::Path::new(&meta.file_path))
        .expect("hash");
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "UPDATE backups_log SET database_hash = ?1 WHERE backup_id = ?2",
            rusqlite::params![fresh_hash, meta.file_name],
        )
        .expect("repoint hash");
    let error = VerifyBackupIntegrityUseCase::new(&harness.backups)
        .execute(
            &session_with(Role::Gerant),
            &backup_dir(&harness),
            &meta.file_name,
        )
        .expect_err("FK violations must fail");
    assert!(is_code(&error, "BACKUP_CORRUPTED"), "unexpected: {error:?}");
    let row = harness
        .backups
        .get_backup(&meta.file_name)
        .expect("get")
        .expect("row");
    assert_eq!(row.status, "CORRUPTED");
}

/// TASK-BRK-094: every capture path records its true size + trigger.
#[test]
fn create_records_real_size_and_trigger_per_path() {
    use baraka_domain::backup::BackupTriggerType;
    let harness = setup();
    let dir = backup_dir(&harness);
    let cases = [
        (BackupTriggerType::Manual, "manual"),
        (BackupTriggerType::ShiftClose, "cash_close"),
        (BackupTriggerType::Scheduled, "scheduled"),
        (BackupTriggerType::Reset, "reset"),
    ];
    for (trigger, code) in cases {
        let meta = CreateBackupUseCase::new(&harness.backups)
            .execute(
                &session_with(Role::Gerant),
                trigger,
                &dir,
                30,
                TEST_TERMINAL,
                APP_VERSION,
            )
            .expect("backup");
        let actual = std::fs::metadata(&meta.file_path).expect("stat").len();
        assert_eq!(meta.file_size_bytes, Some(actual));
        let row = harness
            .backups
            .get_backup(&meta.file_name)
            .expect("get")
            .expect("row");
        assert_eq!(row.file_size_bytes, Some(actual));
        assert_eq!(row.trigger, code);
        let listed = ListBackupsUseCase::new(&harness.backups)
            .execute(&session_with(Role::Gerant), &dir, 10)
            .expect("list");
        let found = listed
            .iter()
            .find(|item| item.file_name == meta.file_name)
            .expect("listed");
        assert_eq!(found.file_size_bytes, Some(actual));
        assert_eq!(found.trigger_type, trigger);
    }
}

/// Legacy rows (NULL size/trigger) surface as unknown, defaulting to manual.
/// UI contract: `None` renders as "—", never 0.
#[test]
fn legacy_null_row_surfaces_unknown_size_and_manual_trigger() {
    let harness = setup();
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO backups_log
             (id, backup_id, app_version, schema_version, created_at,
              source_device_id, database_hash, cipher_version, status,
              file_size_bytes, trigger)
             VALUES ('00000000-0000-7000-8000-000000000099',
                     'baraka_pos_backup_legacy.db', '1.0.0', 16,
                     '2026-01-01T00:00:00.000Z', 'OLD-TERM',
                     'abc123', 'NONE', 'SUCCESS', NULL, NULL)",
            [],
        )
        .expect("legacy row");
    let row = harness
        .backups
        .get_backup("baraka_pos_backup_legacy.db")
        .expect("get")
        .expect("row");
    assert_eq!(row.file_size_bytes, None);
    let listed = ListBackupsUseCase::new(&harness.backups)
        .execute(&session_with(Role::Gerant), &backup_dir(&harness), 10)
        .expect("list");
    let found = listed
        .iter()
        .find(|item| item.file_name == "baraka_pos_backup_legacy.db")
        .expect("listed");
    assert_eq!(found.file_size_bytes, None);
    assert_eq!(
        found.trigger_type,
        baraka_domain::backup::BackupTriggerType::Manual
    );
}

/// Verify re-stats and repairs a drifted stored size.
#[test]
fn verify_corrects_drifted_stored_size() {
    let harness = setup();
    let meta = create(&harness, 30);
    let actual = std::fs::metadata(&meta.file_path).expect("stat").len();
    harness
        .backups
        .correct_file_size(&meta.file_name, actual + 12_345)
        .expect("drift");
    let drifted = harness
        .backups
        .get_backup(&meta.file_name)
        .expect("get")
        .expect("row");
    assert_eq!(drifted.file_size_bytes, Some(actual + 12_345));
    VerifyBackupIntegrityUseCase::new(&harness.backups)
        .execute(
            &session_with(Role::Gerant),
            &backup_dir(&harness),
            &meta.file_name,
        )
        .expect("verify");
    let repaired = harness
        .backups
        .get_backup(&meta.file_name)
        .expect("get")
        .expect("row");
    assert_eq!(repaired.file_size_bytes, Some(actual));
}

/// Migration 0017 replay: fresh databases gain both columns.
#[test]
fn migration_0017_adds_size_and_trigger_columns() {
    let harness = setup();
    let guard = harness.db.lock().expect("lock");
    let mut names: Vec<String> = guard
        .prepare("PRAGMA table_info(backups_log)")
        .expect("pragma")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("names");
    names.sort();
    assert!(names.contains(&"file_size_bytes".to_string()));
    assert!(names.contains(&"trigger".to_string()));
}
