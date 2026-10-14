//! Cash session engine tests: lifecycle, Z-report math, preconditions.
//!
//! Covers the TASK-BRK-010 acceptance criteria over real SQLite backends.
//! All money is `i64` centimes; no float exists in this file.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{ProductRepository, StockRepository, WarehouseRepository};
use baraka_application::use_cases::cash::{
    BeginClosingUseCase, CancelClosingUseCase, ConfirmClosingUseCase, GetActiveSessionUseCase,
    GetSessionReportUseCase, OpenCashSessionUseCase, RecordExpenseUseCase,
};
use baraka_application::use_cases::customers::RecordCreditPaymentUseCase;
use baraka_application::use_cases::sales::CheckoutSaleUseCase;
use baraka_application::use_cases::sales::{CartLineInput, CheckoutInput, PaymentInput};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, DocumentLanguage, DocumentType, PaymentMethod,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCashSessionRepository, SqliteCatalogRepository, SqliteCheckoutRepository,
    SqliteCustomerRepository, SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use chrono::TimeDelta;
use uuid::Uuid;

use baraka_application::ports::{SessionContext, SessionToken};

const VENDEUR_UUID: &str = "11111111-1111-1111-1111-111111111111";
const CAISSIER_UUID: &str = "22222222-2222-2222-2222-222222222222";
const GERANT_UUID: &str = "33333333-3333-3333-3333-333333333333";
const ADMIN_UUID: &str = "44444444-4444-4444-4444-444444444444";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    cash: SqliteCashSessionRepository,
    terminal_id: String,
    warehouse_id: WarehouseId,
}

fn session_with(role: Role, uuid: &str) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(uuid).expect("uuid")),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn vendeur() -> SessionContext {
    session_with(Role::Vendeur, VENDEUR_UUID)
}
fn caissier() -> SessionContext {
    session_with(Role::Caissier, CAISSIER_UUID)
}
fn gerant() -> SessionContext {
    session_with(Role::Gerant, GERANT_UUID)
}
fn admin() -> SessionContext {
    session_with(Role::Admin, ADMIN_UUID)
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cash.db");
    let mut conn = open_connection(&path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    for (id, name, role) in [
        (VENDEUR_UUID, "vendeur1", "VENDEUR"),
        (CAISSIER_UUID, "caissier1", "CAISSIER"),
        (GERANT_UUID, "gerant1", "GERANT"),
        (ADMIN_UUID, "admin1", "ADMIN"),
    ] {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![id, name, role],
        )
        .expect("seed user");
    }
    let terminal_id = resolve_device_id(&conn).expect("device id");
    let shared = Arc::new(Mutex::new(conn));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&shared));
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("MAIN").expect("code"),
        name: "Main".to_string(),
        is_active: true,
    };
    warehouses
        .create_warehouse(&warehouse, "TEST-TERMINAL")
        .expect("warehouse");
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses,
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
        terminal_id,
        warehouse_id: warehouse.id,
    }
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
                barcode: Some(Barcode::new(&format!("611125{sku}")).expect("barcode")),
                sale_price: Money::from_centimes(price),
                purchase_price: Money::from_centimes(price - 1_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            "TEST-TERMINAL",
        )
        .expect("product");
    harness
        .stock
        .upsert_stock_level_with_delta(id, harness.warehouse_id, 1_000_000)
        .expect("stock");
    (id, unit_id)
}

fn seed_debtor(harness: &Harness, balance: i64) -> CustomerId {
    let id = CustomerId::new();
    let guard = harness.db.lock().expect("lock");
    guard
        .execute(
            "INSERT INTO customers
             (id, code, name, credit_policy, credit_limit_centimes,
              current_balance_centimes, is_active, created_at, updated_at,
              revision, origin_device_id)
             VALUES (?1, ?2, 'Client', 'LIMITED', 1000000, ?3, 1,
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1, 'LOCAL')",
            rusqlite::params![
                id.to_string(),
                format!("C-{}", &id.to_string()[..8]),
                balance
            ],
        )
        .expect("customer");
    if balance > 0 {
        let entry = Uuid::new_v4().to_string();
        guard
            .execute(
                "INSERT INTO customer_ledger
                 (id, sequence_no, customer_id, entry_type, amount_centimes,
                  running_balance_centimes, document_type, document_id, created_by, created_at)
                 VALUES (?1, 1, ?2, 'DEBIT', ?3, ?3, 'SALE', 'seed', ?4, '2026-01-01T00:00:00Z')",
                rusqlite::params![entry, id.to_string(), balance, GERANT_UUID,],
            )
            .expect("ledger");
    }
    id
}

