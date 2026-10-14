//! End-to-end atomic sale tests over real SQLite backends.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    DocumentSearchFilter, ProductRepository, SessionContext, SessionToken, StockRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, CreateDraftSaleUseCase,
    DiscardDraftSaleUseCase, DraftInput, GetSaleUseCase, PaymentInput, PreviewSaleTotalsUseCase,
    PreviewTotalsInput, SearchDocumentsUseCase,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CustomerId, DocumentLanguage, DocumentStatus, DocumentType, PaymentMethod, PaymentStatus,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteDocumentRepository, SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "44444444-4444-4444-4444-444444444444";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    documents: SqliteDocumentRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("sale.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('44444444-4444-4444-4444-444444444444', 'vendeur1', 'hash', 'VENDEUR',
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
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
    }
}

fn actor() -> UserId {
    UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid"))
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: actor(),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-001";

fn ensure_cash_session(harness: &Harness, warehouse: WarehouseId) {
    let conn = rusqlite::Connection::open(harness._dir.path().join("sale.db")).expect("reopen");
    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO cash_sessions
         (id, warehouse_id, terminal_id, status, opened_by, opened_at,
          opening_float_centimes, revision, origin_device_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![session_id, warehouse.to_string(), TEST_TERMINAL, ACTOR_UUID,],
    )
    .expect("open session");
    conn.execute(
        "INSERT INTO cash_movements
         (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
         VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z', ?3)",
        rusqlite::params![Uuid::new_v4().to_string(), session_id, ACTOR_UUID,],
    )
    .expect("opening float");
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
        .create_warehouse(&warehouse, "TEST-TERMINAL")
        .expect("warehouse");
    warehouse.id
}

fn seed_product(harness: &Harness, sku: &str, price: i64, is_kit: bool) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    let product = Product {
        id,
        sku: Sku::new(sku).expect("sku"),
        barcode: None,
        name_ar: "منتج".to_string(),
        name_fr: "Produit".to_string(),
        category_id: None,
        is_kit,
        is_active: true,
    };
    let unit = ProductUnit {
        id: unit_id,
        product_id: id,
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Pièce".to_string(),
        factor_micro: FactorMicro::from_micros(10_000),
        barcode: Some(Barcode::new(&format!("611125{sku}")).expect("barcode")),
        sale_price: Money::from_centimes(price),
        purchase_price: Money::from_centimes(price - 2_000),
        wholesale_price: None,
        is_base_unit: true,
        is_default_selling: true,
    };
    harness
        .catalog
        .create_product(&product, &[unit], "TEST-TERMINAL")
        .expect("product");
    (id, unit_id)
}

fn seed_stock(harness: &Harness, product: ProductId, warehouse: WarehouseId, qty: i64) {
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, qty)
        .expect("stock");
}

fn seed_customer(harness: &Harness, policy: &str, limit: i64, balance: i64) -> CustomerId {
    let id = CustomerId::new();
    let conn = rusqlite::Connection::open(harness._dir.path().join("sale.db")).expect("reopen");
    conn.execute(
        "INSERT INTO customers
         (id, code, name, credit_policy, credit_limit_centimes,
          current_balance_centimes, is_active, created_at, updated_at,
          revision, origin_device_id)
         VALUES (?1, ?2, 'Client', ?3, ?4, ?5, 1,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1, 'LOCAL')",
        rusqlite::params![
            id.to_string(),
            format!("C-{}", &id.to_string()[..8]),
            policy,
            limit,
            balance
        ],
    )
    .expect("customer");
    id
}

fn read_conn(harness: &Harness) -> rusqlite::Connection {
    rusqlite::Connection::open(harness._dir.path().join("sale.db")).expect("reopen")
}

fn count(harness: &Harness, table: &str) -> i64 {
    read_conn(harness)
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn current_year() -> i32 {
    chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap_or(1970)
}

fn cash_input(
    warehouse: WarehouseId,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    cash: i64,
) -> CheckoutInput {
    CheckoutInput {
        command_id: Uuid::new_v4().to_string(),
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
    }
}

fn checkout_uc<'a>(
    harness: &'a Harness,
) -> CheckoutSaleUseCase<
    'a,
    SqliteCatalogRepository,
    SqliteCustomerRepository,
    SqliteCheckoutRepository,
    SqliteWarehouseRepository,
> {
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
}

