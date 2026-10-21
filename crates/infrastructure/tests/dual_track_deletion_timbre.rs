//! Dual-track safe deletion + configurable fiscal stamp (ADR-092, TASK-BRK-074).
//!
//! Covers the 8 acceptance behaviors over real SQLite backends:
//! hard purge of unreferenced rows, soft deactivation of historically
//! referenced rows, DEP-01 immutability, non-zero-balance customer guard,
//! warehouse stock guard, and the `fiscal.timbre_enabled` toggle across
//! preview and checkout.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    NewCustomer, NewSupplier, ProductRepository, SessionContext, SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::deletion::{
    DeleteCustomerUseCase, DeleteProductUseCase, DeleteWarehouseUseCase, UpdateWarehouseUseCase,
};
use baraka_application::use_cases::inventory::{
    AdjustStockInput, AdjustStockUseCase, CreateWarehouseUseCase, StockDirection,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, GetFiscalSettingsUseCase, PaymentInput,
    PreviewSaleTotalsUseCase, PreviewTotalsInput, UpdateFiscalSettingsUseCase,
};
use baraka_application::use_cases::suppliers::CreateSupplierUseCase;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::WarehouseId;
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{CreditPolicy, CustomerTier};
use baraka_domain::sales::{DocumentLanguage, DocumentType, PaymentMethod};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteFiscalSettings, SqliteStockRepository, SqliteSupplierRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use rusqlite::Connection;
use uuid::Uuid;

const ADMIN_UUID: &str = "44444444-4444-4444-4444-444444444444";
const DEP01_UUID: &str = "01950000-0000-7000-8000-000000000001";
const TERMINAL: &str = "TEST-TERMINAL-001";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<Connection>>,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    suppliers: SqliteSupplierRepository,
    fiscal: SqliteFiscalSettings,
    checkout: SqliteCheckoutRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("dual.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('44444444-4444-4444-4444-444444444444', 'admin1', 'hash', 'ADMIN',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed admin");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        suppliers: SqliteSupplierRepository::new(Arc::clone(&shared)),
        fiscal: SqliteFiscalSettings::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
    }
}

fn admin() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ADMIN_UUID).expect("uuid")),
        role: Role::Admin,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn dep01() -> WarehouseId {
    WarehouseId::from(Uuid::parse_str(DEP01_UUID).expect("uuid"))
}

fn ensure_cash_session(harness: &Harness) {
    let session_id = Uuid::new_v4().to_string();
    let guard = harness.db.lock().expect("db");
    guard
        .execute(
            "INSERT INTO cash_sessions
             (id, warehouse_id, terminal_id, status, opened_by, opened_at,
              opening_float_centimes, revision, origin_device_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![session_id, DEP01_UUID, TERMINAL, ADMIN_UUID,],
        )
        .expect("open session");
    guard
        .execute(
            "INSERT INTO cash_movements
             (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
             VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z', ?3)",
            rusqlite::params![Uuid::new_v4().to_string(), session_id, ADMIN_UUID,],
        )
        .expect("opening float");
}

fn seed_product(harness: &Harness, sku: &str) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    let product = Product {
        id,
        sku: Sku::new(sku).expect("sku"),
        barcode: None,
        name_ar: "منتج".to_string(),
        name_fr: "Produit".to_string(),
        category_id: None,
        is_kit: false,
        is_active: true,
    };
    let unit = ProductUnit {
        id: unit_id,
        product_id: id,
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Pièce".to_string(),
        factor_micro: FactorMicro::from_micros(10_000),
        barcode: Some(Barcode::new("6111259999999").expect("barcode")),
        sale_price: Money::from_centimes(10_000),
        purchase_price: Money::from_centimes(8_000),
        wholesale_price: None,
        is_base_unit: true,
        is_default_selling: true,
    };
    harness
        .catalog
        .create_product(&product, &[unit], TERMINAL)
        .expect("create product");
    (id, unit_id)
}

fn seed_customer(harness: &Harness, name: &str) -> baraka_domain::sales::CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &admin(),
            NewCustomer {
                name: name.to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Disabled,
                credit_limit_centimes: 0,
                customer_tier: CustomerTier::Retail,
            },
            TERMINAL,
        )
        .expect("create customer")
}

