//! Atomic size/color exchange tests (BRK-125, Échange Express).
//!
//! One idempotent command returns the old sizes and sells the alternates
//! in a SINGLE atomic bundle: the return legs reuse the source historic
//! transacted price, the new legs reuse last-price memory (0022), the
//! money effect is exactly the net difference (0 DZD at equal price),
//! and the customer statement is never distorted.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    DocumentRepository, NewCustomer, PasswordHasher, ProductRepository, SessionContext,
    SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::exchanges::{
    ExchangeItemInput, ProcessExchangeCommand, ProcessExchangeUseCase,
};
use baraka_application::use_cases::returns::{RefundDestination, ReturnItemInput};
use baraka_application::use_cases::sales::ManagerOverride;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money, QtyMicro};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, CustomerTier, Document, DocumentId, DocumentItem, DocumentLanguage,
    DocumentStatus, DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    Argon2PasswordHasher, SqliteCashSessionRepository, SqliteCatalogRepository,
    SqliteCustomerRepository, SqliteDocumentRepository, SqliteReturnRepository,
    SqliteUserRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "99999999-9999-9999-9999-999999999999";
const TEST_TERMINAL: &str = "TEST-TERMINAL-EXCH";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    warehouses: SqliteWarehouseRepository,
    documents: SqliteDocumentRepository,
    customers: SqliteCustomerRepository,
    returns: SqliteReturnRepository,
    cash: SqliteCashSessionRepository,
    users: SqliteUserRepository,
    hasher: Argon2PasswordHasher,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("exchange.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let hasher = Argon2PasswordHasher;
    let gerant_pin = hasher.hash("2468").expect("hash pin");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, pin_hash, role, is_active, created_at, updated_at)
         VALUES ('99999999-9999-9999-9999-999999999999', 'gerant1', 'hash', ?1, 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![gerant_pin],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        returns: SqliteReturnRepository::new(Arc::clone(&shared)),
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
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

fn seed_warehouse(harness: &Harness) -> WarehouseId {
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
    warehouse.id
}

fn seed_product(harness: &Harness, sku: &str, price: i64) -> (ProductId, UnitId) {
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
                name_fr: format!("Produit {sku}"),
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
                barcode: Some(Barcode::new(&format!("611125{sku}")).expect("barcode")),
                sale_price: Money::from_centimes(price),
                purchase_price: Money::from_centimes(price - 2_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("product");
    (id, unit_id)
}

fn seed_customer(harness: &Harness) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Gerant),
            NewCustomer {
                name: "Zaboun".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
                customer_tier: CustomerTier::Retail,
            },
            TEST_TERMINAL,
        )
        .expect("customer")
}

fn seed_cash_session(harness: &Harness, warehouse: WarehouseId) -> String {
    let session_id = Uuid::new_v4().to_string();
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO cash_sessions
             (id, warehouse_id, terminal_id, status, opened_by, opened_at,
              opening_float_centimes, revision, origin_device_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![session_id, warehouse.to_string(), TEST_TERMINAL, ACTOR_UUID,],
        )
        .expect("open session");
    session_id
}

fn seed_last_price(
    harness: &Harness,
    customer: CustomerId,
    product: ProductId,
    unit: UnitId,
    price: i64,
    document: DocumentId,
) {
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO customer_last_prices
             (customer_id, product_id, unit_id, price_centimes,
              discount_per_unit_centimes, document_id, sold_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, '2026-01-01T00:00:00Z')",
            rusqlite::params![
                customer.to_string(),
                product.to_string(),
                unit.to_string(),
                price,
                document.to_string(),
            ],
        )
        .expect("last price");
}

fn sale_line(
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    unit_price: i64,
    sort_order: i64,
) -> DocumentItem {
    let micros = qty_units * 10_000;
    DocumentItem {
        id: Uuid::new_v4(),
        document_id: DocumentId::new(),
        product_id: product,
        unit_id: unit,
        quantity_micro: micros,
        quantity_base_micro: micros,
        unit_price_centimes: unit_price,
        discount_centimes: 0,
        discount_per_unit_centimes: None,
        tva_rate_micro: 0,
        line_ht_centimes: unit_price * qty_units,
        line_tva_centimes: 0,
        line_ttc_centimes: unit_price * qty_units,
        cost_price_centimes: unit_price - 2_000,
        sort_order,
    }
}