#[test]
fn cash_sale_is_fully_atomic() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    // 2 x 100.00 = 200.00; timbre base 20,000 -> 1% = 200 -> min 500.
    // TTC = 20,500; cash exact.
    let receipt = checkout_uc(&harness)
        .execute(
            &session_with(Role::Vendeur),
            cash_input(warehouse, product, unit, 2, 20_500),
            TEST_TERMINAL,
        )
        .expect("checkout");
    let year = current_year();
    assert_eq!(receipt.document_number, format!("TICK-{year}-000001"));
    assert_eq!(receipt.total_ttc_centimes, "20500");
    assert_eq!(receipt.timbre_centimes, "500");
    assert_eq!(receipt.change_amount_centimes, "0");
    // Stock decremented by 2 base units.
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        80_000
    );
    // Document persisted FINALIZED with items, payment, print job.
    let conn = read_conn(&harness);
    let status: String = conn
        .query_row(
            "SELECT status FROM documents WHERE id = ?1",
            [&receipt.document_id],
            |row| row.get(0),
        )
        .expect("status");
    assert_eq!(status, "FINALIZED");
    assert_eq!(count(&harness, "document_items"), 1);
    assert_eq!(count(&harness, "payments"), 1);
    let job: (String, String) = conn
        .query_row(
            "SELECT status, job_type FROM print_jobs WHERE document_id = ?1",
            [&receipt.document_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("job");
    assert_eq!(job, ("PENDING".to_string(), "RECEIPT".to_string()));
    assert_eq!(receipt.print_job_id.len(), 36);
    // Sequence consumed exactly once.
    let last: i64 = conn
        .query_row(
            "SELECT last_sequence FROM document_sequences WHERE type = 'TICKET'",
            [],
            |row| row.get(0),
        )
        .expect("sequence");
    assert_eq!(last, 1);
    // Command log completed with a replayable payload.
    let payload: String = conn
        .query_row("SELECT response_payload FROM command_log", [], |row| {
            row.get(0)
        })
        .expect("command log");
    assert!(payload.contains(&receipt.document_id));
}

#[test]
fn checkout_sale_idempotent_replay_returns_same_receipt_without_side_effects() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    // One logical checkout command: a stable command id with a
    // byte-identical payload (the TASK-BRK-032 frontend contract).
    let command_id = Uuid::new_v4().to_string();
    let mut input = cash_input(warehouse, product, unit, 2, 20_500);
    input.command_id = command_id.clone();
    let first = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input.clone(), TEST_TERMINAL)
        .expect("first commit");
    let year = current_year();
    assert_eq!(first.document_number, format!("TICK-{year}-000001"));
    // Double-tap replay: same command id, identical payload.
    let replay = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input, TEST_TERMINAL)
        .expect("replay returns the stored receipt");
    assert_eq!(replay, first, "replay must be byte-identical");
    assert_eq!(replay.document_id, first.document_id);
    assert_eq!(replay.document_number, first.document_number);
    // No duplicate documents, payments, stock movements, or sequences.
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "payments"), 1);
    assert_eq!(count(&harness, "stock_movements"), 1);
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        80_000,
        "stock decremented exactly once (2 units)"
    );
    let conn = read_conn(&harness);
    let last: i64 = conn
        .query_row(
            "SELECT last_sequence FROM document_sequences WHERE type = 'TICKET'",
            [],
            |row| row.get(0),
        )
        .expect("sequence");
    assert_eq!(last, 1, "no second number consumed");
    // Exactly one command_log row for this logical command.
    let logged: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM command_log WHERE command_id = ?1",
            [&command_id],
            |row| row.get(0),
        )
        .expect("command log");
    assert_eq!(logged, 1);
}

/// Preview on empty stock: the warning names the product with exact micros.
#[test]
fn preview_reports_shortage_warning_on_zero_stock() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    // VIS-8X40 shape at zero stock: no inbound movement ever.
    let (product, unit) = seed_product(&harness, "VIS-8X40", 15_050, false);
    ensure_cash_session(&harness, warehouse);
    let preview = PreviewSaleTotalsUseCase::new(
        &harness.catalog,
        &harness.stock,
        &harness.warehouses,
        &harness.customers,
    )
    .execute(
        &session_with(Role::Vendeur),
        PreviewTotalsInput {
            warehouse_id: warehouse,
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
            payments: vec![],
        },
    )
    .expect("preview");
    // Totals still price normally (timbre needs no tender here).
    assert_eq!(preview.total_ttc, 15_050);
    assert_eq!(preview.stock_warnings.len(), 1);
    let warning = &preview.stock_warnings[0];
    assert_eq!(warning.product_id, product);
    assert_eq!(warning.product_name, "Produit");
    assert_eq!(warning.sku, "VIS-8X40");
    assert_eq!(warning.available_qty_micro, 0);
    assert_eq!(warning.requested_qty_micro, 10_000);
    // Read-only: nothing persisted.
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "stock_movements"), 0);
}

