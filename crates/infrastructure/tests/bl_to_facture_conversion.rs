//! End-to-end BL-to-FACTURE financial closing tests over a real backend.
//!
//! File-based temporary database (WAL/crash standards): every test opens
//! `closing.db` in a fresh tempdir, runs all migrations, and asserts the
//! ADR-046 hard invariant — zero `stock_movements` rows and byte-identical
//! `stock_levels` before and after every successful conversion.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    DocumentRepository, ProductRepository, SessionContext, SessionToken, StockRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::sales::{
    ClosingInput, ClosingPaymentInput, ConvertBlToFactureUseCase,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CustomerId, Document, DocumentId, DocumentItem, DocumentLanguage, DocumentStatus, DocumentType,
    PaymentMethod,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCustomerRepository, SqliteDocumentRepository,
    SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const GERANT_UUID: &str = "66666666-6666-6666-6666-666666666666";
const VENDEUR_UUID: &str = "44444444-4444-4444-4444-444444444444";
const TEST_TERMINAL: &str = "TEST-TERMINAL-014B";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    documents: SqliteDocumentRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("closing.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    for (id, name, role) in [
        (GERANT_UUID, "gerant1", "GERANT"),
        (VENDEUR_UUID, "vendeur1", "VENDEUR"),
    ] {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![id, name, role],
        )
        .expect("seed user");
    }
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
    }
}

fn user_id(value: &str) -> UserId {
    UserId::from(Uuid::parse_str(value).expect("uuid"))
}

fn session_with(role: Role) -> SessionContext {
    let uuid = match role {
        Role::Gerant => GERANT_UUID,
        _ => VENDEUR_UUID,
    };
    SessionContext {
        token: SessionToken::new_random(),
        user_id: user_id(uuid),
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
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
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
    let conn = rusqlite::Connection::open(harness._dir.path().join("closing.db")).expect("reopen");
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
    // Keep the cached balance consistent with real ledger history so the
    // domain invariant (balance == SUM(DEBIT) - SUM(CREDIT)) holds.
    if balance > 0 {
        conn.execute(
            "INSERT INTO customer_ledger
             (id, sequence_no, customer_id, entry_type, amount_centimes,
              running_balance_centimes, document_type, document_id,
              created_by, created_at)
             VALUES (?1, 1, ?2, 'DEBIT', ?3, ?3, 'SALE', 'seed', ?4,
                     '2026-01-01T00:00:00Z')",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                id.to_string(),
                balance,
                GERANT_UUID,
            ],
        )
        .expect("ledger seed");
    }
    id
}

/// One frozen BL line snapshot.
fn bl_line(
    document_id: DocumentId,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    unit_price: i64,
    sort_order: i64,
) -> DocumentItem {
    let micros = qty_units * 10_000;
    let total = unit_price * qty_units;
    DocumentItem {
        id: Uuid::new_v4(),
        document_id,
        product_id: product,
        unit_id: unit,
        quantity_micro: micros,
        quantity_base_micro: micros,
        unit_price_centimes: unit_price,
        discount_centimes: 0,
        tva_rate_micro: 0,
        line_ht_centimes: total,
        line_tva_centimes: 0,
        line_ttc_centimes: total,
        cost_price_centimes: unit_price - 2_000,
        sort_order,
    }
}

/// Persist a BL with the given status (FINALIZED for the happy path).
fn seed_bl(
    harness: &Harness,
    warehouse: WarehouseId,
    status: DocumentStatus,
    number: &str,
    lines: Vec<DocumentItem>,
    customer: Option<CustomerId>,
) -> DocumentId {
    let id = DocumentId::new();
    let ttc: i64 = lines.iter().map(|line| line.line_ttc_centimes).sum();
    let mut fixed_lines = Vec::with_capacity(lines.len());
    for mut line in lines {
        line.document_id = id;
        fixed_lines.push(line);
    }
    let document = Document {
        id,
        language: DocumentLanguage::Fr,
        number: number.to_string(),
        fiscal_year: current_year(),
        doc_type: DocumentType::Bl,
        status,
        warehouse_id: warehouse,
        party_id: customer.map(|id| id.to_string()),
        subtotal_ht_centimes: ttc,
        vat_total_centimes: 0,
        global_discount_centimes: 0,
        timbre_centimes: 0,
        ttc_centimes: ttc,
        paid_amount_centimes: 0,
        change_amount_centimes: 0,
        payment_method: "UNPAID".to_string(),
        notes: None,
        created_at: chrono::Utc::now(),
        created_by: user_id(GERANT_UUID),
    };
    harness
        .documents
        .finalize_document(&document, &fixed_lines, &[], TEST_TERMINAL)
        .expect("seed bl");
    id
}

