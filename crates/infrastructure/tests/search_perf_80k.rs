//! 80k-row search performance probe (TASK-BRK-075, measurement only).
//!
//! Seeds 80,000 domain-valid products through `CreateProductUseCase` on a
//! scratch temp DB (`tempfile::tempdir`, never a live database), then
//! reports p50/p95/max over 50 measured iterations (5 warmup) per query
//! against the REAL `SqliteCatalogRepository::search` on one connection
//! (WAL, synchronous=FULL, FK ON per connection §11 defaults).
//!
//! Slow gate: `#[ignore]` so the default `cargo test --workspace` (G3)
//! stays fast. Explicit run:
//! `cargo test -p baraka-infrastructure --test search_perf_80k -- --ignored --nocapture`
//!
//! Machine (dev box, §32): Windows 10 Pro, Intel i5-3450 @ 3.10GHz,
//! 16 GiB RAM (2x8), SATA SSD (ADATA SU650 / TEAM T253). Build -j2
//! (linker constraint, CARGO_BUILD_JOBS=2).

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

/// Deterministic std-only PRNG (xorshift64*; no new dependencies).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n as u64)) as usize
    }
}

const FR_TERMS: &[&str] = &[
    "Vis Placo",
    "Boulon Hex",
    "Robinet Laiton",
    "Cadenas Acier",
    "Cable Cuivre",
    "Peinture Vinylique",
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
];

const AR_TERMS: &[&str] = &[
    "برغي بلاكو",
    "صامولة حديد",
    "حنفية نحاس",
    "قفل فولاذ",
    "كابل نحاس",
    "دهن فينيل",
    "مطرقة يد",
    "مفك براغي",
    "خابور نيلون",
    "منشار حديد",
    "كماشة قاطعة",
    "ميزان ماء",
    "ماستيك سيليكون",
    "قفل باب",
    "مفصلة اينوكس",
    "مقبض درج",
    "انبوب بلاستيك",
    "راكور نحاس",
    "صمام",
    "ديجونكتور",
    "بريز حائط",
    "انتربتور",
    "مصباح ليد",
    "كول كارلاج",
];

const SIZES: &[&str] = &[
    "25", "32", "40mm", "3.5x25", "M8", "2.5mm", "100ml", "5m", "12V", "750ml",
];

