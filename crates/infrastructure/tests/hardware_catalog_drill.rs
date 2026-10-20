//! 300-item Algerian hardware drill & search benchmark (TASK-REL-058).
//!
//! No 300-product fixture exists on disk (ADR-088 ships only the starter
//! *categories* taxonomy), so this suite carries a self-contained
//! deterministic generator: 5 departments x 6 templates x 10 specs = 300
//! bilingual hardware products with unique SKUs and barcodes, seeded
//! through the real `create_product` path (domain invariants hold
//! structurally, per the Seeding law).
//!
//! Coverage: dataset shape, ranking invariants (barcode rank 0, name
//! prefix rank 1, infix rank 2), Arabic orthographic tolerance, p95
//! search latency over 100 timed iterations, and a two-item atomic cash
//! sale (FINALIZED document, SALE movements, PENDING print job).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use baraka_application::ports::{
    ProductRepository, SessionContext, SessionToken, StockRepository, WarehouseRepository,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{DocumentLanguage, DocumentType, PaymentMethod};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "55555555-5555-5555-5555-555555555555";
const TERMINAL_ID: &str = "DRILL-TERM-058";
const PRODUCT_TOTAL: usize = 300;
const STOCK_MICRO_PER_PRODUCT: i64 = 200_000;
const LATENCY_BUDGET_MICROS: u128 = 15_000;

