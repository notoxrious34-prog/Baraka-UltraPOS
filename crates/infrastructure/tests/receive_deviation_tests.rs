//! Supplier price-deviation preview tests (DSN-109, proposal 7).
//!
//! Margin protection at source: a receipt line is flagged only when its
//! unit cost exceeds BOTH the last purchase price and the 90-day average
//! by more than the threshold. The preview is advisory — it never blocks
//! the atomic receipt.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    NewSupplier, ProductRepository, PurchaseLineInput, SessionContext, SessionToken,
    WarehouseRepository,
};
use baraka_application::use_cases::purchases::{CreatePurchaseOrderUseCase, ReceiveGoodsUseCase};
use baraka_application::use_cases::receive_deviations::{
    PreviewReceiveDeviationsUseCase, ReceiveCostLineInput,
};
use baraka_application::use_cases::suppliers::CreateSupplierUseCase;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqlitePurchaseRepository, SqliteSupplierRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use chrono::{TimeDelta, Utc};
use uuid::Uuid;

const VENDEUR_UUID: &str = "11111111-1111-1111-1111-111111111111";
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
    warehouses: SqliteWarehouseRepository,
    suppliers: SqliteSupplierRepository,
    purchases: SqlitePurchaseRepository,
    terminal_id: String,
    warehouse_id: WarehouseId,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("deviation.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    for (id, name, role) in [
        (VENDEUR_UUID, "vendeur1", "VENDEUR"),
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
        warehouses,
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
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            &harness.terminal_id,
        )
        .expect("product");
    (id, unit_id)
}

fn seed_supplier(harness: &Harness) -> String {
    CreateSupplierUseCase::new(&harness.suppliers)
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

fn receive(
    harness: &Harness,
    supplier_id: &str,
    product: ProductId,
    unit: UnitId,
    units: i64,
    cost: i64,
) {
    let bc = CreatePurchaseOrderUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
    )
    .execute(
        &session_with(Role::Gerant, GERANT_UUID),
        supplier_id,
        harness.warehouse_id,
        vec![PurchaseLineInput {
            product_id: product,
            unit_id: unit,
            quantity_micro: units * 10_000,
            unit_cost_centimes: cost,
        }],
        &harness.terminal_id,
        &Uuid::new_v4().to_string(),
    )
    .expect("bc")
    .id;
    ReceiveGoodsUseCase::new(&harness.purchases, &harness.suppliers, &harness.warehouses)
        .execute(
            &session_with(Role::Gerant, GERANT_UUID),
            Some(bc),
            supplier_id,
            harness.warehouse_id,
            vec![PurchaseLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: units * 10_000,
                unit_cost_centimes: cost,
            }],
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("br");
}

#[test]
fn spike_over_last_and_average_flags_while_small_drift_passes() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "DEV-1");
    let supplier = seed_supplier(&harness);
    receive(&harness, &supplier, product, unit, 100, 8_000);

    let preview = PreviewReceiveDeviationsUseCase::new(&harness.purchases)
        .execute(
            &session_with(Role::Gerant, GERANT_UUID),
            &[ReceiveCostLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 100 * 10_000,
                unit_cost_centimes: 10_000,
            }],
            1_000,
        )
        .expect("preview");
    assert_eq!(preview.len(), 1);
    assert_eq!(preview[0].last_cost_centimes, Some(8_000));
    assert_eq!(preview[0].average90_cost_centimes, Some(8_000));
    assert_eq!(preview[0].delta_vs_last_bps, Some(2_500));
    assert!(preview[0].flagged, "25% over both references must flag");

    let calm = PreviewReceiveDeviationsUseCase::new(&harness.purchases)
        .execute(
            &session_with(Role::Gerant, GERANT_UUID),
            &[ReceiveCostLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 100 * 10_000,
                unit_cost_centimes: 8_200,
            }],
            1_000,
        )
        .expect("preview");
    assert!(!calm[0].flagged, "2.5% drift stays advisory-quiet");
}

#[test]
fn first_receipt_without_history_never_flags() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "DEV-2");
    let preview = PreviewReceiveDeviationsUseCase::new(&harness.purchases)
        .execute(
            &session_with(Role::Gerant, GERANT_UUID),
            &[ReceiveCostLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 10 * 10_000,
                unit_cost_centimes: 99_000,
            }],
            1_000,
        )
        .expect("preview");
    assert_eq!(preview[0].last_cost_centimes, None);
    assert_eq!(preview[0].average90_cost_centimes, None);
    assert!(!preview[0].flagged);
}
