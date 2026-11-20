//! Matrix receiving tests (DSN-109).
//!
//! A size run is received cell-by-cell through the model's default selling
//! units, delegating to the atomic `ReceiveGoodsUseCase` commit — one
//! transaction, idempotent on the command id. Unknown cells and empty
//! runs refuse LOUDLY, before any stock moves.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    NewSupplier, ProductRepository, SessionContext, SessionToken, VariantMatrixRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::receive_matrix::{
    MatrixReceiveCellInput, ReceiveMatrixGoodsInput, ReceiveMatrixGoodsUseCase,
};
use baraka_application::use_cases::suppliers::CreateSupplierUseCase as CreateSupplier;
use baraka_application::use_cases::variant_matrix::{
    GenerateVariantMatrixInput, GenerateVariantMatrixUseCase,
};
use baraka_domain::catalog::{
    Barcode, Product, ProductId, ProductUnit, Sku, UnitId, VariantModelId,
};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqlitePurchaseRepository, SqliteSupplierRepository,
    SqliteVariantMatrixRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use chrono::{TimeDelta, Utc};
use uuid::Uuid;

const ADMIN_UUID: &str = "99999999-9999-9999-9999-999999999999";
const GERANT_UUID: &str = "33333333-3333-3333-3333-333333333333";

fn session_with(role: Role, uuid: &str) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(uuid).expect("uuid")),
        role,
        expires_at: Utc::now() + TimeDelta::hours(1),
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    matrix: SqliteVariantMatrixRepository,
    warehouses: SqliteWarehouseRepository,
    suppliers: SqliteSupplierRepository,
    purchases: SqlitePurchaseRepository,
    terminal_id: String,
    warehouse_id: WarehouseId,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("matrix_receive.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    for (id, name, role) in [
        (ADMIN_UUID, "admin1", "ADMIN"),
        (GERANT_UUID, "gerant1", "GERANT"),
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
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        matrix: SqliteVariantMatrixRepository::new(Arc::clone(&shared)),
        warehouses,
        suppliers: SqliteSupplierRepository::new(Arc::clone(&shared)),
        purchases: SqlitePurchaseRepository::new(Arc::clone(&shared)),
        terminal_id,
        warehouse_id: warehouse.id,
    }
}

fn seed_template(harness: &Harness) -> ProductId {
    let id = ProductId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("RCV-TMPL").expect("sku"),
                barcode: None,
                name_ar: "قالب".to_string(),
                name_fr: "Template".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: UnitId::new(),
                product_id: id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: Some(Barcode::new("6111259000014").expect("barcode")),
                sale_price: Money::from_centimes(20_000),
                purchase_price: Money::from_centimes(8_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            &harness.terminal_id,
        )
        .expect("template");
    id
}

fn seed_model(harness: &Harness) -> VariantModelId {
    let template = seed_template(harness);
    GenerateVariantMatrixUseCase::new(&harness.catalog, &harness.catalog, &harness.matrix)
        .execute(
            &session_with(Role::Admin, ADMIN_UUID),
            GenerateVariantMatrixInput {
                name_ar: "موديل".to_string(),
                name_fr: "Modele".to_string(),
                axis1_name: "Taille".to_string(),
                axis2_name: "Couleur".to_string(),
                axis1_values: vec!["M".to_string(), "L".to_string()],
                axis2_values: vec!["Rouge".to_string()],
                base_sku: "RCV".to_string(),
                category_id: None,
                template_product_id: template,
                overrides: vec![],
            },
            &harness.terminal_id,
        )
        .expect("model")
        .model_id
}

fn seed_supplier(harness: &Harness) -> String {
    CreateSupplier::new(&harness.suppliers)
        .execute(
            &session_with(Role::Gerant, GERANT_UUID),
            NewSupplier {
                name: "Fournisseur".to_string(),
                phone: None,
                address: None,
                nif: None,
                rc: None,
                notes: None,
            },
            &harness.terminal_id,
        )
        .expect("supplier")
        .id
}

#[test]
fn size_run_receives_through_default_units_and_repeats_idempotently() {
    let harness = setup();
    let model = seed_model(&harness);
    let supplier = seed_supplier(&harness);
    let use_case = ReceiveMatrixGoodsUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
        &harness.matrix,
    );
    let input = ReceiveMatrixGoodsInput {
        supplier_id: supplier.clone(),
        warehouse_id: harness.warehouse_id,
        model_id: model,
        cells: vec![
            MatrixReceiveCellInput {
                axis1: "M".to_string(),
                axis2: "Rouge".to_string(),
                quantity_micro: 5 * 10_000,
                unit_cost_centimes: 8_000,
            },
            MatrixReceiveCellInput {
                axis1: "L".to_string(),
                axis2: "Rouge".to_string(),
                quantity_micro: 7 * 10_000,
                unit_cost_centimes: 8_000,
            },
        ],
        source_bc_id: None,
        terminal_id: harness.terminal_id.clone(),
        command_id: "cmd-matrix-rcv-1".to_string(),
    };
    let first = use_case
        .execute(&session_with(Role::Gerant, GERANT_UUID), input.clone())
        .expect("receive");
    let second = use_case
        .execute(&session_with(Role::Gerant, GERANT_UUID), input)
        .expect("replay");
    assert_eq!(first.id, second.id, "same command replays the same BR");

    let rows = harness.matrix.matrix_view(model).expect("view");
    assert_eq!(rows.len(), 2);
    let mut received: Vec<(String, String)> = rows
        .iter()
        .map(|row| (row.axis1_value.clone(), row.axis2_value.clone()))
        .collect();
    received.sort();
    assert_eq!(
        received,
        vec![
            ("L".to_string(), "Rouge".to_string()),
            ("M".to_string(), "Rouge".to_string()),
        ]
    );
    // Stock actually landed: 5 + 7 pieces in base micros.
    let mut on_hand: Vec<(String, i64)> = rows
        .iter()
        .map(|row| (row.axis1_value.clone(), row.on_hand_micro))
        .collect();
    on_hand.sort();
    assert_eq!(
        on_hand,
        vec![("L".to_string(), 70_000), ("M".to_string(), 50_000)]
    );
}

#[test]
fn unknown_cell_and_empty_run_refuse_before_stock_moves() {
    let harness = setup();
    let model = seed_model(&harness);
    let supplier = seed_supplier(&harness);
    let use_case = ReceiveMatrixGoodsUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
        &harness.matrix,
    );
    let ghost = use_case.execute(
        &session_with(Role::Gerant, GERANT_UUID),
        ReceiveMatrixGoodsInput {
            supplier_id: supplier.clone(),
            warehouse_id: harness.warehouse_id,
            model_id: model,
            cells: vec![MatrixReceiveCellInput {
                axis1: "XL".to_string(),
                axis2: "Rouge".to_string(),
                quantity_micro: 5 * 10_000,
                unit_cost_centimes: 8_000,
            }],
            source_bc_id: None,
            terminal_id: harness.terminal_id.clone(),
            command_id: "cmd-matrix-rcv-ghost".to_string(),
        },
    );
    assert!(ghost.is_err(), "unknown size refuses LOUDLY");

    let empty = use_case.execute(
        &session_with(Role::Gerant, GERANT_UUID),
        ReceiveMatrixGoodsInput {
            supplier_id: supplier,
            warehouse_id: harness.warehouse_id,
            model_id: model,
            cells: vec![],
            source_bc_id: None,
            terminal_id: harness.terminal_id.clone(),
            command_id: "cmd-matrix-rcv-empty".to_string(),
        },
    );
    assert!(empty.is_err(), "empty run refuses LOUDLY");
}
