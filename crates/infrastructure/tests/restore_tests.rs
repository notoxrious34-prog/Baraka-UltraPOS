//! Verified database restore integration tests (TASK-BRK-054, ADR-062).
//!
//! File-backed temporary databases: every test opens a live database,
//! seeds consistent retail state, snapshots it through the online backup
//! API, then exercises the read-only precheck report and the destructive
//! restore protocol — quarantine preservation, audit continuity, rollback
//! safety, and older-schema migration — all against real files on disk.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    BackupRepository, CustomerRepository, DocumentRepository, NewCustomer, ProductRepository,
    SessionContext, SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::backup::{
    CreateBackupUseCase, PrecheckRestoreUseCase, RestoreBackupUseCase,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_domain::backup::{LedgerScope, RestoreError};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
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
use baraka_infrastructure::persistence::migration::{run_migrations, MIGRATIONS};
use baraka_infrastructure::persistence::sha256::sha256_hex;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const TEST_TERMINAL: &str = "TEST-TERMINAL-RST";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct Harness {
    _dir: tempfile::TempDir,
    live_path: std::path::PathBuf,
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
         VALUES ('77777777-7777-7777-7777-777777777777', 'admin1', 'hash', 'ADMIN',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        live_path: live_path.clone(),
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

/// Seed consistent retail state: one warehouse, one product with a matching
/// purchase movement (level equals the signed movement sum), one customer
/// whose cached balance equals its ledger, and one finalized document.
fn seed_consistent(harness: &Harness, doc_number: &str) -> (WarehouseId, ProductId, UnitId) {
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
                sku: Sku::new("RS1").expect("sku"),
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
                barcode: Some(Barcode::new("611125RS1001").expect("barcode")),
                sale_price: Money::from_centimes(10_000),
                purchase_price: Money::from_centimes(8_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("product");
    {
        let guard = harness.db.lock().expect("lock");
        // Level matches the signed movement sum below (PURCHASE_IN +100000).
        guard
            .execute(
                "INSERT INTO stock_levels (warehouse_id, product_id, quantity_micro, quantity_base_micro)
                 VALUES (?1, ?2, 100000, 100000)",
                rusqlite::params![warehouse.id.to_string(), product_id.to_string()],
            )
            .expect("stock level");
        guard
            .execute(
                "INSERT INTO stock_movements
                 (id, sequence_no, warehouse_id, product_id, movement_type, unit_id,
                  quantity_micro, quantity_base_micro, created_by, created_at)
                 VALUES (?1, 1, ?2, ?3, 'PURCHASE_IN', ?4, 100000, 100000, ?5,
                         '2026-09-20T08:00:00.000Z')",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    warehouse.id.to_string(),
                    product_id.to_string(),
                    unit_id.to_string(),
                    ACTOR_UUID,
                ],
            )
            .expect("stock movement");
    }
    let customer = CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Admin),
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
    // CASE 2 (directive): `record_credit_debit` is production code
    // (sqlite_customer_repository.rs) and writes the ledger row only; the
    // established per-call pattern (customer_payment.rs) pairs it with a
    // cached-balance update so the invariant holds after every write.
    harness
        .customers
        .update_cached_balance(customer, 20_000)
        .expect("cached balance");
    assert_customer_ledger_invariant(harness);
    seed_document(harness, warehouse.id, product_id, unit_id, doc_number);
    (warehouse.id, product_id, unit_id)
}

/// Recompute every cached customer balance from its signed ledger sum and
/// panic at the seed line (naming the customer and both values) when they
/// diverge — a seeding error must never surface later as a confusing
/// precheck failure.
fn assert_customer_ledger_invariant(harness: &Harness) {
    let guard = harness.db.lock().expect("lock");
    let mut statement = guard
        .prepare("SELECT id, current_balance_centimes FROM customers")
        .expect("prepare customers");
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .expect("query customers");
    for row in rows {
        let (id, cached) = row.expect("customer row");
        let recomputed: i64 = guard
            .query_row(
                "SELECT COALESCE(SUM(CASE WHEN entry_type = 'DEBIT' THEN amount_centimes
                                         WHEN entry_type = 'CREDIT' THEN -amount_centimes
                                         ELSE 0 END), 0)
                 FROM customer_ledger WHERE customer_id = ?1",
                [&id],
                |ledger_row| ledger_row.get(0),
            )
            .expect("ledger sum");
        assert!(
            cached == recomputed,
            "seed invariant broken for customer {id}: cached {cached} != ledger sum {recomputed}"
        );
    }
}