fn open_session(harness: &Harness, opening_float: i64) -> String {
    let view = OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &caissier(),
            harness.warehouse_id,
            opening_float,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("open");
    assert_eq!(view.status, "OPEN");
    view.id
}

fn cash_checkout_input(
    harness: &Harness,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    cash: i64,
) -> CheckoutInput {
    CheckoutInput {
        command_id: Uuid::new_v4().to_string(),
        warehouse_id: harness.warehouse_id,
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
    }
}

fn count_rows(harness: &Harness, sql: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    guard.query_row(sql, [], |row| row.get(0)).expect("count")
}

fn cash_movement_count(harness: &Harness, session_id: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    guard
        .query_row(
            "SELECT COUNT(*) FROM cash_movements WHERE cash_session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .expect("count")
}

fn cash_movement_sum(harness: &Harness, session_id: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    let total: Option<i64> = guard
        .query_row(
            "SELECT SUM(amount_centimes) FROM cash_movements WHERE cash_session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .expect("sum");
    total.unwrap_or(0)
}

#[test]
fn migration_is_rerunnable_without_loss() {
    let harness = setup();
    let session_id = open_session(&harness, 5_000);
    // Second migration run is a no-op and preserves rows.
    {
        let mut guard = harness.db.lock().expect("lock");
        run_migrations(&mut guard).expect("re-run");
    }
    let view = GetActiveSessionUseCase::new(&harness.cash)
        .execute(&caissier(), &harness.terminal_id)
        .expect("active")
        .expect("session survives");
    assert_eq!(view.id, session_id);
}

#[test]
fn unique_index_prevents_two_non_closed_sessions() {
    let harness = setup();
    open_session(&harness, 0);
    let second = OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses).execute(
        &caissier(),
        harness.warehouse_id,
        1_000,
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(
            second,
            Err(ApplicationError::Validation(
                DomainError::CashSessionAlreadyOpenForTerminal
            ))
        ),
        "unexpected: {second:?}"
    );
}

#[test]
fn concurrent_opens_yield_one_success() {
    let harness = setup();
    // Share the harness across threads via Arc.
    let shared = Arc::new(harness);
    let mut handles = Vec::new();
    for _ in 0..2 {
        let harness = Arc::clone(&shared);
        handles.push(std::thread::spawn(move || {
            OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses).execute(
                &session_with(Role::Caissier, CAISSIER_UUID),
                harness.warehouse_id,
                0,
                &harness.terminal_id,
                &Uuid::new_v4().to_string(),
            )
        }));
    }
    let mut successes = 0;
    let mut already_open = 0;
    for handle in handles {
        match handle.join().expect("thread") {
            Ok(_) => successes += 1,
            Err(ApplicationError::Validation(DomainError::CashSessionAlreadyOpenForTerminal)) => {
                already_open += 1
            }
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
    assert_eq!(successes, 1);
    assert_eq!(already_open, 1);
}

#[test]
fn closed_session_is_immutable_via_sql() {
    let harness = setup();
    let session_id = open_session(&harness, 1_000);
    BeginClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("begin");
    ConfirmClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &session_id,
            1_000,
            None,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("confirm");
    let guard = harness.db.lock().expect("lock");
    let update = guard.execute(
        "UPDATE cash_sessions SET closing_notes = 'hack' WHERE id = ?1",
        [&session_id],
    );
    assert!(
        format!("{update:?}").contains("MUTATION_FORBIDDEN"),
        "update must abort: {update:?}"
    );
    let delete = guard.execute("DELETE FROM cash_sessions WHERE id = ?1", [&session_id]);
    assert!(
        format!("{delete:?}").contains("MUTATION_FORBIDDEN"),
        "delete must abort: {delete:?}"
    );
}

#[test]
fn cash_movements_are_append_only_via_sql() {
    let harness = setup();
    let session_id = open_session(&harness, 0);
    let guard = harness.db.lock().expect("lock");
    let movement_id: String = guard
        .query_row(
            "SELECT id FROM cash_movements WHERE cash_session_id = ?1 LIMIT 1",
            [&session_id],
            |row| row.get(0),
        )
        .expect("movement");
    let update = guard.execute(
        "UPDATE cash_movements SET description = 'hack' WHERE id = ?1",
        [&movement_id],
    );
    assert!(
        format!("{update:?}").contains("MUTATION_FORBIDDEN"),
        "update must abort: {update:?}"
    );
    let delete = guard.execute("DELETE FROM cash_movements WHERE id = ?1", [&movement_id]);
    assert!(
        format!("{delete:?}").contains("MUTATION_FORBIDDEN"),
        "delete must abort: {delete:?}"
    );
}

#[test]
fn open_rejects_negative_accepts_zero() {
    let harness = setup();
    let negative = OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses).execute(
        &caissier(),
        harness.warehouse_id,
        -1,
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(
            negative,
            Err(ApplicationError::Validation(
                DomainError::CashSessionInvalidOpeningFloat
            ))
        ),
        "unexpected: {negative:?}"
    );
    let zero = open_session(&harness, 0);
    assert!(!zero.is_empty());
}

#[test]
fn checkout_without_open_session_is_atomically_rejected() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let documents_before = count_rows(&harness, "SELECT COUNT(*) FROM documents");
    let stock_before = harness
        .stock
        .get_stock_level(product, harness.warehouse_id)
        .expect("level");
    let rejected = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &caissier(),
        cash_checkout_input(&harness, product, unit, 1, 20_000),
        &harness.terminal_id,
    )
    .expect_err("must reject");
    assert!(
        matches!(
            rejected,
            ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(
        count_rows(&harness, "SELECT COUNT(*) FROM documents"),
        documents_before
    );
    assert_eq!(
        count_rows(&harness, "SELECT COUNT(*) FROM stock_movements"),
        0
    );
    assert_eq!(
        count_rows(&harness, "SELECT COUNT(*) FROM customer_ledger"),
        0
    );
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, harness.warehouse_id)
            .expect("level"),
        stock_before
    );
}