fn make_unit(barcode: Option<String>, base: bool, factor: i64, price: i64) -> NewUnitInput {
    NewUnitInput {
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: if base {
            "Pièce".to_string()
        } else {
            "Paquet".to_string()
        },
        factor_micro: factor,
        barcode,
        sale_price_centimes: price,
        purchase_price_centimes: price - price / 10,
        wholesale_price_centimes: None,
        is_base_unit: base,
        is_default_selling: base,
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

/// Nearest-rank percentile over a sorted sample.
fn percentile(sorted_ms: &[f64], pct: f64) -> f64 {
    assert!(!sorted_ms.is_empty());
    let rank = ((pct / 100.0) * (sorted_ms.len() as f64)).ceil() as usize;
    sorted_ms[rank.clamp(1, sorted_ms.len()) - 1]
}

#[test]
#[ignore = "slow: seeds 80k products; run explicitly with --ignored --nocapture"]
fn search_80k_perf_probe() {
    println!("MACHINE: Windows 10 Pro | Intel i5-3450 @ 3.10GHz | 16GiB RAM | SATA SSD | CARGO_BUILD_JOBS=2");
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("perf80k.db");
    let mut conn = open_connection(&db_path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let repo = SqliteCatalogRepository::new(Arc::new(Mutex::new(conn)));
    let gerant = session_gerant();
    let create = CreateProductUseCase::new(&repo, &repo);
    let mut rng = Lcg(0x8E5E_4D21_9C77_2B11);

    const COUNT: usize = 80_000;
    const BARCODE_INDEX: usize = 42_424;
    let mut known_barcode = String::new();
    let seed_started = Instant::now();
    for index in 0..COUNT {
        let fr = FR_TERMS[index % FR_TERMS.len()];
        let ar = AR_TERMS[index % AR_TERMS.len()];
        let size = SIZES[(index / FR_TERMS.len()) % SIZES.len()];
        let has_barcode = rng.below(100) < 40 || index == BARCODE_INDEX;
        let barcode = if has_barcode {
            Some(format!("80K{index:08}"))
        } else {
            None
        };
        if index == BARCODE_INDEX {
            known_barcode = barcode.clone().expect("known barcode set");
        }
        let mut units = vec![make_unit(
            barcode,
            true,
            10_000,
            15_000 + (index as i64 % 9000),
        )];
        if index % 5 == 0 {
            units.push(make_unit(None, false, 120_000, 170_000));
        }
        create
            .execute(
                &gerant,
                CreateProductInput {
                    sku: format!("PERF-{index:05}"),
                    barcode: None,
                    name_ar: format!("{ar} {size} رقم {index}"),
                    name_fr: format!("{fr} {size} n°{index}"),
                    category_id: None,
                    is_kit: false,
                    units,
                },
                "PERF-PROBE",
            )
            .unwrap_or_else(|_| panic!("seed product {index}"));
        if index % 20_000 == 19_999 {
            println!(
                "SEED progress {}/{} elapsed {:?}",
                index + 1,
                COUNT,
                seed_started.elapsed()
            );
        }
    }
    println!(
        "SEED done: {COUNT} products in {:?}",
        seed_started.elapsed()
    );
    assert!(!known_barcode.is_empty());

    let search = SearchCatalogUseCase::new(&repo);
    let queries: [(&str, &str, usize); 6] = [
        ("q1-fr", "vis", 100),
        ("q2-ar", "فيس", 100),
        ("q3-fr", "boulon", 100),
        ("q4-infix", "oul", 100),
        ("q5-barcode", &known_barcode, 20),
        ("q6-multi", "cadenas 40mm", 100),
    ];

    println!("QUERY | IT measured | p50_ms | p95_ms | max_ms | hits(last) | verdict_vs_50ms");
    for (label, query, limit) in queries {
        for _ in 0..5 {
            let _ = search.execute(&gerant, query, limit, false);
        }
        // Correctness asserts (first measured iteration).
        let probe = search
            .execute(&gerant, query, limit, false)
            .unwrap_or_else(|_| panic!("{label} executes"));
        match label {
            "q5-barcode" => {
                assert_eq!(
                    probe.len(),
                    1,
                    "barcode-exact returns exactly the seeded row"
                );
                assert_eq!(probe[0].0.sku.as_str(), "PERF-42424");
            }
            // Accepted ADR-101 delta: `oul` matches `Boulon Hex` only
            // mid-token; FTS prefix semantics return nothing. Pinned here
            // and in catalog_search_fts_parity (infix_fragment test).
            "q4-infix" => {
                assert!(
                    probe.is_empty(),
                    "prefix semantics: no token starts with `oul`"
                );
            }
            "q2-ar" => {
                assert!(!probe.is_empty(), "فيس matches vis-token rows");
                assert!(
                    probe
                        .iter()
                        .any(|(p, _)| p.name_fr.to_lowercase().contains("vis")),
                    "at least one hit carries a vis FR title"
                );
            }
            _ => {
                assert!(!probe.is_empty(), "{label} returns rows");
            }
        }
        let mut samples_ms: Vec<f64> = Vec::with_capacity(50);
        for _ in 0..50 {
            let started = Instant::now();
            let hits = search
                .execute(&gerant, query, limit, false)
                .unwrap_or_else(|_| panic!("{label} executes"));
            samples_ms.push(started.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(hits.len(), probe.len(), "{label} deterministic hit count");
        }
        samples_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50 = percentile(&samples_ms, 50.0);
        let p95 = percentile(&samples_ms, 95.0);
        let max = samples_ms[samples_ms.len() - 1];
        let verdict = if p95 < 50.0 {
            "UNDER-TARGET"
        } else {
            "OVER-TARGET"
        };
        println!(
            "{label} {query} | 50 | {p50:.2} | {p95:.2} | {max:.2} | {} | {verdict}",
            probe.len()
        );
    }

    // EXPLAIN proof on the FTS query path (second connection; the executed
    // shape is JOIN + MATCH + bm25, identical to the repository body).
    let explain_conn = open_connection(&db_path).expect("reopen for explain");
    let mut stmt = explain_conn
        .prepare(
            "EXPLAIN QUERY PLAN SELECT DISTINCT p.id FROM products p \
             JOIN products_fts ON products_fts.product_id = p.id \
             WHERE products_fts MATCH 'norm_fr:\"vis\"*' \
             ORDER BY bm25(products_fts), p.sku LIMIT 100",
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
    println!("EXPLAIN plan rows: {}", plan.len());
    for line in &plan {
        println!("EXPLAIN {line}");
    }
    assert!(
        plan.iter().any(|line| line.contains("products_fts")),
        "expected the FTS index in the plan (no leading-% SCAN)"
    );
    assert!(
        !plan
            .iter()
            .map(|line| line.rsplit('|').next().unwrap_or(""))
            .any(|d| d == "SCAN p" || d.starts_with("SCAN products ")),
        "no base-table scan on the FTS path"
    );
    println!("EXPLAIN verdict: FTS-indexed (SCAN-free); BRK-077 cutover proven.");
}
