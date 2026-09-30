//! Product update integration tests over the real SQLite repository.
//!
//! File-based temporary database (WAL standards): every test opens
//! `catalog_update.db` in a fresh tempdir and runs all migrations (now
//! including the DEP-01 warehouse seed, which is irrelevant here — catalog
//! rows live independently of warehouses). Coverage: the failure paths the
//! new `catalog_update_product` IPC command must surface (unknown id,
//! unknown category, unauthorized role, empty names, invalid unit set) plus
//! the mandated identical-payload idempotency proof against real storage.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{ProductRepository, SessionContext, SessionToken};
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, NewUnitInput, UnitUpsert, UpdateProductInput,
    UpdateProductUseCase,
};
use baraka_domain::catalog::{CategoryId, ProductId};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

const TEST_TERMINAL: &str = "TEST-TERMINAL-UPD";

struct Harness {
    _dir: tempfile::TempDir,
    repo: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("catalog_update.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    Harness {
        _dir: dir,
        repo: SqliteCatalogRepository::new(Arc::new(Mutex::new(conn))),
    }
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn base_unit() -> NewUnitInput {
    NewUnitInput {
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Pièce".to_string(),
        factor_micro: 10_000,
        barcode: None,
        sale_price_centimes: 1_000,
        purchase_price_centimes: 800,
        is_base_unit: true,
        is_default_selling: true,
    }
}

fn seed_product(harness: &Harness, sku: &str) -> ProductId {
    let gerant = session_with(Role::Gerant);
    CreateProductUseCase::new(&harness.repo, &harness.repo)
        .execute(
            &gerant,
            CreateProductInput {
                sku: sku.to_string(),
                barcode: None,
                name_ar: "برغي".to_string(),
                name_fr: "Vis".to_string(),
                category_id: None,
                is_kit: false,
                units: vec![base_unit()],
            },
            TEST_TERMINAL,
        )
        .expect("seed product")
}

fn rename_payload(harness: &Harness, id: ProductId, name_fr: &str) -> UpdateProductInput {
    let units = harness.repo.find_by_id(id).expect("lookup").expect("row").1;
    let base = units.iter().find(|unit| unit.is_base_unit).expect("base");
    UpdateProductInput {
        id,
        name_ar: "برغي محدث".to_string(),
        name_fr: name_fr.to_string(),
        category_id: None,
        is_active: true,
        units: vec![UnitUpsert {
            id: Some(base.id),
            unit: NewUnitInput {
                sale_price_centimes: 1_500,
                ..base_unit()
            },
        }],
    }
}

#[test]
fn update_unknown_id_yields_not_found() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let id = seed_product(&harness, "VIS-8X40");
    let mut missing = rename_payload(&harness, id, "Vis");
    missing.id = ProductId::new();
    assert!(matches!(
        UpdateProductUseCase::new(&harness.repo, &harness.repo).execute(&gerant, missing),
        Err(ApplicationError::NotFound(_))
    ));
}

#[test]
fn update_unknown_category_yields_not_found() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let id = seed_product(&harness, "VIS-8X40");
    let mut payload = rename_payload(&harness, id, "Vis");
    payload.category_id = Some(CategoryId::from(uuid::Uuid::new_v4()));
    assert!(matches!(
        UpdateProductUseCase::new(&harness.repo, &harness.repo).execute(&gerant, payload),
        Err(ApplicationError::NotFound(_))
    ));
}

#[test]
fn update_by_vendeur_is_unauthorized() {
    let harness = setup();
    let id = seed_product(&harness, "VIS-8X40");
    let vendeur = session_with(Role::Vendeur);
    assert!(matches!(
        UpdateProductUseCase::new(&harness.repo, &harness.repo)
            .execute(&vendeur, rename_payload(&harness, id, "Vis")),
        Err(ApplicationError::Unauthorized)
    ));
}

#[test]
fn update_empty_names_are_rejected() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let id = seed_product(&harness, "VIS-8X40");
    let mut payload = rename_payload(&harness, id, "Vis");
    payload.name_fr = "   ".to_string();
    assert!(matches!(
        UpdateProductUseCase::new(&harness.repo, &harness.repo).execute(&gerant, payload),
        Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString(_)
        ))
    ));
}

#[test]
fn update_invalid_unit_set_is_rejected() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let id = seed_product(&harness, "VIS-8X40");
    let mut payload = rename_payload(&harness, id, "Vis");
    // Two base units violate the exactly-one-base-unit invariant.
    payload.units.push(UnitUpsert {
        id: None,
        unit: base_unit(),
    });
    assert!(matches!(
        UpdateProductUseCase::new(&harness.repo, &harness.repo).execute(&gerant, payload),
        Err(ApplicationError::Validation(DomainError::BaseUnitMismatch))
    ));
}

#[test]
fn update_twice_with_identical_payload_is_idempotent() {
    let harness = setup();
    let gerant = session_with(Role::Gerant);
    let id = seed_product(&harness, "VIS-8X40");
    let update = UpdateProductUseCase::new(&harness.repo, &harness.repo);
    update
        .execute(&gerant, rename_payload(&harness, id, "Vis MAJ"))
        .expect("first update");
    // Identical repeat: no error, no duplicate unit rows, same values.
    update
        .execute(&gerant, rename_payload(&harness, id, "Vis MAJ"))
        .expect("repeat update");
    let (stored, units) = harness.repo.find_by_id(id).expect("reload").expect("row");
    assert_eq!(stored.name_fr, "Vis MAJ");
    assert_eq!(stored.sku.as_str(), "VIS-8X40");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].sale_price.centimes(), 1_500);
}
