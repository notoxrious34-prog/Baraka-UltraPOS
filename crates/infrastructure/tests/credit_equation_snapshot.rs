//! TASK-BRK-108: wholesale credit-equation snapshot over real SQLite.
//!
//! Proves the ADR-110 Pillar 6.3 contract end-to-end on an isolated scratch
//! database (never the live `%APPDATA%` database):
//!
//! * a credit sale freezes `Ancien Solde`, `Nouveau Solde` and the signed
//!   movement inside the same atomic transaction, and the printed equation
//!   `Ancien + Montant - Versement = Nouveau` holds exactly in centimes;
//! * a cash-only sale records no snapshot at all (NULL, never zero);
//! * migration `0020` is additive: a populated `v1.2.1`-shaped (0019)
//!   database upgrades with zero data loss and legacy rows read back NULL.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    NewCustomer, ProductRepository, SessionContext, SessionToken, StockRepository,
    WarehouseRepository,
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
    CreditPolicy, CreditSnapshot, CustomerId, DocumentLanguage, DocumentType, PaymentMethod,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::{checksum_of, run_migrations, MIGRATIONS};
use chrono::TimeDelta;
use rusqlite::Connection;
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const TEST_TERMINAL: &str = "TEST-TERMINAL-BRK108";
const STAMP: &str = "2026-11-12T09:00:00Z";

struct Harness {
    _dir: tempfile::TempDir,
    db_path: PathBuf,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("credit-equation.db");
    let mut conn = open_connection(&db_path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'admin', 'hash', 'ADMIN', 1, ?2, ?2)",
        rusqlite::params![ACTOR_UUID, STAMP],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        db_path,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(shared),
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

fn read_conn(harness: &Harness) -> Connection {
    Connection::open(&harness.db_path).expect("reopen")
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

fn seed_product(harness: &Harness, sku: &str, sale_price: i64) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    let product = Product {
        id,
        sku: Sku::new(sku).expect("sku"),
        barcode: None,
        name_ar: "منتج".to_string(),
        name_fr: "Produit".to_string(),
        category_id: None,
        is_kit: false,
        is_active: true,
    };
    let unit = ProductUnit {
        id: unit_id,
        product_id: id,
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Pièce".to_string(),
        factor_micro: FactorMicro::from_micros(10_000),
        barcode: Some(Barcode::new(&format!("6111{sku}")).expect("barcode")),
        sale_price: Money::from_centimes(sale_price),
        purchase_price: Money::from_centimes(sale_price / 2),
        wholesale_price: None,
        super_gros_price: None,
        is_base_unit: true,
        is_default_selling: true,
    };
    harness
        .catalog
        .create_product(&product, &[unit], TEST_TERMINAL)
        .expect("product");
    (id, unit_id)
}

fn seed_customer(harness: &Harness) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Gerant),
            NewCustomer {
                name: "Grossiste Série".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Unlimited,
                credit_limit_centimes: 0,
                customer_tier: baraka_domain::sales::CustomerTier::Wholesale,
            },
            TEST_TERMINAL,
        )
        .expect("customer")
}

fn ensure_cash_session(harness: &Harness) {
    let conn = read_conn(harness);
    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO cash_sessions
         (id, warehouse_id, terminal_id, status, opened_by, opened_at,
          opening_float_centimes, revision, origin_device_id, created_at, updated_at)
         SELECT ?1, id, ?2, 'OPEN', ?3, ?4, 0, 1, ?2, ?4, ?4
         FROM warehouses WHERE code = 'MAIN'",
        rusqlite::params![session_id, TEST_TERMINAL, ACTOR_UUID, STAMP],
    )
    .expect("cash session");
}

/// `(before, after, delta)` as stored; all `None` when no movement was frozen.
type SnapshotTriple = (Option<i64>, Option<i64>, Option<i64>);