fn ensure_cash_session(harness: &Harness, warehouse: WarehouseId) {
    let conn = rusqlite::Connection::open(harness._dir.path().join("closing.db")).expect("reopen");
    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO cash_sessions
         (id, warehouse_id, terminal_id, status, opened_by, opened_at,
          opening_float_centimes, revision, origin_device_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![
            session_id,
            warehouse.to_string(),
            TEST_TERMINAL,
            GERANT_UUID,
        ],
    )
    .expect("open session");
    conn.execute(
        "INSERT INTO cash_movements
         (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
         VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z', ?3)",
        rusqlite::params![Uuid::new_v4().to_string(), session_id, GERANT_UUID,],
    )
    .expect("opening float");
}

fn close_uc<'a>(
    harness: &'a Harness,
) -> ConvertBlToFactureUseCase<'a, SqliteCustomerRepository, SqliteDocumentRepository> {
    ConvertBlToFactureUseCase::new(&harness.customers, &harness.documents)
}

fn tender(method: PaymentMethod, amount: i64) -> ClosingPaymentInput {
    ClosingPaymentInput {
        method,
        amount_centimes: amount,
    }
}

fn close_input(source: DocumentId, payments: Vec<ClosingPaymentInput>) -> ClosingInput {
    ClosingInput {
        command_id: Uuid::new_v4().to_string(),
        source_document_id: source,
        language: DocumentLanguage::Fr,
        payments,
    }
}