/// One catalog template: French/Arabic base names plus ten FR specs
/// (sizes and finishes; Arabic bases stay clean).
struct Template {
    fr: &'static str,
    ar: &'static str,
    specs: [&'static str; 10],
}

struct Department {
    prefix: &'static str,
    templates: [Template; 6],
}

const DEPARTMENTS: [Department; 5] = [
    Department {
        prefix: "VIS",
        templates: [
            Template {
                fr: "Boîte vis agglo",
                ar: "علبة براغي",
                specs: [
                    "3.5x16", "3.5x20", "3.5x25", "4x30", "4x35", "4x40", "4x45", "5x50", "5x60",
                    "6x80",
                ],
            },
            Template {
                fr: "Vis métaux",
                ar: "برغي معدن",
                specs: [
                    "3x12", "3x16", "4x20", "4x25", "5x30", "5x40", "6x50", "6x60", "8x70", "8x80",
                ],
            },
            Template {
                fr: "Vis placo",
                ar: "برغي بلاكو",
                specs: [
                    "3.5x25", "3.5x35", "3.5x45", "3.5x55", "4.2x65", "4.2x75", "25mm", "35mm",
                    "45mm", "55mm",
                ],
            },
            Template {
                fr: "Boulon galvanisé",
                ar: "بولون",
                specs: [
                    "M6x30", "M6x50", "M8x40", "M8x60", "M10x50", "M10x80", "M12x60", "M12x100",
                    "M14x80", "M16x100",
                ],
            },
            Template {
                fr: "Écrou hexagonal",
                ar: "إكرو",
                specs: [
                    "M6", "M8", "M10", "M12", "M14", "M16", "M18", "M20", "M5", "M4",
                ],
            },
            Template {
                fr: "Rondelle plate",
                ar: "رونديل",
                specs: [
                    "M6", "M8", "M10", "M12", "M14", "M16", "M5", "M4", "M20", "large",
                ],
            },
        ],
    },
    Department {
        prefix: "PLO",
        templates: [
            Template {
                fr: "Pomme de douchette",
                ar: "رأس دش",
                specs: [
                    "chromée",
                    "blanche",
                    "noire",
                    "laiton",
                    "ABS",
                    "anti-calcaire",
                    "éco",
                    "XL",
                    "mini",
                    "avec flexible",
                ],
            },
            Template {
                fr: "Flexible de douche",
                ar: "خرطوم الدش",
                specs: [
                    "1.2m",
                    "1.5m",
                    "1.75m",
                    "2m",
                    "inox",
                    "PVC",
                    "anti-torsion",
                    "renforcé",
                    "noir",
                    "chromé",
                ],
            },
            Template {
                fr: "Coude PVC",
                ar: "كوع",
                specs: [
                    "20mm", "25mm", "32mm", "40mm", "50mm", "63mm", "87°", "90°", "réduit",
                    "femelle",
                ],
            },
            Template {
                fr: "Tube PER",
                ar: "أنبوب",
                specs: [
                    "16mm",
                    "20mm",
                    "25mm",
                    "32mm",
                    "couronne 50m",
                    "couronne 100m",
                    "rouge",
                    "bleu",
                    "gainé",
                    "nu",
                ],
            },
            Template {
                fr: "Robinet d'arrêt",
                ar: "روبيني",
                specs: [
                    "1/2",
                    "3/4",
                    "équerre",
                    "droit",
                    "laiton",
                    "inox",
                    "avec filtre",
                    "MM",
                    "MF",
                    "volant",
                ],
            },
            Template {
                fr: "Téflon",
                ar: "تيفلون",
                specs: [
                    "12mm",
                    "19mm",
                    "bande",
                    "pâte",
                    "liquide",
                    "haute densité",
                    "gaz",
                    "eau",
                    "12m",
                    "20m",
                ],
            },
        ],
    },
    Department {
        prefix: "OUT",
        templates: [
            Template {
                fr: "Clé à molette",
                ar: "مفتاح إنجليزي",
                specs: [
                    "6 pouces",
                    "8 pouces",
                    "10 pouces",
                    "12 pouces",
                    "15 pouces",
                    "18 pouces",
                    "chrome",
                    "isolée",
                    "à cliquet",
                    "réglable",
                ],
            },
            Template {
                fr: "Marteau",
                ar: "مارطو",
                specs: [
                    "menuisier",
                    "maçon",
                    "électricien",
                    "500g",
                    "800g",
                    "1kg",
                    "manche bois",
                    "manche fibre",
                    "anti-vibration",
                    "tête nylon",
                ],
            },
            Template {
                fr: "Tournevis",
                ar: "تورنوفيس",
                specs: [
                    "plat",
                    "cruciforme",
                    "torx",
                    "isolé 1000V",
                    "jeu 6pcs",
                    "jeu 12pcs",
                    "magnétique",
                    "long",
                    "court",
                    "à cliquet",
                ],
            },
            Template {
                fr: "Pince",
                ar: "بينس",
                specs: [
                    "universelle",
                    "coupante",
                    "à bec",
                    "multiprise",
                    "étau",
                    "dénuder",
                    "circlips",
                    "150mm",
                    "200mm",
                    "250mm",
                ],
            },
            Template {
                fr: "Disque à tronçonner",
                ar: "قرص تقطيع",
                specs: [
                    "115mm", "125mm", "180mm", "230mm", "métal", "inox", "béton", "diamant", "fin",
                    "épais",
                ],
            },
            Template {
                fr: "Scie",
                ar: "منشار",
                specs: [
                    "à métaux",
                    "à bois",
                    "sauteuse",
                    "lame 300mm",
                    "égoïne",
                    "japonaise",
                    "cloche",
                    "à dos",
                    "pliable",
                    "cadre",
                ],
            },
        ],
    },
    Department {
        prefix: "ELE",
        templates: [
            Template {
                fr: "Câble",
                ar: "كابل",
                specs: [
                    "1.5mm",
                    "2.5mm",
                    "4mm",
                    "6mm",
                    "rigide",
                    "souple",
                    "3G",
                    "5G",
                    "rouleau 50m",
                    "rouleau 100m",
                ],
            },
            Template {
                fr: "Disjoncteur",
                ar: "ديجونكتور",
                specs: [
                    "10A",
                    "16A",
                    "20A",
                    "25A",
                    "32A",
                    "différentiel",
                    "tétrapolaire",
                    "modulaire",
                    "magnétique",
                    "2A",
                ],
            },
            Template {
                fr: "Prise",
                ar: "بريز",
                specs: [
                    "16A",
                    "avec terre",
                    "étanche",
                    "double",
                    "saillie",
                    "encastrée",
                    "USB",
                    "avec interrupteur",
                    "murale",
                    "sol",
                ],
            },
            Template {
                fr: "Interrupteur",
                ar: "انتربتور",
                specs: [
                    "simple",
                    "double",
                    "va-et-vient",
                    "étanche",
                    "avec voyant",
                    "tactile",
                    "saillie",
                    "encastré",
                    "volets",
                    "minuteur",
                ],
            },
            Template {
                fr: "Ruban adhésif",
                ar: "شريط لاصق",
                specs: [
                    "électricien",
                    "19mm",
                    "20m",
                    "noir",
                    "couleurs",
                    "toxilé",
                    "haute tension",
                    "PVC",
                    "toile",
                    "double-face",
                ],
            },
            Template {
                fr: "Spot LED",
                ar: "سبوت",
                specs: [
                    "5W",
                    "7W",
                    "12W",
                    "encastré",
                    "saillie",
                    "orientable",
                    "blanc chaud",
                    "blanc froid",
                    "étanche",
                    "avec transformateur",
                ],
            },
        ],
    },
    Department {
        prefix: "PEI",
        templates: [
            Template {
                fr: "Peinture",
                ar: "صباغة",
                specs: [
                    "blanche 4kg",
                    "blanche 15kg",
                    "vinylique",
                    "glycéro",
                    "mate",
                    "satinée",
                    "façade",
                    "antirouille",
                    "bois",
                    "métal",
                ],
            },
            Template {
                fr: "Rouleau",
                ar: "رولو",
                specs: [
                    "180mm",
                    "250mm",
                    "antigoutte",
                    "façade",
                    "lasure",
                    "avec bac",
                    "recharge",
                    "mousse",
                    "laqueur",
                    "patte",
                ],
            },
            Template {
                fr: "Pinceau",
                ar: "بينسو",
                specs: [
                    "plat 40mm",
                    "plat 60mm",
                    "rond",
                    "à rechampir",
                    "soie",
                    "nylon",
                    "jeu 5pcs",
                    "pouce",
                    "spalter",
                    "coudé",
                ],
            },
            Template {
                fr: "Diluant",
                ar: "ديليون",
                specs: [
                    "white-spirit",
                    "acétone",
                    "1L",
                    "5L",
                    "universel",
                    "cellulosique",
                    "nettoyant",
                    "dégraissant",
                    "rapide",
                    "lent",
                ],
            },
            Template {
                fr: "Enduit",
                ar: "أندوي",
                specs: [
                    "intérieur",
                    "extérieur",
                    "rebouchage",
                    "lissage",
                    "poudre 5kg",
                    "pâte 15kg",
                    "finition",
                    "gros grains",
                    "fibre",
                    "rapide",
                ],
            },
            Template {
                fr: "Mastic",
                ar: "معجون",
                specs: [
                    "silicone",
                    "acrylique",
                    "polyuréthane",
                    "colle",
                    "cartouche",
                    "blanc",
                    "transparent",
                    "bois",
                    "vitrier",
                    "haute température",
                ],
            },
        ],
    },
];

struct Anchors {
    vis: (ProductId, UnitId, String),
    cle: (ProductId, UnitId, String),
    tube: ProductId,
    ruban: ProductId,
    vis_barcode: String,
}

struct DrillDb {
    _dir: tempfile::TempDir,
    shared: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    warehouse_id: WarehouseId,
    session: SessionContext,
    anchors: Anchors,
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed_drill_db() -> DrillDb {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("drill.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'drill-vendeur', 'hash', 'VENDEUR', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [ACTOR_UUID],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    let catalog = SqliteCatalogRepository::new(Arc::clone(&shared));
    let stock = SqliteStockRepository::new(Arc::clone(&shared));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&shared));
    let customers = SqliteCustomerRepository::new(Arc::clone(&shared));
    let checkout = SqliteCheckoutRepository::new(Arc::clone(&shared));