/// Stored snapshot triple of one document.
fn stored_snapshot(harness: &Harness, document_id: &str) -> SnapshotTriple {
    read_conn(harness)
        .query_row(
            "SELECT credit_solde_before_centimes, credit_solde_after_centimes,
                    credit_delta_centimes
             FROM documents WHERE id = ?1",
            [document_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("snapshot columns")
}

fn stored_totals(harness: &Harness, document_id: &str) -> (i64, i64) {
    read_conn(harness)
        .query_row(
            "SELECT ttc_centimes, paid_amount_centimes FROM documents WHERE id = ?1",
            [document_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("totals")
}

/// One single-line checkout order under test.
struct SaleOrder {
    warehouse: WarehouseId,
    product: ProductId,
    unit: UnitId,
    quantity_micro: i64,
    customer: Option<CustomerId>,
    cash_centimes: i64,
    credit_centimes: i64,
}

/// Checkout one line; `credit_centimes` is the CREDIT tender (0 = all cash).
fn checkout(harness: &Harness, order: &SaleOrder) -> baraka_application::ports::CheckoutReceipt {
    let mut payments = Vec::new();
    if order.cash_centimes > 0 {
        payments.push(PaymentInput {
            method: PaymentMethod::Cash,
            amount_centimes: order.cash_centimes,
        });
    }
    if order.credit_centimes > 0 {
        payments.push(PaymentInput {
            method: PaymentMethod::Credit,
            amount_centimes: order.credit_centimes,
        });
    }
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &session_with(Role::Admin),
        CheckoutInput {
            command_id: Uuid::new_v4().to_string(),
            warehouse_id: order.warehouse,
            customer_id: order.customer,
            doc_type: DocumentType::Ticket,
            language: DocumentLanguage::Fr,
            lines: vec![CartLineInput {
                product_id: order.product,
                unit_id: order.unit,
                quantity_micro: order.quantity_micro,
                unit_price_override: None,
                line_discount_centimes: 0,
            }],
            global_discount_centimes: 0,
            payments,
            allow_negative_stock: false,
            notes: None,
            timbre_enabled: false,
        },
        TEST_TERMINAL,
    )
    .expect("checkout")
}

#[test]
fn credit_sale_freezes_the_printed_equation() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "SLIP-M", 10_000);
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, 100_000)
        .expect("stock");
    ensure_cash_session(&harness);
    let customer = seed_customer(&harness);

    // 2 pieces at 100.00 = 200.00 DZD; 50.00 cash + 150.00 credit.
    let receipt = checkout(
        &harness,
        &SaleOrder {
            warehouse,
            product,
            unit,
            quantity_micro: 20_000,
            customer: Some(customer),
            cash_centimes: 5_000,
            credit_centimes: 15_000,
        },
    );

    let (before, after, delta) = stored_snapshot(&harness, &receipt.document_id);
    assert_eq!(
        (before, after, delta),
        (Some(0), Some(15_000), Some(15_000)),
        "Ancien 0 + credit 150.00 = Nouveau 150.00"
    );

    // The equation printed from the stored snapshot must hold exactly.
    // Fact (BRK-108 probe): `documents.paid_amount_centimes` counts ALL
    // tenders, the CREDIT leg included — so the printed `Versement` is the
    // non-credit portion `Montant - delta`, never the raw paid column.
    let (ttc, paid) = stored_totals(&harness, &receipt.document_id);
    assert_eq!((ttc, paid), (20_000, 20_000));
    let snapshot =
        CreditSnapshot::freeze(before.expect("before"), delta.expect("delta")).expect("freeze");
    assert_eq!(snapshot.nouveau_solde_centimes(), 15_000);
    let versement = snapshot.versement_for(ttc).expect("versement");
    assert_eq!(versement, 5_000, "Versement = Montant - credit delta");
    assert_eq!(paid - versement, 15_000, "the rest is the CREDIT leg");
    assert!(
        snapshot.verifies(ttc, versement).expect("verify"),
        "Ancien + Montant - Versement == Nouveau"
    );

    // Ledger truth agrees with the frozen snapshot, and the cached balance
    // equals both (the snapshot never diverges from the books).
    let conn = read_conn(&harness);
    let running: i64 = conn
        .query_row(
            "SELECT running_balance_centimes FROM customer_ledger
             WHERE document_id = ?1",
            [&receipt.document_id],
            |row| row.get(0),
        )
        .expect("ledger row");
    assert_eq!(running, after.expect("after"));
    let cached: i64 = conn
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = ?1",
            [customer.to_string()],
            |row| row.get(0),
        )
        .expect("cached balance");
    assert_eq!(cached, after.expect("after"));

    // A SECOND sale stacks on the frozen history: Ancien 150.00 -> 250.00.
    let second = checkout(
        &harness,
        &SaleOrder {
            warehouse,
            product,
            unit,
            quantity_micro: 10_000,
            customer: Some(customer),
            cash_centimes: 0,
            credit_centimes: 10_000,
        },
    );
    assert_eq!(
        stored_snapshot(&harness, &second.document_id),
        (Some(15_000), Some(25_000), Some(10_000))
    );
}

#[test]
fn cash_only_sale_records_no_snapshot() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "VIS-8", 10_000);
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, 100_000)
        .expect("stock");
    ensure_cash_session(&harness);

    let receipt = checkout(
        &harness,
        &SaleOrder {
            warehouse,
            product,
            unit,
            quantity_micro: 10_000,
            customer: None,
            cash_centimes: 10_000,
            credit_centimes: 0,
        },
    );
    assert_eq!(
        stored_snapshot(&harness, &receipt.document_id),
        (None, None, None),
        "no balance movement => NULL, never a fabricated zero"
    );
}