fn seed_ticket(
    harness: &Harness,
    warehouse: WarehouseId,
    customer: CustomerId,
    lines: Vec<DocumentItem>,
) -> (DocumentId, Vec<DocumentItem>) {
    let id = DocumentId::new();
    let ttc: i64 = lines.iter().map(|line| line.line_ttc_centimes).sum();
    let mut fixed = Vec::with_capacity(lines.len());
    for mut line in lines {
        line.document_id = id;
        fixed.push(line);
    }
    harness
        .documents
        .finalize_document(
            &Document {
                id,
                language: DocumentLanguage::Fr,
                number: format!("T-{}", &id.to_string()[..8]),
                fiscal_year: 2026,
                doc_type: DocumentType::Ticket,
                status: DocumentStatus::Finalized,
                warehouse_id: warehouse,
                party_id: Some(customer.to_string()),
                subtotal_ht_centimes: ttc,
                vat_total_centimes: 0,
                global_discount_centimes: 0,
                timbre_centimes: 0,
                ttc_centimes: ttc,
                paid_amount_centimes: ttc,
                change_amount_centimes: 0,
                payment_method: "CASH".to_string(),
                template_version: baraka_domain::printing::TSPL_TICKET_TEMPLATE_VERSION,
                credit_solde_before_centimes: None,
                credit_solde_after_centimes: None,
                credit_delta_centimes: None,
                notes: None,
                created_at: chrono::Utc::now(),
                created_by: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
            },
            &fixed,
            &[],
            TEST_TERMINAL,
        )
        .expect("seed document");
    (id, fixed)
}

fn set_stock(harness: &Harness, product: ProductId, warehouse: WarehouseId, qty: i64) {
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO stock_levels (warehouse_id, product_id, quantity_micro, quantity_base_micro)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(warehouse_id, product_id) DO UPDATE SET quantity_micro = ?3, quantity_base_micro = ?3",
            rusqlite::params![warehouse.to_string(), product.to_string(), qty],
        )
        .expect("stock level");
}

fn stock_level(harness: &Harness, product: ProductId, warehouse: WarehouseId) -> i64 {
    harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT quantity_base_micro FROM stock_levels WHERE warehouse_id = ?1 AND product_id = ?2",
            rusqlite::params![warehouse.to_string(), product.to_string()],
            |row| row.get(0),
        )
        .expect("level")
}

