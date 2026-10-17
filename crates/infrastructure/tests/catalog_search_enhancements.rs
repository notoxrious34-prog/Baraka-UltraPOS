//! Catalog search enhancement tests (TASK-BRK-073).
//!
//! End-to-end coverage of the upgraded `search` over the real SQLite
//! repository: infix matching inside product names, multi-token AND
//! conjunction, exact barcode priority (product-level and unit-level
//! codes), Arabic orthographic tolerance (alef hamza, taa marbuta, yaa),
//! and preservation of empty-query discovery, pagination and limits.

use std::sync::{Arc, Mutex};

use baraka_application::ports::SessionContext;
use baraka_application::ports::SessionToken;
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, NewUnitInput, SearchCatalogUseCase,
};
use baraka_domain::catalog::Product;
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

struct Harness {
    _dir: tempfile::TempDir,
    repo: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("search_enhanced.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    Harness {
        _dir: dir,
        repo: SqliteCatalogRepository::new(Arc::new(Mutex::new(conn))),
    }
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-FIX";

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn unit(name_ar: &str, name_fr: &str, barcode: Option<String>) -> NewUnitInput {
    NewUnitInput {
        unit_name_ar: name_ar.to_string(),
        unit_name_fr: name_fr.to_string(),
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
    name_ar: &str,
    product_barcode: Option<String>,
    unit_barcode: Option<String>,
) {
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    create
        .execute(
            gerant,
            CreateProductInput {
                sku: sku.to_string(),
                barcode: product_barcode,
                name_ar: name_ar.to_string(),
                name_fr: name_fr.to_string(),
                category_id: None,
                is_kit: false,
                units: vec![unit("قطعة", "Pièce", unit_barcode)],
            },
            TEST_TERMINAL,
        )
        .expect("seed product");
}

fn seed_catalog(harness: &Harness, gerant: &SessionContext) {
    seed(
        harness,
        gerant,
        "DOUCH-01",
        "Pomme de douchette chromée",
        "رأس دش كروم",
        None,
        Some("613000000001".to_string()),
    );
    seed(
        harness,
        gerant,
        "FLEX-01",
        "Flexible de douche 1.5m",
        "خرطوم الدش",
        None,
        Some("613000000002".to_string()),
    );
    seed(
        harness,
        gerant,
        "VIS-435",
        "Boîte de vis agglo 4x35 mm",
        "علبة براغي 4x35",
        Some("613000000003".to_string()),
        None,
    );
    seed(
        harness,
        gerant,
        "VIS-420",
        "Vis métaux 4x20",
        "براغي معدنية",
        None,
        Some("613000000004".to_string()),
    );
    seed(
        harness,
        gerant,
        "ADAT-01",
        "Outil coupe-verre",
        "أداة قطع الزجاج",
        None,
        Some("613000000005".to_string()),
    );
    seed(
        harness,
        gerant,
        "MIFT-01",
        "Clé anglaise 10 pouces",
        "مفتاح إنجليزي 10 بوصة",
        None,
        Some("613000000006".to_string()),
    );
}

fn skus_of(hits: &[(Product, Vec<baraka_domain::catalog::ProductUnit>)]) -> Vec<String> {
    hits.iter()
        .map(|(product, _)| product.sku.as_str().to_string())
        .collect()
}

#[test]
fn infix_token_matches_inside_product_name() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_catalog(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    // "douchette" is a secondary noun, not a name prefix.
    let hits = search.execute(&gerant, "douchette", 20).expect("search");
    assert_eq!(skus_of(&hits), vec!["DOUCH-01".to_string()]);
}

#[test]
fn multi_token_query_enforces_and_conjunction() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_catalog(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    for query in ["vis 4x35", "4x35 vis"] {
        let hits = search.execute(&gerant, query, 20).expect("search");
        let skus = skus_of(&hits);
        assert_eq!(skus, vec!["VIS-435".to_string()], "query {query:?}");
        assert!(
            !skus.iter().any(|sku| sku == "VIS-420"),
            "4x20 must not match 4x35 (query {query:?})"
        );
    }
}

#[test]
fn exact_unit_barcode_ranks_first() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_catalog(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    let hits = search.execute(&gerant, "613000000004", 20).expect("search");
    assert_eq!(
        hits.first().map(|(p, _)| p.sku.as_str()),
        Some("VIS-420"),
        "scanned unit barcode ranks first"
    );
}

#[test]
fn exact_product_barcode_ranks_first() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_catalog(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    let hits = search.execute(&gerant, "613000000003", 20).expect("search");
    assert_eq!(
        hits.first().map(|(p, _)| p.sku.as_str()),
        Some("VIS-435"),
        "product-level barcode ranks first"
    );
}

#[test]
fn arabic_alef_hamza_variants_match() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_catalog(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    // Stored with hamza (أداة), queried bare (اداة). The `_`-wildcard
    // tolerance intentionally over-matches short tokens, so the assertion
    // is rank-based: the exact product leads via the name-prefix rank.
    let hits = search.execute(&gerant, "اداة", 20).expect("search");
    let skus = skus_of(&hits);
    assert!(
        skus.contains(&"ADAT-01".to_string()),
        "hamza variant must match, got {skus:?}"
    );
    assert_eq!(
        hits.first().map(|(p, _)| p.sku.as_str()),
        Some("ADAT-01"),
        "exact product ranks first, got {skus:?}"
    );
}

#[test]
fn arabic_multi_token_variant_tolerance() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_catalog(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    // Stored with hamza (إنجليزي), queried bare.
    let hits = search
        .execute(&gerant, "مفتاح انجليزي", 20)
        .expect("search");
    assert_eq!(skus_of(&hits), vec!["MIFT-01".to_string()]);
}

#[test]
fn empty_query_pagination_and_limits_preserved() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_catalog(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    for query in ["", "   "] {
        let found = search.execute(&gerant, query, 20).expect("discovery");
        assert_eq!(found.len(), 6, "query {query:?}");
    }
    let capped = search.execute(&gerant, "", 2).expect("limit");
    assert_eq!(capped.len(), 2);
    let miss = search
        .execute(&gerant, "zzzinox-introuvable", 20)
        .expect("miss");
    assert!(miss.is_empty());
}
