//! v1.0.0 release benchmarks (ADR-015 methodology).
//!
//! Every test in this file is `#[ignore]`: ignored tests add ZERO to the
//! G3 passed count and appear only in the ignored count. Run explicitly:
//!
//! ```sh
//! CARGO_BUILD_JOBS=2 cargo test --release -p baraka-infrastructure \
//!   --test release_benchmarks -- --ignored --nocapture
//! ```
//!
//! Environment contract: temporary FILE-BACKED SQLite databases (never
//! `:memory:`, so WAL semantics hold), seeded through the same repository
//! stack the crash-matrix suite uses. Metrics use `std::time::Instant` and
//! manual percentile computation only — no benchmark dependencies.
//!
//! M1 product_search — the real `ProductRepository::search` path plus exact
//! barcode lookup over the seeded dataset with randomized query sampling.
//! M2 atomic_sale — the full `CheckoutSaleUseCase` path against the
//! file-backed DB, fresh idempotency key per iteration.
//! M3 print_job_insert — direct timed `enqueue_job` calls (the same
//! repository function the sale transaction calls via `enqueue_in_tx`).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use baraka_application::ports::{
    PrintJobRepository, ProductRepository, SessionContext, SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{DocumentId, DocumentLanguage, DocumentType, PaymentMethod};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqlitePrintJobRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ADMIN_UUID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const TERMINAL_ID: &str = "BENCH-TERM";
const PRODUCT_COUNT: usize = 10_000;
const WARMUP: usize = 5;
const MEASURED: usize = 50;
const UNIT_PRICE_CENTIMES: i64 = 10_000;
const CASH_TTC_CENTIMES: i64 = 10_500;
const FACTOR_MICRO: i64 = 10_000;
const STOCK_MICRO_PER_PRODUCT: i64 = 1_000_000_000;

/// Seeded file-backed benchmark database with a `PRODUCT_COUNT` catalog.
struct BenchDb {
    shared: Arc<Mutex<rusqlite::Connection>>,
    warehouse_id: WarehouseId,
    product_id: ProductId,
    unit_id: UnitId,
    session: SessionContext,
    barcodes: Vec<String>,
}

fn seed_bench_db(db_path: &std::path::Path) -> BenchDb {
    let mut conn = open_connection(db_path).expect("bench open");
    run_migrations(&mut conn).expect("bench migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'bench-admin', 'hash', 'ADMIN', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [ADMIN_UUID],
    )
    .expect("seed admin");
    let shared = Arc::new(Mutex::new(conn));
    let catalog = SqliteCatalogRepository::new(Arc::clone(&shared));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&shared));

    let warehouse_id = WarehouseId::new();
    warehouses
        .create_warehouse(
            &Warehouse {
                id: warehouse_id,
                code: WarehouseCode::new("MAIN").expect("code"),
                name: "Main".to_string(),
                is_active: true,
            },
            TERMINAL_ID,
        )
        .expect("seed warehouse");

    // One dedicated checkout product (first row) + the bulk catalog.
    let mut barcodes = Vec::with_capacity(PRODUCT_COUNT);
    let (first_product, first_unit) = seed_one_product(
        &catalog,
        &shared,
        &warehouse_id,
        0,
        UNIT_PRICE_CENTIMES,
        &mut barcodes,
    );
    for index in 1..PRODUCT_COUNT {
        seed_one_product(
            &catalog,
            &shared,
            &warehouse_id,
            index,
            UNIT_PRICE_CENTIMES + (index as i64 % 500),
            &mut barcodes,
        );
    }

    // OPEN cash session (checkout precondition; 0008 column shape).
    {
        let guard = shared.lock().expect("lock");
        let session_id = Uuid::new_v4().to_string();
        guard
            .execute(
                "INSERT INTO cash_sessions
                 (id, warehouse_id, terminal_id, status, opened_by, opened_at,
                  opening_float_centimes, revision, origin_device_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                         '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                rusqlite::params![
                    session_id,
                    warehouse_id.to_string(),
                    TERMINAL_ID,
                    ADMIN_UUID
                ],
            )
            .expect("open cash session");
        guard
            .execute(
                "INSERT INTO cash_movements
                 (id, cash_session_id, movement_type, amount_centimes, description,
                  created_at, created_by)
                 VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float',
                         '2026-01-01T00:00:00Z', ?3)",
                rusqlite::params![Uuid::new_v4().to_string(), session_id, ADMIN_UUID],
            )
            .expect("opening float");
    }

    let session = SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ADMIN_UUID).expect("uuid")),
        role: Role::Admin,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    };

    BenchDb {
        shared,
        warehouse_id,
        product_id: first_product,
        unit_id: first_unit,
        session,
        barcodes,
    }
}