fn seed_supplier(harness: &Harness, name: &str) -> String {
    CreateSupplierUseCase::new(&harness.suppliers)
        .execute(
            &admin(),
            NewSupplier {
                name: name.to_string(),
                phone: None,
                address: None,
                nif: None,
                rc: None,
                notes: None,
            },
            TERMINAL,
        )
        .expect("create supplier")
        .id
}

fn is_active(table: &str, harness: &Harness, id: &str) -> Option<i64> {
    let guard = harness.db.lock().expect("db");
    let sql = format!("SELECT is_active FROM {table} WHERE id = ?1");
    guard.query_row(&sql, [id], |row| row.get(0)).ok()
}

fn row_exists(table: &str, harness: &Harness, id: &str) -> bool {
    let guard = harness.db.lock().expect("db");
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE id = ?1");
    guard
        .query_row(&sql, [id], |row| row.get::<_, i64>(0))
        .map(|count| count > 0)
        .unwrap_or(false)
}

// 1. Unreferenced product purges completely (product + units + levels).
#[test]
fn test_hard_delete_unreferenced_product() {
    let harness = setup();
    let (product, _unit) = seed_product(&harness, "DUAL-001");
    let outcome = DeleteProductUseCase::new(&harness.catalog)
        .execute(&admin(), product, "test cleanup")
        .expect("delete");
    assert_eq!(outcome.mode, "hard_deleted");
    assert!(harness
        .catalog
        .find_by_id(product)
        .expect("lookup")
        .is_none());
    assert!(!row_exists("products", &harness, &product.to_string()));
    let guard = harness.db.lock().expect("db");
    let units: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM product_units WHERE product_id = ?1",
            [product.to_string()],
            |row| row.get(0),
        )
        .expect("units");
    let levels: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM stock_levels WHERE product_id = ?1",
            [product.to_string()],
            |row| row.get(0),
        )
        .expect("levels");
    assert_eq!(units, 0);
    assert_eq!(levels, 0);
}

// 2. Product with a stock movement deactivates instead of purging.
#[test]
fn test_soft_deactivate_referenced_product() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "DUAL-002");
    AdjustStockUseCase::new(&harness.stock)
        .execute(
            &admin(),
            AdjustStockInput {
                product_id: product,
                warehouse_id: dep01(),
                unit_id: unit,
                quantity_micro: 10_000,
                direction: StockDirection::Increase,
                reason: "test adjustment".to_string(),
                authorize_negative_stock: false,
            },
        )
        .expect("adjust");
    let outcome = DeleteProductUseCase::new(&harness.catalog)
        .execute(&admin(), product, "test cleanup")
        .expect("delete");
    assert_eq!(outcome.mode, "deactivated");
    assert!(row_exists("products", &harness, &product.to_string()));
    assert_eq!(
        is_active("products", &harness, &product.to_string()),
        Some(0)
    );
}

// 3. Customer with zero balance and zero documents purges completely.
#[test]
fn test_hard_delete_unreferenced_customer() {
    let harness = setup();
    let id = seed_customer(&harness, "Client Inactif");
    let outcome = DeleteCustomerUseCase::new(&harness.customers)
        .execute(&admin(), id, "test cleanup")
        .expect("delete");
    assert_eq!(outcome.mode, "hard_deleted");
    assert!(!row_exists("customers", &harness, &id.to_string()));
}

// 4. Customer with historical documents but zero balance deactivates.
#[test]
fn test_soft_deactivate_referenced_customer() {
    let harness = setup();
    let id = seed_customer(&harness, "Client Historique");
    harness
        .db
        .lock()
        .expect("db")
        .execute(
            "INSERT INTO documents (id, number, fiscal_year, type, party_type, party_id,
             warehouse_id, document_language, subtotal_ht_centimes, vat_total_centimes,
             global_discount_centimes, timbre_centimes, rounding_centimes, ttc_centimes,
             paid_amount_centimes, change_amount_centimes, payment_method, status,
             created_at, created_by, origin_device_id)
             VALUES (?1, 'T-0001', 2026, 'TICKET', 'CUSTOMER', ?2, ?3, 'FR',
             10000, 0, 0, 0, 0, 10000, 10000, 0, 'CASH', 'FINAL',
             '2026-01-01T00:00:00Z', ?4, 'TEST')",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                id.to_string(),
                DEP01_UUID,
                ADMIN_UUID,
            ],
        )
        .expect("seed document");
    let outcome = DeleteCustomerUseCase::new(&harness.customers)
        .execute(&admin(), id, "test cleanup")
        .expect("delete");
    assert_eq!(outcome.mode, "deactivated");
    assert!(row_exists("customers", &harness, &id.to_string()));
    assert_eq!(is_active("customers", &harness, &id.to_string()), Some(0));
}

