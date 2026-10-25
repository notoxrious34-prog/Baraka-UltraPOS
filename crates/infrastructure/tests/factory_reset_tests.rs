//! Verified factory-reset integration tests (TASK-BRK-091, AUD-043 section 3).
//!
//! File-backed temporary databases only (never the live database): every
//! test opens a migrated temp database, seeds cross-domain retail state,
//! then exercises the read-only precheck and the destructive reset —
//! precondition rejections with zero-write proofs, mandatory-backup
//! ordering, and the full post-reset KEEP/WIPE domain table.
//!
//! The reset builds a FRESH migrated file and quarantine-swaps it into
//! place (wipe-by-DELETE is impossible: `BEFORE DELETE` abort-triggers
//! guard every ledger and log table), so no schema change was needed.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CustomerRepository, FactoryResetRepository, NewCustomer, ProductRepository, SessionContext,
    SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::cash::OpenCashSessionUseCase;
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::factory_reset::{
    ExecuteFactoryResetUseCase, PrecheckFactoryResetUseCase, ResetExecution,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{CreditPolicy, CustomerTier};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteBackupManager, SqliteCashSessionRepository, SqliteCatalogRepository,
    SqliteCustomerRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const TEST_TERMINAL: &str = "TEST-TERMINAL-RESET";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    backups: SqliteBackupManager,
    catalog: SqliteCatalogRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    cash: SqliteCashSessionRepository,
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
        _dir: dir,
        db: Arc::clone(&shared),
        backups: SqliteBackupManager::new(Arc::clone(&shared), live_path),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
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

/// Seed cross-domain retail state: warehouse, product + stock, customer +
/// ledger, supplier + ledger, document + items + payment, a CLOSED cash
/// session, a print job, store identity, an extra user, and log rows.
fn seed_full(harness: &Harness) -> WarehouseId {
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
    harness
        .customers
        .update_cached_balance(customer, 20_000)
        .expect("cached balance");
    let guard = harness.db.lock().expect("lock");
    let wid = warehouse.id.to_string();
    let pid = product_id.to_string();
    let uid = unit_id.to_string();
    let cid = customer.to_string();
    guard
        .execute(
            "INSERT INTO stock_levels (warehouse_id, product_id, quantity_micro, quantity_base_micro)
             VALUES (?1, ?2, 100000, 100000)",
            rusqlite::params![wid, pid],
        )
        .expect("stock level");
    guard
        .execute(
            "INSERT INTO stock_movements
             (id, sequence_no, warehouse_id, product_id, movement_type, unit_id,
              quantity_micro, quantity_base_micro, created_by, created_at)
             VALUES (?1, 1, ?2, ?3, 'PURCHASE_IN', ?4, 100000, 100000, ?5,
                     '2026-09-20T08:00:00.000Z')",
            rusqlite::params![Uuid::new_v4().to_string(), wid, pid, uid, ACTOR_UUID],
        )
        .expect("stock movement");
    guard
        .execute(
            "INSERT INTO suppliers (id, name, current_balance_centimes, is_active, origin_device_id,
                     created_at, updated_at)
             VALUES ('55555555-5555-5555-5555-555555555555', 'Fournisseur', 5000, 1,
                     'TEST-TERMINAL-RESET', '2026-09-20T08:00:00.000Z', '2026-09-20T08:00:00.000Z')",
            [],
        )
        .expect("supplier");
    guard
        .execute(
            "INSERT INTO supplier_ledger
             (id, supplier_id, entry_type, amount_centimes, reference_type, reference_id,
              created_at, created_by)
             VALUES (?1, '55555555-5555-5555-5555-555555555555', 'DEBIT', 5000,
                     'PURCHASE_INVOICE', 'seed-inv-1', '2026-09-20T08:00:00.000Z', ?2)",
            rusqlite::params![Uuid::new_v4().to_string(), ACTOR_UUID],
        )
        .expect("supplier ledger");
    guard
        .execute(
            "INSERT INTO documents
             (id, number, fiscal_year, type, party_type, party_id, warehouse_id, document_language,
              subtotal_ht_centimes, vat_total_centimes, global_discount_centimes, timbre_centimes,
              rounding_centimes, ttc_centimes, paid_amount_centimes, change_amount_centimes,
              payment_method, status, created_at, created_by, origin_device_id)
             VALUES ('66666666-6666-6666-6666-666666666666', 'F-2026-0001', 2026, 'FACTURE',
                     'CUSTOMER', ?1, ?2, 'fr', 10000, 0, 0, 0, 0, 10000, 10000, 0, 'CASH',
                     'FINAL', '2026-09-20T08:00:00.000Z', ?3, 'TEST-TERMINAL-RESET')",
            rusqlite::params![cid, wid, ACTOR_UUID],
        )
        .expect("document");
    guard
        .execute(
            "INSERT INTO document_items
             (id, document_id, product_id, unit_id, quantity_micro, quantity_base_micro,
              unit_price_centimes, discount_centimes, tva_rate_micro, line_ht_centimes,
              line_tva_centimes, line_ttc_centimes, cost_price_centimes, sort_order)
             VALUES (?1, '66666666-6666-6666-6666-666666666666', ?2, ?3, 10000, 10000,
                     10000, 0, 0, 10000, 0, 10000, 8000, 0)",
            rusqlite::params![Uuid::new_v4().to_string(), pid, uid],
        )
        .expect("document item");
    guard
        .execute(
            "INSERT INTO payments
             (id, sequence_no, document_id, customer_id, amount_centimes, payment_method,
              created_by, created_at)
             VALUES (?1, 1, '66666666-6666-6666-6666-666666666666', ?2, 10000, 'CASH', ?3,
                     '2026-09-20T08:00:00.000Z')",
            rusqlite::params![Uuid::new_v4().to_string(), cid, ACTOR_UUID],
        )
        .expect("payment");
    guard
        .execute(
            "INSERT INTO cash_sessions
             (id, terminal_id, warehouse_id, status, opened_by, opened_at, closed_by, closed_at,
              opening_float_centimes, system_expected_centimes, origin_device_id)
             VALUES ('44444444-4444-4444-4444-444444444444', 'TEST-TERMINAL-RESET', ?1, 'CLOSED',
                     ?2, '2026-09-20T08:00:00.000Z', ?2, '2026-09-20T20:00:00.000Z', 50000, 60000,
                     'TEST-TERMINAL-RESET')",
            rusqlite::params![wid, ACTOR_UUID],
        )
        .expect("closed cash session");
    guard
        .execute(
            "INSERT INTO cash_movements
             (id, cash_session_id, movement_type, amount_centimes, description, created_by,
              created_at)
             VALUES (?1, '44444444-4444-4444-4444-444444444444', 'SALE_CASH_IN', 10000, 'seed',
                     ?2, '2026-09-20T09:00:00.000Z')",
            rusqlite::params![Uuid::new_v4().to_string(), ACTOR_UUID],
        )
        .expect("cash movement");
    guard
        .execute(
            "INSERT INTO print_jobs
             (id, document_id, status, attempt_count, is_duplicate, payload_json,
              created_at, updated_at)
             VALUES (?1, '66666666-6666-6666-6666-666666666666', 'DONE', 1, 0, '{}',
                     '2026-09-20T08:00:00.000Z', '2026-09-20T08:00:00.000Z')",
            [Uuid::new_v4().to_string()],
        )
        .expect("print job");
    guard
        .execute(
            "INSERT INTO settings (key, value, updated_at) VALUES
             ('store.name', 'Boutique Test', '2026-09-20T08:00:00.000Z'),
             ('printer.transport', 'SPOOLER', '2026-09-20T08:00:00.000Z')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
             updated_at = excluded.updated_at",
            [],
        )
        .expect("settings");
    guard
        .execute(
            "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
             VALUES ('88888888-8888-8888-8888-888888888888', 'vendeur1', 'hash', 'VENDEUR',
                     1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("extra user");
    guard
        .execute(
            "INSERT INTO document_sequences (type, fiscal_year, last_sequence)
             VALUES ('FACTURE', 2026, 1)",
            [],
        )
        .expect("sequence");
    guard
        .execute(
            "INSERT INTO audit_log
             (prev_hash, record_hash, timestamp, user_id, action, entity_type, entity_id,
              old_value_json, new_value_json)
             VALUES ('GENESIS', 'seed', '2026-09-20T08:00:00.000Z', ?1, 'SEED', 'test', 'seed',
                     NULL, '{}')",
            [ACTOR_UUID],
        )
        .expect("audit seed");
    guard
        .execute(
            "INSERT INTO command_log (command_id, command_type, request_hash, response_payload, created_at)
             VALUES ('cmd-seed-1', 'seed', 'hash', '{}', '2026-09-20T08:00:00.000Z')",
            [],
        )
        .expect("command log seed");
    drop(guard);
    warehouse.id
}

fn table_count(harness: &Harness, table: &str) -> i64 {
    harness
        .db
        .lock()
        .expect("lock")
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn backups_log_count(harness: &Harness) -> i64 {
    table_count(harness, "backups_log")
}

/// Stable wire-code assertion for reset rejections.
fn assert_reset_code(error: &ApplicationError, code: &str) {
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(actual))
            if actual == code
        ),
        "expected {code}, got {error:?}"
    );
}

