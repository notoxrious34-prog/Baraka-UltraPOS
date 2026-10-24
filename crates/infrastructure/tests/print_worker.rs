//! Print worker tests: fake transport/renderer, real SQLite backends.
//!
//! Proves the outbox state machine, retry budget, crash recovery, and the
//! core invariant: print failures never mutate committed accounting rows.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    PrintJobRepository, PrinterSettingsPort, PrinterTransportPort, ProductRepository,
    ReceiptRendererPort, SessionContext, SessionToken, StockRepository, WarehouseRepository,
};
use baraka_application::services::print_worker_service::{
    process_one_job, process_one_job_with_identity, JobOutcome,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::printing::ReceiptDocument;
use baraka_domain::role::Role;
use baraka_domain::sales::{DocumentId, DocumentLanguage, DocumentType, PaymentMethod};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteDocumentRepository, SqlitePrintJobRepository, SqlitePrinterSettings,
    SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";

const TEST_TERMINAL: &str = "TEST-TERMINAL-PRINT";

fn ensure_cash_session(db: &Arc<Mutex<rusqlite::Connection>>, warehouse: WarehouseId) {
    let guard = db.lock().expect("lock");
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
    let mut conn = open_connection(&dir.path().join("print.db")).expect("open");
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

/// In-memory transport with injectable failures; counts physical sends.
struct FakeTransport {
    available: Mutex<bool>,
    failures_left: Mutex<u32>,
    sent: Mutex<Vec<Vec<u8>>>,
}

impl FakeTransport {
    fn working() -> Self {
        Self {
            available: Mutex::new(true),
            failures_left: Mutex::new(0),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn failing_times(times: u32) -> Self {
        Self {
            available: Mutex::new(true),
            failures_left: Mutex::new(times),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn sends(&self) -> usize {
        self.sent.lock().expect("sent").len()
    }
}

impl PrinterTransportPort for FakeTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        let mut failures = self.failures_left.lock().expect("failures");
        if *failures > 0 {
            *failures -= 1;
            return Err(ApplicationError::Validation(
                DomainError::PrinterTransportUnavailable,
            ));
        }
        self.sent.lock().expect("sent").push(bytes.to_vec());
        Ok(())
    }

    fn is_available(&self) -> bool {
        *self.available.lock().expect("available")
    }
}

/// Renderer stub with injectable failure.
struct FakeRenderer {
    fail: bool,
}

impl ReceiptRendererPort for FakeRenderer {
    fn render(&self, _receipt: &ReceiptDocument) -> Result<Vec<u8>, ApplicationError> {
        if self.fail {
            return Err(ApplicationError::Validation(
                DomainError::PrintRenderingFailed,
            ));
        }
        Ok(vec![0x1D, 0x76, 0x30, 0x00, 0x01, 0x00, 0x01, 0x00, 0xFF])
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
                purchase_price: Money::from_centimes(price - 2_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            "TEST-TERMINAL",
        )
        .expect("product");
    (id, unit_id)
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
        .expect("wh");
    warehouse.id
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
    ensure_cash_session(&harness.db, warehouse);
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

            timbre_enabled: true,
        },
        TEST_TERMINAL,
    )
    .expect("checkout");
    DocumentId::from(Uuid::parse_str(&receipt.document_id).expect("uuid"))
}

fn job_status(harness: &Harness, job_id: &str) -> (String, i64, Option<String>) {
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .query_row(
            "SELECT status, attempt_count, last_error FROM print_jobs WHERE id = ?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("job row")
}

fn run_once(
    harness: &Harness,
    transport: &FakeTransport,
    renderer: &FakeRenderer,
) -> Result<JobOutcome, ApplicationError> {
    process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        transport,
        renderer,
    )
}

#[test]
fn happy_path_claims_renders_sends_and_completes() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);
    let transport = FakeTransport::working();
    let renderer = FakeRenderer { fail: false };
    let outcome = run_once(&harness, &transport, &renderer).expect("process");
    let job_id = match outcome {
        JobOutcome::Done { job_id } => job_id,
        other => panic!("expected Done, got {other:?}"),
    };
    assert_eq!(transport.sends(), 1);
    assert_eq!(job_status(&harness, &job_id).0, "DONE");
    let _ = document;
}

#[test]
fn transient_failures_retry_with_backoff_then_done() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);
    let transport = FakeTransport::failing_times(2);
    let renderer = FakeRenderer { fail: false };
    let first = run_once(&harness, &transport, &renderer).expect("first");
    assert!(
        matches!(
            first,
            JobOutcome::RetryAfter { delay, .. } if delay == std::time::Duration::from_secs(1)
        ),
        "unexpected: {first:?}"
    );
    let second = run_once(&harness, &transport, &renderer).expect("second");
    assert!(
        matches!(
            second,
            JobOutcome::RetryAfter { delay, .. } if delay == std::time::Duration::from_secs(2)
        ),
        "unexpected: {second:?}"
    );
    let third = run_once(&harness, &transport, &renderer).expect("third");
    assert!(
        matches!(third, JobOutcome::Done { .. }),
        "unexpected: {third:?}"
    );
    assert_eq!(transport.sends(), 1);
}

