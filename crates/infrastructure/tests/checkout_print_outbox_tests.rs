//! Checkout print-outbox pins (TASK-BRK-078).
//!
//! AUD-038 asserted `sales_checkout` never enqueues a print job. That premise
//! is STALE: step 8 of `checkout_atomic`
//! (`sqlite_checkout_repository.rs`, present since the 05f0f39 baseline)
//! inserts exactly one PENDING `RECEIPT` job inside the sale transaction,
//! and the `CheckoutRepository` port documents "print outbox" as part of the
//! atomic unit of work. These tests pin the correct behavior so no future
//! refactor can silently drop the enqueue:
//! committed sale → exactly 1 PENDING job; replay → still 1 (same receipt);
//! failed sale → 0 jobs.

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

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const TEST_TERMINAL: &str = "TEST-TERMINAL-OUTBOX";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("outbox.db")).expect("open");
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
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
    }
}

fn session_gerant() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        role: Role::Gerant,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed_product(harness: &Harness) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("OUTBOX-1").expect("sku"),
                barcode: None,
                name_ar: "صنف صندوق".to_string(),
                name_fr: "Article Outbox".to_string(),
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
                barcode: Some(Barcode::new("4001").expect("barcode")),
                sale_price: Money::from_centimes(15_000),
                purchase_price: Money::from_centimes(12_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            "TEST-TERMINAL",
        )
        .expect("product");
    (id, unit_id)
}

fn seed_warehouse(harness: &Harness) -> WarehouseId {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("OUTBOX").expect("code"),
        name: "Outbox".to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, "TEST-TERMINAL")
        .expect("wh");
    warehouse.id
}

fn ensure_cash_session(harness: &Harness, warehouse: WarehouseId) {
    let guard = harness.db.lock().expect("lock");
    let session_id = Uuid::new_v4().to_string();
    guard
        .execute(
            "INSERT INTO cash_sessions
             (id, warehouse_id, terminal_id, status, opened_by, opened_at,
              opening_float_centimes, revision, origin_device_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![session_id, warehouse.to_string(), TEST_TERMINAL, ACTOR_UUID,],
        )
        .expect("open session");
}

fn pending_job_count(harness: &Harness) -> i64 {
    let guard = harness.db.lock().expect("lock");
    guard
        .query_row(
            "SELECT COUNT(*) FROM print_jobs WHERE status = 'PENDING'",
            [],
            |row| row.get(0),
        )
        .expect("count")
}

fn commit_ticket(
    harness: &Harness,
    warehouse: WarehouseId,
    product: ProductId,
    unit: UnitId,
    command_id: String,
    quantity_micro: i64,
) -> Result<String, baraka_application::errors::ApplicationError> {
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &session_gerant(),
        CheckoutInput {
            command_id,
            warehouse_id: warehouse,
            customer_id: None,
            doc_type: DocumentType::Ticket,
            language: DocumentLanguage::Fr,
            lines: vec![CartLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro,
                unit_price_override: None,
                line_discount_centimes: 0,
            }],
            global_discount_centimes: 0,
            payments: vec![PaymentInput {
                method: PaymentMethod::Cash,
                amount_centimes: 50_000,
            }],
            allow_negative_stock: false,
            notes: None,
            timbre_enabled: true,
        },
        TEST_TERMINAL,
    )
    .map(|receipt| receipt.document_id)
}

/// Committed sale leaves exactly one PENDING print job; replay adds none.
#[test]
fn committed_sale_enqueues_exactly_one_pending_job() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness);
    let (product, unit) = seed_product(&harness);
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, 100_000)
        .expect("stock");
    ensure_cash_session(&harness, warehouse);
    assert_eq!(pending_job_count(&harness), 0);

    let command_id = Uuid::new_v4().to_string();
    let first = commit_ticket(
        &harness,
        warehouse,
        product,
        unit,
        command_id.clone(),
        20_000,
    )
    .expect("checkout");
    assert_eq!(pending_job_count(&harness), 1);
    let (job_type, document_id): (String, Option<String>) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT job_type, document_id FROM print_jobs WHERE status = 'PENDING'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("job row");
    assert_eq!(job_type, "RECEIPT");
    assert_eq!(document_id.as_deref(), Some(first.as_str()));

    // Replay with the same command_id: same receipt, no second job.
    let second =
        commit_ticket(&harness, warehouse, product, unit, command_id, 20_000).expect("replay");
    assert_eq!(second, first);
    assert_eq!(pending_job_count(&harness), 1);
}

/// Failed sale (insufficient stock) leaves zero jobs.
#[test]
fn failed_sale_enqueues_no_job() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness);
    let (product, unit) = seed_product(&harness);
    // No stock seeded: sale must fail the BRK-033 shortage gate.
    ensure_cash_session(&harness, warehouse);
    let result = commit_ticket(
        &harness,
        warehouse,
        product,
        unit,
        Uuid::new_v4().to_string(),
        20_000,
    );
    assert!(result.is_err(), "short sale must fail, got {result:?}");
    assert_eq!(pending_job_count(&harness), 0);
}
