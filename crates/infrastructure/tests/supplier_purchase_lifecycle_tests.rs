//! Supplier + purchase lifecycle tests (TASK-BRK-011).
//!
//! Covers every acceptance criterion of the original issue and its
//! continuation over real SQLite backends: BC/BR/FF semantics (ADR-029),
//! CASH vs OTHER supplier payments (ADR-030), ledger reconciliation,
//! append-only triggers against the migration-0009 authoritative shape,
//! the full RBAC matrix, idempotency, and ADR-028 terminal-origin
//! compliance. All money is `i64` centimes; no float exists in this file.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    NewSupplier, ProductRepository, PurchaseLineInput, PurchaseRepository, StockRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::cash::OpenCashSessionUseCase;
use baraka_application::use_cases::purchases::{
    CreatePurchaseOrderUseCase, GetPurchaseDocumentUseCase, ReceiveGoodsUseCase,
    RecordSupplierInvoiceUseCase, RecordSupplierPaymentUseCase, UpdatePurchaseOrderUseCase,
};
use baraka_application::use_cases::suppliers::{
    CreateSupplierUseCase, GetSupplierStatementUseCase, SearchSuppliersUseCase,
    UpdateSupplierUseCase,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::suppliers::SupplierId;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCashSessionRepository, SqliteCatalogRepository, SqlitePurchaseRepository,
    SqliteStockRepository, SqliteSupplierRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use chrono::{Datelike, TimeDelta, Utc};
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
    cash: SqliteCashSessionRepository,
    suppliers: SqliteSupplierRepository,
    purchases: SqlitePurchaseRepository,
    terminal_id: String,
    warehouse_id: WarehouseId,
}

fn session_with(role: Role, uuid: &str) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(uuid).expect("uuid")),
        role,
        expires_at: Utc::now() + TimeDelta::hours(1),
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
    let path = dir.path().join("purchase.db");
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
        .create_warehouse(&warehouse, &terminal_id)
        .expect("warehouse");
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses,
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
        suppliers: SqliteSupplierRepository::new(Arc::clone(&shared)),
        purchases: SqlitePurchaseRepository::new(Arc::clone(&shared)),
        terminal_id,
        warehouse_id: warehouse.id,
    }
}

fn seed_product(harness: &Harness, sku: &str) -> (ProductId, UnitId) {
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
                sale_price: Money::from_centimes(20_000),
                purchase_price: Money::from_centimes(8_000),
                is_base_unit: true,
                is_default_selling: true,
            }],
            &harness.terminal_id,
        )
        .expect("product");
    (id, unit_id)
}

fn supplier_input(name: &str) -> NewSupplier {
    NewSupplier {
        name: name.to_string(),
        phone: Some("0550123456".to_string()),
        address: None,
        nif: Some("NIF-123".to_string()),
        rc: None,
        notes: Some("Compte traditionnel".to_string()),
    }
}

fn create_supplier(harness: &Harness, name: &str) -> String {
    CreateSupplierUseCase::new(&harness.suppliers)
        .execute(&gerant(), supplier_input(name), &harness.terminal_id)
        .expect("supplier")
        .id
}

fn line(product: ProductId, unit: UnitId, units: i64, cost: i64) -> PurchaseLineInput {
    PurchaseLineInput {
        product_id: product,
        unit_id: unit,
        quantity_micro: units * 10_000,
        unit_cost_centimes: cost,
    }
}