#[test]
fn migration_0020_is_additive_on_a_populated_0019_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("legacy-0019.db");
    let mut conn = open_connection(&path).expect("open");

    // Stage 1: a faithful 0019 database (pre-BRK-108) with real rows.
    for migration in MIGRATIONS.iter().filter(|m| m.version <= 19) {
        conn.execute_batch(migration.sql)
            .unwrap_or_else(|_| panic!("apply migration {}", migration.version));
        conn.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                migration.version,
                migration.name,
                checksum_of(migration.sql),
                STAMP
            ],
        )
        .expect("record migration");
    }
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'admin', 'hash', 'ADMIN', 1, ?2, ?2)",
        rusqlite::params![ACTOR_UUID, STAMP],
    )
    .expect("user");
    conn.execute(
        "INSERT INTO customers
         (id, code, name, phone, address, is_active, created_at, updated_at,
          credit_policy, credit_limit_centimes, current_balance_centimes,
          revision, origin_device_id, customer_tier)
         VALUES ('cust-legacy', 'C-LEGACY', 'Ancien Client', NULL, NULL, 1, ?1, ?1,
                 'LIMITED', 100000, 450000, 1, 'LOCAL', 'wholesale')",
        [STAMP],
    )
    .expect("legacy customer");
    let warehouse_id: String = conn
        .query_row(
            "SELECT id FROM warehouses WHERE code = 'DEP-01'",
            [],
            |row| row.get(0),
        )
        .expect("seeded warehouse");
    conn.execute(
        "INSERT INTO documents
         (id, number, fiscal_year, type, party_type, party_id, warehouse_id,
          document_language, subtotal_ht_centimes, vat_total_centimes,
          global_discount_centimes, timbre_centimes, rounding_centimes,
          ttc_centimes, paid_amount_centimes, change_amount_centimes,
          payment_method, status, created_at, created_by, revision,
          origin_device_id)
         VALUES ('doc-legacy', 'BL-2026-000001', 2026, 'BL', 'CUSTOMER',
                 'cust-legacy', ?1, 'FR', 450000, 0, 0, 0, 0, 450000, 0, 0,
                 'CREDIT', 'FINALIZED', ?2, ?3, 1, 'LOCAL')",
        rusqlite::params![warehouse_id, STAMP, ACTOR_UUID],
    )
    .expect("legacy finalized document");

    // Stage 2: the official upgrade path.
    run_migrations(&mut conn).expect("upgrade to 0023");
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert_eq!(version, 23, "binary horizon must be 0023");

    // Stage 3: zero data loss + legacy rows read back NULL (no backfilled lie).
    let balance: i64 = conn
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE id = 'cust-legacy'",
            [],
            |row| row.get(0),
        )
        .expect("customer preserved");
    assert_eq!(balance, 450_000);
    let (number, ttc, snapshot): (String, i64, SnapshotTriple) = conn
        .query_row(
            "SELECT number, ttc_centimes, credit_solde_before_centimes,
                    credit_solde_after_centimes, credit_delta_centimes
             FROM documents WHERE id = 'doc-legacy'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    (row.get(2)?, row.get(3)?, row.get(4)?),
                ))
            },
        )
        .expect("document preserved");
    assert_eq!((number.as_str(), ttc), ("BL-2026-000001", 450_000));
    assert_eq!(
        snapshot,
        (None, None, None),
        "pre-0020 documents carry no snapshot (surfaces must render it absent)"
    );
    // Writer lists elsewhere stay explicit: an INSERT that omits the new
    // columns still succeeds (version-rollback safety).
    conn.execute(
        "INSERT INTO documents
         (id, number, fiscal_year, type, warehouse_id, document_language,
          subtotal_ht_centimes, vat_total_centimes, global_discount_centimes,
          timbre_centimes, rounding_centimes, ttc_centimes,
          paid_amount_centimes, change_amount_centimes, payment_method, status,
          created_at, created_by, revision, origin_device_id)
         VALUES ('doc-post', 'TICK-2026-000099', 2026, 'TICKET', ?1, 'FR',
                 1000, 0, 0, 0, 0, 1000, 1000, 0, 'CASH', 'FINALIZED', ?2, ?3, 1, 'LOCAL')",
        rusqlite::params![warehouse_id, STAMP, ACTOR_UUID],
    )
    .expect("legacy column list still inserts");
}