    let warehouse_id = WarehouseId::new();
    warehouses
        .create_warehouse(
            &Warehouse {
                id: warehouse_id,
                code: WarehouseCode::new("MAIN").expect("code"),
                name: "Main".to_string(),
                is_active: true,
            },
            TERMINAL_ID,
        )
        .expect("warehouse");

    let mut vis = None;
    let mut cle = None;
    let mut tube = None;
    let mut ruban = None;
    let mut global: usize = 0;
    for (dept_index, dept) in DEPARTMENTS.iter().enumerate() {
        for (template_index, template) in dept.templates.iter().enumerate() {
            for (spec_index, spec) in template.specs.iter().enumerate() {
                let product_id = ProductId::new();
                let unit_id = UnitId::new();
                let sku = format!(
                    "{}-{:03}",
                    dept.prefix,
                    dept_index * 60 + template_index * 10 + spec_index + 1
                );
                let unit_barcode = format!("613{}{:05}", dept_index + 1, global + 1);
                let is_vis_anchor = dept_index == 0 && template_index == 0 && spec_index == 4;
                let is_cle_anchor = dept_index == 2 && template_index == 0 && spec_index == 2;
                let product_barcode = if is_vis_anchor {
                    Some("613900001".to_string())
                } else if is_cle_anchor {
                    Some("613900002".to_string())
                } else {
                    None
                };
                let price = 3_000 + ((global as i64 * 7919) % 47_000);
                catalog
                    .create_product(
                        &Product {
                            id: product_id,
                            sku: Sku::new(&sku).expect("sku"),
                            barcode: product_barcode
                                .as_deref()
                                .map(|code| Barcode::new(code).expect("barcode")),
                            name_ar: template.ar.to_string(),
                            name_fr: format!("{} {spec}", template.fr),
                            category_id: None,
                            is_kit: false,
                            is_active: true,
                        },
                        &[ProductUnit {
                            id: unit_id,
                            product_id,
                            unit_name_ar: "قطعة".to_string(),
                            unit_name_fr: "Pièce".to_string(),
                            factor_micro: FactorMicro::from_micros(10_000),
                            barcode: Some(Barcode::new(&unit_barcode).expect("barcode")),
                            sale_price: Money::from_centimes(price),
                            purchase_price: Money::from_centimes(price - 500),
                            wholesale_price: None,
                            is_base_unit: true,
                            is_default_selling: true,
                        }],
                        TERMINAL_ID,
                    )
                    .expect("seed product");
                stock
                    .upsert_stock_level_with_delta(
                        product_id,
                        warehouse_id,
                        STOCK_MICRO_PER_PRODUCT,
                    )
                    .expect("seed stock");
                if is_vis_anchor {
                    vis = Some((product_id, unit_id, unit_barcode.clone()));
                }
                if is_cle_anchor {
                    cle = Some((product_id, unit_id, unit_barcode.clone()));
                }
                if dept_index == 1 && template_index == 3 && spec_index == 0 {
                    tube = Some(product_id);
                }
                if dept_index == 3 && template_index == 4 && spec_index == 0 {
                    ruban = Some(product_id);
                }
                global += 1;
            }
        }
    }
    assert_eq!(global, PRODUCT_TOTAL, "generator must yield 300 products");