#[test]
fn permanent_failure_marks_failed_after_three_sends() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);
    let transport = FakeTransport::failing_times(99);
    let renderer = FakeRenderer { fail: false };
    run_once(&harness, &transport, &renderer).expect("attempt 1");
    run_once(&harness, &transport, &renderer).expect("attempt 2");
    let outcome = run_once(&harness, &transport, &renderer).expect("attempt 3");
    let job_id = match outcome {
        JobOutcome::Failed { job_id, .. } => job_id,
        other => panic!("expected Failed, got {other:?}"),
    };
    let (status, attempts, last_error) = job_status(&harness, &job_id);
    assert_eq!(status, "FAILED");
    assert_eq!(
        attempts, 3,
        "three bounded physical attempts are recorded terminally"
    );
    let last_error = last_error.expect("last_error populated");
    assert!(
        last_error.starts_with("EXHAUSTED_MAX_RETRIES:"),
        "terminal reason prefixes budget exhaustion, got {last_error}"
    );
}

#[test]
fn failed_print_leaves_accounting_byte_identical() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);
    let snapshot = snapshot_accounting(&harness);
    // Force a render failure (worst-case print fault class).
    let transport = FakeTransport::working();
    let renderer = FakeRenderer { fail: true };
    let outcome = run_once(&harness, &transport, &renderer).expect("process");
    assert!(
        matches!(outcome, JobOutcome::Failed { .. }),
        "unexpected: {outcome:?}"
    );
    assert_eq!(transport.sends(), 0, "nothing transmitted");
    assert_eq!(
        snapshot_accounting(&harness),
        snapshot,
        "accounting rows must be byte-identical"
    );
    let _ = document;
}

#[test]
fn stale_processing_reset_on_startup_cycle() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);
    // Simulate a crash: claim the job, then backdate its claim far away.
    let claimed = harness
        .jobs
        .claim_next_pending()
        .expect("claim")
        .expect("job");
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .execute(
            "UPDATE print_jobs SET claimed_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
            [&claimed.job_id],
        )
        .expect("backdate");
    let reset = harness.jobs.reclaim_stale_processing(60).expect("reclaim");
    assert_eq!(reset, 1);
    assert_eq!(job_status(&harness, &claimed.job_id).0, "PENDING");
}

/// Full-table snapshot of every accounting table (ordered, debug-rendered).
fn snapshot_accounting(harness: &Harness) -> String {
    let conn = rusqlite::Connection::open(harness._dir.path().join("print.db")).expect("reopen");
    let mut out = String::new();
    for table in [
        "documents",
        "document_items",
        "stock_movements",
        "stock_levels",
        "payments",
        "customer_ledger",
        "customers",
    ] {
        let mut statement = conn
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .expect("prepare");
        let columns = statement.column_count();
        let mut rows = statement.query([]).expect("query");
        while let Some(row) = rows.next().expect("row") {
            out.push_str(table);
            out.push('|');
            for index in 0..columns {
                let value: rusqlite::types::Value = row.get(index).expect("cell");
                out.push_str(&format!("{value:?};"));
            }
            out.push('\n');
        }
    }
    out
}

