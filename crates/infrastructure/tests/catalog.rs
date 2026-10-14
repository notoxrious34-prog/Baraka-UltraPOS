//! Catalog integration tests over the real SQLite repository.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{ProductRepository, SessionContext, SessionToken};
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, DefineKitUseCase, GetProductWithUnitsUseCase,
    NewKitItemInput, NewUnitInput, SearchCatalogUseCase, UnitUpsert, UpdateProductInput,
    UpdateProductUseCase,
};
use baraka_domain::catalog::{Barcode, ProductId, UnitId};
use baraka_domain::errors::DomainError;
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
    let mut conn = open_connection(&dir.path().join("catalog.db")).expect("open");
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

fn base_unit_tagged(suffix: &str) -> NewUnitInput {
    NewUnitInput {
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Pièce".to_string(),
        factor_micro: 10_000,
        barcode: Some(format!("611125{suffix}")),
        sale_price_centimes: 15_000,
        purchase_price_centimes: 12_000,
        wholesale_price_centimes: None,
        is_base_unit: true,
        is_default_selling: true,
    }
}

fn box_unit_tagged(suffix: &str) -> NewUnitInput {
    NewUnitInput {
        unit_name_ar: "علبة".to_string(),
        unit_name_fr: "Boîte".to_string(),
        factor_micro: 1_000_000,
        barcode: Some(format!("611125{suffix}")),
        sale_price_centimes: 1_450_000,
        purchase_price_centimes: 1_150_000,
        wholesale_price_centimes: None,
        is_base_unit: false,
        is_default_selling: false,
    }
}

fn product_input(sku: &str, is_kit: bool) -> CreateProductInput {
    product_input_tagged(sku, is_kit, "0000013", "0000020")
}

fn product_input_tagged(
    sku: &str,
    is_kit: bool,
    base_suffix: &str,
    box_suffix: &str,
) -> CreateProductInput {
    CreateProductInput {
        sku: sku.to_string(),
        barcode: None,
        name_ar: "برغي".to_string(),
        name_fr: "Vis".to_string(),
        category_id: None,
        is_kit,
        units: vec![base_unit_tagged(base_suffix), box_unit_tagged(box_suffix)],
    }
}

fn base_unit_id(harness: &Harness, id: ProductId) -> UnitId {
    harness
        .repo
        .find_by_id(id)
        .expect("lookup")
        .expect("product")
        .1
        .into_iter()
        .find(|unit| unit.is_base_unit)
        .expect("base unit")
        .id
}

#[test]
fn create_persists_product_with_both_units_atomically() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let id = create
        .execute(&gerant, product_input("VIS-8X40", false), TEST_TERMINAL)
        .expect("create");
    let detail = GetProductWithUnitsUseCase::new(&harness.repo)
        .execute(&gerant, id)
        .expect("fetch");
    assert_eq!(detail.product.sku.as_str(), "VIS-8X40");
    assert_eq!(detail.units.len(), 2);
    assert_eq!(detail.kit_items.len(), 0);
}

#[test]
fn duplicate_sku_and_barcode_map_to_named_errors() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    create
        .execute(&gerant, product_input("VIS-8X40", false), TEST_TERMINAL)
        .expect("first");
    let mut second = product_input("VIS-8X40", false);
    second.units[0].barcode = Some("6111250000099".to_string());
    second.units[1].barcode = Some("6111250000098".to_string());
    let dup_sku = create
        .execute(&gerant, second, TEST_TERMINAL)
        .expect_err("dup sku");
    assert!(matches!(
        dup_sku,
        ApplicationError::Validation(DomainError::DuplicateSku)
    ));

    let mut third = product_input("VIS-8X42", false);
    third.units[0].barcode = Some("6111250000013".to_string());
    third.units[1].barcode = Some("6111250000097".to_string());
    let dup_barcode = create
        .execute(&gerant, third, TEST_TERMINAL)
        .expect_err("dup barcode");
    assert!(matches!(
        dup_barcode,
        ApplicationError::Validation(DomainError::DuplicateBarcode)
    ));
}

#[test]
fn barcode_lookup_returns_exact_unit_and_parent() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let id = create
        .execute(&gerant, product_input("VIS-8X40", false), TEST_TERMINAL)
        .expect("create");
    let code = Barcode::new("6111250000020").expect("barcode");
    let (product, unit) = harness
        .repo
        .find_by_barcode(&code)
        .expect("lookup")
        .expect("found");
    assert_eq!(product.id, id);
    assert_eq!(unit.unit_name_fr, "Boîte");
    assert!(!unit.is_base_unit);
}

#[test]
fn kit_cycles_rejected_and_valid_hierarchy_accepted() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let a = create
        .execute(
            &gerant,
            product_input_tagged("KIT-A", true, "1000013", "1000020"),
            TEST_TERMINAL,
        )
        .expect("a");
    let b = create
        .execute(
            &gerant,
            product_input_tagged("KIT-B", true, "2000013", "2000020"),
            TEST_TERMINAL,
        )
        .expect("b");
    let c = create
        .execute(
            &gerant,
            product_input_tagged("KIT-C", true, "3000013", "3000020"),
            TEST_TERMINAL,
        )
        .expect("c");
    let define = DefineKitUseCase::new(&harness.repo);
    let line = |component: ProductId| NewKitItemInput {
        component_id: component,
        unit_id: base_unit_id(&harness, component),
        quantity_micro: 10_000,
    };
    // B -> C, then A -> B: valid multi-level hierarchy.
    define.execute(&gerant, b, vec![line(c)]).expect("b -> c");
    define.execute(&gerant, a, vec![line(b)]).expect("a -> b");
    // C -> A would close A -> B -> C -> A: rejected as a cycle.
    let cycle = define.execute(&gerant, c, vec![line(a)]);
    assert!(
        matches!(
            cycle,
            Err(ApplicationError::Validation(DomainError::KitCycleDetected))
        ),
        "transitive cycle must be rejected: {cycle:?}"
    );
    // A -> A self-reference on a kit header is rejected structurally.
    let itself = define.execute(&gerant, a, vec![line(a)]);
    assert!(matches!(
        itself,
        Err(ApplicationError::Validation(
            DomainError::SelfReferencingKit
        ))
    ));
}