/// Execute parameters for the reset tests (retention + version fixed).
fn reset_params<'a>(backup_dir: &'a std::path::Path, token: &'a str) -> ResetExecution<'a> {
    ResetExecution {
        terminal_id: TEST_TERMINAL,
        backup_dir,
        retention: 30,
        app_version: APP_VERSION,
        typed_token: token,
        actor: ACTOR_UUID,
    }
}

#[test]
fn precheck_reports_writable_state_and_writes_nothing() {
    let harness = setup();
    let wid = seed_full(&harness);
    let before_products = table_count(&harness, "products");
    assert_eq!(before_products, 1);
    let _ = wid;

    let report = PrecheckFactoryResetUseCase::new(&harness.backups, &harness.cash)
        .execute(
            &session_with(Role::Admin),
            TEST_TERMINAL,
            &backup_dir(&harness),
        )
        .expect("precheck");
    assert!(!report.open_session);
    // Backup dir does not exist yet: probe must fail, and the report says so.
    assert!(!report.backup_dir_writable);
    assert!(!report.eligible);
    assert_eq!(report.reasons, vec!["RESET_BACKUP_UNWRITABLE".to_string()]);
    assert!(report.newest_backup.is_none());
    // Zero writes: no log rows, no state change.
    assert_eq!(backups_log_count(&harness), 0);
    assert_eq!(table_count(&harness, "products"), 1);
}