fn job_row(harness: &Harness, job_id: &str) -> String {
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .query_row(
            "SELECT id, document_id, job_type, status, attempt_count FROM print_jobs WHERE id = ?1",
            [job_id],
            |row| {
                let id: String = row.get(0)?;
                let doc: String = row.get(1)?;
                let kind: String = row.get(2)?;
                let status: String = row.get(3)?;
                let attempts: i64 = row.get(4)?;
                Ok(format!("{id}|{doc}|{kind}|{status}|{attempts}"))
            },
        )
        .expect("job row")
}

#[test]
fn reprint_appends_duplicata_and_watermarks_output() {
    use baraka_application::use_cases::printing::ReprintDocumentUseCase;
    use baraka_domain::printing::{ReceiptDocument, ReceiptLanguage, StoreIdentity};
    use baraka_infrastructure::printing::rasterizer::{encode_gs_v0, render_canvas, PaperWidth};

    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);
    let original_id: String = rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .query_row(
            "SELECT id FROM print_jobs WHERE document_id = ?1 ORDER BY rowid LIMIT 1",
            [document.to_string()],
            |row| row.get(0),
        )
        .expect("original job");
    let before = job_row(&harness, &original_id);

    // Vendeur is rejected before any write.
    let denied = ReprintDocumentUseCase::new(&harness.jobs, &harness.documents)
        .execute(&session_with(Role::Vendeur), document);
    assert!(
        matches!(denied, Err(ApplicationError::Unauthorized)),
        "unexpected: {denied:?}"
    );

    // Gerant enqueues a DUPLICATA row; the original is untouched.
    let reprint_id = ReprintDocumentUseCase::new(&harness.jobs, &harness.documents)
        .execute(&session_with(Role::Gerant), document)
        .expect("reprint");
    assert_ne!(reprint_id, original_id);
    assert_eq!(job_row(&harness, &original_id), before);
    let kind: String = rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .query_row(
            "SELECT job_type FROM print_jobs WHERE id = ?1",
            [&reprint_id],
            |row| row.get(0),
        )
        .expect("kind");
    assert_eq!(kind, "DUPLICATA");

    // The watermark changes the raster bytes for the same document.
    let receipt = ReceiptDocument {
        language: ReceiptLanguage::French,
        shop_name: "BARAKA".to_string(),
        number: "TICK-2026-000001".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        lines: Vec::new(),
        totals: baraka_domain::printing::ReceiptTotals {
            subtotal_ht: "100.00".to_string(),
            tva: "0.00".to_string(),
            discount: "0.00".to_string(),
            timbre: "0.00".to_string(),
            total_ttc: "100.00".to_string(),
            paid: "100.00".to_string(),
            change: "0.00".to_string(),
        },
        payments: Vec::new(),
        is_duplicata: false,
        store_identity: StoreIdentity::default(),
        footer_message: String::new(),
    };
    let mut stamped = receipt.clone();
    stamped.is_duplicata = true;
    let plain = encode_gs_v0(&render_canvas(&receipt, PaperWidth::Width80mm).expect("plain"));
    let marked = encode_gs_v0(&render_canvas(&stamped, PaperWidth::Width80mm).expect("stamped"));
    assert_ne!(plain, marked, "watermark must alter output");
}

#[test]
fn worker_thread_processes_then_stops_cleanly() {
    use baraka_infrastructure::printing::worker_thread::{spawn_print_worker, WorkerDeps};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);
    let transport = Arc::new(FakeTransport::working());
    let renderer = Arc::new(FakeRenderer { fail: false });
    let stop = Arc::new(AtomicBool::new(false));
    let handle = spawn_print_worker(
        WorkerDeps {
            documents: Arc::new(SqliteDocumentRepository::new(Arc::clone(&harness.db))),
            jobs: Arc::new(SqlitePrintJobRepository::new(Arc::clone(&harness.db))),
            products: Arc::new(SqliteCatalogRepository::new(Arc::clone(&harness.db))),
            transport: transport.clone(),
            renderer,
            settings: Arc::new(SqlitePrinterSettings::new(Arc::clone(&harness.db))),
            printer: Arc::new(SqlitePrinterSettings::new(Arc::clone(&harness.db))),
        },
        Arc::clone(&stop),
    );
    // Let the loop poll and print (500ms cadence), then stop and join.
    std::thread::sleep(Duration::from_millis(1_500));
    stop.store(true, Ordering::SeqCst);
    let started = Instant::now();
    handle.join().expect("join worker");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "shutdown must be bounded"
    );
    assert_eq!(transport.sends(), 1, "worker printed the queued job");
}

