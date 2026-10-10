//! Debt receipt wiring (TASK-BRK-061): atomic enqueue + worker rendering.
//!
//! Proves the end-to-end pipeline: `record_credit_payment` enqueues a
//! `DEBT_RECEIPT` outbox row (document_id NULL) inside the same atomic
//! transaction, the worker routes it to `render_debt_receipt` and
//! completes it, unparseable payloads fail with a clear reason, and
//! normal sales receipts still claim and complete without regression.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CustomerRepository, NewCustomer, PrintJobRepository, PrinterTransportPort, ProductRepository,
    ReceiptRendererPort, SessionContext, SessionToken, StockRepository, WarehouseRepository,
};
use baraka_application::services::print_worker_service::{process_one_job, JobOutcome};
use baraka_application::use_cases::customers::{CreateCustomerUseCase, RecordCreditPaymentUseCase};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::printing::{DebtReceiptDocument, ReceiptDocument};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, DocumentId, DocumentLanguage, DocumentType, PaymentMethod,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteDocumentRepository, SqlitePrintJobRepository, SqliteStockRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "88888888-8888-8888-8888-888888888888";
const TEST_TERMINAL: &str = "TEST-TERMINAL-DEBT-WIRE";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    documents: SqliteDocumentRepository,
    jobs: SqlitePrintJobRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("debt_wire.db")).expect("open");
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
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
        jobs: SqlitePrintJobRepository::new(Arc::clone(&shared)),
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

/// In-memory transport; counts physical sends.
struct FakeTransport;

impl PrinterTransportPort for FakeTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        assert!(!bytes.is_empty(), "debt raster must be non-empty");
        SENT_COUNT.with(|count| *count.borrow_mut() += 1);
        Ok(())
    }

    fn is_available(&self) -> bool {
        true
    }
}

thread_local! {
    static SENT_COUNT: std::cell::RefCell<usize> = const { std::cell::RefCell::new(0) };
}

fn sent_count() -> usize {
    SENT_COUNT.with(|count| *count.borrow())
}

/// Renderer spy: counts sales vs debt renders separately.
struct DebtSpyRenderer {
    debt_renders: Mutex<usize>,
    sale_renders: Mutex<usize>,
}

impl DebtSpyRenderer {
    fn new() -> Self {
        Self {
            debt_renders: Mutex::new(0),
            sale_renders: Mutex::new(0),
        }
    }

    fn debt_renders(&self) -> usize {
        *self.debt_renders.lock().expect("debt renders")
    }

    fn sale_renders(&self) -> usize {
        *self.sale_renders.lock().expect("sale renders")
    }
}

impl ReceiptRendererPort for DebtSpyRenderer {
    fn render(&self, _receipt: &ReceiptDocument) -> Result<Vec<u8>, ApplicationError> {
        *self.sale_renders.lock().expect("sale renders") += 1;
        Ok(vec![0x1D, 0x76, 0x30, 0x00])
    }

    fn render_debt_receipt(
        &self,
        receipt: &DebtReceiptDocument,
    ) -> Result<Vec<u8>, ApplicationError> {
        assert!(
            !receipt.receipt_number.is_empty(),
            "debt payload must carry a receipt number"
        );
        *self.debt_renders.lock().expect("debt renders") += 1;
        Ok(vec![0x1D, 0x76, 0x30, 0x01])
    }
}