fn seed_one_product(
    catalog: &SqliteCatalogRepository,
    shared: &Arc<Mutex<rusqlite::Connection>>,
    warehouse_id: &WarehouseId,
    index: usize,
    price_centimes: i64,
    barcodes: &mut Vec<String>,
) -> (ProductId, UnitId) {
    let product_id = ProductId::new();
    let unit_id = UnitId::new();
    let barcode = format!("611125{index:06}");
    catalog
        .create_product(
            &Product {
                id: product_id,
                sku: Sku::new(&format!("BENCH{index:05}")).expect("sku"),
                barcode: None,
                name_ar: format!("منتج قياس {index}"),
                name_fr: format!("Produit Benchmark {index}"),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: unit_id,
                product_id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(FACTOR_MICRO),
                barcode: Some(Barcode::new(&barcode).expect("barcode")),
                sale_price: Money::from_centimes(price_centimes),
                purchase_price: Money::from_centimes(8_000),
                is_base_unit: true,
                is_default_selling: true,
            }],
            TERMINAL_ID,
        )
        .expect("seed product");
    {
        let guard = shared.lock().expect("lock");
        guard
            .execute(
                "INSERT INTO stock_levels
                 (warehouse_id, product_id, quantity_micro, quantity_base_micro, revision, updated_at)
                 VALUES (?1, ?2, ?3, ?3, 1, '2026-01-01T00:00:00Z')",
                rusqlite::params![
                    warehouse_id.to_string(),
                    product_id.to_string(),
                    STOCK_MICRO_PER_PRODUCT
                ],
            )
            .expect("seed level");
        guard
            .execute(
                "INSERT INTO stock_movements
                 (id, sequence_no, warehouse_id, product_id, movement_type,
                  quantity_micro, unit_id, quantity_base_micro,
                  created_by, created_at)
                 VALUES (?1, 1, ?2, ?3, 'ADJUSTMENT_POSITIVE', ?4, ?5, ?4, ?6,
                         '2026-01-01T00:00:00Z')",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    warehouse_id.to_string(),
                    product_id.to_string(),
                    STOCK_MICRO_PER_PRODUCT,
                    unit_id.to_string(),
                    ADMIN_UUID
                ],
            )
            .expect("seed opening movement");
    }
    barcodes.push(barcode);
    (product_id, unit_id)
}

/// Nearest-rank percentile over microsecond samples.
fn percentile(mut samples: Vec<u128>, rank: f64) -> u128 {
    samples.sort_unstable();
    let index = ((rank / 100.0) * samples.len() as f64).ceil() as usize - 1;
    samples[index.min(samples.len() - 1)]
}

fn report_ms(metric: &str, samples: &[u128]) {
    let to_ms = |v: u128| v as f64 / 1_000.0;
    println!(
        "BENCH {metric} n={} p50={:.3}ms p95={:.3}ms p99={:.3}ms",
        samples.len(),
        to_ms(percentile(samples.to_vec(), 50.0)),
        to_ms(percentile(samples.to_vec(), 95.0)),
        to_ms(percentile(samples.to_vec(), 99.0)),
    );
}

fn bench_temp_db(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let db = dir.path().join(format!("{name}.db"));
    (dir, db)
}

/// M1 — real search path: alternating prefix search and exact barcode
/// lookup with rotating queries over the full seeded catalog.
#[ignore]
#[test]
fn bench_m1_product_search() {
    let (_dir, db) = bench_temp_db("bench_m1");
    let seed_start = Instant::now();
    let stack = seed_bench_db(&db);
    println!(
        "BENCH seed products={PRODUCT_COUNT} elapsed={:.1}s",
        seed_start.elapsed().as_secs_f64()
    );
    let catalog = SqliteCatalogRepository::new(Arc::clone(&stack.shared));
    // Sanity outside the timed loop: both paths hit the seeded data.
    assert!(
        !catalog
            .search("Produit Benchmark 42", 20)
            .expect("search")
            .is_empty(),
        "prefix search must hit seeded catalog"
    );
    let probe = Barcode::new(&stack.barcodes[42]).expect("barcode");
    assert!(
        catalog.find_by_barcode(&probe).expect("barcode").is_some(),
        "barcode lookup must hit seeded catalog"
    );

    let mut samples = Vec::with_capacity(MEASURED);
    let total = WARMUP + MEASURED;
    for iteration in 0..total {
        // Deterministic rotation (no RNG dependency): spread over the catalog.
        let slot = (iteration * 7919) % PRODUCT_COUNT;
        let start = Instant::now();
        if iteration % 2 == 0 {
            // True prefix query against the real LIKE-prefix path.
            let _ = catalog
                .search(&format!("Produit Benchmark {slot}"), 20)
                .expect("search");
        } else {
            let code = Barcode::new(&stack.barcodes[slot]).expect("barcode");
            let _ = catalog.find_by_barcode(&code).expect("barcode");
        }
        if iteration >= WARMUP {
            samples.push(start.elapsed().as_micros());
        }
    }
    report_ms("product_search", &samples);
}

