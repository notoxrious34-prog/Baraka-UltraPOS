//! Crash-matrix child driver (ADR-063): seed, hook, checkout, die.
//!
//! The driver builds the SAME application/infrastructure stack the
//! integration tests use over a driver-owned file connection, registers
//! SQLite's own connection hooks (zero failpoint hooks in production
//! code), then runs one checkout. Kill scenarios die by
//! `std::process::abort()`; retry/concurrent children exit normally with
//! one JSON status line.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    NewCustomer, ProductRepository, SessionContext, SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, CustomerTier, DocumentLanguage, DocumentType, PaymentMethod,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use rusqlite::hooks::Action;
use uuid::Uuid;

use super::scenario::{
    command_id_for, emit_status_line, KillPoint, SaleKind, CASH_TTC_CENTIMES, CREDIT_TTC_CENTIMES,
    FACTOR_MICRO, QTY_UNITS, SEED_LEVEL_MICRO, UNIT_PRICE_CENTIMES,
};

const ADMIN_UUID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const TERMINAL_ID: &str = "CRASH-TERM";

/// Fully built sale stack over one driver-owned connection.
pub struct DriverStack {
    shared: Arc<Mutex<rusqlite::Connection>>,
    warehouse_id: WarehouseId,
    product_id: ProductId,
    unit_id: UnitId,
    customer_id: baraka_domain::sales::CustomerId,
    session: SessionContext,
}

/// Migrate + deterministically seed a scenario database. Hooks are
/// registered only AFTER this returns, so seed writes never trip them.
pub fn seed_db(db_path: &Path) -> DriverStack {
    let mut conn = open_connection(db_path).expect("driver open");
    run_migrations(&mut conn).expect("driver migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'crash-admin', 'hash', 'ADMIN', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [ADMIN_UUID],
    )
    .expect("seed admin");
    let shared = Arc::new(Mutex::new(conn));
    let catalog = SqliteCatalogRepository::new(Arc::clone(&shared));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&shared));
    let customers = SqliteCustomerRepository::new(Arc::clone(&shared));

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
    let product_id = ProductId::new();
    let unit_id = UnitId::new();
    catalog
        .create_product(
            &Product {
                id: product_id,
                sku: Sku::new("CRASH1").expect("sku"),
                barcode: None,
                name_ar: "منتج".to_string(),
                name_fr: "Produit".to_string(),
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
                barcode: Some(Barcode::new("611125CRASH1").expect("barcode")),
                sale_price: Money::from_centimes(UNIT_PRICE_CENTIMES),
                purchase_price: Money::from_centimes(8_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TERMINAL_ID,
        )
        .expect("seed product");
    // Opening stock WITH its movement row: the reconciliation invariant
    // (level == signed movement sum) must hold from the seed onward, so
    // the repo delta helper (level-only, no movement) is NOT used here.
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
                    SEED_LEVEL_MICRO
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
                    SEED_LEVEL_MICRO,
                    unit_id.to_string(),
                    ADMIN_UUID
                ],
            )
            .expect("seed opening movement");
    }

    // OPEN cash session on the driver terminal (checkout precondition).
    // Columns are the 0008 authoritative shape (no user_id/session_id).
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
        let status: String = guard
            .query_row(
                "SELECT status FROM cash_sessions WHERE id = ?1",
                [&session_id],
                |row| row.get(0),
            )
            .expect("session row");
        assert_eq!(status, "OPEN", "seeded cash session must be OPEN");
    }

    let session = SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ADMIN_UUID).expect("uuid")),
        role: Role::Admin,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    };
    let customer_id = CreateCustomerUseCase::new(&customers)
        .execute(
            &session,
            NewCustomer {
                name: "Crash Debtor".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 1_000_000,
                customer_tier: CustomerTier::Retail,
            },
            TERMINAL_ID,
        )
        .expect("seed customer");

    DriverStack {
        shared,
        warehouse_id,
        product_id,
        unit_id,
        customer_id,
        session,
    }
}