/// Seed one finalized sale document with a single line.
fn seed_document(
    harness: &Harness,
    warehouse: WarehouseId,
    product: ProductId,
    unit: UnitId,
    number: &str,
) {
    use baraka_domain::sales::{Document, DocumentId, DocumentItem, DocumentLanguage};
    use baraka_domain::sales::{DocumentStatus, DocumentType};

    let id = DocumentId::new();
    harness
        .documents
        .finalize_document(
            &Document {
                id,
                language: DocumentLanguage::Fr,
                number: number.to_string(),
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
                id: Uuid::new_v4(),
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
}

fn create(harness: &Harness) -> baraka_domain::backup::BackupMetadata {
    CreateBackupUseCase::new(&harness.backups)
        .execute(
            &session_with(Role::Admin),
            baraka_domain::backup::BackupTriggerType::Manual,
            &backup_dir(harness),
            30,
            TEST_TERMINAL,
            APP_VERSION,
        )
        .expect("backup")
}

fn precheck(
    harness: &Harness,
    backup_id: &str,
) -> Result<baraka_domain::backup::PreRestoreReport, RestoreError> {
    PrecheckRestoreUseCase::new(&harness.backups).execute(
        &session_with(Role::Admin),
        &backup_dir(harness),
        backup_id,
    )
}

fn restore(
    harness: &Harness,
    backup_id: &str,
) -> Result<baraka_domain::backup::RestoreOutcome, RestoreError> {
    RestoreBackupUseCase::new(&harness.backups).execute(
        &session_with(Role::Admin),
        &backup_dir(harness),
        backup_id,
    )
}

/// Re-point the logged hash at the current file bytes (proves a deeper
/// gate fires independently of the SHA-256 binding).
fn repoint_logged_hash(harness: &Harness, file_name: &str, path: &std::path::Path) {
    let fresh = harness.backups.sha256_of(path).expect("hash");
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "UPDATE backups_log SET database_hash = ?1 WHERE backup_id = ?2",
            rusqlite::params![fresh, file_name],
        )
        .expect("repoint hash");
}

fn live_scalar(harness: &Harness, sql: &str) -> i64 {
    harness
        .db
        .lock()
        .expect("lock")
        .query_row(sql, [], |row| row.get(0))
        .expect("scalar")
}

/// Recompute the full audit hash chain of an open connection (test-side
/// validator mirroring the append rule; the production validator lives in
/// the backup manager and is exercised through restore outcomes).
fn audit_chain_valid(conn: &rusqlite::Connection) -> bool {
    let mut statement = conn
        .prepare(
            "SELECT prev_hash, record_hash, timestamp, user_id, action,
                    entity_type, entity_id, new_value_json
             FROM audit_log ORDER BY sequence_no ASC;",
        )
        .expect("prepare");
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .expect("query");
    let mut expected = "GENESIS".to_string();
    for row in rows {
        let (prev, hash, stamp, actor, action, entity, entity_id, new_json) = row.expect("row");
        if prev != expected {
            return false;
        }
        let canonical = format!(
            "{prev}|{stamp}|{actor}|{action}|{entity}|{entity_id}|{}",
            new_json.unwrap_or_default()
        );
        if sha256_hex(canonical.as_bytes()) != hash {
            return false;
        }
        expected = hash;
    }
    true
}

#[test]
fn restore_use_cases_reject_non_admin_sessions() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let gerant = session_with(Role::Gerant);
    let precheck_error = PrecheckRestoreUseCase::new(&harness.backups)
        .execute(&gerant, &backup_dir(&harness), &meta.file_name)
        .expect_err("gerant precheck must fail");
    assert_eq!(precheck_error, RestoreError::Unauthorized);
    let restore_error = RestoreBackupUseCase::new(&harness.backups)
        .execute(&gerant, &backup_dir(&harness), &meta.file_name)
        .expect_err("gerant restore must fail");
    assert_eq!(restore_error, RestoreError::Unauthorized);
}

