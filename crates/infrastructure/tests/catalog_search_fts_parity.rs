//! TASK-BRK-077 parity: FTS5 cutover vs the retired LIKE body.
//!
//! On a fixed deterministic 5,000-row fixture, the new FTS result SET must
//! equal the old LIKE set for 20 sampled prefix-type queries (ranking may
//! differ — both orders are printed for the record). The retired LIKE shape
//! is kept here as a test-only reference (`legacy_like_search`) over the
//! stored norm columns, so future drift fails loudly.
//!
//! Known accepted delta (ADR-101 prefix semantics): leading-infix fragments
//! such as `oul` (matching `boulon` only mid-token) NO LONGER match. That
//! delta is pinned by `infix_fragment_no_longer_matches`, not hidden.
//! A non-dictionary alias row proves the new alias-mirror recall gain.

use std::sync::{Arc, Mutex};

use baraka_application::ports::ProductRepository;
use baraka_application::ports::SessionContext;
use baraka_application::ports::SessionToken;
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, NewUnitInput, SearchCatalogUseCase,
};
use baraka_domain::catalog::search_normalization::{normalize_for_search, tokenize_query};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use rusqlite::Connection;

const TERMINAL: &str = "PARITY-PROBE";

const FR_BASES: &[&str] = &[
    "Vis Placo",
    "Boulon Hex",
    "Robinet Laiton",
    "Cadenas Acier",
    "Cable Cuivre",
    "Marteau Manche",
    "Tournevis Plat",
    "Cheville Nylon",
    "Scie Metaux",
    "Pince Coupante",
    "Niveau Alu",
    "Mastic Silicone",
    "Serrure Porte",
    "Charniere Inox",
    "Poignee Tiroir",
    "Tuyau PVC",
    "Raccord Laiton",
    "Vanne Arret",
    "Disjoncteur",
    "Prise Murale",
    "Interrupteur",
    "Ampoule LED",
    "Colle Carrelage",
    "Lame Scie",
    "Groupe Electrogene",
    "Perceuse Percussion",
    "Meuleuse Disque",
    "Soudeuse Inverter",
    "Compresseur Air",
    "Echelle Aluminium",
    "Brouette Renforcee",
    "Pelle Ronde",
    "Rateau Jardin",
    "Tuyau Arrosage",
    "Vanne Papillon",
    "Compteur Eau",
    "Robinet Flotteur",
    "Chasse Eau",
    "Lavabo Ceramique",
    "Mitigeur Douche",
];

const AR_BASES: &[&str] = &[
    "برغي بلاكو",
    "مسمار سداسي",
    "صنبور نحاس",
    "قفل فولاذ",
    "كابل نحاس",
    "مطرقة يد",
    "مفك براغي",
    "خابور نيلون",
    "منشار معادن",
    "كماشة قاطعة",
    "ميزان ألمنيوم",
    "معجون سيليكون",
    "قفل باب",
    "مفصلة ستانلس",
    "مقبض درج",
    "أنبوب بلاستيك",
    "وصلة نحاس",
    "صمام إيقاف",
    "قاطع كهربائي",
    "مقبس حائط",
    "مفتاح كهربائي",
    "مصباح ليد",
    "غراء بلاط",
    "شفرة منشار",
    "مولد كهربائي",
    "مثقاب طرق",
    "جلاخة قرص",
    "لحامة انفرتر",
    "ضاغط هواء",
    "سلم ألمنيوم",
    "عربة مقواة",
    "مجرفة دائرية",
    "مشط حديقة",
    "خرطوم سقي",
    "صمام فراشة",
    "عداد ماء",
    "صنبور عوامة",
    "سيفون ماء",
    "مغسلة سيراميك",
    "خلاط دش",
];

const SIZES: &[&str] = &[
    "25", "32", "40mm", "3.5x25", "M8", "2.5mm", "100ml", "5m", "12V", "750ml",
];

const COUNT: usize = 5_000;

