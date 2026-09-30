//! End-to-end sales return tests: `ProcessSaleReturnUseCase` against real
//! SQLite repositories (TASK-BRK-045 re-issue).
//!
//! Unlike the fake-port use-case suite and the adapter-only persistence
//! suite, these tests drive the full vertical slice — validation,
//! cumulative ceilings, fixed-point math, settlement legs, and the atomic
//! bundle — over a file-backed temporary database.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    DocumentRepository, NewCustomer, ProductRepository, SessionContext, SessionToken,
    WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::returns::{
    ProcessReturnCommand, ProcessSaleReturnUseCase, RefundDestination, ReturnError, ReturnItemInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money, QtyMicro};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, Document, DocumentId, DocumentItem, DocumentLanguage, DocumentStatus,
    DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCashSessionRepository, SqliteCatalogRepository, SqliteCustomerRepository,
    SqliteDocumentRepository, SqliteReturnRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "99999999-9999-9999-9999-999999999999";
const TEST_TERMINAL: &str = "TEST-TERMINAL-SALERET";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    warehouses: SqliteWarehouseRepository,
    documents: SqliteDocumentRepository,
    customers: SqliteCustomerRepository,
    returns: SqliteReturnRepository,
    cash: SqliteCashSessionRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("sales_return.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('99999999-9999-9999-9999-999999999999', 'gerant1', 'hash', 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
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
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("product");
    (id, unit_id)
}

fn seed_customer(harness: &Harness, name: &str) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Gerant),
            NewCustomer {
                name: name.to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
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

fn sale_line(
    document_id: DocumentId,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    unit_price: i64,
    sort_order: i64,
) -> DocumentItem {
    let micros = qty_units * 10_000;
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
        line_ht_centimes: unit_price * qty_units,
        line_tva_centimes: 0,
        line_ttc_centimes: unit_price * qty_units,
        cost_price_centimes: unit_price - 2_000,
        sort_order,
    }
}