/// Fixed single-line payload: cash-exact or full credit, `qty_units` units.
fn build_input(
    stack: &DriverStack,
    kind: SaleKind,
    command_id: &str,
    qty_units: i64,
) -> CheckoutInput {
    // Cash covers its own timbre-bearing total exactly (change 0); credit
    // bears no timbre (no cash tendered).
    let tender = match kind {
        SaleKind::Cash => CASH_TTC_CENTIMES * qty_units,
        SaleKind::Credit => CREDIT_TTC_CENTIMES * qty_units,
    };
    let (payments, customer_id) = match kind {
        SaleKind::Cash => (
            vec![PaymentInput {
                method: PaymentMethod::Cash,
                amount_centimes: tender,
            }],
            None,
        ),
        SaleKind::Credit => (
            vec![PaymentInput {
                method: PaymentMethod::Credit,
                amount_centimes: tender,
            }],
            Some(stack.customer_id),
        ),
    };
    CheckoutInput {
        command_id: command_id.to_string(),
        warehouse_id: stack.warehouse_id,
        customer_id,
        doc_type: DocumentType::Ticket,
        language: DocumentLanguage::Fr,
        lines: vec![CartLineInput {
            product_id: stack.product_id,
            unit_id: stack.unit_id,
            quantity_micro: qty_units * FACTOR_MICRO,
            unit_price_override: None,
            line_discount_centimes: 0,
        }],
        global_discount_centimes: 0,
        payments,
        allow_negative_stock: false,
        notes: None,
    }
}

fn run_checkout(
    stack: &DriverStack,
    kind: SaleKind,
    command_id: &str,
    qty_units: i64,
) -> Result<String, String> {
    let customers = SqliteCustomerRepository::new(Arc::clone(&stack.shared));
    let catalog = SqliteCatalogRepository::new(Arc::clone(&stack.shared));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&stack.shared));
    let checkout = SqliteCheckoutRepository::new(Arc::clone(&stack.shared));
    let use_case = CheckoutSaleUseCase::new(&catalog, &customers, &checkout, &warehouses);
    use_case
        .execute(
            &stack.session,
            build_input(stack, kind, command_id, qty_units),
            TERMINAL_ID,
        )
        .map(|receipt| receipt.document_number)
        .map_err(|error| format!("{error:?}"))
}

/// Register the scenario's kill hooks on the driver-owned connection.
/// Seed writes are already done, so every hook below can only fire from
/// the checkout's own transaction.
fn register_hooks(stack: &DriverStack, point: KillPoint) {
    let guard = stack.shared.lock().expect("lock");
    match point {
        KillPoint::Kp1PreBegin | KillPoint::Kp7bPostResponse => {}
        KillPoint::Kp2FirstWrite => {
            guard.update_hook(Some(
                move |action: Action, _db: &str, table: &str, _rowid: i64| {
                    if table == "document_sequences"
                        && matches!(action, Action::SQLITE_INSERT | Action::SQLITE_UPDATE)
                    {
                        std::process::abort();
                    }
                },
            ));
        }
        KillPoint::Kp3Documents => {
            guard.update_hook(Some(
                move |action: Action, _db: &str, table: &str, _rowid: i64| {
                    if table == "documents" && action == Action::SQLITE_INSERT {
                        std::process::abort();
                    }
                },
            ));
        }
        KillPoint::Kp4Stock => {
            let documents_seen = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&documents_seen);
            guard.update_hook(Some(
                move |action: Action, _db: &str, table: &str, _rowid: i64| {
                    if table == "documents" && action == Action::SQLITE_INSERT {
                        flag.store(true, Ordering::SeqCst);
                    } else if flag.load(Ordering::SeqCst)
                        && table == "stock_levels"
                        && matches!(action, Action::SQLITE_INSERT | Action::SQLITE_UPDATE)
                    {
                        std::process::abort();
                    }
                },
            ));
        }
        KillPoint::Kp5Ledger => {
            let stock_seen = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&stock_seen);
            guard.update_hook(Some(
                move |action: Action, _db: &str, table: &str, _rowid: i64| {
                    if table == "stock_movements" && action == Action::SQLITE_INSERT {
                        flag.store(true, Ordering::SeqCst);
                    } else if flag.load(Ordering::SeqCst)
                        && table == "customer_ledger"
                        && action == Action::SQLITE_INSERT
                    {
                        std::process::abort();
                    }
                },
            ));
        }
        KillPoint::Kp6PrintJob => {
            guard.update_hook(Some(
                move |action: Action, _db: &str, table: &str, _rowid: i64| {
                    if table == "print_jobs" && action == Action::SQLITE_INSERT {
                        std::process::abort();
                    }
                },
            ));
        }
        KillPoint::Kp7aCommit => {
            // Latch on the first sale-specific write (documents INSERT);
            // abort in the commit hook on the FIRST commit observed after
            // the latch is set. Commit-hook timing is pre-commit (the
            // frame is not yet in the WAL), so this strike deterministically
            // yields NOT COMMITTED. Commit counting is deliberately NOT
            // used: seed/migration commits on the same connection must not
            // matter. (rusqlite 0.32 exposes no wal_hook; see recon.)
            let armed = Arc::new(AtomicBool::new(false));
            let latch = Arc::clone(&armed);
            guard.update_hook(Some(
                move |action: Action, _db: &str, table: &str, _rowid: i64| {
                    if table == "documents" && action == Action::SQLITE_INSERT {
                        latch.store(true, Ordering::SeqCst);
                    }
                },
            ));
            let fire = Arc::clone(&armed);
            guard.commit_hook(Some(move || {
                if fire.load(Ordering::SeqCst) {
                    std::process::abort();
                }
                false
            }));
        }
    }
    drop(guard);
}