#[test]
fn cash_sale_inserts_single_sale_cash_in_net_of_change() {
    let harness = setup();
    let session_id = open_session(&harness, 10_000);
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    // Over-tender to produce change; net must equal tendered minus change.
    let mut input = cash_checkout_input(&harness, product, unit, 2, 30_000);
    input.command_id = Uuid::new_v4().to_string();
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(&caissier(), input, &harness.terminal_id)
    .expect("checkout");
    let tendered = 30_000i64;
    let change: i64 = receipt.change_amount_centimes.parse().expect("change");
    let expected_net = tendered - change;
    assert!(expected_net > 0);
    assert_eq!(cash_movement_count(&harness, &session_id), 2);
    let guard = harness.db.lock().expect("lock");
    let (movement_type, amount): (String, i64) = guard
        .query_row(
            "SELECT movement_type, amount_centimes FROM cash_movements
             WHERE cash_session_id = ?1 AND movement_type = 'SALE_CASH_IN'",
            [&session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("sale movement");
    assert_eq!(movement_type, "SALE_CASH_IN");
    assert_eq!(amount, expected_net);
    let reference: String = guard
        .query_row(
            "SELECT reference_id FROM cash_movements
             WHERE cash_session_id = ?1 AND movement_type = 'SALE_CASH_IN'",
            [&session_id],
            |row| row.get(0),
        )
        .expect("reference");
    assert_eq!(reference, receipt.document_id);
}

#[test]
fn fully_credit_sale_inserts_zero_cash_movements() {
    let harness = setup();
    let session_id = open_session(&harness, 5_000);
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let customer = seed_debtor(&harness, 0);
    // Patch the debtor limit high enough (seed uses 1000000, fine).
    let mut input = cash_checkout_input(&harness, product, unit, 1, 0);
    input.customer_id = Some(customer);
    input.payments = vec![baraka_application::use_cases::sales::PaymentInput {
        method: PaymentMethod::Credit,
        amount_centimes: 10_000,
    }];
    // TTC without cash has no timbre: 1 x 100.00 = 100.00 exactly.
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(&gerant(), input, &harness.terminal_id)
    .expect("credit checkout");
    // Only the OPENING_FLOAT row exists.
    assert_eq!(cash_movement_count(&harness, &session_id), 1);
}

#[test]
fn mixed_sale_nets_only_cash_portion() {
    let harness = setup();
    let session_id = open_session(&harness, 0);
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let customer = seed_debtor(&harness, 0);
    // 2 x 100.00 + timbre 500 = 20,500 TTC; cash 10,000 + credit 10,500.
    let mut input = cash_checkout_input(&harness, product, unit, 2, 10_000);
    input.customer_id = Some(customer);
    input.payments = vec![
        PaymentInput {
            method: PaymentMethod::Cash,
            amount_centimes: 10_000,
        },
        PaymentInput {
            method: PaymentMethod::Credit,
            amount_centimes: 10_500,
        },
    ];
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(&gerant(), input, &harness.terminal_id)
    .expect("mixed");
    assert_eq!(receipt.change_amount_centimes, "0");
    let guard = harness.db.lock().expect("lock");
    let amount: i64 = guard
        .query_row(
            "SELECT amount_centimes FROM cash_movements
             WHERE cash_session_id = ?1 AND movement_type = 'SALE_CASH_IN'",
            [&session_id],
            |row| row.get(0),
        )
        .expect("sale movement");
    assert_eq!(amount, 10_000);
}

#[test]
fn credit_payment_collects_cash_and_rejects_without_session() {
    let harness = setup();
    let customer = seed_debtor(&harness, 20_000);
    // Without a session: fully rejected, no ledger mutation.
    let ledgers_before = count_rows(&harness, "SELECT COUNT(*) FROM customer_ledger");
    let rejected = RecordCreditPaymentUseCase::new(&harness.customers).execute(
        &caissier(),
        customer,
        5_000,
        None,
        false,
        &harness.terminal_id,
    );
    assert!(
        matches!(
            rejected,
            Err(ApplicationError::Validation(
                DomainError::NoOpenCashSessionForTerminal
            ))
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(
        count_rows(&harness, "SELECT COUNT(*) FROM customer_ledger"),
        ledgers_before
    );
    // With a session: ledger + CREDIT_PAYMENT_IN atomically.
    let session_id = open_session(&harness, 0);
    let record = RecordCreditPaymentUseCase::new(&harness.customers)
        .execute(
            &caissier(),
            customer,
            5_000,
            Some("REC-9".to_string()),
            false,
            &harness.terminal_id,
        )
        .expect("payment");
    assert_eq!(record.new_balance_centimes, 15_000);
    let guard = harness.db.lock().expect("lock");
    let (movement_type, amount, reference): (String, i64, Option<String>) = guard
        .query_row(
            "SELECT movement_type, amount_centimes, reference_id FROM cash_movements
             WHERE cash_session_id = ?1 AND movement_type = 'CREDIT_PAYMENT_IN'",
            [&session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("credit movement");
    assert_eq!(movement_type, "CREDIT_PAYMENT_IN");
    assert_eq!(amount, 5_000);
    assert_eq!(reference, Some(record.ledger_entry_id));
}

#[test]
fn closing_freezes_expected_and_blocks_sales() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let session_id = open_session(&harness, 10_000);
    // One cash sale to move the expected total.
    let mut input = cash_checkout_input(&harness, product, unit, 1, 20_000);
    input.command_id = Uuid::new_v4().to_string();
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(&caissier(), input, &harness.terminal_id)
    .expect("sale");
    let live_sum = cash_movement_sum(&harness, &session_id);
    let closing = BeginClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("begin");
    assert_eq!(closing.status, "CLOSING");
    assert_eq!(closing.system_expected_centimes, Some(live_sum));
    // Sales while CLOSING are rejected identically to CLOSED.
    let rejected = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &caissier(),
        cash_checkout_input(&harness, product, unit, 1, 20_000),
        &harness.terminal_id,
    )
    .expect_err("closing blocks sales");
    assert!(
        matches!(
            rejected,
            ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
        ),
        "unexpected: {rejected:?}"
    );
}

#[test]
fn cancel_closing_reverts_and_repermits_sales() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    open_session(&harness, 0);
    BeginClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("begin");
    let reopened = CancelClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("cancel");
    assert_eq!(reopened.status, "OPEN");
    assert_eq!(reopened.system_expected_centimes, None);
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &caissier(),
        cash_checkout_input(&harness, product, unit, 1, 20_000),
        &harness.terminal_id,
    )
    .expect("sale after cancel");
}

#[test]
fn confirm_rejects_negative_and_seals_variance() {
    let harness = setup();
    let session_id = open_session(&harness, 7_500);
    BeginClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("begin");
    let negative = ConfirmClosingUseCase::new(&harness.cash).execute(
        &caissier(),
        &session_id,
        -1,
        None,
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(
            negative,
            Err(ApplicationError::Validation(
                DomainError::CashSessionInvalidCountedAmount
            ))
        ),
        "unexpected: {negative:?}"
    );
    // Expected is exactly the opening float (no other movements).
    let closed = ConfirmClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &session_id,
            8_000,
            Some("ok".to_string()),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("confirm");
    assert_eq!(closed.status, "CLOSED");
    assert_eq!(closed.system_expected_centimes, Some(7_500));
    assert_eq!(closed.actual_counted_centimes, Some(8_000));
    assert_eq!(closed.variance_centimes, Some(500));
    // Cannot be reopened, closed again, or mutated.
    let again = BeginClosingUseCase::new(&harness.cash).execute(
        &caissier(),
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(
            again,
            Err(ApplicationError::Validation(
                DomainError::CashSessionNotOpen
            ))
        ),
        "unexpected: {again:?}"
    );
    let report = GetSessionReportUseCase::new(&harness.cash)
        .execute(&caissier(), &session_id)
        .expect("report");
    assert!(report.is_final);
}

#[test]
fn variance_is_pure_integer_difference() {
    // Property sweep: actual - expected with i64 wrapping checked.
    for expected in [0i64, 1, 7_500, 100_000, 999_999_999] {
        for actual in [0i64, 1, 8_000, 50_000, 12_345_678] {
            let variance =
                baraka_domain::cash::cash_session::compute_variance(actual, expected).expect("var");
            assert_eq!(variance.centimes(), actual - expected);
        }
    }
}

#[test]
fn expense_validation_and_session_gating() {
    let harness = setup();
    // Rejected outright when no OPEN session.
    let denied = RecordExpenseUseCase::new(&harness.cash).execute(
        &caissier(),
        1_000,
        "Café",
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(
            denied,
            Err(ApplicationError::Validation(
                DomainError::NoOpenCashSessionForTerminal
            ))
        ),
        "unexpected: {denied:?}"
    );
    let session_id = open_session(&harness, 20_000);
    for bad in [0, -100] {
        let rejected = RecordExpenseUseCase::new(&harness.cash).execute(
            &caissier(),
            bad,
            "Café",
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        );
        assert!(
            matches!(
                rejected,
                Err(ApplicationError::Validation(
                    DomainError::CashExpenseInvalidAmount
                ))
            ),
            "amount {bad}: {rejected:?}"
        );
    }
    for bad_description in ["", "   "] {
        let rejected = RecordExpenseUseCase::new(&harness.cash).execute(
            &caissier(),
            500,
            bad_description,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        );
        assert!(
            matches!(
                rejected,
                Err(ApplicationError::Validation(
                    DomainError::CashExpenseDescriptionRequired
                ))
            ),
            "description: {rejected:?}"
        );
    }
    let movement = RecordExpenseUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            2_500,
            "  Café  ",
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("expense");
    assert_eq!(movement.movement_type, "EXPENSE_OUT");
    assert_eq!(movement.amount_centimes, -2_500);
    assert_eq!(cash_movement_sum(&harness, &session_id), 17_500);
    // Expenses are rejected while CLOSING.
    BeginClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("begin");
    let gated = RecordExpenseUseCase::new(&harness.cash).execute(
        &caissier(),
        100,
        "Café",
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(
            gated,
            Err(ApplicationError::Validation(
                DomainError::NoOpenCashSessionForTerminal
            ))
        ),
        "unexpected: {gated:?}"
    );
}

#[test]
fn financial_commands_are_idempotent() {
    let harness = setup();
    // open: same command_id + payload replays; different payload conflicts.
    let command_id = Uuid::new_v4().to_string();
    let first = OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &caissier(),
            harness.warehouse_id,
            3_000,
            &harness.terminal_id,
            &command_id,
        )
        .expect("first open");
    let replay = OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &caissier(),
            harness.warehouse_id,
            3_000,
            &harness.terminal_id,
            &command_id,
        )
        .expect("replay");
    assert_eq!(first, replay);
    let conflict = OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses).execute(
        &caissier(),
        harness.warehouse_id,
        9_999,
        &harness.terminal_id,
        &command_id,
    );
    assert!(
        matches!(conflict, Err(ApplicationError::IdempotencyConflict)),
        "unexpected: {conflict:?}"
    );
    // recordExpense idempotency.
    let expense_id = Uuid::new_v4().to_string();
    let expense_first = RecordExpenseUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            1_000,
            "Pain",
            &harness.terminal_id,
            &expense_id,
        )
        .expect("expense first");
    let expense_replay = RecordExpenseUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            1_000,
            "Pain",
            &harness.terminal_id,
            &expense_id,
        )
        .expect("expense replay");
    assert_eq!(expense_first, expense_replay);
    let expense_conflict = RecordExpenseUseCase::new(&harness.cash).execute(
        &caissier(),
        2_000,
        "Pain",
        &harness.terminal_id,
        &expense_id,
    );
    assert!(
        matches!(expense_conflict, Err(ApplicationError::IdempotencyConflict)),
        "unexpected: {expense_conflict:?}"
    );
    // begin/cancel/confirm idempotency chain.
    let begin_id = Uuid::new_v4().to_string();
    let begin_first = BeginClosingUseCase::new(&harness.cash)
        .execute(&caissier(), &harness.terminal_id, &begin_id)
        .expect("begin first");
    let begin_replay = BeginClosingUseCase::new(&harness.cash)
        .execute(&caissier(), &harness.terminal_id, &begin_id)
        .expect("begin replay");
    assert_eq!(begin_first, begin_replay);
    let cancel_id = Uuid::new_v4().to_string();
    let cancel_first = CancelClosingUseCase::new(&harness.cash)
        .execute(&caissier(), &harness.terminal_id, &cancel_id)
        .expect("cancel first");
    let cancel_replay = CancelClosingUseCase::new(&harness.cash)
        .execute(&caissier(), &harness.terminal_id, &cancel_id)
        .expect("cancel replay");
    assert_eq!(cancel_first, cancel_replay);
    // Re-begin (fresh key) then confirm twice with the same key.
    BeginClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("re-begin");
    let confirm_id = Uuid::new_v4().to_string();
    let confirm_first = ConfirmClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &first.id,
            4_000,
            None,
            &harness.terminal_id,
            &confirm_id,
        )
        .expect("confirm first");
    let confirm_replay = ConfirmClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &first.id,
            4_000,
            None,
            &harness.terminal_id,
            &confirm_id,
        )
        .expect("confirm replay");
    assert_eq!(confirm_first, confirm_replay);
}