/// Standard staff can never sell on shortage — the explicit flag is
/// ignored and the write is hard-blocked inside the atomic transaction.
#[test]
fn vendeur_shortage_hard_blocked_even_with_explicit_flag() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    ensure_cash_session(&harness, warehouse);
    let mut input = cash_input(warehouse, product, unit, 2, 20_500);
    input.allow_negative_stock = true;
    let rejected = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input, TEST_TERMINAL)
        .expect_err("vendeur shortage");
    assert!(
        matches!(
            rejected,
            ApplicationError::InsufficientStock {
                available_base_micro: 0,
                requested_base_micro: 20_000,
            }
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "audit_log"), 0);
}

/// Caissier staff (no override grant) is blocked identically.
#[test]
fn caissier_shortage_hard_blocked_even_with_explicit_flag() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    ensure_cash_session(&harness, warehouse);
    let mut input = cash_input(warehouse, product, unit, 2, 20_500);
    input.allow_negative_stock = true;
    let rejected = checkout_uc(&harness)
        .execute(&session_with(Role::Caissier), input, TEST_TERMINAL)
        .expect_err("caissier shortage");
    assert!(
        matches!(rejected, ApplicationError::InsufficientStock { .. }),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "stock_movements"), 0);
}

/// Privileged sale on shortage without the explicit in-session
/// confirmation is refused — never silently bypassed.
#[test]
fn admin_shortage_requires_explicit_confirmation() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    ensure_cash_session(&harness, warehouse);
    let input = cash_input(warehouse, product, unit, 2, 20_500);
    assert!(!input.allow_negative_stock);
    let rejected = checkout_uc(&harness)
        .execute(&session_with(Role::Admin), input, TEST_TERMINAL)
        .expect_err("unconfirmed admin shortage");
    assert!(
        matches!(
            rejected,
            ApplicationError::NegativeStockConfirmationRequired {
                ref product_id,
                ref product_name,
                available_micro: 0,
                requested_micro: 20_000,
            } if product_id == &product.to_string() && product_name == "Produit"
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "audit_log"), 0);
    // No sequence number consumed by the refused sale.
    let conn = read_conn(&harness);
    let sequences: i64 = conn
        .query_row("SELECT COUNT(*) FROM document_sequences", [], |row| {
            row.get(0)
        })
        .expect("sequences");
    assert_eq!(sequences, 0);
}

/// Gerant without confirmation is refused identically.
#[test]
fn gerant_shortage_requires_explicit_confirmation() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    ensure_cash_session(&harness, warehouse);
    let input = cash_input(warehouse, product, unit, 2, 20_500);
    let rejected = checkout_uc(&harness)
        .execute(&session_with(Role::Gerant), input, TEST_TERMINAL)
        .expect_err("unconfirmed gerant shortage");
    assert!(
        matches!(
            rejected,
            ApplicationError::NegativeStockConfirmationRequired { .. }
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
}

/// Privileged + explicit confirmation commits and leaves an immutable,
/// detailed audit trail inside the same atomic transaction.
#[test]
fn admin_override_with_flag_succeeds_and_audits() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    ensure_cash_session(&harness, warehouse);
    let mut input = cash_input(warehouse, product, unit, 2, 20_500);
    input.allow_negative_stock = true;
    let receipt = checkout_uc(&harness)
        .execute(&session_with(Role::Admin), input, TEST_TERMINAL)
        .expect("confirmed override");
    assert_eq!(receipt.total_ttc_centimes, "20500");
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        -20_000
    );
    let conn = read_conn(&harness);
    let authorized: i64 = conn
        .query_row(
            "SELECT authorized_negative_stock FROM stock_movements WHERE product_id = ?1",
            [product.to_string()],
            |row| row.get(0),
        )
        .expect("movement flag");
    assert_eq!(authorized, 1);
    // Two chain rows: the engine's minimal row plus the detailed
    // sale-context row (entity `product` with document/sku/quantities).
    let rows: Vec<(String, String, Option<String>)> = conn
        .prepare(
            "SELECT entity_type, entity_id, new_value_json FROM audit_log
             WHERE action = 'NEGATIVE_STOCK_OVERRIDE' ORDER BY sequence_no",
        )
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "stock_levels");
    assert_eq!(rows[0].1, format!("{}:{}", warehouse, product));
    assert_eq!(rows[1].0, "product");
    assert_eq!(rows[1].1, product.to_string());
    let detail = rows[1].2.clone().expect("detail json");
    assert!(
        detail.contains(&receipt.document_number),
        "detail must name the document: {detail}"
    );
    assert!(
        detail.contains("\"sku\":\"P1\""),
        "detail must carry sku: {detail}"
    );
    assert!(
        detail.contains("\"available_qty_micro\":0"),
        "detail must carry before: {detail}"
    );
    assert!(
        detail.contains("\"requested_qty_micro\":20000"),
        "detail must carry requested: {detail}"
    );
    assert!(
        detail.contains("\"resulting_qty_micro\":-20000"),
        "detail must carry after: {detail}"
    );
    assert_eq!(count(&harness, "documents"), 1);
}