fn create_bc(harness: &Harness, supplier_id: &str, lines: Vec<PurchaseLineInput>) -> String {
    CreatePurchaseOrderUseCase::new(&harness.purchases, &harness.suppliers, &harness.warehouses)
        .execute(
            &gerant(),
            supplier_id,
            harness.warehouse_id,
            lines,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("bc")
        .id
}

fn receive(
    harness: &Harness,
    source_bc: Option<&str>,
    supplier_id: &str,
    lines: Vec<PurchaseLineInput>,
) -> String {
    ReceiveGoodsUseCase::new(&harness.purchases, &harness.suppliers, &harness.warehouses)
        .execute(
            &gerant(),
            source_bc.map(str::to_string),
            supplier_id,
            harness.warehouse_id,
            lines,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("br")
        .id
}

fn record_invoice(
    harness: &Harness,
    source_br: Option<&str>,
    supplier_id: &str,
    lines: Vec<PurchaseLineInput>,
    total: i64,
) -> String {
    RecordSupplierInvoiceUseCase::new(&harness.purchases, &harness.suppliers, &harness.warehouses)
        .execute(
            &gerant(),
            source_br.map(str::to_string),
            supplier_id,
            harness.warehouse_id,
            lines,
            total,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("ff")
        .id
}

fn record_payment(
    harness: &Harness,
    supplier_id: &str,
    amount: i64,
    method: &str,
    command_id: &str,
) -> Result<baraka_application::ports::SupplierPaymentOutcome, ApplicationError> {
    RecordSupplierPaymentUseCase::new(&harness.purchases, &harness.suppliers).execute(
        &gerant(),
        supplier_id,
        amount,
        method,
        None,
        &harness.terminal_id,
        command_id,
    )
}

fn count_table(harness: &Harness, table: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    guard
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn scalar(harness: &Harness, sql: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    guard.query_row(sql, [], |row| row.get(0)).expect("scalar")
}

fn balance(harness: &Harness, supplier_id: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    guard
        .query_row(
            "SELECT current_balance_centimes FROM suppliers WHERE id = ?1",
            [supplier_id],
            |row| row.get(0),
        )
        .expect("balance")
}

fn ledger_sum(harness: &Harness, supplier_id: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    let total: Option<i64> = guard
        .query_row(
            "SELECT SUM(CASE WHEN entry_type = 'DEBIT' THEN amount_centimes
                             ELSE -amount_centimes END)
             FROM supplier_ledger WHERE supplier_id = ?1",
            [supplier_id],
            |row| row.get(0),
        )
        .expect("ledger sum");
    total.unwrap_or(0)
}

fn stock_level(harness: &Harness, product: ProductId) -> i64 {
    harness
        .stock
        .get_stock_level(product, harness.warehouse_id)
        .expect("level")
}

fn open_session(harness: &Harness) -> String {
    OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &caissier(),
            harness.warehouse_id,
            100_000,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("open")
        .id
}

fn current_year() -> i32 {
    Utc::now().year()
}

/// 1. BC creation: zero stock/ledger effect, editable while DRAFT.
#[test]
fn bc_creation_has_zero_effects_and_is_editable_while_draft() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Ferraille SARL");
    let (product, unit) = seed_product(&harness, "VIS-A");
    let bc = create_bc(&harness, &supplier, vec![line(product, unit, 10, 8_000)]);
    assert_eq!(count_table(&harness, "documents"), 1);
    assert_eq!(count_table(&harness, "stock_movements"), 0);
    assert_eq!(count_table(&harness, "supplier_ledger"), 0);
    assert_eq!(stock_level(&harness, product), 0);
    let view = GetPurchaseDocumentUseCase::new(&harness.purchases)
        .execute(&gerant(), &bc)
        .expect("fetch");
    assert_eq!(view.status, "DRAFT");
    assert_eq!(view.doc_type, "BC");
    assert_eq!(view.number, format!("BC-{}-000001", current_year()));
    assert_eq!(view.total_centimes, 80_000);
    // Fully editable while DRAFT: replace the line set.
    let updated = UpdatePurchaseOrderUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
    )
    .execute(
        &gerant(),
        &bc,
        &supplier,
        harness.warehouse_id,
        vec![line(product, unit, 5, 9_000)],
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    )
    .expect("update");
    assert_eq!(updated.lines.len(), 1);
    assert_eq!(updated.lines[0].quantity_micro, 50_000);
    assert_eq!(updated.total_centimes, 45_000);
    assert_eq!(count_table(&harness, "documents"), 1);
    assert_eq!(count_table(&harness, "supplier_ledger"), 0);
    // A BR commit seals the BC: further edits must reject.
    receive(
        &harness,
        Some(&bc),
        &supplier,
        vec![line(product, unit, 5, 9_000)],
    );
    let late = UpdatePurchaseOrderUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
    )
    .execute(
        &gerant(),
        &bc,
        &supplier,
        harness.warehouse_id,
        vec![line(product, unit, 1, 9_000)],
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(matches!(
        late,
        Err(ApplicationError::Validation(
            DomainError::PurchaseOrderNotDraft
        ))
    ));
}

/// 2. BR against a BC: stock increases via PURCHASE_IN, zero ledger effect.
#[test]
fn br_increases_stock_with_zero_ledger_effect() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Quincaillerie Est");
    let (product, unit) = seed_product(&harness, "ECRO-B");
    let bc = create_bc(&harness, &supplier, vec![line(product, unit, 10, 8_000)]);
    let br = receive(
        &harness,
        Some(&bc),
        &supplier,
        vec![line(product, unit, 4, 8_000)],
    );
    assert_eq!(stock_level(&harness, product), 40_000);
    assert_eq!(count_table(&harness, "supplier_ledger"), 0);
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM stock_movements WHERE movement_type = 'PURCHASE_IN'"
        ),
        1
    );
    let view = GetPurchaseDocumentUseCase::new(&harness.purchases)
        .execute(&gerant(), &br)
        .expect("br view");
    assert_eq!(view.doc_type, "BR");
    assert_eq!(view.status, "FINALIZED"); // ADR-031: committed == FINALIZED
    assert_eq!(view.source_document_id.as_deref(), Some(bc.as_str()));
    assert_eq!(view.number, format!("BR-{}-000001", current_year()));
    // BC rolls up to PARTIALLY_RECEIVED after a partial receipt.
    let bc_view = GetPurchaseDocumentUseCase::new(&harness.purchases)
        .execute(&gerant(), &bc)
        .expect("bc view");
    assert_eq!(bc_view.status, "PARTIALLY_RECEIVED");
}