#[test]
fn precheck_returns_eligible_report_for_healthy_backup() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert_eq!(report.backup_id, meta.file_name);
    assert_eq!(report.file_name, meta.file_name);
    assert_eq!(report.file_size_bytes, meta.file_size_bytes.expect("size"));
    assert!(report.checks.file_exists);
    assert!(report.checks.sha256_matches);
    assert!(report.checks.integrity_ok);
    assert!(report.checks.foreign_keys_ok);
    assert!(report.checks.audit_chain_ok);
    assert!(report.checks.customer_ledger_ok);
    assert!(report.checks.supplier_ledger_ok);
    assert!(report.checks.stock_levels_ok);
    assert!(report.ineligible_reasons.is_empty());
    assert!(report.eligible);
    assert_eq!(report.content_summary.products, 1);
    assert_eq!(report.content_summary.customers, 1);
    assert_eq!(report.content_summary.documents_total, 1);
    assert!(report.content_summary.last_document_at.is_some());
    assert_eq!(report.schema_version_app as usize, MIGRATIONS.len());
    assert_eq!(report.schema_version_backup, report.schema_version_app);
}

#[test]
fn precheck_flags_hash_mismatch_on_tampered_file() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let path = backup_dir(&harness).join(&meta.file_name);
    let mut bytes = std::fs::read(&path).expect("read");
    let pivot = bytes.len() / 2;
    bytes[pivot] ^= 0xFF;
    std::fs::write(&path, &bytes).expect("tamper");
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(!report.eligible);
    assert!(!report.checks.sha256_matches);
    assert!(report
        .ineligible_reasons
        .contains(&baraka_domain::backup::RestoreIneligibleReason::HashMismatch));
}

#[test]
fn precheck_flags_integrity_failure_on_corrupted_file_with_updated_hash() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let path = backup_dir(&harness).join(&meta.file_name);
    // Corrupt second-page bytes, then re-point the logged hash so the
    // deeper physical gate must fire independently of the SHA-256 binding.
    let mut bytes = std::fs::read(&path).expect("read");
    assert!(bytes.len() > 8192, "snapshot must span pages");
    for byte in bytes.iter_mut().skip(4096).take(64) {
        *byte = 0xFF;
    }
    std::fs::write(&path, &bytes).expect("corrupt");
    repoint_logged_hash(&harness, &meta.file_name, &path);
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(!report.eligible);
    assert!(report.checks.sha256_matches);
    assert!(!report.checks.integrity_ok);
    assert!(report
        .ineligible_reasons
        .contains(&baraka_domain::backup::RestoreIneligibleReason::IntegrityCheckFailed));
}

#[test]
fn precheck_flags_schema_newer_than_app() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let path = backup_dir(&harness).join(&meta.file_name);
    let app_version = MIGRATIONS.len() as i64;
    {
        let conn = rusqlite::Connection::open(&path).expect("open backup");
        conn.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, '0099_future_schema.sql', 'future', '2026-09-30T00:00:00.000Z')",
            [app_version + 1],
        )
        .expect("future version row");
    }
    repoint_logged_hash(&harness, &meta.file_name, &path);
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(!report.eligible);
    assert!(report.ineligible_reasons.contains(
        &baraka_domain::backup::RestoreIneligibleReason::SchemaNewerThanApp {
            backup_version: (app_version + 1) as u32,
            app_version: app_version as u32,
        }
    ));
    let restore_error = restore(&harness, &meta.file_name).expect_err("newer schema must fail");
    assert_eq!(
        restore_error,
        RestoreError::BackupSchemaNewer {
            backup_version: (app_version + 1) as u32,
            app_version: app_version as u32,
        }
    );
}