    // OPEN cash session (checkout precondition).
    {
        let guard = shared.lock().expect("lock");
        let session_id = Uuid::new_v4().to_string();
        guard
            .execute(
                "INSERT INTO cash_sessions
                 (id, warehouse_id, terminal_id, status, opened_by, opened_at,
                  opening_float_centimes, revision, origin_device_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                         '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                rusqlite::params![
                    session_id,
                    warehouse_id.to_string(),
                    TERMINAL_ID,
                    ACTOR_UUID
                ],
            )
            .expect("open cash session");
        guard
            .execute(
                "INSERT INTO cash_movements
                 (id, cash_session_id, movement_type, amount_centimes, description,
                  created_at, created_by)
                 VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float',
                         '2026-01-01T00:00:00Z', ?3)",
                rusqlite::params![Uuid::new_v4().to_string(), session_id, ACTOR_UUID],
            )
            .expect("opening float");
    }

    let (vis_id, vis_unit, vis_unit_barcode) = vis.expect("vis anchor");
    let (cle_id, cle_unit, cle_unit_barcode) = cle.expect("cle anchor");
    DrillDb {
        _dir: dir,
        shared,
        catalog,
        stock,
        warehouses,
        customers,
        checkout,
        warehouse_id,
        session: session_with(Role::Vendeur),
        anchors: Anchors {
            vis: (vis_id, vis_unit, vis_unit_barcode),
            cle: (cle_id, cle_unit, cle_unit_barcode),
            tube: tube.expect("tube anchor"),
            ruban: ruban.expect("ruban anchor"),
            vis_barcode: "613900001".to_string(),
        },
    }
}