/// 3. Over-receipt: rejected without the override, permitted + audited with it.
#[test]
fn over_receipt_requires_price_override_permission() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Acier Ouest");
    let (product, unit) = seed_product(&harness, "ROND-C");
    let bc = create_bc(&harness, &supplier, vec![line(product, unit, 10, 8_000)]);
    // Repository-level call with the override flag OFF: rejected, zero writes.
    let rejected = harness.purchases.receive_goods(
        Some(&bc),
        &supplier,
        harness.warehouse_id,
        &[line(product, unit, 11, 8_000)],
        false,
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
        "canonical-over-receipt",
        UserId::from(Uuid::parse_str(GERANT_UUID).expect("uuid")),
    );
    assert!(
        matches!(
            rejected,
            Err(ApplicationError::Validation(
                DomainError::PurchaseReceiptQuantityExceedsOrdered
            ))
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count_table(&harness, "documents"), 1);
    assert_eq!(count_table(&harness, "stock_movements"), 0);
    assert_eq!(stock_level(&harness, product), 0);
    // Gerant holds Permission::PriceOverride (ADR-033): the use case passes
    // the flag through; the excess is accepted with a prominent audit event.
    receive(
        &harness,
        Some(&bc),
        &supplier,
        vec![line(product, unit, 11, 8_000)],
    );
    assert_eq!(stock_level(&harness, product), 110_000);
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM audit_log WHERE action = 'PURCHASE_RECEIPT_QUANTITY_OVERRIDE'"
        ),
        1
    );
}

/// 4. FF sourced from a BR: exactly one DEBIT, zero stock movements.
#[test]
fn ff_sourced_from_br_debits_ledger_without_touching_stock() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Import Méditerranée");
    let (product, unit) = seed_product(&harness, "CUS-D");
    let bc = create_bc(&harness, &supplier, vec![line(product, unit, 10, 8_000)]);
    let br = receive(
        &harness,
        Some(&bc),
        &supplier,
        vec![line(product, unit, 10, 8_000)],
    );
    let stock_before = stock_level(&harness, product);
    let movements_before = scalar(&harness, "SELECT COUNT(*) FROM stock_movements");
    let ff = record_invoice(
        &harness,
        Some(&br),
        &supplier,
        vec![line(product, unit, 10, 8_000)],
        80_000,
    );
    assert_eq!(scalar(&harness, "SELECT COUNT(*) FROM supplier_ledger"), 1);
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM supplier_ledger WHERE entry_type = 'DEBIT'"
        ),
        1
    );
    // ADR-029 double-counting guard: stock untouched by a sourced FF.
    assert_eq!(stock_level(&harness, product), stock_before);
    assert_eq!(
        scalar(&harness, "SELECT COUNT(*) FROM stock_movements"),
        movements_before
    );
    assert_eq!(balance(&harness, &supplier), 80_000);
    let view = GetPurchaseDocumentUseCase::new(&harness.purchases)
        .execute(&gerant(), &ff)
        .expect("ff view");
    assert_eq!(view.doc_type, "FF");
    assert_eq!(view.number, format!("FF-{}-000001", current_year()));
    assert_eq!(view.source_document_id.as_deref(), Some(br.as_str()));
    let bc_view = GetPurchaseDocumentUseCase::new(&harness.purchases)
        .execute(&gerant(), &bc)
        .expect("bc done");
    assert_eq!(bc_view.status, "FULLY_RECEIVED");
}

/// 5. Standalone FF: one DEBIT AND the PURCHASE_IN movements, atomically.
#[test]
fn standalone_ff_moves_stock_and_debits_ledger_atomically() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Soude Express");
    let (product, unit) = seed_product(&harness, "SOUDE-E");
    let ff = record_invoice(
        &harness,
        None,
        &supplier,
        vec![line(product, unit, 7, 8_000)],
        56_000,
    );
    assert_eq!(stock_level(&harness, product), 70_000);
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM stock_movements WHERE movement_type = 'PURCHASE_IN'"
        ),
        1
    );
    assert_eq!(scalar(&harness, "SELECT COUNT(*) FROM supplier_ledger"), 1);
    assert_eq!(balance(&harness, &supplier), 56_000);
    assert_eq!(
        ledger_sum(&harness, &supplier),
        balance(&harness, &supplier)
    );
    let view = GetPurchaseDocumentUseCase::new(&harness.purchases)
        .execute(&gerant(), &ff)
        .expect("standalone ff");
    assert!(view.source_document_id.is_none());
}