/// Open cash session for credit payments (dedicated warehouse + terminal).
fn ensure_payment_cash_session(harness: &Harness) {
    let guard = harness.db.lock().expect("lock");
    guard
        .execute(
            "INSERT INTO warehouses (id, code, name, is_active, revision, origin_device_id, created_at, updated_at)
             VALUES ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'DEBT-WH', 'Debt WH', 1, 1, 'LOCAL',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("warehouse");
    let session_id = Uuid::new_v4().to_string();
    guard
        .execute(
            "INSERT INTO cash_sessions
             (id, warehouse_id, terminal_id, status, opened_by, opened_at,
              opening_float_centimes, revision, origin_device_id, created_at, updated_at)
             VALUES (?1, 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', ?2, 'OPEN',
                     '88888888-8888-8888-8888-888888888888', '2026-01-01T00:00:00Z', 0, 1, ?2,
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![session_id, TEST_TERMINAL,],
        )
        .expect("open session");
    guard
        .execute(
            "INSERT INTO cash_movements
             (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
             VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z',
                     '88888888-8888-8888-8888-888888888888')",
            rusqlite::params![Uuid::new_v4().to_string(), session_id,],
        )
        .expect("opening float");
}

fn seed_debtor(harness: &Harness, balance: i64) -> CustomerId {
    let gerant = session_with(Role::Gerant);
    let id = CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &gerant,
            NewCustomer {
                name: "Débiteur".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
            },
            TEST_TERMINAL,
        )
        .expect("customer");
    if balance > 0 {
        let doc = Uuid::new_v4();
        harness
            .customers
            .record_credit_debit(id, balance, balance, doc.into(), actor())
            .expect("debit");
        harness
            .customers
            .update_cached_balance(id, balance)
            .expect("balance");
    }
    id
}

fn reopen(harness: &Harness) -> rusqlite::Connection {
    rusqlite::Connection::open(harness._dir.path().join("debt_wire.db")).expect("reopen")
}

fn job_status(harness: &Harness, job_id: &str) -> (String, Option<String>) {
    reopen(harness)
        .query_row(
            "SELECT status, last_error FROM print_jobs WHERE id = ?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("job row")
}

fn pay_and_fetch_pending_job_id(harness: &Harness) -> String {
    ensure_payment_cash_session(harness);
    let id = seed_debtor(harness, 20_000);
    let record = RecordCreditPaymentUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Caissier),
            id,
            8_000,
            Some("REC-1".to_string()),
            false,
            TEST_TERMINAL,
        )
        .expect("payment");
    assert_eq!(record.new_balance_centimes, 12_000);
    let (job_id,): (String,) = reopen(harness)
        .query_row(
            "SELECT id FROM print_jobs WHERE status = 'PENDING' ORDER BY rowid DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?,)),
        )
        .expect("pending debt job");
    job_id
}

#[test]
fn credit_payment_atomically_enqueues_debt_receipt_job() {
    let harness = setup();
    pay_and_fetch_pending_job_id(&harness);
    let conn = reopen(&harness);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM print_jobs", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 1, "exactly one outbox row per payment");
    let row: (Option<String>, String, String, String) = conn
        .query_row(
            "SELECT document_id, job_type, status, payload_json FROM print_jobs LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("row");
    assert_eq!(row.0, None, "debt jobs reference no sale document");
    assert_eq!(row.1, "DEBT_RECEIPT");
    assert_eq!(row.2, "PENDING");
    assert!(!row.3.is_empty(), "payload_json must be stored");
    let doc: DebtReceiptDocument =
        serde_json::from_str(&row.3).expect("payload parses as DebtReceiptDocument");
    assert_eq!(
        doc.receipt_number, "REC-000002",
        "receipt number reuses the customer_ledger sequence (ADR-082)"
    );
    assert_eq!(doc.amount_text, "80.00");
    assert_eq!(doc.new_balance_text, "120.00");
    assert_eq!(doc.customer_name, "Débiteur");
    assert_eq!(doc.reference_text.as_deref(), Some("REC-1"));
    // The pending-job claim path maps the stored payload into the record.
    let claimed = harness
        .jobs
        .claim_next_pending()
        .expect("claim")
        .expect("job");
    assert_eq!(claimed.document_id, None);
    assert_eq!(claimed.job_type, "DEBT_RECEIPT");
    assert_eq!(claimed.payload_json, row.3);
}

#[test]
fn worker_renders_debt_receipt_and_completes() {
    let harness = setup();
    let job_id = pay_and_fetch_pending_job_id(&harness);
    let renderer = DebtSpyRenderer::new();
    let outcome = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &FakeTransport,
        &renderer,
    )
    .expect("process");
    assert!(
        matches!(&outcome, JobOutcome::Done { job_id: done } if *done == job_id),
        "unexpected: {outcome:?}"
    );
    assert_eq!(renderer.debt_renders(), 1, "debt pipeline rendered once");
    assert_eq!(renderer.sale_renders(), 0, "sales pipeline untouched");
    assert_eq!(sent_count(), 1, "one physical send");
    assert_eq!(job_status(&harness, &job_id).0, "DONE");
}