struct Harness {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
    repo: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("parity.db");
    let mut conn = open_connection(&path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    Harness {
        _dir: dir,
        path,
        repo: SqliteCatalogRepository::new(Arc::new(Mutex::new(conn))),
    }
}

fn session_gerant() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role: Role::Gerant,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn make_unit(barcode: Option<String>) -> NewUnitInput {
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

/// Deterministic 5k fixture via the real create path (triggers sync FTS).
/// Returns the product ids in seed order.
fn seed_fixture(harness: &Harness, gerant: &SessionContext) -> Vec<String> {
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let mut ids = Vec::with_capacity(COUNT);
    for index in 0..COUNT {
        let fr = FR_BASES[index % FR_BASES.len()];
        let ar = AR_BASES[index % AR_BASES.len()];
        let size = SIZES[(index / FR_BASES.len()) % SIZES.len()];
        let barcode = if index % 3 == 0 {
            Some(format!("770000{index:07}"))
        } else {
            None
        };
        let id = create
            .execute(
                gerant,
                CreateProductInput {
                    sku: format!("PAR-{index:05}"),
                    barcode: if index % 7 == 0 {
                        Some(format!("771000{index:07}"))
                    } else {
                        None
                    },
                    name_ar: format!("{ar} {size} رقم {index}"),
                    name_fr: format!("{fr} {size} n{index}"),
                    category_id: None,
                    is_kit: false,
                    units: vec![make_unit(barcode)],
                },
                TERMINAL,
            )
            .unwrap_or_else(|_| panic!("seed product {index}"));
        ids.push(id.to_string());
        if index % 25 == 24 {
            harness
                .repo
                .set_product_active(id, false)
                .expect("deactivate every 25th");
        }
    }
    // Manual non-dictionary aliases on a few rows (alias-mirror coverage).
    {
        let conn = open_connection(&harness.path).expect("reopen for aliases");
        for (slot, alias) in ["مولد احتياطي", "دريل", "compresseur-secours"]
            .iter()
            .enumerate()
        {
            let pid = &ids[slot * 100];
            conn.execute(
                "INSERT OR IGNORE INTO product_aliases (product_id, alias_norm) VALUES (?1, ?2)",
                rusqlite::params![pid, normalize_for_search(alias)],
            )
            .expect("insert alias");
        }
    }
    ids
}

fn like_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Register the same deterministic `baraka_search_norm(text)` scalar the
/// engine registers on every connection (test-only mirror of
/// `register_search_norm`): the retired LIKE body normalized in SQL, so the
/// reference must too. Pure domain logic.
fn register_search_norm_for_reference(conn: &Connection) {
    conn.create_scalar_function(
        "baraka_search_norm",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8
            | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            use rusqlite::types::ValueRef;
            let value = ctx.get_raw(0);
            if value == ValueRef::Null {
                return Ok(String::new());
            }
            let text = value.as_str()?;
            Ok(normalize_for_search(text))
        },
    )
    .expect("register norm scalar");
}

/// Test-only copy of the retired BRK-073 LIKE body (SQL-side normalization,
/// exactly as shipped). Returns ordered ids.
fn legacy_like_search(
    conn: &Connection,
    query: &str,
    limit: usize,
    include_inactive: bool,
) -> Vec<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let groups = tokenize_query(trimmed);
    if groups.is_empty() {
        return Vec::new();
    }
    let mut sql = String::from("SELECT DISTINCT p.id FROM products p WHERE ");
    let mut patterns: Vec<String> = Vec::new();
    for (ti, group) in groups.iter().enumerate() {
        if ti > 0 {
            sql.push_str(" AND ");
        }
        sql.push('(');
        for (vi, variant) in group.iter().enumerate() {
            if vi > 0 {
                sql.push_str(" OR ");
            }
            sql.push_str(
                "(baraka_search_norm(p.name_fr) LIKE ? ESCAPE '\\' \
                 OR baraka_search_norm(p.name_ar) LIKE ? ESCAPE '\\' \
                 OR baraka_search_norm(p.sku) LIKE ? ESCAPE '\\' \
                 OR baraka_search_norm(p.barcode) LIKE ? ESCAPE '\\' \
                 OR EXISTS (SELECT 1 FROM product_units pu WHERE pu.product_id = p.id AND baraka_search_norm(pu.barcode) LIKE ? ESCAPE '\\'))",
            );
            let pattern = format!("%{}%", like_escape(variant));
            for _ in 0..5 {
                patterns.push(pattern.clone());
            }
        }
        sql.push(')');
    }
    if !include_inactive {
        sql.push_str(" AND p.is_active = 1");
    }
    sql.push_str(" ORDER BY p.id LIMIT ?");
    patterns.push(limit.to_string());
    let mut stmt = conn.prepare(&sql).expect("legacy prepare");
    stmt.query_map(rusqlite::params_from_iter(patterns), |row| {
        row.get::<_, String>(0)
    })
    .expect("legacy rows")
    .collect::<Result<Vec<_>, _>>()
    .expect("legacy collect")
}

