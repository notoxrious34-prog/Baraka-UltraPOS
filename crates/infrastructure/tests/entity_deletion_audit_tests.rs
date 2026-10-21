//! Deletion audit + reason + supplier-debt RED tests (TASK-BRK-080).
//!
//! v1 runs against the CURRENT 2-arg `execute(&session, id)` API and must
//! FAIL: no delete path writes `audit_log`, and suppliers carry no
//! open-debt guard. v2 (post-implementation) threads `reason` + audit and
//! these same tests turn green — except the product-audit test, which stays
//! `#[ignore]`d until the product adapter (BRK-076 worktree file) can be
//! touched.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    NewCustomer, NewSupplier, ProductRepository, PurchaseLineInput, SessionContext, SessionToken,
    WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::deletion::{
    DeleteCustomerUseCase, DeleteSupplierUseCase, DeleteWarehouseUseCase,
};
use baraka_application::use_cases::purchases::{
    CreatePurchaseOrderUseCase, ReceiveGoodsUseCase, RecordSupplierInvoiceUseCase,
};
use baraka_application::use_cases::suppliers::CreateSupplierUseCase;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{CreditPolicy, CustomerId, CustomerTier};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCustomerRepository, SqlitePurchaseRepository,
    SqliteStockRepository, SqliteSupplierRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use rusqlite::Connection;
use uuid::Uuid;

const ADMIN_UUID: &str = "44444444-4444-4444-4444-444444444444";
const TERMINAL: &str = "TEST-TERMINAL-001";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<Connection>>,
    catalog: SqliteCatalogRepository,
    #[allow(dead_code)]
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    suppliers: SqliteSupplierRepository,
    purchases: SqlitePurchaseRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("del80.db")).expect("open");
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
        purchases: SqlitePurchaseRepository::new(Arc::clone(&shared)),
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

fn audit_rows(harness: &Harness, entity_type: &str, entity_id: &str) -> Vec<(String, String)> {
    let guard = harness.db.lock().expect("db");
    let mut stmt = guard
        .prepare(
            "SELECT action, new_value_json FROM audit_log WHERE entity_type = ?1 AND entity_id = ?2 ORDER BY sequence_no",
        )
        .expect("prepare");
    stmt.query_map(rusqlite::params![entity_type, entity_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .expect("query")
    .collect::<Result<Vec<_>, _>>()
    .expect("collect")
}

fn seed_customer(harness: &Harness) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &admin(),
            NewCustomer {
                name: "Client Audit".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Disabled,
                credit_limit_centimes: 0,
                customer_tier: CustomerTier::Retail,
            },
            TERMINAL,
        )
        .expect("customer")
}

fn seed_warehouse(harness: &Harness, code: &str) -> WarehouseId {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new(code).expect("code"),
        name: code.to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, TERMINAL)
        .expect("warehouse");
    warehouse.id
}

fn seed_supplier(harness: &Harness) -> String {
    CreateSupplierUseCase::new(&harness.suppliers)
        .execute(
            &admin(),
            NewSupplier {
                name: "Fournisseur Audit".to_string(),
                phone: None,
                address: None,
                nif: None,
                rc: None,
                notes: None,
            },
            TERMINAL,
        )
        .expect("supplier")
        .id
}

fn seed_product(harness: &Harness, sku: &str) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new(sku).expect("sku"),
                barcode: None,
                name_ar: "منتج".to_string(),
                name_fr: "Produit".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: unit_id,
                product_id: id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Piece".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: Some(Barcode::new("6111258888888").expect("barcode")),
                sale_price: Money::from_centimes(10_000),
                purchase_price: Money::from_centimes(8_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TERMINAL,
        )
        .expect("product");
    (id, unit_id)
}

/// Give a supplier an UNPAID invoice (open purchase debt) through the real
/// purchase chain: order -> receive -> invoice, no payment.
fn seed_supplier_debt(harness: &Harness, warehouse: WarehouseId) -> String {
    let supplier_id = seed_supplier(harness);
    let (product, unit) = seed_product(harness, "DEBT-1");
    let lines = vec![PurchaseLineInput {
        product_id: product,
        unit_id: unit,
        quantity_micro: 20_000,
        unit_cost_centimes: 8_000,
    }];
    let bc = CreatePurchaseOrderUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
    )
    .execute(
        &admin(),
        &supplier_id,
        warehouse,
        lines.clone(),
        TERMINAL,
        &Uuid::new_v4().to_string(),
    )
    .expect("bc")
    .id;
    let br = ReceiveGoodsUseCase::new(&harness.purchases, &harness.suppliers, &harness.warehouses)
        .execute(
            &admin(),
            Some(bc),
            &supplier_id,
            warehouse,
            lines.clone(),
            TERMINAL,
            &Uuid::new_v4().to_string(),
        )
        .expect("br")
        .id;
    RecordSupplierInvoiceUseCase::new(&harness.purchases, &harness.suppliers, &harness.warehouses)
        .execute(
            &admin(),
            Some(br),
            &supplier_id,
            warehouse,
            lines,
            16_000,
            TERMINAL,
            &Uuid::new_v4().to_string(),
        )
        .expect("invoice");
    supplier_id
}

#[test]
fn red_customer_hard_delete_writes_audit_row() {
    let harness = setup();
    let id = seed_customer(&harness);
    DeleteCustomerUseCase::new(&harness.customers)
        .execute(&admin(), id, "operator reason")
        .expect("delete");
    let rows = audit_rows(&harness, "customers", &id.to_string());
    assert!(!rows.is_empty(), "no audit row for customer hard-delete");
}

#[test]
fn red_warehouse_hard_delete_writes_audit_row() {
    let harness = setup();
    let id = seed_warehouse(&harness, "AUX");
    DeleteWarehouseUseCase::new(&harness.warehouses)
        .execute(&admin(), id, "operator reason")
        .expect("delete");
    let rows = audit_rows(&harness, "warehouses", &id.to_string());
    assert!(!rows.is_empty(), "no audit row for warehouse hard-delete");
}

#[test]
fn red_supplier_hard_delete_writes_audit_row() {
    let harness = setup();
    let id = seed_supplier(&harness);
    DeleteSupplierUseCase::new(&harness.suppliers)
        .execute(&admin(), &id, "operator reason")
        .expect("delete");
    let rows = audit_rows(&harness, "suppliers", &id);
    assert!(!rows.is_empty(), "no audit row for supplier hard-delete");
}

#[test]
fn red_supplier_with_open_debt_is_refused() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "DEBTWH");
    let id = seed_supplier_debt(&harness, warehouse);
    let result =
        DeleteSupplierUseCase::new(&harness.suppliers).execute(&admin(), &id, "operator reason");
    assert!(
        result.is_err(),
        "supplier with open debt deleted without error"
    );
}