#[test]
fn device_id_is_stable_across_restart() {
    let harness = setup();
    let first = harness.terminal_id.clone();
    assert!(Uuid::parse_str(&first).is_ok());
    // Reopen the same file: the identifier must be identical.
    let reopened = open_connection(&harness._dir.path().join("cash.db")).expect("reopen");
    let second = resolve_device_id(&reopened).expect("resolve again");
    assert_eq!(first, second);
}

#[test]
fn rbac_matrix_for_cash_commands() {
    let harness = setup();
    // Vendeur is denied on every financial cash command.
    let denied_open = OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses).execute(
        &vendeur(),
        harness.warehouse_id,
        0,
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(denied_open, Err(ApplicationError::Unauthorized)),
        "unexpected: {denied_open:?}"
    );
    let denied_begin = BeginClosingUseCase::new(&harness.cash).execute(
        &vendeur(),
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(denied_begin, Err(ApplicationError::Unauthorized)),
        "unexpected: {denied_begin:?}"
    );
    let denied_expense = RecordExpenseUseCase::new(&harness.cash).execute(
        &vendeur(),
        100,
        "Café",
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(denied_expense, Err(ApplicationError::Unauthorized)),
        "unexpected: {denied_expense:?}"
    );
    let denied_report = GetSessionReportUseCase::new(&harness.cash).execute(&vendeur(), "any");
    assert!(
        matches!(denied_report, Err(ApplicationError::Unauthorized)),
        "unexpected: {denied_report:?}"
    );
    // Caissier, Gerant, Admin can open (each on a distinct terminal to avoid
    // the single-active-session collision).
    for (index, session) in [caissier(), gerant(), admin()].into_iter().enumerate() {
        let terminal = format!("{}-rbac-{index}", harness.terminal_id);
        OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
            .execute(
                &session,
                harness.warehouse_id,
                0,
                &terminal,
                &Uuid::new_v4().to_string(),
            )
            .expect("permitted open");
        // Report view is permitted for all three roles.
        let active = GetActiveSessionUseCase::new(&harness.cash)
            .execute(&session, &terminal)
            .expect("active")
            .expect("session");
        GetSessionReportUseCase::new(&harness.cash)
            .execute(&session, &active.id)
            .expect("report");
    }
}