#[test]
fn execute_rejects_open_session_with_zero_writes() {
    let harness = setup();
    let wid = seed_full(&harness);
    std::fs::create_dir_all(backup_dir(&harness)).expect("backup dir");
    OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &session_with(Role::Admin),
            wid,
            50_000,
            TEST_TERMINAL,
            "cmd-open-1",
        )
        .expect("open session");

    let dir = backup_dir(&harness);
    let error = ExecuteFactoryResetUseCase::new(&harness.backups, &harness.cash, &harness.backups)
        .execute(
            &session_with(Role::Admin),
            &reset_params(&dir, "Boutique Test"),
        )
        .expect_err("open session must reject");
    assert_reset_code(&error, "RESET_OPEN_SESSION");
    // Zero-write proof: no backup rows, no backup files, seeded data intact.
    assert_eq!(backups_log_count(&harness), 0);
    assert_eq!(
        std::fs::read_dir(backup_dir(&harness))
            .expect("read dir")
            .count(),
        0
    );
    assert_eq!(table_count(&harness, "products"), 1);
    assert_eq!(table_count(&harness, "users"), 2);
}

#[test]
fn execute_rejects_unwritable_backup_dir_with_zero_writes() {
    let harness = setup();
    seed_full(&harness);
    // Point the backup dir at an existing FILE: probe and creation both fail.
    let blocker = harness._dir.path().join("blocker");
    std::fs::write(&blocker, b"x").expect("blocker");

    let error = ExecuteFactoryResetUseCase::new(&harness.backups, &harness.cash, &harness.backups)
        .execute(
            &session_with(Role::Admin),
            &reset_params(&blocker, "Boutique Test"),
        )
        .expect_err("unwritable dir must reject");
    assert_reset_code(&error, "RESET_BACKUP_UNWRITABLE");
    assert_eq!(backups_log_count(&harness), 0);
    assert_eq!(table_count(&harness, "products"), 1);
}