#[test]
fn precheck_reports_customer_ledger_mismatch() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let path = backup_dir(&harness).join(&meta.file_name);
    {
        let conn = rusqlite::Connection::open(&path).expect("open backup");
        conn.execute(
            "UPDATE customers SET current_balance_centimes = current_balance_centimes + 1",
            [],
        )
        .expect("drift balance");
    }
    repoint_logged_hash(&harness, &meta.file_name, &path);
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(!report.eligible);
    assert!(!report.checks.customer_ledger_ok);
    assert!(report.ineligible_reasons.contains(
        &baraka_domain::backup::RestoreIneligibleReason::CustomerLedgerMismatch {
            mismatch_count: 1,
        }
    ));
    let restore_error = restore(&harness, &meta.file_name).expect_err("ledger drift must fail");
    assert_eq!(
        restore_error,
        RestoreError::BackupLedgerInconsistent {
            scope: LedgerScope::Customer,
            mismatch_count: 1,
        }
    );
}

#[test]
fn precheck_warns_for_open_cash_session_and_newer_live_documents() {
    let harness = setup();
    let (warehouse, product, unit) = seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    // Open session recorded inside the BACKUP file only.
    let opened_at = "2026-09-29T07:30:00.000Z";
    {
        let conn = rusqlite::Connection::open(backup_dir(&harness).join(&meta.file_name))
            .expect("open backup");
        conn.execute(
            "INSERT INTO cash_sessions
             (id, warehouse_id, terminal_id, status, opened_by, opened_at,
              opening_float_centimes, origin_device_id)
             VALUES (?1, ?2, ?3, 'OPEN', ?4, ?5, 0, ?3)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                warehouse.to_string(),
                TEST_TERMINAL,
                ACTOR_UUID,
                opened_at,
            ],
        )
        .expect("open session in backup");
    }
    repoint_logged_hash(
        &harness,
        &meta.file_name,
        &backup_dir(&harness).join(&meta.file_name),
    );
    // A newer live document lands after the backup instant.
    seed_document(&harness, warehouse, product, unit, "TICK-2026-000002");
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(report.eligible);
    assert!(report.warnings.contains(
        &baraka_domain::backup::RestoreWarning::OpenCashSessionInBackup {
            opened_at: opened_at.to_string(),
        }
    ));
    assert!(report.warnings.contains(
        &baraka_domain::backup::RestoreWarning::NewerActivityWillBeQuarantined {
            document_count: 1,
        }
    ));
}

#[test]
fn restore_success_swaps_active_db_and_preserves_quarantine() {
    let harness = setup();
    let (warehouse, product, unit) = seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let backup_bytes_before =
        std::fs::read(backup_dir(&harness).join(&meta.file_name)).expect("read");
    // Mutate the live database after the backup: new product + new document.
    let extra_id = ProductId::new();
    let extra_unit = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id: extra_id,
                sku: Sku::new("RS2").expect("sku"),
                barcode: None,
                name_ar: "منتج2".to_string(),
                name_fr: "Produit2".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: extra_unit,
                product_id: extra_id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: None,
                sale_price: Money::from_centimes(5_000),
                purchase_price: Money::from_centimes(4_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("extra product");
    seed_document(&harness, warehouse, product, unit, "TICK-2026-000002");
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM products"), 2);
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM documents"), 2);

    let outcome = restore(&harness, &meta.file_name).expect("restore");
    assert_eq!(outcome.restored_backup_id, meta.file_name);
    assert!(outcome.quarantined_to.starts_with("active-"));
    assert!(outcome.quarantined_to.ends_with(".db"));
    assert!(outcome.restart_required);
    // The mutation is absent from the active database ...
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM products"), 1);
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM documents"), 1);
    // ... present when opening the quarantined file read-only ...
    let quarantined_path = backup_dir(&harness)
        .join("quarantine")
        .join(&outcome.quarantined_to);
    assert!(quarantined_path.is_file());
    let quarantined = rusqlite::Connection::open_with_flags(
        &quarantined_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open quarantine");
    let quarantined_products: i64 = quarantined
        .query_row("SELECT COUNT(*) FROM products;", [], |row| row.get(0))
        .expect("count");
    let quarantined_documents: i64 = quarantined
        .query_row("SELECT COUNT(*) FROM documents;", [], |row| row.get(0))
        .expect("count");
    assert_eq!(quarantined_products, 2);
    assert_eq!(quarantined_documents, 2);
    drop(quarantined);
    // ... and the source backup file is byte-identical.
    let backup_bytes_after =
        std::fs::read(backup_dir(&harness).join(&meta.file_name)).expect("read");
    assert_eq!(backup_bytes_before, backup_bytes_after);
}

#[test]
fn restore_appends_audit_event_and_extends_chain() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let outcome = restore(&harness, &meta.file_name).expect("restore");
    let guard = harness.db.lock().expect("lock");
    let tail: String = guard
        .query_row(
            "SELECT action FROM audit_log ORDER BY sequence_no DESC LIMIT 1;",
            [],
            |row| row.get(0),
        )
        .expect("tail");
    assert_eq!(tail, "RESTORE_PERFORMED");
    let payload: String = guard
        .query_row(
            "SELECT new_value_json FROM audit_log ORDER BY sequence_no DESC LIMIT 1;",
            [],
            |row| row.get(0),
        )
        .expect("payload");
    assert!(payload.contains(&meta.file_name));
    assert!(payload.contains(&outcome.quarantined_to));
    assert!(audit_chain_valid(&guard));
}

#[test]
fn restore_rejects_tampered_backup_and_leaves_active_untouched() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let path = backup_dir(&harness).join(&meta.file_name);
    let mut bytes = std::fs::read(&path).expect("read");
    let pivot = bytes.len() / 2;
    bytes[pivot] ^= 0xFF;
    std::fs::write(&path, &bytes).expect("tamper");
    let live_before = std::fs::read(&harness.live_path).expect("read live");
    let restore_error = restore(&harness, &meta.file_name).expect_err("tamper must fail");
    assert_eq!(restore_error, RestoreError::BackupTampered);
    // Active database byte-untouched and no quarantine entry exists.
    let live_after = std::fs::read(&harness.live_path).expect("read live");
    assert_eq!(live_before, live_after);
    assert!(!backup_dir(&harness).join("quarantine").exists());
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM products"), 1);
}