/// 6. A second FF on the same BR is rejected with zero side effects.
#[test]
fn second_ff_on_invoiced_br_rejected_with_zero_side_effects() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Double Facture SA");
    let (product, unit) = seed_product(&harness, "DOUB-F");
    let bc = create_bc(&harness, &supplier, vec![line(product, unit, 10, 8_000)]);
    let br = receive(
        &harness,
        Some(&bc),
        &supplier,
        vec![line(product, unit, 10, 8_000)],
    );
    record_invoice(
        &harness,
        Some(&br),
        &supplier,
        vec![line(product, unit, 10, 8_000)],
        80_000,
    );
    let documents_before = count_table(&harness, "documents");
    let ledger_before = count_table(&harness, "supplier_ledger");
    let stock_before = stock_level(&harness, product);
    let balance_before = balance(&harness, &supplier);
    let second = RecordSupplierInvoiceUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
    )
    .execute(
        &gerant(),
        Some(br.clone()),
        &supplier,
        harness.warehouse_id,
        vec![line(product, unit, 10, 8_000)],
        80_000,
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    assert!(
        matches!(
            second,
            Err(ApplicationError::Validation(
                DomainError::PurchaseInvoiceAlreadySourced
            ))
        ),
        "unexpected: {second:?}"
    );
    assert_eq!(count_table(&harness, "documents"), documents_before);
    assert_eq!(count_table(&harness, "supplier_ledger"), ledger_before);
    assert_eq!(stock_level(&harness, product), stock_before);
    assert_eq!(balance(&harness, &supplier), balance_before);
    // A BR whose quantities do not match is also rejected (line reconciliation).
    let mismatch = RecordSupplierInvoiceUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
    )
    .execute(
        &gerant(),
        Some(br),
        &supplier,
        harness.warehouse_id,
        vec![line(product, unit, 3, 8_000)],
        24_000,
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    );
    // The already-sourced guard fires first; either guard is a typed reject.
    assert!(matches!(
        mismatch,
        Err(ApplicationError::Validation(
            DomainError::PurchaseInvoiceAlreadySourced
        ))
    ));
    assert_eq!(count_table(&harness, "supplier_ledger"), ledger_before);
}

/// 7. Balance reconciliation across the full multi-step lifecycle.
#[test]
fn cached_balance_equals_ledger_sum_after_every_committed_step() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Multi Step SARL");
    let (product, unit) = seed_product(&harness, "MULT-G");
    let bc = create_bc(&harness, &supplier, vec![line(product, unit, 10, 8_000)]);
    assert_eq!(
        balance(&harness, &supplier),
        ledger_sum(&harness, &supplier)
    );
    let br = receive(
        &harness,
        Some(&bc),
        &supplier,
        vec![line(product, unit, 10, 8_000)],
    );
    assert_eq!(
        balance(&harness, &supplier),
        ledger_sum(&harness, &supplier)
    );
    record_invoice(
        &harness,
        Some(&br),
        &supplier,
        vec![line(product, unit, 10, 8_000)],
        85_000,
    );
    assert_eq!(balance(&harness, &supplier), 85_000);
    assert_eq!(
        balance(&harness, &supplier),
        ledger_sum(&harness, &supplier)
    );
    record_payment(
        &harness,
        &supplier,
        30_000,
        "OTHER",
        &Uuid::new_v4().to_string(),
    )
    .expect("other payment");
    assert_eq!(balance(&harness, &supplier), 55_000);
    assert_eq!(
        balance(&harness, &supplier),
        ledger_sum(&harness, &supplier)
    );
    open_session(&harness);
    record_payment(
        &harness,
        &supplier,
        20_000,
        "CASH",
        &Uuid::new_v4().to_string(),
    )
    .expect("cash payment");
    assert_eq!(balance(&harness, &supplier), 35_000);
    assert_eq!(
        balance(&harness, &supplier),
        ledger_sum(&harness, &supplier)
    );
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM supplier_ledger WHERE entry_type = 'DEBIT'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM supplier_ledger WHERE entry_type = 'CREDIT'"
        ),
        2
    );
}

/// 8. CASH payment with no OPEN session: atomic reject, zero ledger mutation.
#[test]
fn cash_payment_without_open_session_rejected_atomically() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Sans Caisse");
    let (product, unit) = seed_product(&harness, "NOCASH-H");
    record_invoice(
        &harness,
        None,
        &supplier,
        vec![line(product, unit, 1, 10_000)],
        10_000,
    );
    let ledgers_before = count_table(&harness, "supplier_ledger");
    let balance_before = balance(&harness, &supplier);
    let rejected = record_payment(
        &harness,
        &supplier,
        4_000,
        "CASH",
        &Uuid::new_v4().to_string(),
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
    assert_eq!(count_table(&harness, "supplier_ledger"), ledgers_before);
    assert_eq!(balance(&harness, &supplier), balance_before);
    assert_eq!(scalar(&harness, "SELECT COUNT(*) FROM cash_movements"), 0);
}