#[test]
fn arabic_document_renders_through_worker() {
    use baraka_infrastructure::printing::rasterizer::EscPosRasterizer;

    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, 100_000)
        .expect("stock");
    ensure_cash_session(&harness.db, warehouse);
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
            language: DocumentLanguage::Ar,
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

            timbre_enabled: true,
        },
        TEST_TERMINAL,
    )
    .expect("arabic checkout");
    // Stored language drives the worker: real rasterizer, fake transport.
    let transport = FakeTransport::working();
    let outcome = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &EscPosRasterizer,
    )
    .expect("process");
    assert!(
        matches!(outcome, JobOutcome::Done { .. }),
        "unexpected: {outcome:?}"
    );
    assert_eq!(transport.sends(), 1);
    let _ = receipt;
}

// ---------------------------------------------------------------------------
// TASK-BRK-034: bounded outbox lifecycle (canonical names).
// DONE is the canonical COMPLETED terminal state in this codebase; the
// worker never performs more than PRINT_MAX_ATTEMPTS (3) physical sends.
// ---------------------------------------------------------------------------

fn job_full(harness: &Harness, job_id: &str) -> (String, i64, Option<String>, i64) {
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .query_row(
            "SELECT status, attempt_count, last_error, is_duplicate FROM print_jobs WHERE id = ?1",
            [job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("job row")
}

fn latest_job_id(harness: &Harness, document: DocumentId) -> String {
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .query_row(
            "SELECT id FROM print_jobs WHERE document_id = ?1 ORDER BY rowid DESC LIMIT 1",
            [document.to_string()],
            |row| row.get(0),
        )
        .expect("latest job")
}

/// Renderer that records the duplicata watermark flag per render call.
struct CapturingRenderer {
    seen: Mutex<Vec<bool>>,
}

impl ReceiptRendererPort for CapturingRenderer {
    fn render(&self, receipt: &ReceiptDocument) -> Result<Vec<u8>, ApplicationError> {
        self.seen.lock().expect("seen").push(receipt.is_duplicata);
        Ok(vec![0x1D, 0x76, 0x30, 0x00])
    }
}

/// Transport that is never available (unconfigured printer edge).
struct UnavailableTransport;

impl PrinterTransportPort for UnavailableTransport {
    fn send_raster(&self, _bytes: &[u8]) -> Result<(), ApplicationError> {
        Err(ApplicationError::Validation(
            DomainError::PrinterTransportUnavailable,
        ))
    }

    fn is_available(&self) -> bool {
        false
    }
}

#[test]
fn test_print_worker_successful_delivery_transitions_to_completed() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);
    let job_id = latest_job_id(&harness, document);
    let transport = FakeTransport::working();
    let renderer = FakeRenderer { fail: false };
    let outcome = run_once(&harness, &transport, &renderer).expect("process");
    assert!(
        matches!(outcome, JobOutcome::Done { job_id: ref id } if id == &job_id),
        "unexpected: {outcome:?}"
    );
    let (status, attempts, _, _) = job_full(&harness, &job_id);
    // DONE is the canonical COMPLETED terminal state.
    assert_eq!(status, "DONE");
    assert_eq!(attempts, 0, "first-try success consumes no retry budget");
    assert_eq!(transport.sends(), 1);
    // Queue drained: no further attempts are performed.
    let idle = run_once(&harness, &transport, &renderer).expect("idle");
    assert!(matches!(idle, JobOutcome::Idle), "unexpected: {idle:?}");
}

