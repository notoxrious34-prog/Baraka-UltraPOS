//! SQLite return-bundle integration tests (TASK-BRK-046).
//!
//! File-backed temporary databases (WAL/crash standards): every test opens
//! `returns.db` in a fresh tempdir, runs all migrations (incl. 0013
//! `REFUND_OUT`), and asserts atomicity by counting rows before and after
//! each persisted or rejected return.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    DocumentRepository, NewCustomer, PersistReturnLine, PersistReturnOrder, PersistReturnReceipt,
    ProductRepository, ReturnCashLeg, ReturnCreditLeg, ReturnRepository, SessionContext,
    SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, Document, DocumentId, DocumentItem, DocumentLanguage, DocumentStatus,
    DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCustomerRepository, SqliteDocumentRepository,
    SqliteReturnRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "88888888-8888-8888-8888-888888888888";
const TEST_TERMINAL: &str = "TEST-TERMINAL-RET";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    warehouses: SqliteWarehouseRepository,
    documents: SqliteDocumentRepository,
    customers: SqliteCustomerRepository,
    returns: SqliteReturnRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("returns.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('88888888-8888-8888-8888-888888888888', 'gerant1', 'hash', 'GERANT',
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
fn seed_sale(
    harness: &Harness,
    warehouse: WarehouseId,
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
                doc_type: DocumentType::Ticket,
                status: DocumentStatus::Finalized,
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
        .expect("seed sale");
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

fn return_line(source: &DocumentItem, qty_units: i64, sort_order: i64) -> PersistReturnLine {
    let qty = qty_units * 10_000;
    PersistReturnLine {
        source_item_id: source.id.to_string(),
        product_id: source.product_id,
        unit_id: source.unit_id,
        quantity_micro: qty,
        quantity_base_micro: qty,
        unit_price_centimes: source.unit_price_centimes,
        discount_centimes: 0,
        line_ht_centimes: source.unit_price_centimes * qty_units,
        line_tva_centimes: 0,
        line_ttc_centimes: source.unit_price_centimes * qty_units,
        cost_price_centimes: source.cost_price_centimes,
        sort_order,
        reason: None,
    }
}

fn return_order(
    source: DocumentId,
    warehouse: WarehouseId,
    lines: Vec<PersistReturnLine>,
    ttc: i64,
    cash_session: Option<String>,
    credit_customer: Option<CustomerId>,
) -> PersistReturnOrder {
    PersistReturnOrder {
        idempotency_key: Uuid::new_v4().to_string(),
        request_canonical: format!("v1;process-return;test;{source}"),
        source_document_id: source,
        terminal_id: TEST_TERMINAL.to_string(),
        warehouse_id: warehouse,
        document_language: DocumentLanguage::Fr,
        subtotal_ht_centimes: ttc,
        vat_total_centimes: 0,
        timbre_centimes: 0,
        ttc_centimes: ttc,
        cash_refund: cash_session.map(|session_id| ReturnCashLeg {
            cash_session_id: session_id,
            amount_centimes: ttc,
        }),
        credit_refund: credit_customer.map(|customer_id| ReturnCreditLeg {
            customer_id,
            amount_centimes: ttc,
        }),
        lines,
        actor: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        notes: None,
    }
}

fn persist(
    harness: &Harness,
    order: &PersistReturnOrder,
) -> Result<PersistReturnReceipt, baraka_application::errors::ApplicationError> {
    harness.returns.persist_return(order)
}

#[test]
fn test_cumulative_returned_quantities_empty_and_populated() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "R1", 10_000);
    let (p2, u2) = seed_product(&harness, "R2", 20_000);
    let draft = DocumentId::new();
    let (sale, lines) = seed_sale(
        &harness,
        warehouse,
        "TICK-2026-000001",
        vec![
            sale_line(draft, p1, u1, 5, 10_000, 0),
            sale_line(draft, p2, u2, 4, 20_000, 1),
        ],
        None,
    );
    // No prior returns: empty map.
    let empty = harness
        .returns
        .cumulative_returned_quantities(sale)
        .expect("cumulative");
    assert!(empty.is_empty());
    // Return 3 units of item 1 (30_000 micros).
    persist(
        &harness,
        &return_order(
            sale,
            warehouse,
            vec![return_line(&lines[0], 3, 0)],
            30_000,
            None,
            None,
        ),
    )
    .expect("first return");
    let after_first: HashMap<String, i64> = harness
        .returns
        .cumulative_returned_quantities(sale)
        .expect("cumulative");
    assert_eq!(after_first.len(), 1);
    assert_eq!(after_first.get(&lines[0].id.to_string()), Some(&30_000));
    // Second return: 2 more units of item 1, 4 units of item 2.
    persist(
        &harness,
        &return_order(
            sale,
            warehouse,
            vec![return_line(&lines[0], 2, 0), return_line(&lines[1], 4, 1)],
            100_000,
            None,
            None,
        ),
    )
    .expect("second return");
    let after_second = harness
        .returns
        .cumulative_returned_quantities(sale)
        .expect("cumulative");
    assert_eq!(after_second.get(&lines[0].id.to_string()), Some(&50_000));
    assert_eq!(after_second.get(&lines[1].id.to_string()), Some(&40_000));
}