fn new_search_ids(
    harness: &Harness,
    gerant: &SessionContext,
    query: &str,
    limit: usize,
    include_inactive: bool,
) -> Vec<String> {
    let search = SearchCatalogUseCase::new(&harness.repo);
    search
        .execute(gerant, query, limit, include_inactive)
        .unwrap_or_else(|_| panic!("new search executes for {query:?}"))
        .iter()
        .map(|(p, _)| p.id.to_string())
        .collect()
}

const PARITY_QUERIES: &[&str] = &[
    "boulon",
    "cadenas 40mm",
    "marteau",
    "مطرقة",
    "tournevis plat",
    "cable 2.5mm",
    "lame scie",
    "PAR-001",
    "colle carrelage",
    "خرطوم",
    "mastic silicone",
    "CÂBLE",
    "robinet laiton",
    "disjoncteur",
    "interrupteur",
    "perceuse percussion",
    "مصباح",
    "cheville nylon",
    "vanne arret",
    "tuyau pvc",
];

#[test]
fn fts_parity_set_equals_legacy_for_sampled_queries() {
    let harness = setup();
    let gerant = session_gerant();
    let _ids = seed_fixture(&harness, &gerant);
    let conn = open_connection(&harness.path).expect("reopen for legacy");
    register_search_norm_for_reference(&conn);
    assert_eq!(PARITY_QUERIES.len(), 20, "parity sample is exactly 20");
    for query in PARITY_QUERIES {
        for include_inactive in [false, true] {
            let mut new_ids = new_search_ids(&harness, &gerant, query, 500, include_inactive);
            let mut legacy_ids = legacy_like_search(&conn, query, 500, include_inactive);
            new_ids.sort();
            legacy_ids.sort();
            println!(
                "PARITY q={query:?} inactive={include_inactive} n={} new_top={:?} legacy_top={:?}",
                new_ids.len(),
                &new_ids[..new_ids.len().min(3)],
                &legacy_ids[..legacy_ids.len().min(3)],
            );
            assert!(
                !new_ids.is_empty(),
                "query {query:?} must hit (non-vacuous parity)"
            );
            assert_eq!(
                new_ids, legacy_ids,
                "FTS set equals LIKE set for {query:?} (inactive={include_inactive})"
            );
        }
    }
}

#[test]
fn barcode_exact_returns_exactly_one_row() {
    let harness = setup();
    let gerant = session_gerant();
    let _ids = seed_fixture(&harness, &gerant);
    // Index 7: product-level barcode 7710000000007 (7 % 7 == 0, 7 % 3 != 0).
    let hits = new_search_ids(&harness, &gerant, "7710000000007", 20, false);
    assert_eq!(hits.len(), 1, "barcode-exact returns exactly 1 row");
}

#[test]
fn infix_fragment_no_longer_matches() {
    // Accepted ADR-101 delta: `oul` matches `boulon` only mid-token, and FTS
    // prefix semantics no longer return it. Pinned, not hidden.
    let harness = setup();
    let gerant = session_gerant();
    let _ids = seed_fixture(&harness, &gerant);
    let hits = new_search_ids(&harness, &gerant, "oul", 100, false);
    assert!(
        hits.is_empty(),
        "prefix semantics: no token starts with `oul`, got {hits:?}"
    );
}

#[test]
fn phonetic_query_divergence_is_infix_only() {
    // `vis` / `فيس` additionally match `tournevis` mid-token under LIKE.
    // The FTS set must be a PROPER subset of the legacy set (recall lost =
    // infix only, never a missing prefix row), and every legacy-only row
    // must be a `tournevis` row. Characterizes the accepted delta exactly.
    let harness = setup();
    let gerant = session_gerant();
    let ids = seed_fixture(&harness, &gerant);
    let conn = open_connection(&harness.path).expect("reopen for legacy");
    register_search_norm_for_reference(&conn);
    let tournevis: Vec<String> = ids
        .iter()
        .enumerate()
        .filter(|(i, _)| i % FR_BASES.len() == 6)
        .map(|(_, id)| id.clone())
        .collect();
    assert!(!tournevis.is_empty(), "fixture has tournevis rows");
    for query in ["vis", "فيس"] {
        let mut new_ids = new_search_ids(&harness, &gerant, query, 500, false);
        let mut legacy_ids = legacy_like_search(&conn, query, 500, false);
        new_ids.sort();
        legacy_ids.sort();
        println!(
            "DIVERGENCE q={query:?} fts={} legacy={}",
            new_ids.len(),
            legacy_ids.len()
        );
        assert!(!new_ids.is_empty(), "prefix rows still found for {query:?}");
        assert!(
            legacy_ids.len() > new_ids.len(),
            "legacy strictly wider for {query:?} (infix recall)"
        );
        assert!(
            new_ids.iter().all(|id| legacy_ids.contains(id)),
            "every FTS row is a legacy row for {query:?}"
        );
        let only_legacy: Vec<&String> = legacy_ids
            .iter()
            .filter(|id| !new_ids.contains(id))
            .collect();
        assert!(
            only_legacy.iter().all(|id| tournevis.contains(id)),
            "legacy-only rows are exactly the tournevis rows for {query:?}, got {only_legacy:?}"
        );
    }
}