#[test]
fn unparseable_debt_payload_marks_failed_with_clear_reason() {
    let harness = setup();
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO print_jobs
             (id, document_id, job_type, status, payload, payload_json,
              attempt_count, is_duplicate, created_at, updated_at)
             VALUES ('job-bogus-1', NULL, 'DEBT_RECEIPT', 'PENDING', 'not-json', 'not-json',
                     0, 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("insert bogus job");
    let renderer = DebtSpyRenderer::new();
    let outcome = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &FakeTransport,
        &renderer,
    )
    .expect("process");
    let (job_id, reason) = match outcome {
        JobOutcome::Failed { job_id, reason } => (job_id, reason),
        other => panic!("expected Failed, got {other:?}"),
    };
    assert_eq!(job_id, "job-bogus-1");
    assert!(
        reason.contains("DEBT_PAYLOAD_INVALID"),
        "reason must name the fault, got {reason}"
    );
    assert_eq!(renderer.debt_renders(), 0, "nothing rendered");
    let (status, last_error) = job_status(&harness, &job_id);
    assert_eq!(status, "FAILED");
    assert!(
        last_error
            .unwrap_or_default()
            .contains("DEBT_PAYLOAD_INVALID"),
        "stored error must name the fault"
    );
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
                purchase_price: Money::from_centimes(price - 2_000),
                is_base_unit: true,
                is_default_selling: true,
            }],
            "TEST-TERMINAL",
        )
        .expect("product");
    (id, unit_id)
}

/// Commit a real two-unit cash sale; returns its document id.
fn commit_sale(
    harness: &Harness,
    warehouse: WarehouseId,
    product: ProductId,
    unit: UnitId,
) -> DocumentId {
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, 100_000)
        .expect("stock");
    {
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
        guard
            .execute(
                "INSERT INTO cash_movements
                 (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
                 VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z', ?3)",
                rusqlite::params![Uuid::new_v4().to_string(), session_id, ACTOR_UUID,],
            )
            .expect("opening float");
    }
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &session_with(Role::Gerant),
        CheckoutInput {
            command_id: Uuid::new_v4().to_string(),
            warehouse_id: warehouse,
            customer_id: None,
            doc_type: DocumentType::Ticket,
            language: DocumentLanguage::Fr,
            lines: vec![CartLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 20_000,
                unit_price_override: None,
                line_discount_centimes: 0,
            }],
            global_discount_centimes: 0,
            payments: vec![PaymentInput {
                method: PaymentMethod::Cash,
                amount_centimes: 30_000,
            }],
            allow_negative_stock: false,
            notes: None,
        },
        TEST_TERMINAL,
    )
    .expect("checkout");
    DocumentId::from(Uuid::parse_str(&receipt.document_id).expect("uuid"))
}

#[test]
fn sales_receipt_still_claims_and_completes() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);
    let renderer = DebtSpyRenderer::new();
    let outcome = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &FakeTransport,
        &renderer,
    )
    .expect("process");
    let job_id = match outcome {
        JobOutcome::Done { job_id } => job_id,
        other => panic!("expected Done, got {other:?}"),
    };
    assert_eq!(renderer.sale_renders(), 1, "sales pipeline rendered once");
    assert_eq!(renderer.debt_renders(), 0, "debt pipeline untouched");
    assert_eq!(job_status(&harness, &job_id).0, "DONE");
}