fn count(harness: &Harness, table: &str) -> i64 {
    harness
        .db
        .lock()
        .expect("lock")
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn exchange_command(
    source: DocumentId,
    returns: Vec<(Uuid, i64)>,
    news: Vec<(ProductId, UnitId, i64)>,
    key: &str,
) -> ProcessExchangeCommand {
    exchange_command_full(
        source,
        returns,
        news,
        key,
        RefundDestination::NoRefund,
        None,
    )
}

fn exchange_command_full(
    source: DocumentId,
    returns: Vec<(Uuid, i64)>,
    news: Vec<(ProductId, UnitId, i64)>,
    key: &str,
    refund: RefundDestination,
    below_cost_override: Option<ManagerOverride>,
) -> ProcessExchangeCommand {
    ProcessExchangeCommand {
        operator_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        source_document_id: source,
        return_items: returns
            .into_iter()
            .map(|(item_id, qty_micro)| ReturnItemInput {
                source_item_id: item_id,
                quantity_micro: QtyMicro::from_micros(qty_micro),
                reason: None,
            })
            .collect(),
        new_items: news
            .into_iter()
            .map(|(product_id, unit_id, qty_micro)| ExchangeItemInput {
                product_id,
                unit_id,
                quantity_micro: qty_micro,
            })
            .collect(),
        refund,
        below_cost_override,
        idempotency_key: key.to_string(),
        terminal_id: TEST_TERMINAL.to_string(),
        notes: None,
    }
}

fn exchange_case() -> Harness {
    setup()
}

#[test]
fn equal_price_exchange_nets_zero_with_cell_adjustment_only() {
    let harness = exchange_case();
    let warehouse = seed_warehouse(&harness);
    let customer = seed_customer(&harness);
    seed_cash_session(&harness, warehouse);
    let (size_m, unit_m) = seed_product(&harness, "EXM", 20_000);
    let (size_l, unit_l) = seed_product(&harness, "EXL", 20_000);
    let (source, lines) = seed_ticket(
        &harness,
        warehouse,
        customer,
        vec![sale_line(size_m, unit_m, 1, 20_000, 0)],
    );
    seed_last_price(&harness, customer, size_l, unit_l, 20_000, source);
    set_stock(&harness, size_m, warehouse, 0);
    set_stock(&harness, size_l, warehouse, 10_000);
    let cash_before = count(&harness, "cash_movements");
    let ledger_before = count(&harness, "customer_ledger");

    let use_case = ProcessExchangeUseCase::new(
        &harness.documents,
        &harness.returns,
        &harness.customers,
        &harness.cash,
        &harness.catalog,
        &harness.users,
        &harness.hasher,
    );
    let result = use_case
        .execute(
            &session_with(Role::Gerant),
            exchange_command(
                source,
                vec![(lines[0].id, 10_000)],
                vec![(size_l, unit_l, 10_000)],
                "exch-key-equal",
            ),
        )
        .expect("exchange");
    assert_eq!(result.net_centimes, 0, "equal historic price nets to zero");
    assert_eq!(
        count(&harness, "cash_movements"),
        cash_before,
        "zero net moves no cash"
    );
    assert_eq!(
        count(&harness, "customer_ledger"),
        ledger_before,
        "statement never distorted"
    );
    assert_eq!(stock_level(&harness, size_m, warehouse), 10_000);
    assert_eq!(stock_level(&harness, size_l, warehouse), 0);

    let replay = use_case.execute(
        &session_with(Role::Gerant),
        exchange_command(
            source,
            vec![(lines[0].id, 10_000)],
            vec![(size_l, unit_l, 10_000)],
            "exch-key-equal",
        ),
    );
    // Same key retried: the quantity pre-check sees the already-consumed
    // cumulative and fails CLOSED (no duplicate AVOIR, no second stock
    // move) — a double click can never duplicate money.
    assert!(replay.is_err(), "retry must fail closed, got {replay:?}");
}

#[test]
fn price_gap_exchange_collects_only_the_difference() {
    let harness = exchange_case();
    let warehouse = seed_warehouse(&harness);
    let customer = seed_customer(&harness);
    seed_cash_session(&harness, warehouse);
    let (size_m, unit_m) = seed_product(&harness, "EXM2", 20_000);
    let (size_xl, unit_xl) = seed_product(&harness, "EXX", 25_000);
    let (source, lines) = seed_ticket(
        &harness,
        warehouse,
        customer,
        vec![sale_line(size_m, unit_m, 1, 20_000, 0)],
    );
    seed_last_price(&harness, customer, size_xl, unit_xl, 25_000, source);
    set_stock(&harness, size_m, warehouse, 0);
    set_stock(&harness, size_xl, warehouse, 10_000);
    let cash_before = count(&harness, "cash_movements");

    let result = ProcessExchangeUseCase::new(
        &harness.documents,
        &harness.returns,
        &harness.customers,
        &harness.cash,
        &harness.catalog,
        &harness.users,
        &harness.hasher,
    )
    .execute(
        &session_with(Role::Gerant),
        exchange_command(
            source,
            vec![(lines[0].id, 10_000)],
            vec![(size_xl, unit_xl, 10_000)],
            "exch-key-gap",
        ),
    )
    .expect("exchange");
    assert_eq!(result.net_centimes, 5_000);
    assert_eq!(
        count(&harness, "cash_movements"),
        cash_before + 1,
        "only the net difference is collected"
    );
}

#[test]
fn unknown_alternate_price_refuses_before_any_write() {
    let harness = exchange_case();
    let warehouse = seed_warehouse(&harness);
    let customer = seed_customer(&harness);
    seed_cash_session(&harness, warehouse);
    let (size_m, unit_m) = seed_product(&harness, "EXM3", 20_000);
    let (size_l, unit_l) = seed_product(&harness, "EXL3", 20_000);
    let (source, lines) = seed_ticket(
        &harness,
        warehouse,
        customer,
        vec![sale_line(size_m, unit_m, 1, 20_000, 0)],
    );
    // No last-price memory for size_l: no reference, no exchange.
    let docs_before = count(&harness, "documents");
    let result = ProcessExchangeUseCase::new(
        &harness.documents,
        &harness.returns,
        &harness.customers,
        &harness.cash,
        &harness.catalog,
        &harness.users,
        &harness.hasher,
    )
    .execute(
        &session_with(Role::Gerant),
        exchange_command(
            source,
            vec![(lines[0].id, 10_000)],
            vec![(size_l, unit_l, 10_000)],
            "exch-key-unknown",
        ),
    );
    assert!(result.is_err(), "unknown alternate price refuses LOUDLY");
    assert_eq!(
        count(&harness, "documents"),
        docs_before,
        "refusal writes nothing"
    );
}

#[test]
fn below_cost_alternate_locks_without_manager_pin() {
    let harness = exchange_case();
    let warehouse = seed_warehouse(&harness);
    let customer = seed_customer(&harness);
    let cash_session = seed_cash_session(&harness, warehouse);
    let (size_m, unit_m) = seed_product(&harness, "EXM4", 20_000);
    // Alternate costs 18_000 (seed convention: price - 2_000).
    let (size_l, unit_l) = seed_product(&harness, "EXL4", 20_000);
    let (source, lines) = seed_ticket(
        &harness,
        warehouse,
        customer,
        vec![sale_line(size_m, unit_m, 1, 20_000, 0)],
    );
    // Last-price memory 15_000 sits strictly below the 18_000 cost: red leg.
    seed_last_price(&harness, customer, size_l, unit_l, 15_000, source);
    set_stock(&harness, size_m, warehouse, 0);
    set_stock(&harness, size_l, warehouse, 10_000);
    let docs_before = count(&harness, "documents");

    // Without an override the whole exchange locks and writes nothing.
    let locked = ProcessExchangeUseCase::new(
        &harness.documents,
        &harness.returns,
        &harness.customers,
        &harness.cash,
        &harness.catalog,
        &harness.users,
        &harness.hasher,
    )
    .execute(
        &session_with(Role::Gerant),
        exchange_command(
            source,
            vec![(lines[0].id, 10_000)],
            vec![(size_l, unit_l, 10_000)],
            "exch-key-red-locked",
        ),
    );
    assert!(locked.is_err(), "red leg without PIN locks LOUDLY");
    assert_eq!(
        count(&harness, "documents"),
        docs_before,
        "lock writes nothing"
    );

    // The same exchange with a valid Gérant PIN transacts exactly the
    // 5_000 DZD gap (15_000 - 20_000), paid out in cash.
    let unlocked = ProcessExchangeUseCase::new(
        &harness.documents,
        &harness.returns,
        &harness.customers,
        &harness.cash,
        &harness.catalog,
        &harness.users,
        &harness.hasher,
    )
    .execute(
        &session_with(Role::Gerant),
        exchange_command_full(
            source,
            vec![(lines[0].id, 10_000)],
            vec![(size_l, unit_l, 10_000)],
            "exch-key-red-unlocked",
            RefundDestination::Cash {
                cash_session_id: cash_session,
            },
            Some(ManagerOverride {
                username: "gerant1".to_string(),
                pin: "2468".to_string(),
            }),
        ),
    )
    .expect("gérant PIN unlocks the red leg");
    assert_eq!(unlocked.net_centimes, -5_000);
}