#[test]
fn test_print_worker_transient_failure_increments_attempts_and_sets_duplicata() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);
    let job_id = latest_job_id(&harness, document);
    let transport = FakeTransport::failing_times(1);
    let renderer = CapturingRenderer {
        seen: Mutex::new(Vec::new()),
    };
    let first = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
    )
    .expect("first");
    assert!(
        matches!(first, JobOutcome::RetryAfter { .. }),
        "unexpected: {first:?}"
    );
    let (status, attempts, last_error, is_duplicate) = job_full(&harness, &job_id);
    assert_eq!(status, "PENDING");
    assert_eq!(attempts, 1, "one retry budget unit consumed");
    assert_eq!(is_duplicate, 1, "retried attempt tags is_duplicata");
    assert!(last_error.is_some(), "last_error recorded on retry");
    // The retry renders watermarked as a duplicata.
    let second = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
    )
    .expect("second");
    assert!(
        matches!(second, JobOutcome::Done { .. }),
        "unexpected: {second:?}"
    );
    let seen = renderer.seen.lock().expect("seen").clone();
    assert_eq!(seen, vec![false, true], "retry renders duplicata watermark");
    let (status, attempts, _, is_duplicate) = job_full(&harness, &job_id);
    assert_eq!(status, "DONE");
    assert_eq!(attempts, 1);
    assert_eq!(is_duplicate, 1);
}

#[test]
fn test_print_worker_exhausted_retries_transitions_to_failed() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);
    let job_id = latest_job_id(&harness, document);
    let transport = FakeTransport::failing_times(99);
    let renderer = FakeRenderer { fail: false };
    let first = run_once(&harness, &transport, &renderer).expect("attempt 1");
    assert!(
        matches!(first, JobOutcome::RetryAfter { .. }),
        "unexpected: {first:?}"
    );
    let second = run_once(&harness, &transport, &renderer).expect("attempt 2");
    assert!(
        matches!(second, JobOutcome::RetryAfter { .. }),
        "unexpected: {second:?}"
    );
    let third = run_once(&harness, &transport, &renderer).expect("attempt 3");
    assert!(
        matches!(third, JobOutcome::Failed { .. }),
        "unexpected: {third:?}"
    );
    let (status, attempts, last_error, is_duplicate) = job_full(&harness, &job_id);
    assert_eq!(status, "FAILED");
    assert_eq!(attempts, 3, "bounded at exactly 3 physical attempts");
    let last_error = last_error.expect("terminal last_error");
    assert!(
        last_error.starts_with("EXHAUSTED_MAX_RETRIES:"),
        "got {last_error}"
    );
    assert_eq!(is_duplicate, 1);
    // Terminal jobs are never retried again: worker goes idle.
    let idle = run_once(&harness, &transport, &renderer).expect("idle");
    assert!(matches!(idle, JobOutcome::Idle), "unexpected: {idle:?}");
    assert_eq!(transport.sends(), 0);
}

#[test]
fn test_stale_pending_jobs_reconciled_on_startup() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let stale_doc = commit_sale(&harness, warehouse, product, unit);
    let stale_job = latest_job_id(&harness, stale_doc);
    // Backdate far beyond the 1-hour horizon to simulate the 8 orphaned
    // DUPLICATA PENDING rows from the TASK-BRK-030 audit.
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .execute(
            "UPDATE print_jobs SET created_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
            [&stale_job],
        )
        .expect("backdate");
    // A fresh job (append-only reprint row, no second cash session needed)
    // must survive reconciliation.
    let fresh_job = harness.jobs.enqueue_reprint(stale_doc).expect("reprint");
    let reconciled = harness
        .jobs
        .reconcile_stale_pending(3600)
        .expect("reconcile");
    assert_eq!(reconciled, 1, "only the stale row is abandoned");
    let (status, _, last_error, _) = job_full(&harness, &stale_job);
    assert_eq!(status, "FAILED");
    assert_eq!(last_error.as_deref(), Some("ABANDONED_STALE_SESSION"));
    let (fresh_status, _, _, _) = job_full(&harness, &fresh_job);
    assert_eq!(fresh_status, "PENDING");
    // Exhausted-budget rows are abandoned regardless of age.
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .execute(
            "UPDATE print_jobs SET attempt_count = 3 WHERE id = ?1",
            [&fresh_job],
        )
        .expect("exhaust budget");
    let reconciled = harness
        .jobs
        .reconcile_stale_pending(3600)
        .expect("reconcile exhausted");
    assert_eq!(reconciled, 1);
    let (status, _, last_error, _) = job_full(&harness, &fresh_job);
    assert_eq!(status, "FAILED");
    assert_eq!(last_error.as_deref(), Some("ABANDONED_STALE_SESSION"));
}