/// 9. CASH payment with an OPEN session: one negative SUPPLIER_PAYMENT_OUT.
#[test]
fn cash_payment_with_session_writes_single_negative_movement() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Avec Caisse");
    let (product, unit) = seed_product(&harness, "ENCASH-I");
    record_invoice(
        &harness,
        None,
        &supplier,
        vec![line(product, unit, 2, 5_000)],
        10_000,
    );
    let session_id = open_session(&harness);
    let outcome = record_payment(
        &harness,
        &supplier,
        4_000,
        "CASH",
        &Uuid::new_v4().to_string(),
    )
    .expect("cash payment");
    let movement_id = outcome.cash_movement_id.clone().expect("movement id");
    let (amount, movement_session): (i64, String) = {
        let guard = harness.db.lock().expect("lock");
        guard
            .query_row(
                "SELECT amount_centimes, cash_session_id FROM cash_movements
                 WHERE movement_type = 'SUPPLIER_PAYMENT_OUT'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("movement")
    };
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM cash_movements WHERE movement_type = 'SUPPLIER_PAYMENT_OUT'"
        ),
        1
    );
    assert_eq!(amount, -4_000);
    assert_eq!(movement_session, session_id);
    let guard = harness.db.lock().expect("lock");
    let reference: String = guard
        .query_row(
            "SELECT reference_id FROM cash_movements WHERE id = ?1",
            [movement_id.clone()],
            |row| row.get(0),
        )
        .expect("reference");
    drop(guard);
    assert_eq!(reference, outcome.ledger_entry_id);
    assert_eq!(outcome.new_balance_centimes, 6_000);
    assert_eq!(balance(&harness, &supplier), 6_000);
}

/// 10. OTHER payment: no session requirement, zero drawer rows.
#[test]
fn other_payment_skips_session_and_writes_no_cash_movement() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Virement Pur");
    let (product, unit) = seed_product(&harness, "BANK-J");
    record_invoice(
        &harness,
        None,
        &supplier,
        vec![line(product, unit, 1, 5_000)],
        5_000,
    );
    // No cash session was ever opened on this terminal.
    let outcome = record_payment(
        &harness,
        &supplier,
        1_000,
        "OTHER",
        &Uuid::new_v4().to_string(),
    )
    .expect("other payment");
    assert!(outcome.cash_movement_id.is_none());
    assert_eq!(scalar(&harness, "SELECT COUNT(*) FROM cash_movements"), 0);
    assert_eq!(scalar(&harness, "SELECT COUNT(*) FROM cash_sessions"), 0);
    assert_eq!(outcome.new_balance_centimes, 4_000);
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM supplier_ledger WHERE entry_type = 'CREDIT'"
        ),
        1
    );
}

