//! TASK-BRK-111: last transacted price memory over real SQLite.
//!
//! Proves the read-only recall contract end-to-end on an isolated scratch
//! database (never the live `%APPDATA%` database):
//!
//! * a named-customer sale records the transacted legs
//!   (`unit_price`, `discount_per_unit`) inside the same atomic checkout
//!   transaction, and recall returns them exactly;
//! * a later cheaper sale overwrites the cell (latest wins);
//! * a guest sale records nothing, another customer reads `None`
//!   (isolation), and a never-sold product reads `None` (empty = unknown).

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    CustomerLastPrice, NewCustomer, ProductRepository, SessionContext, SessionToken,
    StockRepository, WarehouseRepository,
};
use baraka_application::use_cases::customers::{CreateCustomerUseCase, GetLastPriceUseCase};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_domain::catalog::{Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, DocumentLanguage, DocumentType, PaymentMethod,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    Argon2PasswordHasher, SqliteCatalogRepository, SqliteCheckoutRepository,
    SqliteCustomerRepository, SqliteStockRepository, SqliteUserRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const TEST_TERMINAL: &str = "TEST-TERMINAL-BRK111";
const STAMP: &str = "2026-11-12T09:00:00Z";

struct Harness {
    _dir: tempfile::TempDir,
    db_path: std::path::PathBuf,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    users: SqliteUserRepository,
    hasher: Argon2PasswordHasher,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("last-price.db");
    let mut conn = open_connection(&db_path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'admin', 'hash', 'ADMIN', 1, ?2, ?2)",
        rusqlite::params![ACTOR_UUID, STAMP],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        db_path,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
        users: SqliteUserRepository::new(Arc::clone(&shared)),
        hasher: Argon2PasswordHasher,
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

fn seed_warehouse(harness: &Harness, code: &str) -> WarehouseId {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new(code).expect("code"),
        name: code.to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, TEST_TERMINAL)
        .expect("warehouse");
    warehouse.id
}

fn seed_product(
    harness: &Harness,
    warehouse: WarehouseId,
    sku: &str,
    sale_price: i64,
) -> (ProductId, UnitId) {
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
        barcode: None,
        sale_price: Money::from_centimes(sale_price),
        purchase_price: Money::from_centimes(sale_price / 2),
        wholesale_price: None,
        super_gros_price: None,
        is_base_unit: true,
        is_default_selling: true,
    };
    harness
        .catalog
        .create_product(&product, &[unit], TEST_TERMINAL)
        .expect("product");
    harness
        .stock
        .upsert_stock_level_with_delta(id, warehouse, 100_000)
        .expect("stock");
    (id, unit_id)
}

fn seed_customer(harness: &Harness, name: &str) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Admin),
            NewCustomer {
                name: name.to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Disabled,
                credit_limit_centimes: 0,
                customer_tier: baraka_domain::sales::CustomerTier::Retail,
            },
            TEST_TERMINAL,
        )
        .expect("customer")
}

fn ensure_cash_session(harness: &Harness) {
    let conn = rusqlite::Connection::open(&harness.db_path).expect("reopen");
    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO cash_sessions
         (id, warehouse_id, terminal_id, status, opened_by, opened_at,
          opening_float_centimes, revision, origin_device_id, created_at, updated_at)
         SELECT ?1, id, ?2, 'OPEN', ?3, ?4, 0, 1, ?2, ?4, ?4
         FROM warehouses WHERE code = 'MAIN'",
        rusqlite::params![session_id, TEST_TERMINAL, ACTOR_UUID, STAMP],
    )
    .expect("cash session");
}

/// One single-line cash checkout; override/discount shape the transacted legs.
fn checkout(
    harness: &Harness,
    warehouse: WarehouseId,
    customer: Option<CustomerId>,
    product: ProductId,
    unit: UnitId,
    unit_price_override: Option<i64>,
    line_discount_centimes: i64,
) -> String {
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
        &harness.users,
        &harness.hasher,
    )
    .execute(
        &session_with(Role::Admin),
        CheckoutInput {
            command_id: Uuid::new_v4().to_string(),
            warehouse_id: warehouse,
            customer_id: customer,
            doc_type: DocumentType::Ticket,
            language: DocumentLanguage::Fr,
            lines: vec![CartLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 10_000,
                unit_price_override,
                line_discount_centimes,
            }],
            global_discount_centimes: 0,
            payments: vec![PaymentInput {
                method: PaymentMethod::Cash,
                amount_centimes: 10_000,
            }],
            allow_negative_stock: false,
            below_cost_override: None,
            notes: None,
            timbre_enabled: false,
            cart_opened_at: None,
        },
        TEST_TERMINAL,
    )
    .expect("checkout")
    .document_id
}

fn recall(
    harness: &Harness,
    customer: CustomerId,
    product: ProductId,
    unit: UnitId,
) -> Option<CustomerLastPrice> {
    GetLastPriceUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Admin),
            customer,
            product.to_string(),
            unit.to_string(),
        )
        .expect("recall")
}

#[test]
fn named_sale_records_the_transacted_legs() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    ensure_cash_session(&harness);
    let (product, unit) = seed_product(&harness, warehouse, "SLIP-M", 10_000);
    let customer = seed_customer(&harness, "Grossiste");

    let document_id = checkout(
        &harness,
        warehouse,
        Some(customer),
        product,
        unit,
        Some(9_500),
        500,
    );

    let cell = recall(&harness, customer, product, unit).expect("memory cell");
    assert_eq!(cell.price_centimes, 9_500);
    assert_eq!(cell.discount_per_unit_centimes, 500);
    assert_eq!(cell.document_id, document_id);
    assert!(!cell.sold_at.is_empty());
}

#[test]
fn later_sale_overwrites_and_guests_record_nothing() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    ensure_cash_session(&harness);
    let (product, unit) = seed_product(&harness, warehouse, "SLIP-L", 10_000);
    let (other_product, other_unit) = seed_product(&harness, warehouse, "SLIP-XL", 12_000);
    let customer = seed_customer(&harness, "Grossiste");
    let stranger = seed_customer(&harness, "Passant");

    checkout(
        &harness,
        warehouse,
        Some(customer),
        product,
        unit,
        Some(9_500),
        0,
    );
    // Guest sale: same product, no customer — must not touch the cell.
    // (Above cost so the Margin Guard stays out of this memory probe.)
    checkout(&harness, warehouse, None, product, unit, Some(6_000), 0);
    // Later cheaper sale wins.
    checkout(
        &harness,
        warehouse,
        Some(customer),
        product,
        unit,
        Some(8_000),
        200,
    );

    let cell = recall(&harness, customer, product, unit).expect("memory cell");
    assert_eq!(cell.price_centimes, 8_000);
    assert_eq!(cell.discount_per_unit_centimes, 200);
    // Isolation: another customer and a never-sold product read `None`.
    assert!(recall(&harness, stranger, product, unit).is_none());
    assert!(recall(&harness, customer, other_product, other_unit).is_none());
}