#[test]
fn execute_rejects_wrong_token_with_zero_writes() {
    let harness = setup();
    seed_full(&harness);
    std::fs::create_dir_all(backup_dir(&harness)).expect("backup dir");

    let dir = backup_dir(&harness);
    let error = ExecuteFactoryResetUseCase::new(&harness.backups, &harness.cash, &harness.backups)
        .execute(
            &session_with(Role::Admin),
            &reset_params(&dir, "Wrong Name"),
        )
        .expect_err("wrong token must reject");
    assert_reset_code(&error, "RESET_TOKEN_MISMATCH");
    // Token validation runs BEFORE the backup: zero rows, zero files.
    assert_eq!(backups_log_count(&harness), 0);
    assert_eq!(
        std::fs::read_dir(backup_dir(&harness))
            .expect("read dir")
            .count(),
        0
    );
    assert_eq!(table_count(&harness, "products"), 1);
}

#[test]
fn execute_full_reset_returns_first_run_state() {
    let harness = setup();
    seed_full(&harness);
    std::fs::create_dir_all(backup_dir(&harness)).expect("backup dir");

    let dir = backup_dir(&harness);
    let outcome =
        ExecuteFactoryResetUseCase::new(&harness.backups, &harness.cash, &harness.backups)
            .execute(
                &session_with(Role::Admin),
                &reset_params(&dir, "Boutique Test"),
            )
            .expect("reset");
    assert!(outcome.restart_required);
    // Mandatory verified backup exists on disk and is carried in the log.
    assert!(backup_dir(&harness)
        .join(&outcome.reset_backup_file_name)
        .is_file());
    assert_eq!(backups_log_count(&harness), 1);

    // WIPE domain: everything merchant goes to zero.
    for table in [
        "users",
        "products",
        "product_units",
        "stock_levels",
        "stock_movements",
        "documents",
        "document_items",
        "payments",
        "customers",
        "customer_ledger",
        "suppliers",
        "supplier_ledger",
        "cash_sessions",
        "cash_movements",
        "print_jobs",
        "document_sequences",
        "command_log",
        "products_fts",
    ] {
        assert_eq!(
            table_count(&harness, table),
            0,
            "table {table} must be empty"
        );
    }
    // Settings return to first-run seed state: exactly the 4 migration-0006
    // rows (no store.name, transport back to the SERIAL default).
    assert_eq!(table_count(&harness, "settings"), 4);
    let transport: String = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT value FROM settings WHERE key = 'printer.transport'",
            [],
            |row| row.get(0),
        )
        .expect("seeded transport");
    assert_eq!(transport, "SERIAL");
    let store_name: Option<String> = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT value FROM settings WHERE key = 'store.name'",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .expect("store name lookup");
    assert_eq!(store_name, None);
    // KEEP domain: backup history survives (the safety net).
    assert_eq!(backups_log_count(&harness), 1);
    // Reseed: DEP-01 invariant restored (fixed UUID from the seed migration).
    let depot: (String, String, i64) = harness
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT id, code, is_active FROM warehouses", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("DEP-01");
    assert_eq!(depot.0, "01950000-0000-7000-8000-000000000001");
    assert_eq!(depot.1, "DEP-01");
    assert_eq!(depot.2, 1);
    // schema_migrations untouched: still the full chain.
    let migrations: i64 = table_count(&harness, "schema_migrations");
    assert!(migrations > 10, "migrations kept, got {migrations}");
    // RESET_PERFORMED is the first (and only) audit row, GENESIS-chained.
    let audit: (String, String, String, String) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT prev_hash, action, entity_type, user_id FROM audit_log",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("audit row");
    assert_eq!(table_count(&harness, "audit_log"), 1);
    assert_eq!(audit.0, "GENESIS");
    assert_eq!(audit.1, "RESET_PERFORMED");
    assert_eq!(audit.2, "factory_reset");
    assert_eq!(audit.3, ACTOR_UUID);
    // Fresh database integrity: quick check passes on the reopened file.
    let check: String = harness
        .db
        .lock()
        .expect("lock")
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .expect("quick check");
    assert_eq!(check, "ok");
    // Quarantine holds the previous live database with the old data intact.
    let quarantine = harness._dir.path().join("quarantine");
    assert!(quarantine.join(&outcome.quarantined_db_name).is_file());
    let old = rusqlite::Connection::open(quarantine.join(&outcome.quarantined_db_name))
        .expect("open quarantined");
    let old_products: i64 = old
        .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
        .expect("old products");
    assert_eq!(old_products, 1);
    // Token now falls back to DEP-01 (store.name wiped): first-run reads work.
    assert_eq!(
        FactoryResetRepository::expected_reset_token(&harness.backups).expect("token"),
        "DEP-01"
    );
}