fn read_conn(harness: &Harness) -> rusqlite::Connection {
    rusqlite::Connection::open(harness._dir.path().join("closing.db")).expect("reopen")
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

fn doc_status(harness: &Harness, id: DocumentId) -> String {
    read_conn(harness)
        .query_row(
            "SELECT status FROM documents WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .expect("status")
}

fn stock_level(harness: &Harness, product: ProductId, warehouse: WarehouseId) -> i64 {
    harness
        .stock
        .get_stock_level(product, warehouse)
        .expect("level")
}

fn customer_balance(harness: &Harness, customer: CustomerId) -> i64 {
    read_conn(harness)
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = ?1",
            [customer.to_string()],
            |row| row.get(0),
        )
        .expect("balance")
}

/// Snapshot of every inventory observable for ADR-046 assertions.
fn inventory_snapshot(harness: &Harness, products: &[(ProductId, WarehouseId)]) -> (i64, Vec<i64>) {
    let movements = count(harness, "stock_movements");
    let levels = products
        .iter()
        .map(|(product, warehouse)| stock_level(harness, *product, *warehouse))
        .collect();
    (movements, levels)
}

#[test]
fn cash_closing_creates_facture_movement_and_seals_source() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    let (p2, u2) = seed_product(&harness, "P2", 20_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    seed_stock(&harness, p2, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![
            bl_line(bl, p1, u1, 2, 10_000, 0),
            bl_line(bl, p2, u2, 1, 20_000, 1),
        ],
        None,
    );
    let before = inventory_snapshot(&harness, &[(p1, warehouse), (p2, warehouse)]);

    let result = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(bl, vec![tender(PaymentMethod::Cash, 40_000)]),
            TEST_TERMINAL,
        )
        .expect("close");
    let year = current_year();
    assert_eq!(result.new_document_number, format!("FAC-{year}-000001"));
    assert_eq!(result.new_document_status, "FINALIZED");
    assert_eq!(result.total_ttc_centimes, 40_000);
    assert_eq!(result.items_count, 2);
    assert_eq!(result.customer_new_balance_centimes, None);

    // FACTURE header: source link, copied parties/totals, request language.
    let conn = read_conn(&harness);
    let header: (String, String, String, Option<String>, i64, String) = conn
        .query_row(
            "SELECT type, status, source_document_id, party_id, ttc_centimes, origin_device_id
             FROM documents WHERE id = ?1",
            [result.new_document_id.clone()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("facture header");
    assert_eq!(header.0, "FACTURE");
    assert_eq!(header.1, "FINALIZED");
    assert_eq!(header.2, bl.to_string());
    assert_eq!(header.3, None);
    assert_eq!(header.4, 40_000);
    assert_eq!(header.5, TEST_TERMINAL);

    // Byte-identical items.
    let source_items = harness.documents.get_items(bl).expect("source items");
    let facture_items = harness
        .documents
        .get_items(DocumentId::from(
            Uuid::parse_str(&result.new_document_id).expect("uuid"),
        ))
        .expect("facture items");
    assert_eq!(facture_items.len(), source_items.len());
    for (actual, expected) in facture_items.iter().zip(source_items.iter()) {
        assert_eq!(actual.document_id.to_string(), result.new_document_id);
        assert_ne!(actual.id, expected.id);
        assert_eq!(actual.product_id, expected.product_id);
        assert_eq!(actual.unit_id, expected.unit_id);
        assert_eq!(actual.quantity_micro, expected.quantity_micro);
        assert_eq!(actual.quantity_base_micro, expected.quantity_base_micro);
        assert_eq!(actual.unit_price_centimes, expected.unit_price_centimes);
        assert_eq!(actual.discount_centimes, expected.discount_centimes);
        assert_eq!(actual.tva_rate_micro, expected.tva_rate_micro);
        assert_eq!(actual.line_ht_centimes, expected.line_ht_centimes);
        assert_eq!(actual.line_tva_centimes, expected.line_tva_centimes);
        assert_eq!(actual.line_ttc_centimes, expected.line_ttc_centimes);
        assert_eq!(actual.cost_price_centimes, expected.cost_price_centimes);
        assert_eq!(actual.sort_order, expected.sort_order);
    }

    // Source sealed; cash movement references the FACTURE.
    assert_eq!(doc_status(&harness, bl), "CONVERTED");
    let movement: (String, i64, String) = conn
        .query_row(
            "SELECT movement_type, amount_centimes, reference_id FROM cash_movements
             WHERE movement_type = 'SALE_CASH_IN' ORDER BY rowid DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("cash movement");
    assert_eq!(movement.0, "SALE_CASH_IN");
    assert_eq!(movement.1, 40_000);
    assert_eq!(movement.2, result.new_document_id);

    // ADR-046: zero inventory side effects, byte-identical levels.
    assert_eq!(
        inventory_snapshot(&harness, &[(p1, warehouse), (p2, warehouse)]),
        before
    );

    // Print outbox + audit trail.
    let job: (String, String) = conn
        .query_row(
            "SELECT status, job_type FROM print_jobs WHERE document_id = ?1",
            [result.new_document_id.clone()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("print job");
    assert_eq!(job, ("PENDING".to_string(), "RECEIPT".to_string()));
    let audit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'DOCUMENT_CONVERTED'",
            [],
            |row| row.get(0),
        )
        .expect("audit count");
    assert_eq!(audit_count, 1);
}

#[test]
fn credit_closing_writes_ledger_debit_and_updates_balance() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let customer = seed_customer(&harness, "LIMITED", 100_000, 10_000);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 2, 10_000, 0)],
        Some(customer),
    );
    let before = inventory_snapshot(&harness, &[(p1, warehouse)]);

    let result = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(bl, vec![tender(PaymentMethod::Credit, 20_000)]),
            TEST_TERMINAL,
        )
        .expect("credit close");
    assert_eq!(result.total_ttc_centimes, 20_000);
    assert_eq!(result.customer_new_balance_centimes, Some(30_000));
    assert_eq!(customer_balance(&harness, customer), 30_000);

    let conn = read_conn(&harness);
    let entry: (String, i64, i64, String) = conn
        .query_row(
            "SELECT entry_type, amount_centimes, running_balance_centimes, document_id
             FROM customer_ledger WHERE customer_id = ?1
             ORDER BY sequence_no DESC LIMIT 1",
            [customer.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("ledger");
    assert_eq!(entry.0, "DEBIT");
    assert_eq!(entry.1, 20_000);
    assert_eq!(entry.2, 30_000);
    assert_eq!(entry.3, result.new_document_id);
    // No drawer movement for a pure-credit closing.
    let cash_moves: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cash_movements WHERE movement_type = 'SALE_CASH_IN'",
            [],
            |row| row.get(0),
        )
        .expect("cash moves");
    assert_eq!(cash_moves, 0);
    assert_eq!(doc_status(&harness, bl), "CONVERTED");
    assert_eq!(inventory_snapshot(&harness, &[(p1, warehouse)]), before);
}

