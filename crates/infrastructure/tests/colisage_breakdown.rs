//! TASK-BRK-109: Colisage breakdown over real SQLite.
//!
//! A document line stores its quantity in base micros; merchants think in
//! selling rungs. This suite proves `get_items_colisage` translates one into
//! the other on an isolated scratch database:
//!
//! * 150 pieces on [Pièce ×1, Demi-Dz ×6, Douzaine ×12] → 12 × Douzaine +
//!   1 × Demi-Dz (piece rung omitted, largest first, live unit names);
//! * 12.5 m of cable on [Mètre, Rouleau ×100] → empty vector (sub-rung
//!   fraction is refused, never rounded; surfaces show the base quantity).

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    DocumentRepository, ProductRepository, SessionContext, SessionToken, StockRepository,
    WarehouseRepository,
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

const ACTOR_UUID: &str = "88888888-8888-8888-8888-888888888888";
const TEST_TERMINAL: &str = "TEST-TERMINAL-BRK109";
const STAMP: &str = "2026-11-12T10:00:00Z";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    documents: baraka_infrastructure::adapters::SqliteDocumentRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("colisage.db");
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
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
        documents: baraka_infrastructure::adapters::SqliteDocumentRepository::new(shared),
    }
}

fn session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        role: Role::Admin,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed_warehouse(harness: &Harness) -> WarehouseId {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("MAIN").expect("code"),
        name: "MAIN".to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, TEST_TERMINAL)
        .expect("warehouse");
    warehouse.id
}

fn ensure_cash_session(harness: &Harness) {
    use rusqlite::Connection;
    let db_path = harness._dir.path().join("colisage.db");
    let conn = Connection::open(&db_path).expect("reopen");
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

/// Product with an explicit multi-rung ladder; returns the default unit.
fn seed_laddered(
    harness: &Harness,
    sku: &str,
    rungs: &[(&str, &str, i64, i64)],
) -> (ProductId, UnitId) {
    let id = ProductId::new();
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
    let mut units = Vec::new();
    let mut default = None;
    for (index, (fr, ar, factor, price)) in rungs.iter().enumerate() {
        let unit_id = UnitId::new();
        if index == 0 {
            default = Some(unit_id);
        }
        units.push(ProductUnit {
            id: unit_id,
            product_id: id,
            unit_name_ar: (*ar).to_string(),
            unit_name_fr: (*fr).to_string(),
            factor_micro: FactorMicro::from_micros(*factor),
            barcode: Some(Barcode::new(&format!("611109{sku}{index}")).expect("barcode")),
            sale_price: Money::from_centimes(*price),
            purchase_price: Money::from_centimes(price / 2),
            wholesale_price: None,
            super_gros_price: None,
            is_base_unit: index == 0,
            is_default_selling: index == 0,
        });
    }
    harness
        .catalog
        .create_product(&product, &units, TEST_TERMINAL)
        .expect("product");
    (id, default.expect("default"))
}

#[test]
fn document_lines_break_down_to_whole_rungs() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness);
    ensure_cash_session(&harness);
    let (slip, piece) = seed_laddered(
        &harness,
        "SLIP-M",
        &[
            ("Pièce", "قطعة", 10_000, 10_000),
            ("Demi-Douzaine", "نصف دزينة", 60_000, 60_000),
            ("Douzaine", "دزينة", 120_000, 120_000),
        ],
    );
    let (cable, metre) = seed_laddered(
        &harness,
        "CABLE-10",
        &[
            ("Mètre", "متر", 10_000, 1_000),
            ("Rouleau", "لفافة", 1_000_000, 90_000),
        ],
    );
    harness
        .stock
        .upsert_stock_level_with_delta(slip, warehouse, 10_000_000)
        .expect("stock");
    harness
        .stock
        .upsert_stock_level_with_delta(cable, warehouse, 10_000_000)
        .expect("stock");

    // 150 × 100.00 pieces + 12.5 × 10.00 metres, all cash.
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &session(),
        CheckoutInput {
            command_id: Uuid::new_v4().to_string(),
            warehouse_id: warehouse,
            customer_id: None,
            doc_type: DocumentType::Ticket,
            language: DocumentLanguage::Fr,
            lines: vec![
                CartLineInput {
                    product_id: slip,
                    unit_id: piece,
                    quantity_micro: 1_500_000,
                    unit_price_override: None,
                    line_discount_centimes: 0,
                },
                CartLineInput {
                    product_id: cable,
                    unit_id: metre,
                    quantity_micro: 125_000,
                    unit_price_override: None,
                    line_discount_centimes: 0,
                },
            ],
            global_discount_centimes: 0,
            payments: vec![PaymentInput {
                method: PaymentMethod::Cash,
                amount_centimes: 1_512_500,
            }],
            allow_negative_stock: false,
            notes: None,
            timbre_enabled: false,
        },
        TEST_TERMINAL,
    )
    .expect("checkout");

    let colisage = harness
        .documents
        .get_items_colisage(baraka_domain::sales::DocumentId::from(
            Uuid::parse_str(&receipt.document_id).expect("uuid"),
        ))
        .expect("colisage");
    assert_eq!(colisage.len(), 2);

    // 150 pieces = 12 dozen + 1 half-dozen, piece rung omitted.
    assert_eq!(
        colisage[0]
            .iter()
            .map(|share| (share.unit_name_fr.as_str(), share.count))
            .collect::<Vec<_>>(),
        vec![("Douzaine", 12), ("Demi-Douzaine", 1)]
    );
    assert!(colisage[0].iter().all(|share| share.factor_micro > 0));

    // 12.5 m has no whole-rung expression: empty, never rounded.
    assert_eq!(colisage[1], Vec::new());
}