#[allow(clippy::too_many_arguments)]
fn seed_document(
    harness: &Harness,
    warehouse: WarehouseId,
    doc_type: DocumentType,
    status: DocumentStatus,
    number: &str,
    lines: Vec<DocumentItem>,
    party: Option<CustomerId>,
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
                number: number.to_string(),
                fiscal_year: chrono::Utc::now()
                    .format("%Y")
                    .to_string()
                    .parse()
                    .unwrap_or(1970),
                doc_type,
                status,
                warehouse_id: warehouse,
                party_id: party.map(|id| id.to_string()),
                subtotal_ht_centimes: ttc,
                vat_total_centimes: 0,
                global_discount_centimes: 0,
                timbre_centimes: 0,
                ttc_centimes: ttc,
                paid_amount_centimes: ttc,
                change_amount_centimes: 0,
                payment_method: "CASH".to_string(),
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

fn return_command(
    source: DocumentId,
    lines: &[(Uuid, i64)],
    refund: RefundDestination,
) -> ProcessReturnCommand {
    ProcessReturnCommand {
        operator_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        source_document_id: source,
        items: lines
            .iter()
            .map(|(line_id, qty_units)| ReturnItemInput {
                source_item_id: *line_id,
                quantity_micro: QtyMicro::from_micros(qty_units * 10_000),
                reason: None,
            })
            .collect(),
        refund,
        idempotency_key: Uuid::new_v4().to_string(),
        terminal_id: TEST_TERMINAL.to_string(),
        notes: None,
    }
}

fn execute(
    harness: &Harness,
    command: ProcessReturnCommand,
) -> Result<baraka_application::use_cases::returns::ProcessReturnResult, ApplicationError> {
    ProcessSaleReturnUseCase::new(
        &harness.documents,
        &harness.returns,
        &harness.customers,
        &harness.cash,
    )
    .execute(&session_with(Role::Gerant), command)
}

fn is_quantity_exceeded(error: &ApplicationError) -> bool {
    matches!(
        error,
        ApplicationError::Validation(DomainError::InvalidCanonicalString(code))
            if code == "RETURN_QUANTITY_EXCEEDED"
    )
}

#[test]
fn full_return_with_cash_refund_creates_numbered_avoir() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "SR1", 10_000);
    let (p2, u2) = seed_product(&harness, "SR2", 20_000);
    set_stock(&harness, p1, warehouse, 80_000);
    set_stock(&harness, p2, warehouse, 90_000);
    let session_id = seed_cash_session(&harness, warehouse);
    let draft = DocumentId::new();
    let (sale, lines) = seed_document(
        &harness,
        warehouse,
        DocumentType::Ticket,
        DocumentStatus::Finalized,
        "TICK-2026-000001",
        vec![
            sale_line(draft, p1, u1, 2, 10_000, 0),
            sale_line(draft, p2, u2, 1, 20_000, 1),
        ],
        None,
    );
    let result = execute(
        &harness,
        return_command(
            sale,
            &[(lines[0].id, 2), (lines[1].id, 1)],
            RefundDestination::Cash {
                cash_session_id: session_id.clone(),
            },
        ),
    )
    .expect("full return");
    let year: i32 = chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap_or(1970);
    assert_eq!(result.return_document_number, format!("AVO-{year}-000001"));
    assert_eq!(result.total_refund_ht.centimes(), 40_000);
    assert_eq!(result.total_refund_ttc.centimes(), 40_000);
    assert_eq!(result.items_returned_count, 2);
    // AVOIR header is FINALIZED and source-linked.
    let header: (String, String, Option<String>) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT type, status, source_document_id FROM documents WHERE id = ?1",
            [result.return_document_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("header");
    assert_eq!(header.0, "AVOIR");
    assert_eq!(header.1, "FINALIZED");
    assert_eq!(header.2.as_deref(), Some(sale.to_string()).as_deref());
    // Stock restored on both products.
    assert_eq!(stock_level(&harness, p1, warehouse), 100_000);
    assert_eq!(stock_level(&harness, p2, warehouse), 100_000);
    // Drawer paid out the full TTC as a negative REFUND_OUT.
    let refund: (String, i64) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT movement_type, amount_centimes FROM cash_movements
             WHERE cash_session_id = ?1 AND movement_type = 'REFUND_OUT'",
            [&session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("refund");
    assert_eq!(refund, ("REFUND_OUT".to_string(), -40_000));
}

#[test]
fn partial_return_with_customer_credit_reduces_debt() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "SR1", 10_000);
    set_stock(&harness, p1, warehouse, 80_000);
    let customer = seed_customer(&harness, "Cherif Benz");
    let draft = DocumentId::new();
    let (sale, lines) = seed_document(
        &harness,
        warehouse,
        DocumentType::Facture,
        DocumentStatus::Finalized,
        "FAC-2026-000001",
        vec![sale_line(draft, p1, u1, 2, 10_000, 0)],
        Some(customer),
    );
    let result = execute(
        &harness,
        return_command(
            sale,
            &[(lines[0].id, 1)],
            RefundDestination::CustomerCredit {
                customer_id: customer,
            },
        ),
    )
    .expect("partial credit return");
    assert_eq!(result.total_refund_ttc.centimes(), 10_000);
    assert_eq!(result.items_returned_count, 1);
    // Ledger CREDIT leg references the AVOIR; cached balance drops.
    let ledger: (String, i64, i64, String) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT entry_type, amount_centimes, running_balance_centimes, document_type
             FROM customer_ledger WHERE customer_id = ?1 ORDER BY rowid DESC LIMIT 1",
            [customer.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("ledger");
    assert_eq!(ledger.0, "CREDIT");
    assert_eq!(ledger.1, 10_000);
    assert_eq!(ledger.2, -10_000);
    assert_eq!(ledger.3, "AVOIR");
    let balance: i64 = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = ?1",
            [customer.to_string()],
            |row| row.get(0),
        )
        .expect("balance");
    assert_eq!(balance, -10_000);
    assert_eq!(stock_level(&harness, p1, warehouse), 90_000);
}