#[test]
fn mixed_split_creates_both_legs_summing_to_total() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let customer = seed_customer(&harness, "LIMITED", 100_000, 0);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 2, 10_000, 0)],
        Some(customer),
    );
    let before = inventory_snapshot(&harness, &[(p1, warehouse)]);

    let result = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(
                bl,
                vec![
                    tender(PaymentMethod::Cash, 8_000),
                    tender(PaymentMethod::Credit, 12_000),
                ],
            ),
            TEST_TERMINAL,
        )
        .expect("mixed close");
    assert_eq!(result.customer_new_balance_centimes, Some(12_000));
    assert_eq!(customer_balance(&harness, customer), 12_000);

    let conn = read_conn(&harness);
    let cash_amount: i64 = conn
        .query_row(
            "SELECT amount_centimes FROM cash_movements WHERE movement_type = 'SALE_CASH_IN'",
            [],
            |row| row.get(0),
        )
        .expect("cash leg");
    let credit_amount: i64 = conn
        .query_row(
            "SELECT amount_centimes FROM customer_ledger WHERE customer_id = ?1",
            [customer.to_string()],
            |row| row.get(0),
        )
        .expect("credit leg");
    assert_eq!(cash_amount + credit_amount, 20_000);
    // Payment rows mirror checkout population (CASH bound to session).
    let cash_session_bound: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM payments
             WHERE document_id = ?1 AND payment_method = 'CASH' AND cash_session_id IS NOT NULL",
            [result.new_document_id.clone()],
            |row| row.get(0),
        )
        .expect("payment bindings");
    assert_eq!(cash_session_bound, 1);
    assert_eq!(inventory_snapshot(&harness, &[(p1, warehouse)]), before);
}

#[test]
fn cash_without_open_session_fails_with_checkout_error() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 1, 10_000, 0)],
        None,
    );
    // No cash session opened on this terminal.
    let ledgers_before = count(&harness, "customer_ledger");
    let error = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(bl, vec![tender(PaymentMethod::Cash, 10_000)]),
            TEST_TERMINAL,
        )
        .expect_err("no session");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "customer_ledger"), ledgers_before);
    assert_eq!(count(&harness, "cash_movements"), 0);
    assert_eq!(count(&harness, "command_log"), 0);
    assert_eq!(doc_status(&harness, bl), "FINALIZED");
}

#[test]
fn credit_over_limit_fails_with_checkout_error() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let customer = seed_customer(&harness, "LIMITED", 15_000, 10_000);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 2, 10_000, 0)],
        Some(customer),
    );
    // Vendeur lacks CreditLimitOverride: 10_000 + 20_000 > 15_000 rejects.
    let error = close_uc(&harness)
        .execute(
            &session_with(Role::Vendeur),
            close_input(bl, vec![tender(PaymentMethod::Credit, 20_000)]),
            TEST_TERMINAL,
        )
        .expect_err("over limit");
    assert!(
        matches!(
            error,
            ApplicationError::CreditLimitExceeded {
                limit_centimes: 15_000,
                proposed_centimes: 30_000,
            }
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(
        count(&harness, "customer_ledger"),
        1,
        "only the seed row; the rejected closing wrote nothing"
    );
    assert_eq!(customer_balance(&harness, customer), 10_000);
    assert_eq!(doc_status(&harness, bl), "FINALIZED");
}

#[test]
fn second_closing_returns_already_converted() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 1, 10_000, 0)],
        None,
    );
    close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(bl, vec![tender(PaymentMethod::Cash, 10_000)]),
            TEST_TERMINAL,
        )
        .expect("first");
    let before = (
        count(&harness, "documents"),
        count(&harness, "customer_ledger"),
        count(&harness, "cash_movements"),
        count(&harness, "print_jobs"),
        count(&harness, "command_log"),
    );
    let mut second = close_input(bl, vec![tender(PaymentMethod::Cash, 10_000)]);
    second.command_id = Uuid::new_v4().to_string();
    let error = close_uc(&harness)
        .execute(&session_with(Role::Gerant), second, TEST_TERMINAL)
        .expect_err("second closing");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::DocumentAlreadyConverted)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(
        (
            count(&harness, "documents"),
            count(&harness, "customer_ledger"),
            count(&harness, "cash_movements"),
            count(&harness, "print_jobs"),
            count(&harness, "command_log"),
        ),
        before
    );
}