#[test]
fn test_unconfigured_printer_is_bounded_and_never_spins() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);
    let job_id = latest_job_id(&harness, document);
    let transport = UnavailableTransport;
    let renderer = FakeRenderer { fail: false };
    let first = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
    )
    .expect("attempt 1");
    assert!(
        matches!(first, JobOutcome::RetryAfter { .. }),
        "unavailable printer must back off, not spin: {first:?}"
    );
    let second = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
    )
    .expect("attempt 2");
    assert!(
        matches!(second, JobOutcome::RetryAfter { .. }),
        "unexpected: {second:?}"
    );
    let third = process_one_job(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
    )
    .expect("attempt 3");
    let reason = match third {
        JobOutcome::Failed { reason, .. } => reason,
        other => panic!("expected terminal Failed, got {other:?}"),
    };
    assert!(
        reason.contains("NO_PRINTER_CONFIGURED"),
        "terminal reason must name the missing printer, got {reason}"
    );
    let (status, attempts, last_error, _) = job_full(&harness, &job_id);
    assert_eq!(status, "FAILED");
    assert_eq!(attempts, 3);
    assert!(
        last_error
            .expect("last_error")
            .contains("NO_PRINTER_CONFIGURED"),
        "last_error descriptive"
    );
}

#[test]
fn null_document_id_job_enqueues_claims_and_completes() {
    // Migration 0014: non-sale jobs (debt receipts, Z-reports, test prints)
    // carry no sales document; the row, the claim, and the terminal
    // transition must all round-trip `document_id == None`.
    let harness = setup();
    let job_id = harness
        .jobs
        .enqueue_job(None, "RECEIPT", "{}")
        .expect("enqueue null");
    let stored: Option<String> = harness
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT document_id FROM print_jobs WHERE id = ?1",
            [&job_id],
            |row| row.get(0),
        )
        .expect("read");
    assert_eq!(stored, None, "NULL must persist as NULL");
    let claimed = harness
        .jobs
        .claim_next_pending()
        .expect("claim")
        .expect("pending job");
    assert_eq!(claimed.job_id, job_id);
    assert_eq!(claimed.document_id, None);
    harness.jobs.mark_done(&job_id).expect("done");
    let (status, _, _, _) = job_full(&harness, &job_id);
    assert_eq!(status, "DONE");
}

#[test]
fn worker_honors_configured_58mm_paper_width() {
    use baraka_infrastructure::printing::rasterizer::EscPosRasterizer;

    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);

    // Merchant hardware: compact 58mm printer persisted in settings.
    let printer = SqlitePrinterSettings::new(Arc::clone(&harness.db));
    printer
        .update_printer_settings("COM1", 9_600, Some(58), None, None, None)
        .expect("store 58mm");

    let transport = FakeTransport::working();
    let renderer = EscPosRasterizer;
    let outcome = process_one_job_with_identity(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
        &printer,
        &printer,
    )
    .expect("process");
    assert!(
        matches!(outcome, JobOutcome::Done { .. }),
        "unexpected: {outcome:?}"
    );
    assert_eq!(transport.sends(), 1, "one raster payload transmitted");
    let sent = transport.sent.lock().expect("sent");
    let stream = &sent[0];
    // Encoder output opens with the GS v 0 band header; 58mm stride is
    // 48 bytes/scanline (0x30), never the 80mm 72 (0x48).
    assert!(
        stream.len() >= 8,
        "raster stream carries a full band header"
    );
    assert_eq!(
        &stream[0..4],
        &[0x1D, 0x76, 0x30, 0x00],
        "stream opens with GS v 0"
    );
    assert_eq!(
        &stream[4..6],
        &[0x30, 0x00],
        "worker honored 58mm (48-byte stride)"
    );
}

