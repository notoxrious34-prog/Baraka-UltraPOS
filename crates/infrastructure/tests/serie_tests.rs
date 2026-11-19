//! Série composer over a real SQLite backend (TASK-BRK-115).
//!
//! FAIL-before: no composer exists — kits are hand-wired edge by edge with
//! no size-run resolution and no availability precheck. After: a 2M+4L+
//! 4XL+2XXL run on a matrix color resolves to cell products, persists the
//! kit graph atomically, and REPORTS shortfalls without blocking (the
//! série template is defined before its components are received).

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    CategoryRepository, ProductRepository, SessionContext, SessionToken, StockRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::variant_matrix::{
    ComposeSerieInput, ComposeSerieUseCase, GenerateVariantMatrixInput,
    GenerateVariantMatrixUseCase, SerieLineInput,
};
use baraka_domain::catalog::{Category, CategoryId, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteStockRepository, SqliteVariantMatrixRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::{Duration, Utc};

const TEST_TERMINAL: &str = "TEST-TERMINAL-SERIE";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    matrix: SqliteVariantMatrixRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("serie.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        matrix: SqliteVariantMatrixRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
    }
}

fn session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId(uuid::Uuid::new_v4()),
        role: Role::Admin,
        expires_at: Utc::now() + Duration::hours(1),
    }
}

struct NoCategories;

impl CategoryRepository for NoCategories {
    fn create_category(
        &self,
        _: &Category,
    ) -> Result<(), baraka_application::errors::ApplicationError> {
        Ok(())
    }

    fn find_category_by_id(
        &self,
        _: CategoryId,
    ) -> Result<Option<Category>, baraka_application::errors::ApplicationError> {
        Ok(None)
    }

    fn list_categories(
        &self,
    ) -> Result<Vec<Category>, baraka_application::errors::ApplicationError> {
        Ok(Vec::new())
    }

    fn update_category(
        &self,
        _: &Category,
    ) -> Result<(), baraka_application::errors::ApplicationError> {
        Ok(())
    }
}

fn seed_template(harness: &Harness) -> ProductId {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("CTN-100").expect("sku"),
                barcode: None,
                name_ar: "سيرية قطن".to_string(),
                name_fr: "SERIE COTON".to_string(),
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
                barcode: None,
                sale_price: Money::from_centimes(25_000),
                purchase_price: Money::from_centimes(15_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("template");
    id
}

fn seed_kit_header(harness: &Harness) -> ProductId {
    let id = ProductId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("SERIE-NOIR-2442").expect("sku"),
                barcode: None,
                name_ar: "سيرية نوار".to_string(),
                name_fr: "SERIE NOIR".to_string(),
                category_id: None,
                is_kit: true,
                is_active: true,
            },
            &[ProductUnit {
                id: UnitId::new(),
                product_id: id,
                unit_name_ar: "تشكيلة".to_string(),
                unit_name_fr: "Assortiment".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: None,
                sale_price: Money::from_centimes(290_000),
                purchase_price: Money::from_centimes(180_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("kit header");
    id
}

#[test]
fn compose_classic_serie_reports_shortfalls_without_blocking() {
    let harness = setup();
    let template_id = seed_template(&harness);
    let kit_id = seed_kit_header(&harness);
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("DEPOT").expect("code"),
        name: "Depot".to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, TEST_TERMINAL)
        .expect("warehouse");

    let generate =
        GenerateVariantMatrixUseCase::new(&harness.catalog, &NoCategories, &harness.matrix);
    let output = generate
        .execute(
            &session(),
            GenerateVariantMatrixInput {
                name_ar: "سيرية قطن".to_string(),
                name_fr: "SERIE COTON".to_string(),
                axis1_name: "Taille".to_string(),
                axis2_name: "Couleur".to_string(),
                axis1_values: vec![
                    "M".to_string(),
                    "L".to_string(),
                    "XL".to_string(),
                    "XXL".to_string(),
                ],
                axis2_values: vec!["NOIR".to_string()],
                base_sku: "CTN-100".to_string(),
                category_id: None,
                template_product_id: template_id,
                overrides: Vec::new(),
            },
            TEST_TERMINAL,
        )
        .expect("generates");
    assert_eq!(output.cell_count, 4);

    // Stock: M covers 2 séries exactly, L runs short, XL/XXL cover exactly.
    for (sku, level) in [
        ("CTN-100-M-NOIR", 40_000),
        ("CTN-100-L-NOIR", 10_000),
        ("CTN-100-XL-NOIR", 80_000),
        ("CTN-100-XXL-NOIR", 40_000),
    ] {
        let (product, _) = harness
            .catalog
            .find_by_sku(&Sku::new(sku).expect("sku"))
            .expect("cell")
            .expect("cell product");
        harness
            .stock
            .upsert_stock_level_with_delta(product.id, warehouse.id, level)
            .expect("level");
    }

    let compose = ComposeSerieUseCase::new(&harness.catalog, &harness.matrix, &harness.stock);
    let output = compose
        .execute(
            &session(),
            ComposeSerieInput {
                kit_id,
                model_id: output.model_id,
                axis2_value: "NOIR".to_string(),
                lines: vec![
                    SerieLineInput {
                        axis1_value: "M".to_string(),
                        pieces: 2,
                    },
                    SerieLineInput {
                        axis1_value: "L".to_string(),
                        pieces: 4,
                    },
                    SerieLineInput {
                        axis1_value: "XL".to_string(),
                        pieces: 4,
                    },
                    SerieLineInput {
                        axis1_value: "XXL".to_string(),
                        pieces: 2,
                    },
                ],
                warehouse_id: Some(warehouse.id),
                serie_count: 2,
            },
        )
        .expect("composes");
    assert_eq!(output.component_count, 4);
    assert_eq!(output.total_pieces, 12);
    // 2 séries need L: 4×2×10_000 = 80_000, on hand 10_000 → exactly one shortfall.
    assert_eq!(output.shortfalls.len(), 1);
    assert_eq!(output.shortfalls[0].required_micro, 80_000);
    assert_eq!(output.shortfalls[0].on_hand_micro, 10_000);

    // Tolerant: the kit graph persisted despite the shortfall.
    let edges = harness.catalog.get_direct_kit_items(kit_id).expect("edges");
    assert_eq!(edges.len(), 4);
    let total: i64 = edges.iter().map(|edge| edge.quantity_micro).sum();
    assert_eq!(total, 120_000, "2+4+4+2 pieces × 10_000 micros");
}