#[test]
fn vendeur_is_rejected_from_mutations() {
    let harness = setup();
    let vendeur = session_with(Role::Vendeur);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    assert!(matches!(
        create.execute(&vendeur, product_input("VIS-1", false), TEST_TERMINAL),
        Err(ApplicationError::Unauthorized)
    ));
    let define = DefineKitUseCase::new(&harness.repo);
    assert!(matches!(
        define.execute(&vendeur, ProductId::new(), Vec::new()),
        Err(ApplicationError::Unauthorized)
    ));
}

#[test]
fn search_and_update_behave() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let id = create
        .execute(&gerant, product_input("VIS-8X40", false), TEST_TERMINAL)
        .expect("create");
    let search = SearchCatalogUseCase::new(&harness.repo);
    let hits = search.execute(&gerant, "VIS-8", 20).expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0.id, id);
    let blank = search.execute(&gerant, "   ", 20).expect("blank");
    assert_eq!(blank.len(), 1);
    assert_eq!(blank[0].0.id, id);
    assert!(search.execute(&gerant, "ZZZ", 20).expect("miss").is_empty());

    // Barcode-exact search resolves through the unit mapping.
    let scanned = search
        .execute(&gerant, "6111250000020", 20)
        .expect("barcode search");
    assert_eq!(scanned.len(), 1);

    // Update renames and reprices the base unit, keeping the SKU.
    let units = harness.repo.find_by_id(id).expect("lookup").expect("row").1;
    let base = units.iter().find(|unit| unit.is_base_unit).expect("base");
    let update = UpdateProductUseCase::new(&harness.repo, &harness.repo);
    update
        .execute(
            &gerant,
            UpdateProductInput {
                id,
                name_ar: "برغي محدث".to_string(),
                name_fr: "Vis MAJ".to_string(),
                category_id: None,
                is_active: true,
                units: vec![UnitUpsert {
                    id: Some(base.id),
                    unit: NewUnitInput {
                        unit_name_ar: base.unit_name_ar.clone(),
                        unit_name_fr: base.unit_name_fr.clone(),
                        factor_micro: 10_000,
                        barcode: Some("6111250000013".to_string()),
                        sale_price_centimes: 16_000,
                        purchase_price_centimes: 12_000,
                        wholesale_price_centimes: None,
                        is_base_unit: true,
                        is_default_selling: true,
                    },
                }],
            },
        )
        .expect("update");
    let after = harness.repo.find_by_id(id).expect("reload").expect("row");
    assert_eq!(after.0.name_fr, "Vis MAJ");
    assert_eq!(after.0.sku.as_str(), "VIS-8X40");
    // Merge semantics: the untouched box unit survives alongside the base.
    assert_eq!(after.1.len(), 2);
    let repriced = after.1.iter().find(|unit| unit.is_base_unit).expect("base");
    assert_eq!(repriced.sale_price.centimes(), 16_000);
    assert!(after.1.iter().any(|unit| !unit.is_base_unit));
}

#[test]
fn created_product_persists_terminal_origin_device_id() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let terminal = "TERMINAL-ORIGIN-002";
    let id = create
        .execute(&gerant, product_input("ORIGIN-001", false), terminal)
        .expect("create");
    let stored: String = rusqlite::Connection::open(harness._dir.path().join("catalog.db"))
        .expect("reopen")
        .query_row(
            "SELECT origin_device_id FROM products WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .expect("origin");
    assert_eq!(stored, terminal);
}

#[test]
fn catalog_search_infix_and_empty_discovery() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let create = CreateProductUseCase::new(&harness.repo, &harness.repo);
    let id = create
        .execute(
            &gerant,
            CreateProductInput {
                sku: "PLOMB-01".to_string(),
                barcode: None,
                name_ar: "مقبض دوش كروم".to_string(),
                name_fr: "Pomme de douchette chromée".to_string(),
                category_id: None,
                is_kit: false,
                units: vec![base_unit_tagged("0000999"), box_unit_tagged("0000998")],
            },
            TEST_TERMINAL,
        )
        .expect("create");
    let search = SearchCatalogUseCase::new(&harness.repo);

    // Infix FR: middle word matches.
    let fr = search.execute(&gerant, "douchette", 20).expect("infix fr");
    assert_eq!(fr.len(), 1);
    assert_eq!(fr[0].0.id, id);

    // Infix AR: middle word matches.
    let ar = search.execute(&gerant, "دوش", 20).expect("infix ar");
    assert_eq!(ar.len(), 1);
    assert_eq!(ar[0].0.id, id);

    // SKU infix: middle of the code matches.
    let sku = search.execute(&gerant, "LOMB", 20).expect("infix sku");
    assert_eq!(sku.len(), 1);
    assert_eq!(sku[0].0.id, id);

    // Barcode stays exact-only.
    let scanned = search
        .execute(&gerant, "6111250000999", 20)
        .expect("barcode exact");
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].0.id, id);

    // Empty and blank queries discover (not empty) up to the limit.
    for query in ["", " "] {
        let found = search.execute(&gerant, query, 20).expect("discovery");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0.id, id);
    }
}