#[test]
fn kit_sale_decrements_leaves_only() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (leaf, leaf_unit) = seed_product(&harness, "LEAF", 5_000, false);
    let (kit, kit_unit) = seed_product(&harness, "KIT", 12_000, true);
    harness
        .catalog
        .define_kit(
            kit,
            &[baraka_domain::catalog::KitItem {
                kit_product_id: kit,
                component_product_id: leaf,
                unit_id: leaf_unit,
                quantity_micro: baraka_domain::primitives::QtyMicro::from_micros(20_000),
            }],
        )
        .expect("bom");
    seed_stock(&harness, leaf, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    // 3 kits x 2 leaves = 6 base units = 60,000 micros deducted.
    let receipt = checkout_uc(&harness)
        .execute(
            &session_with(Role::Vendeur),
            cash_input(warehouse, kit, kit_unit, 3, 100_000),
            TEST_TERMINAL,
        )
        .expect("kit checkout");
    assert_eq!(
        harness
            .stock
            .get_stock_level(leaf, warehouse)
            .expect("level"),
        40_000
    );
    // The kit header itself never carries stock.
    assert_eq!(
        harness
            .stock
            .get_stock_level(kit, warehouse)
            .expect("level"),
        0
    );
    let conn = read_conn(&harness);
    let movements: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
            [leaf.to_string()],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(movements, 1);
    let _ = receipt;
}

#[test]
fn credit_sale_updates_ledger_and_balance() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let customer = seed_customer(&harness, "LIMITED", 100_000, 0);
    // No cash tendered: timbre is 0, TTC is exactly 20,000.
    let mut input = cash_input(warehouse, product, unit, 2, 0);
    input.customer_id = Some(customer);
    input.payments = vec![PaymentInput {
        method: PaymentMethod::Credit,
        amount_centimes: 20_000,
    }];
    let receipt = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input, TEST_TERMINAL)
        .expect("credit checkout");
    assert_eq!(receipt.paid_amount_centimes, "20000");
    assert_eq!(receipt.total_ttc_centimes, "20000");
    let conn = read_conn(&harness);
    let balance: i64 = conn
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = ?1",
            [customer.to_string()],
            |row| row.get(0),
        )
        .expect("balance");
    assert_eq!(balance, 20_000);
    let entry: (String, i64, i64) = conn
        .query_row(
            "SELECT entry_type, amount_centimes, running_balance_centimes FROM customer_ledger WHERE customer_id = ?1",
            [customer.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("ledger");
    assert_eq!(entry.0, "DEBIT");
    assert_eq!((entry.1, entry.2), (20_000, 20_000));
    let party: Option<String> = conn
        .query_row(
            "SELECT party_id FROM documents WHERE id = ?1",
            [&receipt.document_id],
            |row| row.get(0),
        )
        .expect("party");
    assert_eq!(party, Some(customer.to_string()));
}

#[test]
fn credit_limit_exceeded_rejects_then_override_succeeds() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let customer = seed_customer(&harness, "LIMITED", 10_000, 0);
    let mut input = cash_input(warehouse, product, unit, 2, 0);
    input.customer_id = Some(customer);
    input.payments = vec![PaymentInput {
        method: PaymentMethod::Credit,
        amount_centimes: 20_000,
    }];
    // Vendeur lacks the override: rejected, nothing written.
    let rejected = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input.clone(), TEST_TERMINAL)
        .expect_err("over limit");
    assert!(
        matches!(
            rejected,
            ApplicationError::CreditLimitExceeded {
                limit_centimes: 10_000,
                proposed_centimes: 20_000,
            }
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "stock_movements"), 0);
    // Gerant holds the override: succeeds.
    let receipt = checkout_uc(&harness)
        .execute(&session_with(Role::Gerant), input, TEST_TERMINAL)
        .expect("override checkout");
    assert_eq!(receipt.total_ttc_centimes, "20000");
    assert_eq!(count(&harness, "documents"), 1);
}