#[test]
fn restore_older_schema_backup_is_migrated() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let app_version = MIGRATIONS.len() as i64;
    assert!(app_version >= 2, "migration replay needs a prior version");
    // Mark the backup one version behind through the migration runner's own
    // bookkeeping table, then re-bind the logged hash to the edited bytes.
    // The columns added by the dropped version must go too: replaying an
    // ADD COLUMN migration over a backup that still carries the column
    // fails with "duplicate column name", so the backup is returned to a
    // genuine prior-version shape before the hash is re-pointed.
    {
        let conn = rusqlite::Connection::open(backup_dir(&harness).join(&meta.file_name))
            .expect("open backup");
        conn.execute(
            "DELETE FROM schema_migrations WHERE version = ?1;",
            [app_version],
        )
        .expect("drop latest version row");
        if app_version >= 15 {
            conn.execute("ALTER TABLE customers DROP COLUMN customer_tier;", [])
                .expect("drop v15 customer column");
            conn.execute(
                "ALTER TABLE product_units DROP COLUMN wholesale_price_centimes;",
                [],
            )
            .expect("drop v15 unit column");
        }
        if app_version >= 16 {
            // Genuine v15 backups predate migration 0016, so all of its
            // artifacts must go: DROP TRIGGER first (DROP COLUMN errors
            // while a trigger references the table), then the FTS tables,
            // then the norm columns. The unconditional DROPs mirror the v15
            // block above: the harness DB is built at the current version,
            // so `app_version >= 16` already implies every artifact below is
            // present; a genuine older backup would never carry them.
            // Trigger names verified live in 0016 (8 total).
            for trigger in [
                "trg_products_fts_ai",
                "trg_products_fts_ad",
                "trg_products_fts_au",
                "trg_units_fts_ai",
                "trg_units_fts_au",
                "trg_units_fts_ad",
                "trg_aliases_fts_ai",
                "trg_aliases_fts_ad",
            ] {
                conn.execute(&format!("DROP TRIGGER IF EXISTS {trigger};"), [])
                    .expect("drop v16 fts trigger");
            }
            conn.execute("DROP TABLE IF EXISTS products_fts;", [])
                .expect("drop v16 fts table");
            conn.execute("DROP TABLE IF EXISTS product_aliases;", [])
                .expect("drop v16 aliases table");
            conn.execute("ALTER TABLE products DROP COLUMN search_norm_fr;", [])
                .expect("drop v16 norm column fr");
            conn.execute("ALTER TABLE products DROP COLUMN search_norm_ar;", [])
                .expect("drop v16 norm column ar");
        }
        if app_version >= 17 {
            // Genuine v16 backups predate migration 0017, so its two
            // NULL-able `backups_log` columns must go, or replaying the
            // ADD COLUMN migration fails with "duplicate column name".
            conn.execute("ALTER TABLE backups_log DROP COLUMN file_size_bytes;", [])
                .expect("drop v17 size column");
            conn.execute("ALTER TABLE backups_log DROP COLUMN trigger;", [])
                .expect("drop v17 trigger column");
        }
        if app_version >= 18 {
            // Genuine v17 backups predate migration 0018, so the NULL-able
            // `document_items.discount_per_unit_centimes` column must go,
            // or replaying the ADD COLUMN migration fails the same way.
            conn.execute(
                "ALTER TABLE document_items DROP COLUMN discount_per_unit_centimes;",
                [],
            )
            .expect("drop v18 per-unit column");
        }
    }
    repoint_logged_hash(
        &harness,
        &meta.file_name,
        &backup_dir(&harness).join(&meta.file_name),
    );
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(report.eligible);
    assert!(report.warnings.contains(
        &baraka_domain::backup::RestoreWarning::OlderSchemaWillMigrate {
            from_version: (app_version - 1) as u32,
            to_version: app_version as u32,
        }
    ));
    restore(&harness, &meta.file_name).expect("restore");
    let final_version: i64 = harness
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT MAX(version) FROM schema_migrations;", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert_eq!(final_version, app_version);
    let integrity: String = harness
        .db
        .lock()
        .expect("lock")
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .expect("integrity");
    assert_eq!(integrity, "ok");
}