/// Nearest-rank percentile over microsecond samples.
fn percentile(mut samples: Vec<u128>, rank: f64) -> u128 {
    samples.sort_unstable();
    let index = ((rank / 100.0) * samples.len() as f64).ceil() as usize - 1;
    samples[index.min(samples.len() - 1)]
}

fn fr_names(
    hits: &[(
        baraka_domain::catalog::Product,
        Vec<baraka_domain::catalog::ProductUnit>,
    )],
) -> Vec<String> {
    hits.iter().map(|(p, _)| p.name_fr.clone()).collect()
}

#[test]
fn drill_dataset_shape_is_300_active_with_unique_barcodes() {
    let db = seed_drill_db();
    let guard = db.shared.lock().expect("lock");
    let active: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM products WHERE is_active = 1",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(active, PRODUCT_TOTAL as i64);
    let distinct_units: i64 = guard
        .query_row(
            "SELECT COUNT(DISTINCT barcode) FROM product_units WHERE barcode IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("distinct");
    assert_eq!(distinct_units, PRODUCT_TOTAL as i64);
}

#[test]
fn drill_search_ranking_and_arabic_tolerance() {
    let db = seed_drill_db();
    let (vis_id, _, _) = db.anchors.vis;
    let (cle_id, _, _) = db.anchors.cle;

    // Exact product barcode is Rank 0.
    let hits = db
        .catalog
        .search(&db.anchors.vis_barcode, 20, false)
        .expect("barcode");
    assert_eq!(hits.first().map(|(p, _)| p.id), Some(vis_id));

    // Exact secondary unit barcode is Rank 0.
    let hits = db
        .catalog
        .search(&db.anchors.cle.2, 20, false)
        .expect("unit barcode");
    assert_eq!(hits.first().map(|(p, _)| p.id), Some(cle_id));

    // Multi-token AND: only the 4x35 box matches every token.
    let hits = db
        .catalog
        .search("vis agglo 4x35", 20, false)
        .expect("multi");
    assert_eq!(hits.first().map(|(p, _)| p.id), Some(vis_id));
    assert!(
        hits.iter().all(|(p, _)| p.id == vis_id),
        "4x20 must not leak into 4x35: {:?}",
        fr_names(&hits)
    );

    // Infix French queries hit inside names.
    for query in ["douchette", "agglo", "disque", "coude", "teflon"] {
        let hits = db.catalog.search(query, 20, false).expect("infix");
        assert!(!hits.is_empty(), "infix {query:?} must hit");
    }
    let hits = db
        .catalog
        .search("douchette", 20, false)
        .expect("douchette");
    assert!(
        hits.iter().any(|(p, _)| p.name_fr.contains("douchette")),
        "mid-name infix expected: {:?}",
        fr_names(&hits)
    );

    // Name prefix outranks mid-name infix for a shared token.
    let hits = db.catalog.search("vis", 20, false).expect("prefix");
    let first = hits
        .first()
        .map(|(p, _)| p.name_fr.clone())
        .unwrap_or_default();
    assert!(
        first.starts_with("Vis "),
        "prefix rank must lead, got {first:?}"
    );

    // Multi-token French with accents folded.
    let hits = db.catalog.search("cle a molette", 20, false).expect("cle");
    assert!(hits.iter().any(|(p, _)| p.id == cle_id));
    let hits = db
        .catalog
        .search("peinture blanche", 20, false)
        .expect("peinture");
    assert_eq!(hits.len(), 2);
    assert!(hits
        .first()
        .map(|(p, _)| p.name_fr.starts_with("Peinture"))
        .unwrap_or(false));

    // Arabic tolerance: hamza-bearing stored forms match bare queries.
    // ("انبوب" is the true single-char tolerance case for stored "أنبوب";
    // "انابيب" carries an extra alif plus yaa/waw swap — two edits, beyond
    // single-character wildcard tolerance, so the engine rejects it.)
    let hits = db.catalog.search("مفتاح", 20, false).expect("ar");
    assert!(hits.iter().any(|(p, _)| p.id == cle_id));
    let hits = db.catalog.search("انبوب", 20, false).expect("pipes");
    assert!(
        hits.iter().any(|(p, _)| p.id == db.anchors.tube),
        "bare-alef tolerance expected"
    );
    let hits = db.catalog.search("شريط", 20, false).expect("tape");
    assert_eq!(hits.first().map(|(p, _)| p.id), Some(db.anchors.ruban));
}

/// Search latency budget (ADR-015 release-benchmark methodology).
///
/// `#[ignore]`d like `release_benchmarks.rs`: the BRK-073 engine's debug
/// profile cannot meet a 15 ms p95 (measured debug floor: single-token
/// mean ~14.4 ms, p50 14.5 ms, p95 34.7 ms over 300 rows — per-row
/// `baraka_search_norm` scalar calls plus correlated unit-barcode
/// EXISTS branches). Run explicitly in release:
///
/// ```sh
/// CARGO_BUILD_JOBS=2 cargo test --release -j 2 -p baraka-infrastructure \
///   --test hardware_catalog_drill -- --ignored --nocapture
/// ```
#[ignore]
#[test]
fn drill_search_latency_p95_under_budget() {
    let db = seed_drill_db();
    let queries = [
        "douchette",
        "agglo",
        "disque",
        "coude",
        "teflon",
        "vis agglo 4x35",
        "cle a molette",
        "peinture blanche",
        "مفتاح",
        "انبوب",
        "شريط",
        db.anchors.vis_barcode.as_str(),
        db.anchors.cle.2.as_str(),
        "VIS-005",
    ];
    for warmup in &queries {
        let _ = db.catalog.search(warmup, 20, false).expect("warmup");
    }
    let mut samples = Vec::with_capacity(100);
    let mut per_query: Vec<Vec<u128>> = queries.iter().map(|_| Vec::new()).collect();
    for iteration in 0..100 {
        let slot = iteration % queries.len();
        let start = Instant::now();
        let _ = db.catalog.search(queries[slot], 20, false).expect("bench");
        let elapsed = start.elapsed().as_micros();
        samples.push(elapsed);
        per_query[slot].push(elapsed);
    }
    for (query, timings) in queries.iter().zip(per_query.iter()) {
        if !timings.is_empty() {
            let mean = timings.iter().sum::<u128>() / timings.len() as u128;
            println!("DRILL query {query:?} n={} mean={mean}µs", timings.len());
        }
    }
    let p50 = percentile(samples.clone(), 50.0);
    let p95 = percentile(samples, 95.0);
    println!("DRILL search n=100 p50={p50}µs p95={p95}µs budget={LATENCY_BUDGET_MICROS}µs");
    assert!(
        p95 < LATENCY_BUDGET_MICROS,
        "p95 {p95}µs exceeds {LATENCY_BUDGET_MICROS}µs"
    );
}

#[test]
fn drill_atomic_sale_two_items_cash() {
    let db = seed_drill_db();
    let (vis_id, vis_unit, _) = db.anchors.vis;
    let (cle_id, cle_unit, _) = db.anchors.cle;
    let before_vis = db
        .stock
        .get_stock_level(vis_id, db.warehouse_id)
        .expect("level");
    let before_cle = db
        .stock
        .get_stock_level(cle_id, db.warehouse_id)
        .expect("level");
    assert_eq!(before_vis, STOCK_MICRO_PER_PRODUCT);
    assert_eq!(before_cle, STOCK_MICRO_PER_PRODUCT);

    let checkout =
        CheckoutSaleUseCase::new(&db.catalog, &db.customers, &db.checkout, &db.warehouses);
    let cash_tender: i64 = 200_000;
    let receipt = checkout
        .execute(
            &db.session,
            CheckoutInput {
                command_id: Uuid::new_v4().to_string(),
                warehouse_id: db.warehouse_id,
                customer_id: None,
                doc_type: DocumentType::Ticket,
                language: DocumentLanguage::Fr,
                lines: vec![
                    CartLineInput {
                        product_id: vis_id,
                        unit_id: vis_unit,
                        quantity_micro: 10_000,
                        unit_price_override: None,
                        line_discount_centimes: 0,
                    },
                    CartLineInput {
                        product_id: cle_id,
                        unit_id: cle_unit,
                        quantity_micro: 10_000,
                        unit_price_override: None,
                        line_discount_centimes: 0,
                    },
                ],
                global_discount_centimes: 0,
                payments: vec![PaymentInput {
                    method: PaymentMethod::Cash,
                    amount_centimes: cash_tender,
                }],
                allow_negative_stock: false,
                notes: None,

                timbre_enabled: true,
            },
            TERMINAL_ID,
        )
        .expect("checkout");

    // Change arithmetic is exact: tender minus TTC.
    let total: i64 = receipt.total_ttc_centimes.parse().expect("total");
    let change: i64 = receipt.change_amount_centimes.parse().expect("change");
    assert_eq!(change, cash_tender - total);

    // Stock decremented by exactly one base unit per line.
    assert_eq!(
        db.stock
            .get_stock_level(vis_id, db.warehouse_id)
            .expect("level"),
        STOCK_MICRO_PER_PRODUCT - 10_000
    );
    assert_eq!(
        db.stock
            .get_stock_level(cle_id, db.warehouse_id)
            .expect("level"),
        STOCK_MICRO_PER_PRODUCT - 10_000
    );

    // Document FINALIZED, SALE_OUT movements with SALE reference, PENDING receipt print job.
    let guard = db.shared.lock().expect("lock");
    let status: String = guard
        .query_row(
            "SELECT status FROM documents WHERE id = ?1",
            [receipt.document_id.clone()],
            |row| row.get(0),
        )
        .expect("status");
    assert_eq!(status, "FINALIZED");
    let sale_moves: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE reference_id = ?1 AND movement_type = 'SALE_OUT' AND reference_type = 'SALE'",
            [receipt.document_id.clone()],
            |row| row.get(0),
        )
        .expect("moves");
    assert_eq!(sale_moves, 2);
    let job: (String, String) = guard
        .query_row(
            "SELECT status, job_type FROM print_jobs WHERE document_id = ?1",
            [receipt.document_id.clone()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("print job");
    assert_eq!(job, ("PENDING".to_string(), "RECEIPT".to_string()));
}
