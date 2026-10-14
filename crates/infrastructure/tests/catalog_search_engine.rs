//! Quincaillerie hybrid search engine tests (TASK-BRK-066).
//!
//! End-to-end coverage of the multi-token phonetic search over the real
//! SQLite repository: disordered tokens, Arabic-to-French phonetic lookup,
//! accent resilience, barcode fast path, discovery rows, and a latency
//! probe over 100 products.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use baraka_application::ports::SessionContext;
use baraka_application::ports::SessionToken;
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, NewUnitInput, SearchCatalogUseCase,
};
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
    let mut conn = open_connection(&dir.path().join("search.db")).expect("open");
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
    barcode: Option<String>,
) {
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    create
        .execute(
            gerant,
            CreateProductInput {
                sku: sku.to_string(),
                barcode: None,
                name_ar: name_ar.to_string(),
                name_fr: name_fr.to_string(),
                category_id: None,
                is_kit: false,
                units: vec![unit("قطعة", "Pièce", barcode)],
            },
            TEST_TERMINAL,
        )
        .expect("seed product");
}

fn seed_hardware(harness: &Harness, gerant: &SessionContext) {
    seed(
        harness,
        gerant,
        "VIS-PLA-25",
        "Vis Placo 3.5x25",
        "برغي بلاكو",
        Some("6111250090011".to_string()),
    );
    seed(
        harness,
        gerant,
        "COU-PVC-32",
        "Coude PVC 32",
        "كود بلاستيك",
        None,
    );
    seed(
        harness,
        gerant,
        "CAB-2.5",
        "Câble 2.5mm",
        "كابل كهرباء",
        None,
    );
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

#[test]
fn disordered_multi_token_search_matches() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_hardware(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    for query in ["25 vis", "vis 25", "3.5 placo vis"] {
        let hits = search.execute(&gerant, query, 20).expect("search");
        assert_eq!(
            skus_of(&hits),
            vec!["VIS-PLA-25".to_string()],
            "query {query:?}"
        );
    }
}

#[test]
fn arabic_phonetic_search_finds_french_titles() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_hardware(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    let vis = search
        .execute(&gerant, "فيس بلاكو", 20)
        .expect("phonetic vis");
    println!("DEMO search \"فيس بلاكو\" -> {:?}", skus_of(&vis));
    assert_eq!(skus_of(&vis), vec!["VIS-PLA-25".to_string()]);
    let coude = search
        .execute(&gerant, "كود 32", 20)
        .expect("phonetic coude");
    println!("DEMO search \"كود 32\" -> {:?}", skus_of(&coude));
    assert_eq!(skus_of(&coude), vec!["COU-PVC-32".to_string()]);
}

#[test]
fn accent_and_case_variants_match() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_hardware(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    for query in ["cable", "CÂBLE", "câble 2.5"] {
        let hits = search.execute(&gerant, query, 20).expect("search");
        assert_eq!(
            skus_of(&hits),
            vec!["CAB-2.5".to_string()],
            "query {query:?}"
        );
    }
}

#[test]
fn exact_sku_ranks_first() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_hardware(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    let hits = search.execute(&gerant, "VIS-PLA-25", 20).expect("search");
    assert_eq!(
        hits.first().map(|(p, _)| p.sku.as_str()),
        Some("VIS-PLA-25")
    );
}

#[test]
fn barcode_fast_path_and_discovery_preserved() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    seed_hardware(&harness, &gerant);
    let search = SearchCatalogUseCase::new(&harness.repo);
    let scanned = search
        .execute(&gerant, "6111250090011", 20)
        .expect("barcode");
    assert_eq!(skus_of(&scanned), vec!["VIS-PLA-25".to_string()]);
    assert!(search
        .execute(&gerant, "0000000000000", 20)
        .expect("miss")
        .is_empty());
    for query in ["", "   "] {
        let found = search.execute(&gerant, query, 20).expect("discovery");
        assert_eq!(found.len(), 3);
    }
}

#[test]
fn hundred_product_search_returns_promptly() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    for index in 0..100 {
        seed(
            &harness,
            &gerant,
            &format!("PERF-{index:03}"),
            &format!("Vis Placo Série {index}"),
            "برغي",
            None,
        );
    }
    let search = SearchCatalogUseCase::new(&harness.repo);
    let started = Instant::now();
    let hits = search.execute(&gerant, "فيس 25", 100).expect("search");
    let elapsed = started.elapsed();
    println!("PERF 100-product hybrid search took {:?}", elapsed);
    assert!(!hits.is_empty());
    assert!(
        elapsed.as_millis() < 100,
        "search took {elapsed:?}, budget 100ms"
    );
}