// 5. Non-zero balance blocks both deletion tracks; the row stays active.
#[test]
fn test_reject_customer_deletion_with_non_zero_balance() {
    let harness = setup();
    let id = seed_customer(&harness, "Client Endetté");
    harness
        .db
        .lock()
        .expect("db")
        .execute(
            "UPDATE customers SET current_balance_centimes = 500000 WHERE id = ?1",
            [id.to_string()],
        )
        .expect("set balance");
    let rejected =
        DeleteCustomerUseCase::new(&harness.customers).execute(&admin(), id, "test cleanup");
    assert!(
        matches!(rejected, Err(ApplicationError::Validation(_))),
        "unexpected: {rejected:?}"
    );
    assert!(row_exists("customers", &harness, &id.to_string()));
    assert_eq!(is_active("customers", &harness, &id.to_string()), Some(1));
}

// 6. DEP-01 rejects deletion and deactivation alike.
#[test]
fn test_reject_dep01_deletion_and_deactivation() {
    let harness = setup();
    let warehouse = harness
        .warehouses
        .get_by_id(dep01())
        .expect("lookup")
        .expect("DEP-01 seeded by migration 0012");
    assert_eq!(warehouse.code.as_str(), "DEP-01");
    let deleted =
        DeleteWarehouseUseCase::new(&harness.warehouses).execute(&admin(), dep01(), "test cleanup");
    assert!(
        matches!(deleted, Err(ApplicationError::Validation(_))),
        "unexpected: {deleted:?}"
    );
    let deactivated = UpdateWarehouseUseCase::new(&harness.warehouses).execute(
        &admin(),
        dep01(),
        None,
        Some(false),
    );
    assert!(
        matches!(deactivated, Err(ApplicationError::Validation(_))),
        "unexpected: {deactivated:?}"
    );
    assert_eq!(is_active("warehouses", &harness, DEP01_UUID), Some(1));
}

// 7. Warehouses: non-zero stock blocks deletion; empty depots purge;
//    sparse update (rename + deactivate) works on non-primary depots.
#[test]
fn test_warehouse_deletion_and_stock_guard() {
    let harness = setup();
    let stocked_id = CreateWarehouseUseCase::new(&harness.warehouses)
        .execute(&admin(), "WH-02", "Dépôt Secondaire", TERMINAL)
        .expect("create stocked");
    let (product, unit) = seed_product(&harness, "DUAL-003");
    AdjustStockUseCase::new(&harness.stock)
        .execute(
            &admin(),
            AdjustStockInput {
                product_id: product,
                warehouse_id: stocked_id,
                unit_id: unit,
                quantity_micro: 50_000,
                direction: StockDirection::Increase,
                reason: "test adjustment".to_string(),
                authorize_negative_stock: false,
            },
        )
        .expect("adjust");
    let blocked = DeleteWarehouseUseCase::new(&harness.warehouses).execute(
        &admin(),
        stocked_id,
        "test cleanup",
    );
    assert!(
        matches!(blocked, Err(ApplicationError::Validation(_))),
        "unexpected: {blocked:?}"
    );
    assert!(row_exists("warehouses", &harness, &stocked_id.to_string()));

    let empty_id = CreateWarehouseUseCase::new(&harness.warehouses)
        .execute(&admin(), "WH-03", "Dépôt Vide", TERMINAL)
        .expect("create empty");
    UpdateWarehouseUseCase::new(&harness.warehouses)
        .execute(&admin(), empty_id, Some("Dépôt Renommé".to_string()), None)
        .expect("rename");
    let renamed = harness
        .warehouses
        .get_by_id(empty_id)
        .expect("lookup")
        .expect("row");
    assert_eq!(renamed.name, "Dépôt Renommé");
    UpdateWarehouseUseCase::new(&harness.warehouses)
        .execute(&admin(), empty_id, None, Some(false))
        .expect("deactivate");
    assert_eq!(
        is_active("warehouses", &harness, &empty_id.to_string()),
        Some(0)
    );

    let fresh_id = CreateWarehouseUseCase::new(&harness.warehouses)
        .execute(&admin(), "WH-04", "Dépôt Éphémère", TERMINAL)
        .expect("create fresh");
    let outcome = DeleteWarehouseUseCase::new(&harness.warehouses)
        .execute(&admin(), fresh_id, "test cleanup")
        .expect("delete");
    assert_eq!(outcome.mode, "hard_deleted");
    assert!(!row_exists("warehouses", &harness, &fresh_id.to_string()));
    // Supplier helper is exercised so the supplier repository stays wired.
    let supplier_id = seed_supplier(&harness, "Fournisseur Test");
    assert!(!supplier_id.is_empty());
}

