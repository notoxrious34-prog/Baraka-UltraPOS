//! `Bon de Préparation` builder tests (TASK-BRK-126).
//!
//! The prep sheet explodes cart lines into whole-rung Colisage shares
//! with ZERO prices by construction: the output carries no price field
//! at all, so a warehouse checklist can never leak a selling price.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{ProductRepository, SessionContext, SessionToken};
use baraka_application::use_cases::prep_sheet::{BuildPrepSheetUseCase, PrepSheetLineInput};
use baraka_domain::catalog::{Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteCatalogRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const VENDEUR_UUID: &str = "11111111-1111-1111-1111-111111111111";

fn session_with(role: Role, uuid: &str) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(uuid).expect("uuid")),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("prep.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES (?1, 'vendeur1', 'hash', 'VENDEUR', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![VENDEUR_UUID],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
    }
}

/// Underwear ladder: Pièce ×1, Paquet ×6, Douzaine ×12.
fn seed_ladder_product(harness: &Harness) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let piece = UnitId::new();
    let pack = UnitId::new();
    let dozen = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("SLIP-PREP").expect("sku"),
                barcode: None,
                name_ar: "سليب".to_string(),
                name_fr: "Slip".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[
                ProductUnit {
                    id: piece,
                    product_id: id,
                    unit_name_ar: "قطعة".to_string(),
                    unit_name_fr: "Pièce".to_string(),
                    factor_micro: FactorMicro::from_micros(10_000),
                    barcode: None,
                    sale_price: Money::from_centimes(10_000),
                    purchase_price: Money::from_centimes(5_000),
                    wholesale_price: None,
                    super_gros_price: None,
                    is_base_unit: true,
                    is_default_selling: true,
                },
                ProductUnit {
                    id: pack,
                    product_id: id,
                    unit_name_ar: "حزمة".to_string(),
                    unit_name_fr: "Paquet".to_string(),
                    factor_micro: FactorMicro::from_micros(60_000),
                    barcode: None,
                    sale_price: Money::from_centimes(60_000),
                    purchase_price: Money::from_centimes(30_000),
                    wholesale_price: None,
                    super_gros_price: None,
                    is_base_unit: false,
                    is_default_selling: false,
                },
                ProductUnit {
                    id: dozen,
                    product_id: id,
                    unit_name_ar: "دزينة".to_string(),
                    unit_name_fr: "Douzaine".to_string(),
                    factor_micro: FactorMicro::from_micros(120_000),
                    barcode: None,
                    sale_price: Money::from_centimes(120_000),
                    purchase_price: Money::from_centimes(60_000),
                    wholesale_price: None,
                    super_gros_price: None,
                    is_base_unit: false,
                    is_default_selling: false,
                },
            ],
            "TEST-TERMINAL",
        )
        .expect("product");
    (id, piece)
}

#[test]
fn sheet_explodes_150_pieces_into_whole_rungs_without_prices() {
    let harness = setup();
    let (product, piece) = seed_ladder_product(&harness);
    let sheet = BuildPrepSheetUseCase::new(&harness.catalog)
        .execute(
            &session_with(Role::Vendeur, VENDEUR_UUID),
            &[PrepSheetLineInput {
                product_id: product,
                unit_id: piece,
                quantity_micro: 1_500_000,
            }],
        )
        .expect("sheet");
    assert_eq!(sheet.len(), 1);
    let line = &sheet[0];
    assert_eq!(line.quantity_base_micro, 1_500_000);
    // 150 = 12 × Douzaine + 1 × Paquet + 0 × Pièce (omitted).
    assert_eq!(line.shares.len(), 2);
    assert_eq!(line.shares[0].count, 12);
    assert_eq!(line.shares[0].factor_micro, 120_000);
    assert_eq!(line.shares[0].unit_name_fr, "Douzaine");
    assert_eq!(line.shares[1].count, 1);
    assert_eq!(line.shares[1].factor_micro, 60_000);
    assert_eq!(line.shares[1].unit_name_ar, "حزمة");
}

#[test]
fn unknown_product_locks_loudly_and_zero_qty_yields_empty_shares() {
    let harness = setup();
    let (product, piece) = seed_ladder_product(&harness);
    let use_case = BuildPrepSheetUseCase::new(&harness.catalog);
    let session = session_with(Role::Vendeur, VENDEUR_UUID);
    let rejected = use_case
        .execute(
            &session,
            &[PrepSheetLineInput {
                product_id: ProductId::new(),
                unit_id: piece,
                quantity_micro: 10_000,
            }],
        )
        .expect_err("unknown product locks");
    assert_eq!(
        format!("{rejected:?}"),
        format!(
            "{:?}",
            baraka_application::errors::ApplicationError::NotFound("product".to_string())
        )
    );
    let sheet = use_case
        .execute(
            &session,
            &[PrepSheetLineInput {
                product_id: product,
                unit_id: piece,
                quantity_micro: 0,
            }],
        )
        .expect("zero qty sheet");
    assert_eq!(sheet.len(), 1);
    assert!(sheet[0].shares.is_empty());
}