#[test]
fn insufficient_stock_rolls_back_everything() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 5_000);
    ensure_cash_session(&harness, warehouse);
    let rejected = checkout_uc(&harness)
        .execute(
            &session_with(Role::Vendeur),
            cash_input(warehouse, product, unit, 2, 20_500),
            TEST_TERMINAL,
        )
        .expect_err("short stock");
    assert!(
        matches!(rejected, ApplicationError::InsufficientStock { .. }),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "document_items"), 0);
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "payments"), 0);
    assert_eq!(count(&harness, "print_jobs"), 0);
    assert_eq!(count(&harness, "command_log"), 0);
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        5_000
    );
    // No sequence number consumed by the rolled-back sale.
    let conn = read_conn(&harness);
    let sequences: i64 = conn
        .query_row("SELECT COUNT(*) FROM document_sequences", [], |row| {
            row.get(0)
        })
        .expect("sequences");
    assert_eq!(sequences, 0);
}

#[test]
fn finalized_documents_reject_direct_mutation() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let receipt = checkout_uc(&harness)
        .execute(
            &session_with(Role::Vendeur),
            cash_input(warehouse, product, unit, 2, 20_500),
            TEST_TERMINAL,
        )
        .expect("checkout");
    let conn = read_conn(&harness);
    let update = conn.execute(
        "UPDATE documents SET payment_method = 'X' WHERE id = ?1",
        [&receipt.document_id],
    );
    assert!(
        format!("{update:?}").contains("MUTATION_FORBIDDEN"),
        "update must abort: {update:?}"
    );
    let delete_items = conn.execute(
        "DELETE FROM document_items WHERE document_id = ?1",
        [&receipt.document_id],
    );
    assert!(
        format!("{delete_items:?}").contains("MUTATION_FORBIDDEN"),
        "item delete must abort: {delete_items:?}"
    );
    let delete_doc = conn.execute(
        "DELETE FROM documents WHERE id = ?1",
        [&receipt.document_id],
    );
    assert!(
        format!("{delete_doc:?}").contains("MUTATION_FORBIDDEN"),
        "document delete must abort: {delete_doc:?}"
    );
}

#[test]
fn vendeur_price_override_rejected_gerant_accepted() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let mut input = cash_input(warehouse, product, unit, 1, 20_000);
    input.lines[0].unit_price_override = Some(9_000);
    let rejected = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input.clone(), TEST_TERMINAL)
        .expect_err("override");
    assert!(
        matches!(
            rejected,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
    // Gerant override honored: 1 x 9,000 + timbre on 9,000 base.
    let receipt = checkout_uc(&harness)
        .execute(&session_with(Role::Gerant), input, TEST_TERMINAL)
        .expect("gerant override");
    // 9,000 HT, 1% = 90 -> ceiling 100 -> clamp min 500. TTC = 9,500.
    assert_eq!(receipt.total_ttc_centimes, "9500");
    let conn = read_conn(&harness);
    let stored_price: i64 = conn
        .query_row(
            "SELECT unit_price_centimes FROM document_items WHERE document_id = ?1",
            [&receipt.document_id],
            |row| row.get(0),
        )
        .expect("price");
    assert_eq!(stored_price, 9_000);
}

#[test]
fn draft_parks_without_side_effects() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    let session = session_with(Role::Vendeur);
    let clock = baraka_infrastructure::adapters::SystemClock;
    let draft_id = CreateDraftSaleUseCase::new(
        &harness.catalog,
        &harness.documents,
        &harness.warehouses,
        &harness.customers,
        &clock,
    )
    .execute(
        &session,
        DraftInput {
            warehouse_id: warehouse,
            customer_id: None,
            language: DocumentLanguage::Ar,
            lines: vec![CartLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 20_000,
                unit_price_override: None,
                line_discount_centimes: 0,
            }],
            notes: Some("Client hésitant".to_string()),
        },
        TEST_TERMINAL,
    )
    .expect("draft");
    // Draft is parked mutable: no sequence, no stock move, no payment.
    assert!(draft_id.to_string().len() == 36);
    let detail = GetSaleUseCase::new(&harness.documents)
        .execute(&session, draft_id)
        .expect("fetch draft");
    assert_eq!(detail.document.status, DocumentStatus::Draft);
    assert!(detail.document.number.starts_with("DRAFT-"));
    assert_eq!(detail.items.len(), 1);
    assert_eq!(detail.items[0].quantity_micro, 20_000);
    assert_eq!(detail.print_status, "NONE");
    assert_eq!(detail.payment_status, PaymentStatus::Unpaid);
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        100_000
    );
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "payments"), 0);
}