#[test]
fn draft_and_cancelled_bl_are_rejected() {
    for status in [DocumentStatus::Draft, DocumentStatus::Cancelled] {
        let harness = setup();
        let warehouse = seed_warehouse(&harness, "MAIN");
        let (p1, u1) = seed_product(&harness, "P1", 10_000);
        seed_stock(&harness, p1, warehouse, 100_000);
        let bl = DocumentId::new();
        let bl = seed_bl(
            &harness,
            warehouse,
            status,
            "BL-1",
            vec![bl_line(bl, p1, u1, 1, 10_000, 0)],
            None,
        );
        let error = close_uc(&harness)
            .execute(
                &session_with(Role::Gerant),
                close_input(bl, vec![tender(PaymentMethod::Cash, 10_000)]),
                TEST_TERMINAL,
            )
            .expect_err("non-finalized");
        assert!(
            matches!(
                error,
                ApplicationError::Validation(DomainError::InvalidConversionSourceStatus)
            ),
            "unexpected for {status:?}: {error:?}"
        );
        assert_eq!(count(&harness, "documents"), 1);
        assert_eq!(count(&harness, "cash_movements"), 0);
    }
}

#[test]
fn non_bl_sources_are_rejected() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    // A finalized DEVIS is a valid document but the wrong source type here.
    let devis = DocumentId::new();
    let document = Document {
        id: devis,
        language: DocumentLanguage::Fr,
        number: "DEV-1".to_string(),
        fiscal_year: current_year(),
        doc_type: DocumentType::Devis,
        status: DocumentStatus::Finalized,
        warehouse_id: warehouse,
        party_id: None,
        subtotal_ht_centimes: 10_000,
        vat_total_centimes: 0,
        global_discount_centimes: 0,
        timbre_centimes: 0,
        ttc_centimes: 10_000,
        paid_amount_centimes: 0,
        change_amount_centimes: 0,
        payment_method: "UNPAID".to_string(),
        notes: None,
        created_at: chrono::Utc::now(),
        created_by: user_id(GERANT_UUID),
    };
    harness
        .documents
        .finalize_document(
            &document,
            &[bl_line(devis, p1, u1, 1, 10_000, 0)],
            &[],
            TEST_TERMINAL,
        )
        .expect("seed devis");
    let error = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(devis, vec![tender(PaymentMethod::Cash, 10_000)]),
            TEST_TERMINAL,
        )
        .expect_err("devis source");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidConversionSourceType)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "cash_movements"), 0);
}

#[test]
fn missing_source_returns_not_found() {
    let harness = setup();
    seed_warehouse(&harness, "MAIN");
    let error = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(DocumentId::new(), vec![tender(PaymentMethod::Cash, 10_000)]),
            TEST_TERMINAL,
        )
        .expect_err("missing");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::SourceDocumentNotFound)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
}

#[test]
fn tender_mismatch_is_rejected_both_directions() {
    for (label, amount) in [("under", 15_000), ("over", 25_000)] {
        let harness = setup();
        let warehouse = seed_warehouse(&harness, "MAIN");
        let (p1, u1) = seed_product(&harness, "P1", 10_000);
        seed_stock(&harness, p1, warehouse, 100_000);
        ensure_cash_session(&harness, warehouse);
        let bl = DocumentId::new();
        let bl = seed_bl(
            &harness,
            warehouse,
            DocumentStatus::Finalized,
            "BL-1",
            vec![bl_line(bl, p1, u1, 2, 10_000, 0)],
            None,
        );
        let error = close_uc(&harness)
            .execute(
                &session_with(Role::Gerant),
                close_input(bl, vec![tender(PaymentMethod::Cash, amount)]),
                TEST_TERMINAL,
            )
            .expect_err("mismatch");
        assert!(
            matches!(
                error,
                ApplicationError::Validation(DomainError::InsufficientPayment)
            ),
            "unexpected for {label}: {error:?}"
        );
        assert_eq!(count(&harness, "documents"), 1);
        assert_eq!(count(&harness, "cash_movements"), 1);
        assert_eq!(doc_status(&harness, bl), "FINALIZED");
    }
}