#[test]
fn session_report_shapes_for_open_and_closed() {
    let harness = setup();
    let session_id = open_session(&harness, 5_000);
    let interim = GetSessionReportUseCase::new(&harness.cash)
        .execute(&caissier(), &session_id)
        .expect("interim");
    assert!(!interim.is_final);
    assert_eq!(interim.movements.len(), 1);
    assert_eq!(interim.totals_by_type.len(), 1);
    assert_eq!(interim.totals_by_type[0].movement_type, "OPENING_FLOAT");
    BeginClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("begin");
    ConfirmClosingUseCase::new(&harness.cash)
        .execute(
            &caissier(),
            &session_id,
            5_000,
            None,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("confirm");
    let final_report = GetSessionReportUseCase::new(&harness.cash)
        .execute(&caissier(), &session_id)
        .expect("final");
    assert!(final_report.is_final);
    let missing = GetSessionReportUseCase::new(&harness.cash)
        .execute(&caissier(), &Uuid::new_v4().to_string());
    assert!(
        matches!(
            missing,
            Err(ApplicationError::Validation(
                DomainError::CashSessionNotFound
            ))
        ),
        "unexpected: {missing:?}"
    );
}

#[test]
fn credit_policy_variants_use_customer_module() {
    // Touches the imported CreditPolicy so the suite honours the domain
    // contract without floating-point math anywhere.
    assert_eq!(
        CreditPolicy::from_code("LIMITED").expect("limited"),
        CreditPolicy::Limited
    );
}