/// M2 — full atomic checkout path, cash-exact single line, fresh
/// idempotency key per iteration (no physical print involved).
#[ignore]
#[test]
fn bench_m2_atomic_sale() {
    let (_dir, db) = bench_temp_db("bench_m2");
    let stack = seed_bench_db(&db);
    let catalog = SqliteCatalogRepository::new(Arc::clone(&stack.shared));
    let customers = SqliteCustomerRepository::new(Arc::clone(&stack.shared));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&stack.shared));
    let checkout = SqliteCheckoutRepository::new(Arc::clone(&stack.shared));
    let use_case = CheckoutSaleUseCase::new(&catalog, &customers, &checkout, &warehouses);

    let mut samples = Vec::with_capacity(MEASURED);
    for iteration in 0..(WARMUP + MEASURED) {
        let input = CheckoutInput {
            command_id: Uuid::new_v4().to_string(),
            warehouse_id: stack.warehouse_id,
            customer_id: None,
            doc_type: DocumentType::Ticket,
            language: DocumentLanguage::Fr,
            lines: vec![CartLineInput {
                product_id: stack.product_id,
                unit_id: stack.unit_id,
                quantity_micro: FACTOR_MICRO,
                unit_price_override: None,
                line_discount_centimes: 0,
            }],
            global_discount_centimes: 0,
            payments: vec![PaymentInput {
                method: PaymentMethod::Cash,
                amount_centimes: CASH_TTC_CENTIMES,
            }],
            allow_negative_stock: false,
            notes: None,
        };
        let start = Instant::now();
        use_case
            .execute(&stack.session, input, TERMINAL_ID)
            .expect("checkout");
        if iteration >= WARMUP {
            samples.push(start.elapsed().as_micros());
        }
    }
    report_ms("atomic_sale", &samples);
}

/// M3 — direct print-job persistence: the same repository function the
/// sale transaction calls, against a real prior document id.
#[ignore]
#[test]
fn bench_m3_print_job_insert() {
    let (_dir, db) = bench_temp_db("bench_m3");
    let stack = seed_bench_db(&db);
    let catalog = SqliteCatalogRepository::new(Arc::clone(&stack.shared));
    let customers = SqliteCustomerRepository::new(Arc::clone(&stack.shared));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&stack.shared));
    let checkout = SqliteCheckoutRepository::new(Arc::clone(&stack.shared));
    let use_case = CheckoutSaleUseCase::new(&catalog, &customers, &checkout, &warehouses);
    use_case
        .execute(
            &stack.session,
            CheckoutInput {
                command_id: Uuid::new_v4().to_string(),
                warehouse_id: stack.warehouse_id,
                customer_id: None,
                doc_type: DocumentType::Ticket,
                language: DocumentLanguage::Fr,
                lines: vec![CartLineInput {
                    product_id: stack.product_id,
                    unit_id: stack.unit_id,
                    quantity_micro: FACTOR_MICRO,
                    unit_price_override: None,
                    line_discount_centimes: 0,
                }],
                global_discount_centimes: 0,
                payments: vec![PaymentInput {
                    method: PaymentMethod::Cash,
                    amount_centimes: CASH_TTC_CENTIMES,
                }],
                allow_negative_stock: false,
                notes: None,
            },
            TERMINAL_ID,
        )
        .expect("seed document");
    let document_id: String = stack
        .shared
        .lock()
        .expect("lock")
        .query_row("SELECT id FROM documents LIMIT 1", [], |row| row.get(0))
        .expect("document row");
    let document_id = DocumentId::from(Uuid::parse_str(&document_id).expect("uuid"));

    let jobs = SqlitePrintJobRepository::new(Arc::clone(&stack.shared));
    let mut samples = Vec::with_capacity(MEASURED);
    for iteration in 0..(WARMUP + MEASURED) {
        let start = Instant::now();
        jobs.enqueue_job(document_id, "TICKET", "{}")
            .expect("enqueue");
        if iteration >= WARMUP {
            samples.push(start.elapsed().as_micros());
        }
    }
    report_ms("print_job_insert", &samples);
}