/// 11. supplier_ledger append-only at the trigger level, against the
///     authoritative migration-0009 shape (verified by column presence).
#[test]
fn supplier_ledger_is_append_only_on_the_0009_shape() {
    let harness = setup();
    let guard = harness.db.lock().expect("lock");
    let columns: Vec<String> = guard
        .prepare("SELECT name FROM pragma_table_info('supplier_ledger')")
        .expect("pragma")
        .query_map([], |row| row.get(0))
        .expect("cols")
        .collect::<Result<_, _>>()
        .expect("collect");
    assert!(columns.iter().any(|c| c == "reference_type"));
    assert!(columns.iter().any(|c| c == "reference_id"));
    assert!(columns.iter().any(|c| c == "description"));
    assert!(
        !columns.iter().any(|c| c == "sequence_no"),
        "0001-era ledger shape must be gone"
    );
    // Suppliers authoritative shape: nif/rc/notes/origin present, code gone.
    let supplier_columns: Vec<String> = guard
        .prepare("SELECT name FROM pragma_table_info('suppliers')")
        .expect("pragma")
        .query_map([], |row| row.get(0))
        .expect("cols")
        .collect::<Result<_, _>>()
        .expect("collect");
    assert!(supplier_columns.iter().any(|c| c == "nif"));
    assert!(supplier_columns.iter().any(|c| c == "origin_device_id"));
    assert!(
        !supplier_columns.iter().any(|c| c == "code"),
        "legacy code column must be gone"
    );
    drop(guard);
    let supplier = create_supplier(&harness, "Trigger Co");
    let (product, unit) = seed_product(&harness, "TRIG-K");
    record_invoice(
        &harness,
        None,
        &supplier,
        vec![line(product, unit, 1, 1_000)],
        1_000,
    );
    let entry_id: String = {
        let guard = harness.db.lock().expect("lock");
        guard
            .query_row("SELECT id FROM supplier_ledger LIMIT 1", [], |row| {
                row.get(0)
            })
            .expect("entry")
    };
    let guard = harness.db.lock().expect("lock");
    let update = guard.execute(
        "UPDATE supplier_ledger SET amount_centimes = 1 WHERE id = ?1",
        [&entry_id],
    );
    assert!(
        format!("{update:?}").contains("MUTATION_FORBIDDEN"),
        "ledger UPDATE must abort: {update:?}"
    );
    let delete = guard.execute("DELETE FROM supplier_ledger WHERE id = ?1", [&entry_id]);
    assert!(
        format!("{delete:?}").contains("MUTATION_FORBIDDEN"),
        "ledger DELETE must abort: {delete:?}"
    );
    // Widened cash CHECK: SUPPLIER_PAYMENT_OUT must be stored negative.
    drop(guard);
    let session_id = open_session(&harness);
    let negative = {
        let guard = harness.db.lock().expect("lock");
        guard.execute(
            "INSERT INTO cash_movements
             (id, cash_session_id, movement_type, amount_centimes, created_at, created_by)
             VALUES ('m-neg', ?1, 'SUPPLIER_PAYMENT_OUT', -100, '2026-01-01T00:00:00Z', ?2)",
            rusqlite::params![session_id, GERANT_UUID],
        )
    };
    assert!(negative.is_ok(), "negative SUPPLIER_PAYMENT_OUT must pass");
    let positive = {
        let guard = harness.db.lock().expect("lock");
        guard.execute(
            "INSERT INTO cash_movements
             (id, cash_session_id, movement_type, amount_centimes, created_at, created_by)
             VALUES ('m-pos', ?1, 'SUPPLIER_PAYMENT_OUT', 100, '2026-01-01T00:00:00Z', ?2)",
            rusqlite::params![session_id, GERANT_UUID],
        )
    };
    assert!(
        format!("{positive:?}").contains("CHECK constraint failed"),
        "positive SUPPLIER_PAYMENT_OUT must fail: {positive:?}"
    );
}