#[test]
fn non_dictionary_alias_gains_recall() {
    // Recall the LIKE body could never provide: a manual alias outside the
    // phonetic dictionary is found through the alias mirror.
    let harness = setup();
    let gerant = session_gerant();
    let _ids = seed_fixture(&harness, &gerant);
    let hits = new_search_ids(&harness, &gerant, "دريل", 20, false);
    assert_eq!(hits.len(), 1, "manual alias `دريل` resolves to its row");
}

#[test]
fn hostile_tokens_never_error() {
    let harness = setup();
    let gerant = session_gerant();
    let _ids = seed_fixture(&harness, &gerant);
    // Lone `"`, CJK passthrough, emoji, empty — must execute, never error.
    for query in ["\"", "钻头", "🔧", "   ", "\"* OR 1=1 --", ":::"] {
        let hits = new_search_ids(&harness, &gerant, query, 20, false);
        println!("HOSTILE q={query:?} n={}", hits.len());
    }
}

#[test]
fn fts_query_plan_uses_index() {
    // EXPLAIN proof: the FTS leg never falls back to a base-table scan.
    let harness = setup();
    let gerant = session_gerant();
    let _ids = seed_fixture(&harness, &gerant);
    let conn = open_connection(&harness.path).expect("reopen for explain");
    let mut stmt = conn
        .prepare(
            "EXPLAIN QUERY PLAN SELECT DISTINCT p.id FROM products p \
             JOIN products_fts ON products_fts.product_id = p.id \
             WHERE products_fts MATCH 'norm_fr:\"vis\"*' ORDER BY bm25(products_fts), p.sku LIMIT 100",
        )
        .expect("explain prepare");
    let plan: Vec<String> = stmt
        .query_map([], |row| {
            Ok(format!(
                "{}|{}|{}|{}",
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?
            ))
        })
        .expect("explain rows")
        .collect::<Result<_, _>>()
        .expect("collect plan");
    for line in &plan {
        println!("EXPLAIN {line}");
    }
    assert!(
        plan.iter().any(|line| line.contains("products_fts")),
        "plan touches the FTS index"
    );
    // Detail-field comparison: `SCAN products_fts ...` is the FTS index
    // itself — only a bare base-table scan (`SCAN p`, `SCAN products ...`)
    // proves index-less execution.
    let details: Vec<&str> = plan
        .iter()
        .map(|line| line.rsplit('|').next().unwrap_or(""))
        .collect();
    assert!(
        !details
            .iter()
            .any(|d| *d == "SCAN p" || d.starts_with("SCAN products ")),
        "no base-table scan, got {details:?}"
    );
}

#[test]
fn fts_stays_consistent_across_writes() {
    // Spec 4: insert/update/delete through the repository, then verify.
    let harness = setup();
    let gerant = session_gerant();
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let id = create
        .execute(
            &gerant,
            CreateProductInput {
                sku: "WRITE-001".to_string(),
                barcode: None,
                name_ar: "صنف اختبار".to_string(),
                name_fr: "Produit Test Ecriture".to_string(),
                category_id: None,
                is_kit: false,
                units: vec![make_unit(Some("779999000001".to_string()))],
            },
            TERMINAL,
        )
        .expect("insert");
    let hits = new_search_ids(&harness, &gerant, "ecriture", 20, false);
    assert_eq!(hits.len(), 1, "inserted row is FTS-visible");
    let (mut product, units) = harness
        .repo
        .find_by_id(id)
        .expect("fetch")
        .expect("inserted row exists");
    product.name_fr = "Produit Test Modifie".to_string();
    harness
        .repo
        .update_product(&product, &units)
        .expect("update");
    let hits = new_search_ids(&harness, &gerant, "modifie", 20, false);
    assert_eq!(hits.len(), 1, "updated name is FTS-visible");
    harness.repo.hard_delete_product(id).expect("delete");
    let hits = new_search_ids(&harness, &gerant, "modifie", 20, false);
    assert!(hits.is_empty(), "deleted row leaves the index");
    harness
        .repo
        .verify_fts_consistency()
        .expect("fts consistent after write cycle");
}