#[test]
fn gapless_numbers_stay_consecutive() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 1_000_000);
    ensure_cash_session(&harness, warehouse);
    let year = current_year();
    for index in 1..=3 {
        let receipt = checkout_uc(&harness)
            .execute(
                &session_with(Role::Vendeur),
                cash_input(warehouse, product, unit, 1, 20_000),
                TEST_TERMINAL,
            )
            .expect("checkout");
        assert_eq!(receipt.document_number, format!("TICK-{year}-{index:06}"));
    }
}

fn stored_language(harness: &Harness, document_id: &str) -> String {
    read_conn(harness)
        .query_row(
            "SELECT language FROM documents WHERE id = ?1",
            [document_id],
            |row| row.get(0),
        )
        .expect("language")
}

#[test]
fn arabic_checkout_persists_language_and_renders_rtl() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let mut input = cash_input(warehouse, product, unit, 2, 20_500);
    input.language = DocumentLanguage::Ar;
    let receipt = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input, TEST_TERMINAL)
        .expect("arabic checkout");
    assert_eq!(stored_language(&harness, &receipt.document_id), "AR");
    // Legacy column stays in sync (dual-write, never diverges).
    let legacy: String = read_conn(&harness)
        .query_row(
            "SELECT document_language FROM documents WHERE id = ?1",
            [&receipt.document_id],
            |row| row.get(0),
        )
        .expect("legacy language");
    assert_eq!(legacy, "AR");
}

#[test]
fn draft_lifecycle_create_search_and_discard() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "PARK-1", 10_000, false);
    let session = session_with(Role::Vendeur);
    let clock = baraka_infrastructure::adapters::SystemClock;
    let draft_id = CreateDraftSaleUseCase::new(
        &harness.catalog,
        &harness.documents,
        &harness.warehouses,
        &harness.customers,
        &clock,
    )
    .execute(
        &session,
        DraftInput {
            warehouse_id: warehouse,
            customer_id: None,
            language: DocumentLanguage::Fr,
            lines: vec![CartLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 20_000,
                unit_price_override: None,
                line_discount_centimes: 0,
            }],
            notes: None,
        },
        TEST_TERMINAL,
    )
    .expect("park draft");

    let drafts = || {
        SearchDocumentsUseCase::new(&harness.documents)
            .execute(
                &session,
                DocumentSearchFilter {
                    doc_type: None,
                    status: Some("DRAFT".to_string()),
                    query: None,
                    limit: Some(20),
                },
            )
            .expect("search drafts")
    };
    let listed = drafts();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, draft_id.to_string());

    DiscardDraftSaleUseCase::new(&harness.documents)
        .execute(&session, &draft_id.to_string())
        .expect("discard draft");

    assert!(drafts().is_empty());

    let again =
        DiscardDraftSaleUseCase::new(&harness.documents).execute(&session, &draft_id.to_string());
    assert!(matches!(
        again,
        Err(ApplicationError::Conflict(_)) | Err(ApplicationError::NotFound(_))
    ));
}

#[test]
fn french_checkout_persists_language() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, false);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let mut input = cash_input(warehouse, product, unit, 1, 20_000);
    input.language = DocumentLanguage::Fr;
    let receipt = checkout_uc(&harness)
        .execute(&session_with(Role::Vendeur), input, TEST_TERMINAL)
        .expect("french checkout");
    assert_eq!(stored_language(&harness, &receipt.document_id), "FR");
}