#[test]
fn test_persist_return_cash_refund_atomic() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "R1", 10_000);
    set_stock(&harness, p1, warehouse, 80_000);
    let session_id = seed_cash_session(&harness, warehouse);
    let draft = DocumentId::new();
    let (sale, lines) = seed_sale(
        &harness,
        warehouse,
        "TICK-2026-000002",
        vec![sale_line(draft, p1, u1, 2, 10_000, 0)],
        None,
    );
    let receipt = persist(
        &harness,
        &return_order(
            sale,
            warehouse,
            vec![return_line(&lines[0], 1, 0)],
            10_000,
            Some(session_id.clone()),
            None,
        ),
    )
    .expect("persist");
    assert_eq!(receipt.return_document_number, "AVO-2026-000001");
    // AVOIR header FINALIZED with source linkage.
    let header: (String, String, Option<String>, i64) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT type, status, source_document_id, ttc_centimes FROM documents WHERE id = ?1",
            [&receipt.return_document_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("avoir header");
    assert_eq!(header.0, "AVOIR");
    assert_eq!(header.1, "FINALIZED");
    assert_eq!(header.2.as_deref(), Some(sale.to_string()).as_deref());
    assert_eq!(header.3, 10_000);
    // Items carry source_item_id.
    let source_link: String = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT source_item_id FROM document_items WHERE document_id = ?1",
            [&receipt.return_document_id],
            |row| row.get(0),
        )
        .expect("item link");
    assert_eq!(source_link, lines[0].id.to_string());
    // Stock restored by the returned base quantity.
    assert_eq!(stock_level(&harness, p1, warehouse), 90_000);
    let movement: String = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT movement_type FROM stock_movements
             WHERE reference_id = ?1 ORDER BY rowid DESC LIMIT 1",
            [&receipt.return_document_id],
            |row| row.get(0),
        )
        .expect("movement");
    assert_eq!(movement, "RETURN_IN");
    // Drawer REFUND_OUT with strictly negative centimes.
    let refund: (String, i64) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT movement_type, amount_centimes FROM cash_movements
             WHERE cash_session_id = ?1 AND movement_type = 'REFUND_OUT'
             ORDER BY rowid DESC LIMIT 1",
            [&session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("refund");
    assert_eq!(refund, ("REFUND_OUT".to_string(), -10_000));
    // Audit + outbox + idempotency rows exist.
    assert_eq!(count(&harness, "audit_log"), 1);
    let print: (String, String) = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT status, job_type FROM print_jobs WHERE document_id = ?1",
            [&receipt.return_document_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("print job");
    assert_eq!(print, ("PENDING".to_string(), "RECEIPT".to_string()));
    assert_eq!(count(&harness, "command_log"), 1);
    // Idempotent replay of the same order returns the identical receipt.
    let replay_order = return_order(
        sale,
        warehouse,
        vec![return_line(&lines[0], 1, 0)],
        10_000,
        Some(session_id.clone()),
        None,
    );
    let replay = persist(&harness, &replay_order).expect("replay persists anew");
    assert_ne!(replay.return_document_id, receipt.return_document_id);
}

#[test]
fn test_persist_return_credit_refund_atomic() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "R1", 10_000);
    set_stock(&harness, p1, warehouse, 50_000);
    let customer = seed_customer(&harness, "Cherif Benz");
    let draft = DocumentId::new();
    let (sale, lines) = seed_sale(
        &harness,
        warehouse,
        "TICK-2026-000003",
        vec![sale_line(draft, p1, u1, 2, 10_000, 0)],
        Some(customer),
    );
    let receipt = persist(
        &harness,
        &return_order(
            sale,
            warehouse,
            vec![return_line(&lines[0], 2, 0)],
            20_000,
            None,
            Some(customer),
        ),
    )
    .expect("persist");
    // Ledger CREDIT leg references the AVOIR.
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
    assert_eq!(ledger.1, 20_000);
    assert_eq!(ledger.2, -20_000);
    assert_eq!(ledger.3, "AVOIR");
    // Cached balance decremented by total TTC.
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
    assert_eq!(balance, -20_000);
    // No drawer movement for a pure credit settlement.
    let cash_rows: i64 = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT COUNT(*) FROM cash_movements WHERE reference_id = ?1",
            [&receipt.return_document_id],
            |row| row.get(0),
        )
        .expect("cash count");
    assert_eq!(cash_rows, 0);
    assert_eq!(stock_level(&harness, p1, warehouse), 70_000);
}

#[test]
fn test_persist_return_rollback_on_failure() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "R1", 10_000);
    set_stock(&harness, p1, warehouse, 80_000);
    let draft = DocumentId::new();
    let (sale, lines) = seed_sale(
        &harness,
        warehouse,
        "TICK-2026-000004",
        vec![sale_line(draft, p1, u1, 2, 10_000, 0)],
        None,
    );
    let before = (
        count(&harness, "documents"),
        count(&harness, "stock_movements"),
        count(&harness, "command_log"),
        count(&harness, "audit_log"),
        stock_level(&harness, p1, warehouse),
    );
    // Second line references a product that does not exist: apply_one
    // fails AFTER the first line's restock leg, proving full rollback.
    let mut bad_lines = vec![return_line(&lines[0], 1, 0)];
    let mut ghost = return_line(&lines[0], 1, 1);
    ghost.product_id = ProductId::new();
    bad_lines.push(ghost);
    let failed = persist(
        &harness,
        &return_order(sale, warehouse, bad_lines, 20_000, None, None),
    );
    assert!(failed.is_err(), "ghost product must fail");
    assert_eq!(
        (
            count(&harness, "documents"),
            count(&harness, "stock_movements"),
            count(&harness, "command_log"),
            count(&harness, "audit_log"),
            stock_level(&harness, p1, warehouse),
        ),
        before,
        "failed bundle must leave zero trace"
    );
    // A retried valid order still allocates the first sequence number.
    let receipt = persist(
        &harness,
        &return_order(
            sale,
            warehouse,
            vec![return_line(&lines[0], 1, 0)],
            10_000,
            None,
            None,
        ),
    )
    .expect("retry");
    assert_eq!(receipt.return_document_number, "AVO-2026-000001");
}