#[test]
fn restore_rollback_on_swap_failure_preserves_active_db() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let backup_path = backup_dir(&harness).join(&meta.file_name);
    // Deterministic post-R2 failure injection: drop the latest applied
    // migration row AND corrupt the recorded checksum of the new tail
    // version inside the BACKUP file. P1-P9 cannot see either edit (page
    // structure, chain, and ledgers stay valid; the backup simply reads as
    // one version older), so the precheck stays eligible and the
    // destructive path proceeds through quarantine (R2) and copy (R3)
    // before the R4a migration runner reports the checksum conflict and
    // the Rollback Rule fires.
    {
        let conn = rusqlite::Connection::open(&backup_path).expect("open backup");
        let app_version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations;", [], |row| {
                row.get(0)
            })
            .expect("version");
        conn.execute(
            "DELETE FROM schema_migrations WHERE version = ?1;",
            [app_version],
        )
        .expect("drop latest version row");
        conn.execute(
            "UPDATE schema_migrations SET checksum = 'tampered-checksum'
             WHERE version = (SELECT MAX(version) FROM schema_migrations);",
            [],
        )
        .expect("tamper checksum");
    }
    repoint_logged_hash(&harness, &meta.file_name, &backup_path);
    let backup_bytes_before = std::fs::read(&backup_path).expect("read backup");
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(report.eligible, "injection must stay invisible to P1-P9");
    // Baseline: checkpoint the live database, then hash the main file.
    {
        let guard = harness.db.lock().expect("lock");
        guard
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint");
    }
    let live_before = std::fs::read(&harness.live_path).expect("read live");
    let restore_error = restore(&harness, &meta.file_name).expect_err("swap must fail");
    let detail = match restore_error {
        RestoreError::SwapFailedAndRolledBack { detail } => detail,
        other => panic!("expected rollback, got {other:?}"),
    };
    assert!(
        detail.contains("migration"),
        "detail names the cause: {detail}"
    );
    // Active database byte-identical after rollback ...
    {
        let guard = harness.db.lock().expect("lock");
        guard
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint");
    }
    let live_after = std::fs::read(&harness.live_path).expect("read live");
    assert_eq!(live_before, live_after);
    // ... no residue from this attempt under quarantine/ ...
    let quarantine_dir = backup_dir(&harness).join("quarantine");
    if quarantine_dir.exists() {
        let leftovers: Vec<_> = std::fs::read_dir(&quarantine_dir)
            .expect("read quarantine")
            .collect();
        assert!(
            leftovers.is_empty(),
            "quarantine residue after rollback: {} entries",
            leftovers.len()
        );
    }
    // ... the reopened manager serves the original database ...
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM products"), 1);
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM documents"), 1);
    // ... and the source backup file is byte-identical.
    let backup_bytes_after = std::fs::read(&backup_path).expect("read backup");
    assert_eq!(backup_bytes_before, backup_bytes_after);
}