/// Latest audit action recorded for a print job row.
fn last_job_audit_action(harness: &Harness, job_id: &str) -> Option<String> {
    rusqlite::Connection::open(harness._dir.path().join("print.db"))
        .expect("reopen")
        .query_row(
            "SELECT action FROM audit_log WHERE entity_type = 'print_jobs' AND entity_id = ?1
             ORDER BY sequence_no DESC LIMIT 1",
            [job_id],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .expect("audit read")
}

#[test]
fn graphique_mode_frontend_renders_without_bytes_and_audits() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);

    let printer = SqlitePrinterSettings::new(Arc::clone(&harness.db));
    printer
        .update_printer_settings("COM1", 9_600, Some(80), None, None, Some("graphique"))
        .expect("store graphique");
    assert_eq!(
        printer
            .get_printer_settings()
            .expect("reread")
            .ticket_render_mode,
        "graphique"
    );

    let transport = FakeTransport::working();
    let renderer = FakeRenderer { fail: false };
    let outcome = process_one_job_with_identity(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
        &printer,
        &printer,
    )
    .expect("process");
    let job_id = match outcome {
        JobOutcome::FrontendRendered { job_id } => job_id,
        other => panic!("expected FrontendRendered, got {other:?}"),
    };
    // Zero-bytes proof: the transport spy saw no payload at all.
    assert_eq!(transport.sends(), 0, "graphique emits zero ESC/POS bytes");
    let (status, attempts, _) = job_status(&harness, &job_id);
    assert_eq!(status, "FRONTEND_RENDERED");
    assert_eq!(attempts, 0, "graphique consumes no attempt budget");
    assert_eq!(
        last_job_audit_action(&harness, &job_id).as_deref(),
        Some("TICKET_FRONTEND_RENDERED"),
        "terminal transition carries its audit event"
    );
}

#[test]
fn escpos_mode_stays_on_the_byte_path() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    commit_sale(&harness, warehouse, product, unit);

    let printer = SqlitePrinterSettings::new(Arc::clone(&harness.db));
    printer
        .update_printer_settings("COM1", 9_600, Some(80), None, None, Some("escpos"))
        .expect("store escpos");

    let transport = FakeTransport::working();
    let renderer = FakeRenderer { fail: false };
    let outcome = process_one_job_with_identity(
        &harness.documents,
        &harness.jobs,
        &harness.catalog,
        &transport,
        &renderer,
        &printer,
        &printer,
    )
    .expect("process");
    assert!(
        matches!(outcome, JobOutcome::Done { .. }),
        "unexpected: {outcome:?}"
    );
    assert_eq!(transport.sends(), 1, "escpos still transmits one payload");
    assert!(last_job_audit_action(
        &harness,
        &match outcome {
            JobOutcome::Done { job_id } => job_id,
            other => panic!("expected Done, got {other:?}"),
        }
    )
    .is_none());
}

#[test]
fn graphique_reprint_returns_terminal_status_without_bytes() {
    use baraka_application::use_cases::printing::ReprintDocumentUseCase;

    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000);
    let document = commit_sale(&harness, warehouse, product, unit);

    let printer = SqlitePrinterSettings::new(Arc::clone(&harness.db));
    printer
        .update_printer_settings("COM1", 9_600, Some(80), None, None, Some("graphique"))
        .expect("store graphique");

    let reprint_id = ReprintDocumentUseCase::new(&harness.jobs, &harness.documents)
        .execute(&session_with(Role::Gerant), document)
        .expect("reprint");
    let transport = FakeTransport::working();
    let renderer = FakeRenderer { fail: false };
    // Two PENDING rows exist (original sale + DUPLICATA reprint); the
    // worker claims in row order, so both must drain through graphique.
    for _ in 0..2 {
        let outcome = process_one_job_with_identity(
            &harness.documents,
            &harness.jobs,
            &harness.catalog,
            &transport,
            &renderer,
            &printer,
            &printer,
        )
        .expect("process");
        assert!(
            matches!(outcome, JobOutcome::FrontendRendered { .. }),
            "unexpected: {outcome:?}"
        );
    }
    assert_eq!(transport.sends(), 0, "reprint emits zero bytes");
    let (status, _, _) = job_status(&harness, &reprint_id);
    assert_eq!(status, "FRONTEND_RENDERED");
    let row = harness
        .jobs
        .get_job_status(document)
        .expect("status read")
        .expect("row");
    assert_eq!(row.status_code, "FRONTEND_RENDERED");
}