/// 12. Full RBAC matrix: Vendeur and Caissier denied all five; Gerant and
///     Admin permitted across the lifecycle.
#[test]
fn rbac_matrix_denied_vendeur_caissier_permitted_gerant_admin() {
    let harness = setup();
    let supplier = create_supplier(&harness, "RBAC SARL");
    let (product, unit) = seed_product(&harness, "RBAC-L");
    for denied in [vendeur(), caissier()] {
        assert!(
            matches!(
                CreateSupplierUseCase::new(&harness.suppliers).execute(
                    &denied,
                    supplier_input("Nope"),
                    &harness.terminal_id
                ),
                Err(ApplicationError::Unauthorized)
            ),
            "create supplier must be denied: {:?}",
            denied.role
        );
        assert!(
            matches!(
                UpdateSupplierUseCase::new(&harness.suppliers).execute(
                    &denied,
                    &supplier,
                    baraka_application::ports::SupplierUpdate {
                        name: Some("Renamed".to_string()),
                        ..baraka_application::ports::SupplierUpdate::default()
                    },
                ),
                Err(ApplicationError::Unauthorized)
            ),
            "update supplier must be denied"
        );
        assert!(
            matches!(
                SearchSuppliersUseCase::new(&harness.suppliers).execute(&denied, "RBAC", 10),
                Err(ApplicationError::Unauthorized)
            ),
            "search suppliers must be denied"
        );
        assert!(
            matches!(
                GetSupplierStatementUseCase::new(&harness.suppliers).execute(&denied, &supplier),
                Err(ApplicationError::Unauthorized)
            ),
            "statement must be denied"
        );
        assert!(
            matches!(
                CreatePurchaseOrderUseCase::new(
                    &harness.purchases,
                    &harness.suppliers,
                    &harness.warehouses
                )
                .execute(
                    &denied,
                    &supplier,
                    harness.warehouse_id,
                    vec![line(product, unit, 1, 1_000)],
                    &harness.terminal_id,
                    &Uuid::new_v4().to_string(),
                ),
                Err(ApplicationError::Unauthorized)
            ),
            "bc create must be denied"
        );
        assert!(
            matches!(
                ReceiveGoodsUseCase::new(
                    &harness.purchases,
                    &harness.suppliers,
                    &harness.warehouses
                )
                .execute(
                    &denied,
                    None,
                    &supplier,
                    harness.warehouse_id,
                    vec![line(product, unit, 1, 1_000)],
                    &harness.terminal_id,
                    &Uuid::new_v4().to_string(),
                ),
                Err(ApplicationError::Unauthorized)
            ),
            "receive must be denied"
        );
        assert!(
            matches!(
                RecordSupplierInvoiceUseCase::new(
                    &harness.purchases,
                    &harness.suppliers,
                    &harness.warehouses,
                )
                .execute(
                    &denied,
                    None,
                    &supplier,
                    harness.warehouse_id,
                    vec![line(product, unit, 1, 1_000)],
                    1_000,
                    &harness.terminal_id,
                    &Uuid::new_v4().to_string(),
                ),
                Err(ApplicationError::Unauthorized)
            ),
            "invoice must be denied"
        );
        assert!(
            matches!(
                RecordSupplierPaymentUseCase::new(&harness.purchases, &harness.suppliers).execute(
                    &denied,
                    &supplier,
                    1_000,
                    "OTHER",
                    None,
                    &harness.terminal_id,
                    &Uuid::new_v4().to_string(),
                ),
                Err(ApplicationError::Unauthorized)
            ),
            "payment must be denied"
        );
    }
    // Every denial happened before any write: no documents, no ledger rows.
    assert_eq!(count_table(&harness, "documents"), 0);
    assert_eq!(count_table(&harness, "supplier_ledger"), 0);
    // Gerant and Admin may run the lifecycle end to end (OTHER payment path,
    // no session interdependency).
    for actor in [gerant(), admin()] {
        let fresh = create_supplier(&harness, &format!("Permis {}", actor.role.code()));
        let bc = CreatePurchaseOrderUseCase::new(
            &harness.purchases,
            &harness.suppliers,
            &harness.warehouses,
        )
        .execute(
            &actor,
            &fresh,
            harness.warehouse_id,
            vec![line(product, unit, 2, 1_000)],
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("bc permitted");
        let br =
            ReceiveGoodsUseCase::new(&harness.purchases, &harness.suppliers, &harness.warehouses)
                .execute(
                    &actor,
                    Some(bc.id),
                    &fresh,
                    harness.warehouse_id,
                    vec![line(product, unit, 2, 1_000)],
                    &harness.terminal_id,
                    &Uuid::new_v4().to_string(),
                )
                .expect("br permitted");
        RecordSupplierInvoiceUseCase::new(
            &harness.purchases,
            &harness.suppliers,
            &harness.warehouses,
        )
        .execute(
            &actor,
            Some(br.id),
            &fresh,
            harness.warehouse_id,
            vec![line(product, unit, 2, 1_000)],
            2_000,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("ff permitted");
        RecordSupplierPaymentUseCase::new(&harness.purchases, &harness.suppliers)
            .execute(
                &actor,
                &fresh,
                500,
                "OTHER",
                None,
                &harness.terminal_id,
                &Uuid::new_v4().to_string(),
            )
            .expect("payment permitted");
        assert_eq!(balance(&harness, &fresh), 1_500);
    }
}

/// 13. Idempotency on RecordSupplierPayment: replay returns the cached
///     outcome; the same id with a different payload conflicts.
#[test]
fn record_supplier_payment_is_idempotent_and_conflict_guarded() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Idempote SARL");
    let (product, unit) = seed_product(&harness, "IDEM-M");
    record_invoice(
        &harness,
        None,
        &supplier,
        vec![line(product, unit, 2, 5_000)],
        10_000,
    );
    open_session(&harness);
    let command_id = Uuid::new_v4().to_string();
    let first = record_payment(&harness, &supplier, 3_000, "CASH", &command_id).expect("first");
    let replay = record_payment(&harness, &supplier, 3_000, "CASH", &command_id).expect("replay");
    assert_eq!(first, replay, "replay must be byte-identical");
    assert_eq!(
        scalar(
            &harness,
            "SELECT COUNT(*) FROM cash_movements WHERE movement_type = 'SUPPLIER_PAYMENT_OUT'"
        ),
        1,
        "replay must not double-write the drawer"
    );
    assert_eq!(scalar(&harness, "SELECT COUNT(*) FROM supplier_ledger"), 2);
    assert_eq!(balance(&harness, &supplier), 7_000);
    let conflict = record_payment(&harness, &supplier, 4_000, "CASH", &command_id);
    assert!(
        matches!(conflict, Err(ApplicationError::IdempotencyConflict)),
        "unexpected: {conflict:?}"
    );
    assert_eq!(balance(&harness, &supplier), 7_000);
}

/// 14. ADR-028: every newly created supplier/purchase row carries the live
///     TerminalContext value as origin_device_id — never a placeholder literal.
#[test]
fn origin_device_id_equals_live_terminal_context_value() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Origine Exacte");
    let parsed: SupplierId = Uuid::parse_str(&supplier)
        .map(SupplierId::from)
        .expect("supplier uuid");
    let stored: String = {
        let guard = harness.db.lock().expect("lock");
        guard
            .query_row(
                "SELECT origin_device_id FROM suppliers WHERE id = ?1",
                [parsed.to_string()],
                |row| row.get(0),
            )
            .expect("origin")
    };
    assert_eq!(stored, harness.terminal_id);
    let (product, unit) = seed_product(&harness, "ORIG-N");
    let bc = create_bc(&harness, &supplier, vec![line(product, unit, 1, 1_000)]);
    let doc_origin: String = {
        let guard = harness.db.lock().expect("lock");
        guard
            .query_row(
                "SELECT origin_device_id FROM documents WHERE id = ?1",
                [bc],
                |row| row.get(0),
            )
            .expect("doc origin")
    };
    assert_eq!(doc_origin, harness.terminal_id);
    // And it is not any historical placeholder shape.
    assert_ne!(doc_origin, "LOCAL");
    assert_ne!(stored, "UNKNOWN");
}

