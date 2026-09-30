//! Kit expansion end-to-end over real persisted catalog rows.
//!
//! Scenario (§2.4, exact):
//!   D physical (piece), C physical (piece),
//!   B kit = { C x1, D x5 } per base unit,
//!   A kit = { B x2, C x3 } per base unit.
//! Expand A for 2 base units -> C = 10 (4 via B + 6 direct), D = 20.

use std::sync::{Arc, Mutex};

use baraka_application::ports::ProductRepository;
use baraka_application::services::kit_expansion_service::expand_for_movement;
use baraka_domain::catalog::{Barcode, KitItem, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::primitives::{FactorMicro, Money, QtyMicro};
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;

struct Harness {
    _dir: tempfile::TempDir,
    repo: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("kit.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    Harness {
        _dir: dir,
        repo: SqliteCatalogRepository::new(Arc::new(Mutex::new(conn))),
    }
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
        is_base_unit: true,
        is_default_selling: true,
    };
    harness
        .repo
        .create_product(&product, &[unit], "TEST-TERMINAL")
        .expect("product");
    (id, unit_id)
}

fn link(
    harness: &Harness,
    kit: ProductId,
    component: ProductId,
    unit: UnitId,
    qty_base_units: i64,
) {
    let mut items: Vec<KitItem> = harness
        .repo
        .get_kit_items(kit)
        .expect("read")
        .into_iter()
        .collect();
    items.push(KitItem {
        kit_product_id: kit,
        component_product_id: component,
        unit_id: unit,
        quantity_micro: QtyMicro::from_micros(qty_base_units * 10_000),
    });
    harness.repo.define_kit(kit, &items).expect("define");
}

#[test]
fn expansion_matches_expected_quantities() {
    let harness = setup();
    let (d, d_unit) = seed_product(&harness, "D", false);
    let (c, c_unit) = seed_product(&harness, "C", false);
    let (b, b_unit) = seed_product(&harness, "B", true);
    let (a, _) = seed_product(&harness, "A", true);

    link(&harness, b, c, c_unit, 1);
    link(&harness, b, d, d_unit, 5);
    link(&harness, a, b, b_unit, 2);
    link(&harness, a, c, c_unit, 3);

    let out = expand_for_movement(&harness.repo, a, 20_000).expect("expand");
    assert_eq!(out.len(), 2, "exactly C and D: {out:?}");
    let qty_of = |id: ProductId| {
        out.iter()
            .find(|leaf| leaf.product_id == id)
            .unwrap_or_else(|| panic!("missing leaf {id}"))
            .quantity_base_micro
    };
    assert_eq!(qty_of(c), 100_000, "C = 10 base units");
    assert_eq!(qty_of(d), 200_000, "D = 20 base units");
    // Sorted deterministically by (product_id, base_unit_id).
    let mut sorted = out.clone();
    sorted.sort_by(|x, y| {
        (x.product_id.0, x.base_unit_id.0).cmp(&(y.product_id.0, y.base_unit_id.0))
    });
    assert_eq!(out, sorted);
}
