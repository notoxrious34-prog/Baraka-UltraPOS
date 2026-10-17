//! Sale idempotency tests: exact replay, payload conflict, claim race.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    ProductRepository, SessionContext, SessionToken, StockRepository, WarehouseRepository,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{DocumentLanguage, DocumentType, PaymentMethod};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("idem.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('55555555-5555-5555-5555-555555555555', 'vendeur1', 'hash', 'VENDEUR',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
    }
}

fn session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(
            Uuid::parse_str("55555555-5555-5555-5555-555555555555").expect("uuid"),
        ),
        role: Role::Vendeur,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed_sale_context(harness: &Harness) -> (WarehouseId, ProductId, UnitId) {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("MAIN").expect("code"),
        name: "Main".to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, "TEST-TERMINAL")
        .expect("wh");
    let id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("P1").expect("sku"),
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
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: Some(Barcode::new("611125P1").expect("barcode")),
                sale_price: Money::from_centimes(10_000),
                purchase_price: Money::from_centimes(8_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            "TEST-TERMINAL",
        )
        .expect("product");
    harness
        .stock
        .upsert_stock_level_with_delta(id, warehouse.id, 100_000)
        .expect("stock");
    (warehouse.id, id, unit_id)
}

fn input_for(
    warehouse: WarehouseId,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    cash: i64,
    command_id: &str,
) -> CheckoutInput {
    CheckoutInput {
        command_id: command_id.to_string(),
        warehouse_id: warehouse,
        customer_id: None,
        doc_type: DocumentType::Ticket,
        language: DocumentLanguage::Fr,
        lines: vec![CartLineInput {
            product_id: product,
            unit_id: unit,
            quantity_micro: qty_units * 10_000,
            unit_price_override: None,
            line_discount_centimes: 0,
        }],
        global_discount_centimes: 0,
        payments: vec![PaymentInput {
            method: PaymentMethod::Cash,
            amount_centimes: cash,
        }],
        allow_negative_stock: false,
        notes: None,
        timbre_enabled: true,
    }
}

fn checkout(
    harness: &Harness,
    input: CheckoutInput,
) -> Result<baraka_application::ports::CheckoutReceipt, baraka_application::errors::ApplicationError>
{
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(&session(), input, TEST_TERMINAL)
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-IDEM";

fn ensure_cash_session(harness: &Harness, warehouse: WarehouseId) {
    let conn = rusqlite::Connection::open(harness._dir.path().join("idem.db")).expect("reopen");
    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO cash_sessions
         (id, warehouse_id, terminal_id, status, opened_by, opened_at,
          opening_float_centimes, revision, origin_device_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'OPEN', '55555555-5555-5555-5555-555555555555',
                 '2026-01-01T00:00:00Z', 0, 1, ?3,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![session_id, warehouse.to_string(), TEST_TERMINAL,],
    )
    .expect("open session");
    conn.execute(
        "INSERT INTO cash_movements
         (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
         VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z',
                 '55555555-5555-5555-5555-555555555555')",
        rusqlite::params![Uuid::new_v4().to_string(), session_id,],
    )
    .expect("opening float");
}

fn count(harness: &Harness, table: &str) -> i64 {
    rusqlite::Connection::open(harness._dir.path().join("idem.db"))
        .expect("reopen")
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

#[test]
fn replay_returns_cached_response_without_side_effects() {
    let harness = setup();
    let (warehouse, product, unit) = seed_sale_context(&harness);
    ensure_cash_session(&harness, warehouse);
    let command_id = Uuid::new_v4().to_string();
    let first = checkout(
        &harness,
        input_for(warehouse, product, unit, 2, 20_500, &command_id),
    )
    .expect("first");
    let second = checkout(
        &harness,
        input_for(warehouse, product, unit, 2, 20_500, &command_id),
    )
    .expect("replay");
    assert_eq!(first, second, "replay must be byte-identical");
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "stock_movements"), 1);
    assert_eq!(count(&harness, "payments"), 1);
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        80_000,
        "stock decremented exactly once"
    );
    let conn = rusqlite::Connection::open(harness._dir.path().join("idem.db")).expect("reopen");
    let last: i64 = conn
        .query_row(
            "SELECT last_sequence FROM document_sequences WHERE type = 'TICKET'",
            [],
            |row| row.get(0),
        )
        .expect("sequence");
    assert_eq!(last, 1, "no second number consumed");
}

#[test]
fn same_command_different_payload_conflicts() {
    let harness = setup();
    let (warehouse, product, unit) = seed_sale_context(&harness);
    ensure_cash_session(&harness, warehouse);
    let command_id = Uuid::new_v4().to_string();
    checkout(
        &harness,
        input_for(warehouse, product, unit, 2, 20_500, &command_id),
    )
    .expect("first");
    let conflict = checkout(
        &harness,
        input_for(warehouse, product, unit, 5, 60_000, &command_id),
    )
    .expect_err("conflict");
    assert!(
        matches!(
            conflict,
            baraka_application::errors::ApplicationError::IdempotencyConflict
        ),
        "unexpected: {conflict:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        80_000,
        "conflicting payload moved nothing"
    );
}

#[test]
fn concurrent_same_command_yields_single_document() {
    let harness = setup();
    let (warehouse, product, unit) = seed_sale_context(&harness);
    ensure_cash_session(&harness, warehouse);
    let command_id = Uuid::new_v4().to_string();
    let shared = Arc::new(harness);
    let mut handles = Vec::new();
    for _ in 0..2 {
        let harness = Arc::clone(&shared);
        let command_id = command_id.clone();
        handles.push(std::thread::spawn(move || {
            checkout(
                &harness,
                input_for(warehouse, product, unit, 2, 20_500, &command_id),
            )
        }));
    }
    let mut receipts = Vec::new();
    for handle in handles {
        receipts.push(handle.join().expect("thread").expect("checkout"));
    }
    assert_eq!(receipts[0], receipts[1], "both callers see one receipt");
    assert_eq!(count(&shared, "documents"), 1);
    assert_eq!(
        shared
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        80_000
    );
}