#[test]
fn idempotent_replay_and_conflict() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 1, 10_000, 0)],
        None,
    );
    let command_id = Uuid::new_v4().to_string();
    let first = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            ClosingInput {
                command_id: command_id.clone(),
                source_document_id: bl,
                language: DocumentLanguage::Fr,
                payments: vec![tender(PaymentMethod::Cash, 10_000)],
            },
            TEST_TERMINAL,
        )
        .expect("first");
    let second = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            ClosingInput {
                command_id: command_id.clone(),
                source_document_id: bl,
                language: DocumentLanguage::Fr,
                payments: vec![tender(PaymentMethod::Cash, 10_000)],
            },
            TEST_TERMINAL,
        )
        .expect("replay");
    assert_eq!(first, second);
    assert_eq!(count(&harness, "documents"), 2);
    assert_eq!(
        count(&harness, "cash_movements"),
        2,
        "opening float + exactly one sale movement"
    );
    assert_eq!(count(&harness, "print_jobs"), 1);
    // Divergent payload under the same id: conflict, zero new writes.
    let bl2 = DocumentId::new();
    let bl2 = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-2",
        vec![bl_line(bl2, p1, u1, 1, 10_000, 0)],
        None,
    );
    let conflict = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            ClosingInput {
                command_id,
                source_document_id: bl2,
                language: DocumentLanguage::Fr,
                payments: vec![tender(PaymentMethod::Cash, 10_000)],
            },
            TEST_TERMINAL,
        )
        .expect_err("conflict");
    assert!(
        matches!(conflict, ApplicationError::IdempotencyConflict),
        "unexpected: {conflict:?}"
    );
    assert_eq!(count(&harness, "documents"), 3);
    assert_eq!(count(&harness, "cash_movements"), 2);
    assert_eq!(count(&harness, "customer_ledger"), 0);
}

#[test]
fn post_ledger_failure_rolls_back_everything() {
    // A seeded ledger/balance inconsistency trips the domain invariant
    // re-check AFTER the ledger insert and balance update: the strongest
    // deterministic equivalent of a crash between step (m) and COMMIT.
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let customer = seed_customer(&harness, "LIMITED", 1_000_000, 0);
    // Corrupt the cached balance out of band: ledger says 5_000 owed.
    {
        let conn = read_conn(&harness);
        conn.execute(
            "INSERT INTO customer_ledger
             (id, sequence_no, customer_id, entry_type, amount_centimes,
              running_balance_centimes, document_type, document_id,
              created_by, created_at)
             VALUES (?1, 1, ?2, 'DEBIT', 5000, 5000, 'SALE', 'seed', ?3,
                     '2026-01-01T00:00:00Z')",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                customer.to_string(),
                GERANT_UUID
            ],
        )
        .expect("ledger seed");
        conn.execute(
            "UPDATE customers SET current_balance_centimes = 0 WHERE id = ?1",
            [customer.to_string()],
        )
        .expect("corrupt balance");
    }
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 1, 10_000, 0)],
        Some(customer),
    );
    let error = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(bl, vec![tender(PaymentMethod::Credit, 10_000)]),
            TEST_TERMINAL,
        )
        .expect_err("invariant");
    assert!(
        matches!(error, ApplicationError::ConcurrencyInvariantViolated),
        "unexpected: {error:?}"
    );
    // Full rollback: no FACTURE, no extra ledger row, balance untouched,
    // source unsealed, no print job, no idempotency row.
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "customer_ledger"), 1);
    assert_eq!(customer_balance(&harness, customer), 0);
    assert_eq!(count(&harness, "print_jobs"), 0);
    assert_eq!(count(&harness, "command_log"), 0);
    assert_eq!(doc_status(&harness, bl), "FINALIZED");
}

#[test]
fn empty_and_non_positive_tenders_are_rejected() {
    // All four roles hold CreateSale (mirroring standalone BL creation, per
    // spec — no Unauthorized probe is constructible); the permission gate
    // itself is byte-identical to CheckoutSaleUseCase. Tender shape is
    // validated here instead.
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let bl = DocumentId::new();
    let bl = seed_bl(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "BL-1",
        vec![bl_line(bl, p1, u1, 1, 10_000, 0)],
        None,
    );
    let empty = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(bl, Vec::new()),
            TEST_TERMINAL,
        )
        .expect_err("empty tender");
    assert!(
        matches!(empty, ApplicationError::Validation(DomainError::EmptyCart)),
        "unexpected: {empty:?}"
    );
    let non_positive = close_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            close_input(bl, vec![tender(PaymentMethod::Cash, 0)]),
            TEST_TERMINAL,
        )
        .expect_err("zero tender");
    assert!(
        matches!(
            non_positive,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
        ),
        "unexpected: {non_positive:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "cash_movements"), 0);
}
