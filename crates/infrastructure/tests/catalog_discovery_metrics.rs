//! Catalog discovery inactive-leak and metrics acceptance tests (TASK-BRK-074).
//!
//! End-to-end proof over the real SQLite repository that:
//! - blank discovery and the barcode fast path exclude `is_active = 0` rows
//!   unless `include_inactive` is set (POS leak fix);
//! - the typed token path keeps its `is_active = 1` default;
//! - `get_metrics` returns exact database aggregates;
//! - discovery limits above 100 are honored (clamp raised to 500).

use std::sync::{Arc, Mutex};

use baraka_application::ports::ProductRepository;
use baraka_application::ports::SessionContext;
use baraka_application::ports::SessionToken;
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, GetCatalogMetricsUseCase, NewUnitInput,
    SearchCatalogUseCase,
};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

struct Harness {
    _dir: tempfile::TempDir,
    conn: Arc<Mutex<rusqlite::Connection>>,
    repo: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open_connection(&dir.path().join("discovery.db")).expect("open");
    let mut conn = conn;
    run_migrations(&mut conn).expect("migrate");
    let conn = Arc::new(Mutex::new(conn));
    let repo = SqliteCatalogRepository::new(Arc::clone(&conn));
    Harness {
        _dir: dir,
        conn,
        repo,
    }
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-FIX";

/// Canonical seeded depot id (migration 0012).
const DEPOT_ID: &str = "01950000-0000-7000-8000-000000000001";

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn unit(barcode: Option<String>) -> NewUnitInput {
    NewUnitInput {
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Pièce".to_string(),
        factor_micro: 10_000,
        barcode,
        sale_price_centimes: 15_000,
        purchase_price_centimes: 12_000,
        wholesale_price_centimes: None,
        is_base_unit: true,
        is_default_selling: true,
    }
}

fn seed(
    harness: &Harness,
    gerant: &SessionContext,
    sku: &str,
    name_fr: &str,
    barcode: Option<String>,
) -> baraka_domain::catalog::ProductId {
    CreateProductUseCase::new(&harness.repo, &harness.repo)
        .execute(
            gerant,
            CreateProductInput {
                sku: sku.to_string(),
                barcode: None,
                name_ar: "صنف".to_string(),
                name_fr: name_fr.to_string(),
                category_id: None,
                is_kit: false,
                units: vec![unit(barcode)],
            },
            TEST_TERMINAL,
        )
        .expect("seed product")
}

fn skus_of(
    hits: &[(
        baraka_domain::catalog::Product,
        Vec<baraka_domain::catalog::ProductUnit>,
    )],
) -> Vec<String> {
    hits.iter()
        .map(|(product, _)| product.sku.as_str().to_string())
        .collect()
}

fn seed_trio(harness: &Harness, gerant: &SessionContext) {
    seed(
        harness,
        gerant,
        "ACT-001",
        "Produit Actif Un",
        Some("6111250090100".to_string()),
    );
    seed(harness, gerant, "ACT-002", "Produit Actif Deux", None);
    let dead = seed(harness, gerant, "BAT-002", "Produit Inactif", None);
    harness
        .repo
        .set_product_active(dead, false)
        .expect("deactivate");
}

#[test]
fn blank_discovery_excludes_inactive_by_default() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let vendeur = session_with(Role::Vendeur);
    seed_trio(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);

    let visible = search.execute(&vendeur, "", 20, false).expect("discovery");
    let skus = skus_of(&visible);
    assert_eq!(skus.len(), 2, "only active rows surface: {skus:?}");
    assert!(skus.contains(&"ACT-001".to_string()));
    assert!(skus.contains(&"ACT-002".to_string()));
    assert!(
        !skus.contains(&"BAT-002".to_string()),
        "inactive BAT-002 must not leak into POS discovery"
    );

    let all = search
        .execute(&vendeur, "", 20, true)
        .expect("discovery all");
    let all_skus = skus_of(&all);
    assert_eq!(
        all_skus.len(),
        3,
        "include_inactive admits all: {all_skus:?}"
    );
    assert!(all_skus.contains(&"BAT-002".to_string()));
}

#[test]
fn barcode_fast_path_excludes_inactive_by_default() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let vendeur = session_with(Role::Vendeur);
    let dead = seed(
        &harness,
        &gerant,
        "BAT-003",
        "Produit Inactif Code",
        Some("6111250090200".to_string()),
    );
    harness
        .repo
        .set_product_active(dead, false)
        .expect("deactivate");
    let search = SearchCatalogUseCase::new(&harness.repo);

    let hidden = search
        .execute(&vendeur, "6111250090200", 5, false)
        .expect("barcode miss");
    assert!(
        hidden.is_empty(),
        "inactive barcode must not resolve for POS scanning"
    );

    let found = search
        .execute(&vendeur, "6111250090200", 5, true)
        .expect("barcode hit");
    assert_eq!(skus_of(&found), vec!["BAT-003".to_string()]);
}

#[test]
fn token_search_excludes_inactive_by_default() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let vendeur = session_with(Role::Vendeur);
    seed_trio(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);

    let hidden = search
        .execute(&vendeur, "Inactif", 20, false)
        .expect("token miss");
    assert!(
        hidden.is_empty(),
        "typed search must not return inactive rows by default"
    );

    let found = search
        .execute(&vendeur, "Inactif", 20, true)
        .expect("token hit");
    assert_eq!(skus_of(&found), vec!["BAT-002".to_string()]);
}

#[test]
fn get_metrics_counts_match_database() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let vendeur = session_with(Role::Vendeur);
    let stocked = seed(&harness, &gerant, "STK-001", "Produit Stocke", None);
    seed(&harness, &gerant, "STK-002", "Produit Sans Stock", None);
    let dead = seed(&harness, &gerant, "BAT-004", "Produit Inactif", None);
    harness
        .repo
        .set_product_active(dead, false)
        .expect("deactivate");
    harness
        .conn
        .lock()
        .expect("conn")
        .execute(
            "INSERT INTO stock_levels
             (warehouse_id, product_id, quantity_micro, quantity_base_micro, revision)
             VALUES (?1, ?2, ?3, ?3, 0)",
            rusqlite::params![DEPOT_ID, stocked.to_string(), 50_000i64],
        )
        .expect("seed level");

    let metrics = GetCatalogMetricsUseCase::new(&harness.repo)
        .execute(&vendeur)
        .expect("metrics");
    assert_eq!(metrics.total_products, 3, "every row counts");
    assert_eq!(metrics.inactive, 1, "one deactivated row");
    assert_eq!(metrics.in_stock, 1, "positive level counts once");
    assert_eq!(metrics.out_of_stock, 1, "active row without level");
}

#[test]
fn discovery_limit_beyond_100_is_honored() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let vendeur = session_with(Role::Vendeur);
    for index in 0..105 {
        seed(
            &harness,
            &gerant,
            &format!("LOAD-{index:03}"),
            &format!("Produit Vrac {index:03}"),
            None,
        );
    }
    let search = SearchCatalogUseCase::new(&harness.repo);

    let capped = search.execute(&vendeur, "", 100, false).expect("capped");
    assert_eq!(capped.len(), 100);
    let wide = search.execute(&vendeur, "", 500, false).expect("wide");
    assert_eq!(
        wide.len(),
        105,
        "raised clamp surfaces the full catalog slice"
    );
}