// 8. Timbre toggle: disabled forces 0 across preview + checkout;
//    enabled restores the 500-centime floor stamp.
#[test]
fn test_configurable_timbre_preview_and_checkout() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "DUAL-004");
    assert!(
        GetFiscalSettingsUseCase::new(&harness.fiscal)
            .execute(&admin())
            .expect("fiscal default"),
        "absent fiscal.timbre_enabled must default to true"
    );

    let preview = |timbre_enabled: bool| {
        PreviewSaleTotalsUseCase::new(
            &harness.catalog,
            &harness.stock,
            &harness.warehouses,
            &harness.customers,
        )
        .execute(
            &admin(),
            PreviewTotalsInput {
                warehouse_id: dep01(),
                doc_type: DocumentType::Ticket,
                lines: vec![CartLineInput {
                    product_id: product,
                    unit_id: unit,
                    quantity_micro: 10_000,
                    unit_price_override: None,
                    line_discount_centimes: 0,
                }],
                customer_id: None,
                global_discount_centimes: 0,
                payments: vec![PaymentInput {
                    method: PaymentMethod::Cash,
                    amount_centimes: 100_000,
                }],
                timbre_enabled,
            },
        )
        .expect("preview")
    };
    assert_eq!(preview(true).timbre, 500);
    assert_eq!(preview(false).timbre, 0);

    ensure_cash_session(&harness);
    let checkout = |timbre_enabled: bool| {
        CheckoutSaleUseCase::new(
            &harness.catalog,
            &harness.customers,
            &harness.checkout,
            &harness.warehouses,
        )
        .execute(
            &admin(),
            CheckoutInput {
                command_id: Uuid::new_v4().to_string(),
                warehouse_id: dep01(),
                customer_id: None,
                doc_type: DocumentType::Ticket,
                language: DocumentLanguage::Fr,
                lines: vec![CartLineInput {
                    product_id: product,
                    unit_id: unit,
                    quantity_micro: 10_000,
                    unit_price_override: None,
                    line_discount_centimes: 0,
                }],
                global_discount_centimes: 0,
                payments: vec![PaymentInput {
                    method: PaymentMethod::Cash,
                    amount_centimes: 100_000,
                }],
                allow_negative_stock: true,
                notes: None,
                timbre_enabled,
            },
            TERMINAL,
        )
        .expect("checkout")
    };
    assert_eq!(checkout(true).timbre_centimes, "500");
    assert_eq!(checkout(false).timbre_centimes, "0");

    // Fiscal round-trip through the settings port.
    assert!(!UpdateFiscalSettingsUseCase::new(&harness.fiscal)
        .execute(&admin(), false)
        .expect("disable timbre"));
    assert!(!GetFiscalSettingsUseCase::new(&harness.fiscal)
        .execute(&admin())
        .expect("read back"));
    assert!(UpdateFiscalSettingsUseCase::new(&harness.fiscal)
        .execute(&admin(), true)
        .expect("re-enable timbre"));
}