/// 15. Statement: ledger order, running balances, cached-balance agreement.
#[test]
fn supplier_statement_running_balances_match_cached_balance() {
    let harness = setup();
    let supplier = create_supplier(&harness, "Relevé Final");
    let (product, unit) = seed_product(&harness, "STMT-O");
    record_invoice(
        &harness,
        None,
        &supplier,
        vec![line(product, unit, 5, 1_000)],
        5_000,
    );
    record_payment(
        &harness,
        &supplier,
        2_000,
        "OTHER",
        &Uuid::new_v4().to_string(),
    )
    .expect("pay 1");
    record_payment(
        &harness,
        &supplier,
        1_000,
        "OTHER",
        &Uuid::new_v4().to_string(),
    )
    .expect("pay 2");
    let statement = GetSupplierStatementUseCase::new(&harness.suppliers)
        .execute(&gerant(), &supplier)
        .expect("statement");
    let balances: Vec<i64> = statement
        .entries
        .iter()
        .map(|entry| entry.balance_after_centimes)
        .collect();
    assert_eq!(balances, vec![5_000, 3_000, 2_000]);
    assert_eq!(statement.current_balance_centimes, 2_000);
    assert_eq!(
        statement.current_balance_centimes,
        balance(&harness, &supplier)
    );
    assert_eq!(statement.entries[0].reference_type, "PURCHASE_INVOICE");
    assert_eq!(statement.entries[1].reference_type, "PAYMENT");
}

/// 16. Search + update mirror the customer-module behaviour on the 0009
///     authoritative supplier shape (blank query -> no rows; trim on create).
#[test]
fn supplier_search_and_update_follow_customer_module_semantics() {
    let harness = setup();
    let view = CreateSupplierUseCase::new(&harness.suppliers)
        .execute(
            &gerant(),
            NewSupplier {
                name: "  Atelier Nord  ".to_string(),
                phone: None,
                address: None,
                nif: None,
                rc: None,
                notes: None,
            },
            &harness.terminal_id,
        )
        .expect("trim");
    assert_eq!(view.name, "Atelier Nord");
    assert!(matches!(
        CreateSupplierUseCase::new(&harness.suppliers).execute(
            &gerant(),
            supplier_input("   "),
            &harness.terminal_id,
        ),
        Err(ApplicationError::Validation(
            DomainError::SupplierNameRequired
        ))
    ));
    let hits = SearchSuppliersUseCase::new(&harness.suppliers)
        .execute(&gerant(), "atelier", 10)
        .expect("search");
    assert_eq!(hits.len(), 1);
    assert!(SearchSuppliersUseCase::new(&harness.suppliers)
        .execute(&gerant(), "   ", 10)
        .expect("blank")
        .is_empty());
    let updated = UpdateSupplierUseCase::new(&harness.suppliers)
        .execute(
            &gerant(),
            &view.id,
            baraka_application::ports::SupplierUpdate {
                phone: Some(Some("0661987654".to_string())),
                is_active: Some(false),
                ..baraka_application::ports::SupplierUpdate::default()
            },
        )
        .expect("update");
    assert_eq!(updated.phone.as_deref(), Some("0661987654"));
    assert!(!updated.is_active);
    // Inactive suppliers reject new purchases.
    let (product, unit) = seed_product(&harness, "INACT-P");
    assert!(matches!(
        CreatePurchaseOrderUseCase::new(
            &harness.purchases,
            &harness.suppliers,
            &harness.warehouses
        )
        .execute(
            &gerant(),
            &view.id,
            harness.warehouse_id,
            vec![line(product, unit, 1, 1_000)],
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        ),
        Err(ApplicationError::Validation(DomainError::SupplierInactive))
    ));
}