#[cfg(windows)]
#[test]
fn restore_r2_first_move_failure_aborts_without_touching_active_db() {
    use std::os::windows::fs::OpenOptionsExt;
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let backup_path = backup_dir(&harness).join(&meta.file_name);
    let backup_bytes_before = std::fs::read(&backup_path).expect("read backup");
    {
        let guard = harness.db.lock().expect("lock");
        guard
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint");
    }
    let live_before = std::fs::read(&harness.live_path).expect("read live");
    // External read handle deliberately WITHOUT FILE_SHARE_DELETE: the
    // manager open (R0/R1) and quarantine-directory creation succeed, but
    // the FIRST R2 rename deterministically fails with a sharing
    // violation. No source hooks, no new dependencies.
    // Win32 share-mode ABI values (stable): READ = 0x1, WRITE = 0x2.
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    let share_lock = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&harness.live_path)
        .expect("external share handle");
    let error = restore(&harness, &meta.file_name).expect_err("first move must fail");
    // Abort path (empty move ledger): Io via abort_before_swap.
    assert!(
        matches!(error, RestoreError::Io(_)),
        "expected abort-path Io, got {error:?}"
    );
    // The exact artifact the pre-fix bug destroyed: the active file exists
    // and is byte-identical to the baseline.
    assert!(harness.live_path.exists(), "active db must still exist");
    {
        let guard = harness.db.lock().expect("lock");
        guard
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint");
    }
    let live_after = std::fs::read(&harness.live_path).expect("read live");
    assert_eq!(live_before, live_after);
    // After dropping the external handle the reopened manager serves the
    // original data ...
    drop(share_lock);
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM products"), 1);
    assert_eq!(live_scalar(&harness, "SELECT COUNT(*) FROM documents"), 1);
    // ... with no residue from this attempt under quarantine/ ...
    let quarantine_dir = backup_dir(&harness).join("quarantine");
    if quarantine_dir.exists() {
        let leftovers: Vec<_> = std::fs::read_dir(&quarantine_dir)
            .expect("read quarantine")
            .collect();
        assert!(
            leftovers.is_empty(),
            "quarantine residue after abort: {} entries",
            leftovers.len()
        );
    }
    // ... and the source backup file byte-identical.
    let backup_bytes_after = std::fs::read(&backup_path).expect("read backup");
    assert_eq!(backup_bytes_before, backup_bytes_after);
}

#[cfg(not(windows))]
#[test]
#[ignore = "Directive I injection uses Win32 share-mode locks; no POSIX equivalent wired (TASK-BRK-054 Directive #3)"]
fn restore_r2_first_move_failure_aborts_without_touching_active_db() {}

#[test]
fn precheck_missing_schema_migrations_maps_to_corrupted() {
    let harness = setup();
    seed_consistent(&harness, "TICK-2026-000001");
    let meta = create(&harness);
    let backup_path = backup_dir(&harness).join(&meta.file_name);
    {
        let conn = rusqlite::Connection::open(&backup_path).expect("open backup");
        conn.execute_batch("DROP TABLE schema_migrations;")
            .expect("drop bookkeeping");
    }
    repoint_logged_hash(&harness, &meta.file_name, &backup_path);
    // No panic, no unwrap: the unreadable version degrades to an integrity
    // failure in the report ...
    let report = precheck(&harness, &meta.file_name).expect("precheck");
    assert!(!report.eligible);
    assert!(report
        .ineligible_reasons
        .contains(&baraka_domain::backup::RestoreIneligibleReason::IntegrityCheckFailed));
    // ... and to BackupCorrupted on the destructive path.
    let restore_error = restore(&harness, &meta.file_name).expect_err("must fail");
    assert_eq!(restore_error, RestoreError::BackupCorrupted);
}