/// `--crash-child <scenario-id> --db <path>`: seed, arm, checkout, die.
pub fn crash_child(scenario_id: &str, kind: SaleKind, point: KillPoint, db_path: &Path) -> ! {
    let stack = seed_db(db_path);
    if point == KillPoint::Kp1PreBegin {
        std::process::abort();
    }
    register_hooks(&stack, point);
    let command_id = command_id_for(scenario_id);
    match run_checkout(&stack, kind, &command_id, QTY_UNITS) {
        Ok(_) => {
            if point == KillPoint::Kp7bPostResponse {
                std::process::abort();
            }
            // Any other kill point reaching here means its hook never
            // fired: loud setup failure, never a silent pass.
            eprintln!("crash child exited normally without dying at {scenario_id}");
            std::process::exit(2);
        }
        Err(error) => {
            eprintln!("crash child checkout failed at {scenario_id}: {error}");
            std::process::exit(2);
        }
    }
}

/// `--retry-child <scenario-id> --db <path>`: same payload + command id,
/// no hooks; prints the JSON status line and exits 0 on success.
pub fn retry_child(scenario_id: &str, kind: SaleKind, db_path: &Path) -> ! {
    let stack = seed_db_existing(db_path);
    let command_id = command_id_for(scenario_id);
    let had_row: bool = stack
        .shared
        .lock()
        .expect("lock")
        .query_row(
            "SELECT COUNT(*) FROM command_log WHERE command_id = ?1",
            [&command_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);
    match run_checkout(&stack, kind, &command_id, QTY_UNITS) {
        Ok(number) => {
            emit_status_line(if had_row { "cached" } else { "committed-new" }, &number);
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("retry child failed at {scenario_id}: {error}");
            std::process::exit(2);
        }
    }
}

fn parse_uuid_id<T>(raw: String, ctor: fn(Uuid) -> T) -> T {
    ctor(Uuid::parse_str(&raw).expect("uuid id"))
}
/// Open an already-seeded database (retry/concurrent children never seed).
fn seed_db_existing(db_path: &Path) -> DriverStack {
    let conn = open_connection(db_path).expect("child reopen");
    let shared = Arc::new(Mutex::new(conn));
    // Static ids are re-read from the seeded rows. The warehouse is
    // anchored on the OPEN cash session (migrations seed their own
    // warehouse row, so LIMIT 1 over warehouses would pick the wrong one).
    let (warehouse_id, product_id, unit_id, customer_id) = {
        let guard = shared.lock().expect("lock");
        let warehouse: String = guard
            .query_row(
                "SELECT warehouse_id FROM cash_sessions WHERE status = 'OPEN' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("session warehouse");
        let (product, unit): (String, String) = guard
            .query_row(
                "SELECT p.id, u.id FROM products p JOIN product_units u ON u.product_id = p.id LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("product row");
        let customer: String = guard
            .query_row("SELECT id FROM customers LIMIT 1", [], |row| row.get(0))
            .expect("customer row");
        (
            parse_uuid_id(warehouse, WarehouseId::from),
            parse_uuid_id(product, ProductId::from),
            parse_uuid_id(unit, UnitId::from),
            parse_uuid_id(customer, CustomerId::from),
        )
    };
    let session = SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ADMIN_UUID).expect("uuid")),
        role: Role::Admin,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    };
    DriverStack {
        shared,
        warehouse_id,
        product_id,
        unit_id,
        customer_id,
        session,
    }
}

/// `--concurrent-child <worker-id> --db <path>`: three sequential cash
/// checkouts (one unit each) with worker-scoped command ids against the
/// shared database.
pub fn concurrent_child(worker_id: &str, db_path: &Path) -> ! {
    let stack = seed_db_existing(db_path);
    let mut last_number = String::new();
    for attempt in 0..3 {
        let command_id = format!("crash-concurrent-{worker_id}-{attempt}");
        match run_checkout(&stack, SaleKind::Cash, &command_id, QTY_UNITS) {
            Ok(number) => last_number = number,
            Err(error) => {
                eprintln!("concurrent child {worker_id} failed: {error}");
                std::process::exit(2);
            }
        }
    }
    emit_status_line("committed-new", &last_number);
    std::process::exit(0);
}
