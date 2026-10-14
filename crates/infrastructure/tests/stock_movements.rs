//! Stock movement engine tests over real SQLite: atomicity, rollback,
//! authorization, transfers, kit rejection, and audit chaining.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    NewStockMovement, ProductRepository, SessionContext, SessionToken, StockRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::inventory::{
    AdjustStockInput, AdjustStockUseCase, StockDirection, TransferStockInput, TransferStockUseCase,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{StockMovementType, Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

struct Harness {
    _dir: tempfile::TempDir,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    catalog: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("stock.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let shared = Arc::new(Mutex::new(conn));
    let harness = Harness {
        _dir: dir,
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
    };
    seed_user(&harness);
    harness
}

fn session_with(role: Role) -> SessionContext {
    // All actor ids reference a real seeded users row (created_by FK).
    let user_id =
        UserId::from(uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid"));
    SessionContext {
        token: SessionToken::new_random(),
        user_id,
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed_user(harness: &Harness) {
    let conn = rusqlite::Connection::open(harness._dir.path().join("stock.db")).expect("reopen");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('22222222-2222-2222-2222-222222222222', 'gerant1', 'hash', 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
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

fn seed_product(harness: &Harness, sku: &str, is_kit: bool) -> (ProductId, UnitId) {
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
        sale_price: Money::from_centimes(1_000),
        purchase_price: Money::from_centimes(800),
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

struct AdjustParams {
    role: Role,
    qty: i64,
    direction: StockDirection,
    authorize: bool,
}

fn adjust(
    harness: &Harness,
    product: ProductId,
    warehouse: WarehouseId,
    unit: UnitId,
    params: AdjustParams,
) -> Result<(String, i64), ApplicationError> {
    let session = session_with(params.role);
    AdjustStockUseCase::new(&harness.stock)
        .execute(
            &session,
            AdjustStockInput {
                product_id: product,
                warehouse_id: warehouse,
                unit_id: unit,
                quantity_micro: params.qty,
                direction: params.direction,
                reason: "Inventaire annuel".to_string(),
                authorize_negative_stock: params.authorize,
            },
        )
        .map(|outcome| (outcome.movement_id, outcome.new_quantity_base_micro))
}

fn increase(
    harness: &Harness,
    role: Role,
    product: ProductId,
    warehouse: WarehouseId,
    unit: UnitId,
    qty: i64,
    authorize: bool,
) -> Result<(String, i64), ApplicationError> {
    adjust(
        harness,
        product,
        warehouse,
        unit,
        AdjustParams {
            role,
            qty,
            direction: StockDirection::Increase,
            authorize,
        },
    )
}

fn decrease(
    harness: &Harness,
    role: Role,
    product: ProductId,
    warehouse: WarehouseId,
    unit: UnitId,
    qty: i64,
    authorize: bool,
) -> Result<(String, i64), ApplicationError> {
    adjust(
        harness,
        product,
        warehouse,
        unit,
        AdjustParams {
            role,
            qty,
            direction: StockDirection::Decrease,
            authorize,
        },
    )
}

fn movement_count(harness: &Harness) -> i64 {
    // Test introspection through the repository's own connection is
    // unavailable (private handle), so count via a fresh read handle.
    let conn = rusqlite::Connection::open(harness._dir.path().join("stock.db")).expect("reopen");
    conn.query_row("SELECT COUNT(*) FROM stock_movements", [], |row| row.get(0))
        .expect("count")
}

fn level_of(harness: &Harness, product: ProductId, warehouse: WarehouseId) -> i64 {
    harness
        .stock
        .get_stock_level(product, warehouse)
        .expect("level")
}

fn audit_count(harness: &Harness) -> i64 {
    let conn = rusqlite::Connection::open(harness._dir.path().join("stock.db")).expect("reopen");
    conn.query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
        .expect("count")
}

#[test]
fn increase_from_zero_writes_movement_and_level() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", false);
    let (movement_id, new_qty) = increase(
        &harness,
        Role::Gerant,
        product,
        warehouse,
        unit,
        50_000,
        false,
    )
    .expect("adjust");
    assert!(!movement_id.is_empty());
    assert_eq!(new_qty, 50_000);
    assert_eq!(level_of(&harness, product, warehouse), 50_000);
    assert_eq!(movement_count(&harness), 1);
    assert_eq!(audit_count(&harness), 0);
}

#[test]
fn decrease_below_zero_is_rejected_with_full_rollback() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", false);
    increase(
        &harness,
        Role::Gerant,
        product,
        warehouse,
        unit,
        10_000,
        false,
    )
    .expect("seed");
    let before = movement_count(&harness);
    let rejected = decrease(
        &harness,
        Role::Gerant,
        product,
        warehouse,
        unit,
        50_000,
        false,
    )
    .expect_err("must reject");
    assert!(
        matches!(
            rejected,
            ApplicationError::InsufficientStock {
                available_base_micro: 10_000,
                requested_base_micro: 50_000,
            }
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(movement_count(&harness), before, "no partial rows");
    assert_eq!(level_of(&harness, product, warehouse), 10_000);
    assert_eq!(audit_count(&harness), 0);
}

#[test]
fn authorized_negative_decrease_writes_flag_and_audit() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", false);
    let (_, new_qty) = decrease(
        &harness,
        Role::Gerant,
        product,
        warehouse,
        unit,
        25_000,
        true,
    )
    .expect("override");
    assert_eq!(new_qty, -25_000);
    assert_eq!(level_of(&harness, product, warehouse), -25_000);
    let conn = rusqlite::Connection::open(harness._dir.path().join("stock.db")).expect("reopen");
    let flag: i64 = conn
        .query_row(
            "SELECT authorized_negative_stock FROM stock_movements",
            [],
            |row| row.get(0),
        )
        .expect("flag");
    assert_eq!(flag, 1);
    assert_eq!(audit_count(&harness), 1);
    let (action, prev, hash): (String, String, String) = conn
        .query_row(
            "SELECT action, prev_hash, record_hash FROM audit_log",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("audit row");
    assert_eq!(action, "NEGATIVE_STOCK_OVERRIDE");
    assert_eq!(prev, "GENESIS");
    assert_eq!(hash.len(), 64);
}

#[test]
fn empty_reason_rejected_before_any_write() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", false);
    let session = session_with(Role::Gerant);
    for reason in ["", "   ", "ab"] {
        let rejected = AdjustStockUseCase::new(&harness.stock).execute(
            &session,
            AdjustStockInput {
                product_id: product,
                warehouse_id: warehouse,
                unit_id: unit,
                quantity_micro: 1_000,
                direction: StockDirection::Increase,
                reason: reason.to_string(),
                authorize_negative_stock: false,
            },
        );
        assert!(
            matches!(
                rejected,
                Err(ApplicationError::Validation(
                    DomainError::InvalidCanonicalString(_)
                ))
            ),
            "reason={reason:?} must fail validation"
        );
    }
    assert_eq!(movement_count(&harness), 0);
    assert_eq!(level_of(&harness, product, warehouse), 0);
}

#[test]
fn transfer_moves_atomically_with_shared_batch() {
    let harness = setup();
    let src = seed_warehouse(&harness, "SRC");
    let dst = seed_warehouse(&harness, "DST");
    let (product, unit) = seed_product(&harness, "P1", false);
    increase(&harness, Role::Gerant, product, src, unit, 100_000, false).expect("seed");
    let session = session_with(Role::Gerant);
    let result = TransferStockUseCase::new(&harness.stock, &harness.warehouses)
        .execute(
            &session,
            TransferStockInput {
                source_warehouse_id: src,
                destination_warehouse_id: dst,
                product_id: product,
                unit_id: unit,
                quantity_micro: 30_000,
                reason: Some("Réassort".to_string()),
            },
        )
        .expect("transfer");
    assert!(!result.transfer_batch_id.is_empty());
    assert_eq!(result.source_new_quantity_base_micro, 70_000);
    assert_eq!(result.destination_new_quantity_base_micro, 30_000);
    assert_eq!(level_of(&harness, product, src), 70_000);
    assert_eq!(level_of(&harness, product, dst), 30_000);
    let conn = rusqlite::Connection::open(harness._dir.path().join("stock.db")).expect("reopen");
    let rows: Vec<(String, String)> = conn
        .prepare("SELECT movement_type, transfer_batch_id FROM stock_movements WHERE transfer_batch_id IS NOT NULL ORDER BY movement_type")
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "TRANSFER_IN");
    assert_eq!(rows[1].0, "TRANSFER_OUT");
    assert_eq!(rows[0].1, result.transfer_batch_id);
    assert_eq!(rows[1].1, result.transfer_batch_id);
}

#[test]
fn transfer_same_warehouse_rejected_before_transaction() {
    let harness = setup();
    let src = seed_warehouse(&harness, "SRC");
    let (product, unit) = seed_product(&harness, "P1", false);
    let session = session_with(Role::Gerant);
    let rejected = TransferStockUseCase::new(&harness.stock, &harness.warehouses).execute(
        &session,
        TransferStockInput {
            source_warehouse_id: src,
            destination_warehouse_id: src,
            product_id: product,
            unit_id: unit,
            quantity_micro: 1_000,
            reason: None,
        },
    );
    assert!(
        matches!(
            rejected,
            Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString(_)
            ))
        ),
        "same-warehouse must fail: {rejected:?}"
    );
    assert_eq!(movement_count(&harness), 0);
}

#[test]
fn transfer_insufficient_source_rolls_back_fully() {
    let harness = setup();
    let src = seed_warehouse(&harness, "SRC");
    let dst = seed_warehouse(&harness, "DST");
    let (product, unit) = seed_product(&harness, "P1", false);
    increase(&harness, Role::Gerant, product, src, unit, 10_000, false).expect("seed");
    let before = movement_count(&harness);
    let session = session_with(Role::Gerant);
    let rejected = TransferStockUseCase::new(&harness.stock, &harness.warehouses).execute(
        &session,
        TransferStockInput {
            source_warehouse_id: src,
            destination_warehouse_id: dst,
            product_id: product,
            unit_id: unit,
            quantity_micro: 50_000,
            reason: None,
        },
    );
    assert!(
        matches!(rejected, Err(ApplicationError::InsufficientStock { .. })),
        "must fail: {rejected:?}"
    );
    assert_eq!(movement_count(&harness), before, "zero partial rows");
    assert_eq!(level_of(&harness, product, src), 10_000);
    assert_eq!(level_of(&harness, product, dst), 0);
}

#[test]
fn kit_product_direct_movement_rejected_with_nothing_written() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (kit, unit) = seed_product(&harness, "KIT", true);
    let rejected = increase(&harness, Role::Gerant, kit, warehouse, unit, 1_000, false);
    assert!(
        matches!(
            rejected,
            Err(ApplicationError::CannotMoveStockForKitProduct)
        ),
        "kits never move directly: {rejected:?}"
    );
    assert_eq!(movement_count(&harness), 0);
    assert_eq!(level_of(&harness, kit, warehouse), 0);
}

#[test]
fn vendeur_rejected_from_mutations_before_any_write() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let other = seed_warehouse(&harness, "OTHER");
    let (product, unit) = seed_product(&harness, "P1", false);
    let vendeur = session_with(Role::Vendeur);
    assert!(matches!(
        AdjustStockUseCase::new(&harness.stock).execute(
            &vendeur,
            AdjustStockInput {
                product_id: product,
                warehouse_id: warehouse,
                unit_id: unit,
                quantity_micro: 1_000,
                direction: StockDirection::Increase,
                reason: "Inventaire".to_string(),
                authorize_negative_stock: false,
            },
        ),
        Err(ApplicationError::Unauthorized)
    ));
    assert!(matches!(
        TransferStockUseCase::new(&harness.stock, &harness.warehouses).execute(
            &vendeur,
            TransferStockInput {
                source_warehouse_id: warehouse,
                destination_warehouse_id: other,
                product_id: product,
                unit_id: unit,
                quantity_micro: 1_000,
                reason: None,
            },
        ),
        Err(ApplicationError::Unauthorized)
    ));
    assert_eq!(movement_count(&harness), 0);
}

#[test]
fn granular_seams_round_trip() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", false);
    let actor = session_with(Role::Admin).user_id;
    let factor = harness
        .stock
        .get_product_unit_conversion(product, unit)
        .expect("factor");
    assert_eq!(factor.micros(), 10_000);
    let id = harness
        .stock
        .insert_stock_movement(&NewStockMovement {
            product_id: product,
            warehouse_id: warehouse,
            unit_id: unit,
            movement_type: StockMovementType::PurchaseIn,
            quantity_micro: 7_000,
            quantity_base_micro: 7_000,
            reference_type: None,
            reference_id: None,
            transfer_batch_id: None,
            authorized_negative_stock: false,
            reason: None,
            actor,
        })
        .expect("insert");
    assert!(!id.is_empty());
    // Direct appends intentionally bypass level maintenance.
    assert_eq!(level_of(&harness, product, warehouse), 0);
    let new = harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, 7_000)
        .expect("upsert");
    assert_eq!(new, 7_000);
    assert_eq!(level_of(&harness, product, warehouse), 7_000);
    assert_eq!(movement_count(&harness), 1);
}

#[test]
fn all_seven_types_flow_through_the_engine() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", false);
    let session = session_with(Role::Admin);
    let by_type = [
        (StockMovementType::PurchaseIn, 40_000, 40_000),
        (StockMovementType::SaleOut, 40_000, 0),
        (StockMovementType::ReturnIn, 40_000, 40_000),
        (StockMovementType::AdjustmentPositive, 40_000, 80_000),
        (StockMovementType::AdjustmentNegative, 40_000, 40_000),
    ];
    for (movement_type, qty, expected) in by_type {
        let record = harness
            .stock
            .apply_stock_movement(&baraka_application::ports::StockMovementCommand {
                product_id: product,
                warehouse_id: warehouse,
                unit_id: unit,
                quantity_micro: qty,
                movement_type,
                reference_type: None,
                reference_id: None,
                transfer_batch_id: None,
                authorize_negative_stock: false,
                reason: None,
                actor: session.user_id,
            })
            .unwrap_or_else(|error| panic!("{movement_type:?}: {error:?}"));
        assert_eq!(record.new_quantity_base_micro, expected);
    }
    assert_eq!(level_of(&harness, product, warehouse), 40_000);
    assert_eq!(movement_count(&harness), 5);
}
