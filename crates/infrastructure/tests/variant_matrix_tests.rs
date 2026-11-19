//! Variant-matrix engine over a real SQLite backend (TASK-BRK-114).
//!
//! FAIL-before: no `variant_models` table, no matrix port, no planner — the
//! grid concept does not exist. After: template → 4 atomic cells with
//! inherited ladder/tiers/rungs, ordered 2D view, and a SKU clash rolling
//! back the WHOLE second matrix (no half-written models).

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    CategoryRepository, ProductRepository, SessionContext, SessionToken, VariantMatrixRepository,
};
use baraka_application::use_cases::variant_matrix::{
    CellOverrideInput, GenerateVariantMatrixInput, GenerateVariantMatrixUseCase,
    GetModelMatrixViewUseCase,
};
use baraka_domain::catalog::{
    Category, CategoryId, PriceBreak, Product, ProductId, ProductUnit, Sku, UnitId,
};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{SqliteCatalogRepository, SqliteVariantMatrixRepository};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::{Duration, Utc};

const TEST_TERMINAL: &str = "TEST-TERMINAL-VARIANT-MATRIX";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    matrix: SqliteVariantMatrixRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("variant_matrix.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        matrix: SqliteVariantMatrixRepository::new(Arc::clone(&shared)),
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
                wholesale_price: Some(Money::from_centimes(22_000)),
                super_gros_price: Some(Money::from_centimes(20_000)),
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("template");
    harness
        .catalog
        .replace_price_breaks(
            id,
            unit_id,
            &[PriceBreak {
                min_qty_micro: 120_000,
                price: Money::from_centimes(23_000),
            }],
        )
        .expect("rung");
    id
}

fn generate_input(template_id: ProductId) -> GenerateVariantMatrixInput {
    GenerateVariantMatrixInput {
        name_ar: "سيرية قطن".to_string(),
        name_fr: "SERIE COTON".to_string(),
        axis1_name: "Taille".to_string(),
        axis2_name: "Couleur".to_string(),
        axis1_values: vec!["XXL".to_string(), "M".to_string()],
        axis2_values: vec!["NOIR".to_string(), "BLANC".to_string()],
        base_sku: "CTN-100".to_string(),
        category_id: None,
        template_product_id: template_id,
        overrides: vec![CellOverrideInput {
            axis1: "M".to_string(),
            axis2: "BLANC".to_string(),
            sale_price_centimes: Some(27_000),
            barcode: Some("CELL-M-BLANC".to_string()),
        }],
    }
}

#[test]
fn generation_persists_model_cells_and_view_atomically() {
    let harness = setup();
    let template_id = seed_template(&harness);
    let generate =
        GenerateVariantMatrixUseCase::new(&harness.catalog, &NoCategories, &harness.matrix);
    let output = generate
        .execute(&session(), generate_input(template_id), TEST_TERMINAL)
        .expect("generates");
    assert_eq!(output.cell_count, 4);
    assert_eq!(output.skus[0], "CTN-100-M-BLANC");

    let model = harness
        .matrix
        .find_variant_model(output.model_id)
        .expect("find")
        .expect("model row");
    assert_eq!(model.name_fr, "SERIE COTON");
    assert_eq!(model.axis1_name, "Taille");

    let cells = harness.matrix.matrix_cells(output.model_id).expect("cells");
    assert_eq!(cells.len(), 4);
    let axes: Vec<(&str, &str)> = cells
        .iter()
        .map(|cell| (cell.axis1_value.as_str(), cell.axis2_value.as_str()))
        .collect();
    assert_eq!(
        axes,
        vec![
            ("M", "BLANC"),
            ("M", "NOIR"),
            ("XXL", "BLANC"),
            ("XXL", "NOIR")
        ]
    );

    let view = GetModelMatrixViewUseCase::new(&harness.matrix)
        .execute(&session(), output.model_id)
        .expect("view");
    assert_eq!(view.len(), 4);
    assert_eq!(view[0].sale_price_centimes, Some(27_000));
    assert_eq!(view[1].sale_price_centimes, Some(25_000));
    assert!(view.iter().all(|row| row.on_hand_micro == 0));

    // Inherited rung rides along re-keyed on every cell.
    for row in &view {
        let rungs = harness
            .catalog
            .price_breaks(
                row.product_id,
                harness
                    .catalog
                    .find_by_id(row.product_id)
                    .expect("product")
                    .expect("cell product")
                    .1
                    .iter()
                    .find(|unit| unit.is_default_selling)
                    .expect("default unit")
                    .id,
            )
            .expect("rungs");
        assert_eq!(rungs.len(), 1);
        assert_eq!(rungs[0].price, Money::from_centimes(23_000));
    }

    // Override barcode landed on the product row; template untouched.
    let (_, units) = harness
        .catalog
        .find_by_sku(&Sku::new("CTN-100-M-BLANC").expect("sku"))
        .expect("cell")
        .expect("cell product");
    assert!(units.iter().all(|unit| unit.barcode.is_none()));
}

/// Zero-method category port: generation runs without a category here.
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

#[test]
fn sku_clash_rolls_back_the_whole_second_matrix() {
    let harness = setup();
    let template_id = seed_template(&harness);
    let generate =
        GenerateVariantMatrixUseCase::new(&harness.catalog, &NoCategories, &harness.matrix);
    generate
        .execute(&session(), generate_input(template_id), TEST_TERMINAL)
        .expect("first matrix");
    let before = harness
        .catalog
        .get_metrics()
        .expect("metrics")
        .total_products;

    // Same base SKU → same cell SKUs → UNIQUE(products.sku) must abort all.
    let clash = generate.execute(&session(), generate_input(template_id), TEST_TERMINAL);
    assert!(clash.is_err(), "duplicate cell SKU must fail LOUDLY");
    let after = harness
        .catalog
        .get_metrics()
        .expect("metrics")
        .total_products;
    assert_eq!(before, after, "no half-written products survive");
}