#[test]
fn two_successive_partials_reach_exact_full_quantity() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "SR1", 10_000);
    set_stock(&harness, p1, warehouse, 80_000);
    let draft = DocumentId::new();
    let (sale, lines) = seed_document(
        &harness,
        warehouse,
        DocumentType::Ticket,
        DocumentStatus::Finalized,
        "TICK-2026-000002",
        vec![sale_line(draft, p1, u1, 2, 10_000, 0)],
        None,
    );
    let first = execute(
        &harness,
        return_command(sale, &[(lines[0].id, 1)], RefundDestination::NoRefund),
    )
    .expect("first partial");
    let second = execute(
        &harness,
        return_command(sale, &[(lines[0].id, 1)], RefundDestination::NoRefund),
    )
    .expect("second partial");
    assert_ne!(
        first.return_document_id.to_string(),
        second.return_document_id.to_string()
    );
    // Gapless, distinct AVO numbers across the two returns.
    let year: i32 = chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap_or(1970);
    assert_eq!(first.return_document_number, format!("AVO-{year}-000001"));
    assert_eq!(second.return_document_number, format!("AVO-{year}-000002"));
    assert_eq!(stock_level(&harness, p1, warehouse), 100_000);
    // The ceiling is now exactly exhausted: one more micro fails.
    let error = execute(
        &harness,
        return_command(sale, &[(lines[0].id, 1)], RefundDestination::NoRefund),
    )
    .expect_err("exhausted ceiling");
    assert!(is_quantity_exceeded(&error), "unexpected: {error:?}");
    // The structured taxonomy agrees with the wire code.
    let taxonomy = ReturnError::QuantityExceeded {
        item_id: lines[0].id,
        requested_micro: 10_000,
        allowable_micro: 0,
    };
    let mapped = ApplicationError::from(taxonomy);
    assert!(is_quantity_exceeded(&mapped));
}

#[test]
fn over_return_rejected_with_quantity_exceeded() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "SR1", 10_000);
    set_stock(&harness, p1, warehouse, 80_000);
    let draft = DocumentId::new();
    let (sale, lines) = seed_document(
        &harness,
        warehouse,
        DocumentType::Ticket,
        DocumentStatus::Finalized,
        "TICK-2026-000003",
        vec![sale_line(draft, p1, u1, 2, 10_000, 0)],
        None,
    );
    let before = (
        count(&harness, "documents"),
        count(&harness, "stock_movements"),
        stock_level(&harness, p1, warehouse),
    );
    let error = execute(
        &harness,
        return_command(sale, &[(lines[0].id, 3)], RefundDestination::NoRefund),
    )
    .expect_err("over-return");
    assert!(is_quantity_exceeded(&error), "unexpected: {error:?}");
    assert_eq!(
        (
            count(&harness, "documents"),
            count(&harness, "stock_movements"),
            stock_level(&harness, p1, warehouse),
        ),
        before,
        "rejected validation must not persist anything"
    );
}

#[test]
fn cash_return_without_active_session_fails() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "SR1", 10_000);
    set_stock(&harness, p1, warehouse, 80_000);
    // Intentionally no cash session seeded.
    let draft = DocumentId::new();
    let (sale, lines) = seed_document(
        &harness,
        warehouse,
        DocumentType::Ticket,
        DocumentStatus::Finalized,
        "TICK-2026-000004",
        vec![sale_line(draft, p1, u1, 1, 10_000, 0)],
        None,
    );
    let error = execute(
        &harness,
        return_command(
            sale,
            &[(lines[0].id, 1)],
            RefundDestination::Cash {
                cash_session_id: Uuid::new_v4().to_string(),
            },
        ),
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
}

#[test]
fn return_on_devis_is_rejected() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "SR1", 10_000);
    set_stock(&harness, p1, warehouse, 80_000);
    let draft = DocumentId::new();
    let (devis, lines) = seed_document(
        &harness,
        warehouse,
        DocumentType::Devis,
        DocumentStatus::Finalized,
        "DEV-2026-000001",
        vec![sale_line(draft, p1, u1, 1, 10_000, 0)],
        None,
    );
    let error = execute(
        &harness,
        return_command(devis, &[(lines[0].id, 1)], RefundDestination::NoRefund),
    )
    .expect_err("devis return");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidConversionSourceType)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(stock_level(&harness, p1, warehouse), 80_000);
}
