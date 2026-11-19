//! Margin-guard lock tests (TASK-DSN-115, `Plancher de Négociation`).
//!
//! FAIL-before: a below-cost line checks out silently — the preview reports
//! `is_below_cost` but nothing enforces it. After: any line priced strictly
//! below cost rejects the whole commit with `MARGIN_GUARD_BELOW_COST`
//! unless a Gérant/Admin PIN override accompanies it; a present-but-invalid
//! override rejects LOUDLY with `MARGIN_GUARD_OVERRIDE_REJECTED`.
//! Nothing persists on either rejection.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{PasswordHasher, ProductRepository, SessionContext};
use baraka_application::ports::{SessionToken, StockRepository, WarehouseRepository};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, ManagerOverride, PaymentInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{DocumentLanguage, DocumentType, PaymentMethod};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    Argon2PasswordHasher, SqliteCashSessionRepository, SqliteCatalogRepository,
    SqliteCheckoutRepository, SqliteCustomerRepository, SqliteStockRepository,
    SqliteUserRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use chrono::TimeDelta;
use uuid::Uuid;

const VENDEUR_UUID: &str = "11111111-1111-1111-1111-111111111111";
const CAISSIER_UUID: &str = "22222222-2222-2222-2222-222222222222";
const GERANT_UUID: &str = "33333333-3333-3333-3333-333333333333";

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
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    cash: SqliteCashSessionRepository,
    users: SqliteUserRepository,
    hasher: Argon2PasswordHasher,
    terminal_id: String,
    warehouse_id: WarehouseId,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("margin.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let hasher = Argon2PasswordHasher;
    let gerant_pin = hasher.hash("2468").expect("hash pin");
    let vendeur_pin = hasher.hash("1357").expect("hash pin");
    for (id, name, role, pin) in [
        (VENDEUR_UUID, "vendeur1", "VENDEUR", Some(vendeur_pin)),
        (CAISSIER_UUID, "caissier1", "CAISSIER", None),
        (GERANT_UUID, "gerant1", "GERANT", Some(gerant_pin)),
    ] {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, pin_hash, role, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, ?4, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![id, name, pin, role],
        )
        .expect("seed user");
    }
    let terminal_id = resolve_device_id(&conn).expect("device id");
    let shared = Arc::new(Mutex::new(conn));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&shared));
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("MAIN").expect("code"),
        name: "Main".to_string(),
        is_active: true,
    };
    warehouses
        .create_warehouse(&warehouse, "TEST-TERMINAL")
        .expect("warehouse");
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses,
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
        users: SqliteUserRepository::new(Arc::clone(&shared)),
        hasher: Argon2PasswordHasher,
        terminal_id,
        warehouse_id: warehouse.id,
    }
}

/// Product sold at 20_000 while costing 25_000: every unit price is red.
fn seed_below_cost_product(harness: &Harness) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("MARGIN-RED").expect("sku"),
                barcode: None,
                name_ar: "منتج".to_string(),
                name_fr: "Produit".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[baraka_domain::catalog::ProductUnit {
                id: unit_id,
                product_id: id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: Some(Barcode::new("611125MARGINRED").expect("barcode")),
                sale_price: Money::from_centimes(20_000),
                purchase_price: Money::from_centimes(25_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            "TEST-TERMINAL",
        )
        .expect("product");
    harness
        .stock
        .upsert_stock_level_with_delta(id, harness.warehouse_id, 1_000_000)
        .expect("stock");
    (id, unit_id)
}

fn open_session(harness: &Harness) {
    use baraka_application::use_cases::cash::OpenCashSessionUseCase;
    OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &session_with(Role::Caissier, CAISSIER_UUID),
            harness.warehouse_id,
            10_000,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("cash session");
}

fn checkout_input(harness: &Harness, product: ProductId, unit: UnitId) -> CheckoutInput {
    CheckoutInput {
        command_id: Uuid::new_v4().to_string(),
        warehouse_id: harness.warehouse_id,
        customer_id: None,
        doc_type: DocumentType::Ticket,
        language: DocumentLanguage::Fr,
        lines: vec![CartLineInput {
            product_id: product,
            unit_id: unit,
            quantity_micro: 10_000,
            unit_price_override: None,
            line_discount_centimes: 0,
        }],
        global_discount_centimes: 0,
        payments: vec![PaymentInput {
            method: PaymentMethod::Cash,
            amount_centimes: 20_000,
        }],
        allow_negative_stock: false,
        below_cost_override: None,
        notes: None,
        timbre_enabled: false,
    }
}

fn document_count(harness: &Harness) -> i64 {
    harness
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .expect("count")
}

fn assert_code(error: ApplicationError, expected: &str) {
    match error {
        ApplicationError::Validation(DomainError::InvalidCanonicalString(code)) => {
            assert_eq!(code, expected, "wrong guard code")
        }
        other => panic!("expected coded rejection, got: {other:?}"),
    }
}

#[test]
fn below_cost_line_rejects_the_whole_commit_without_override() {
    let harness = setup();
    open_session(&harness);
    let (product, unit) = seed_below_cost_product(&harness);
    let before = document_count(&harness);
    let rejected = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
        &harness.users,
        &harness.hasher,
    )
    .execute(
        &session_with(Role::Vendeur, VENDEUR_UUID),
        checkout_input(&harness, product, unit),
        &harness.terminal_id,
    )
    .expect_err("below-cost must lock without a manager override");
    assert_code(rejected, "MARGIN_GUARD_BELOW_COST");
    assert_eq!(document_count(&harness), before, "nothing persists on lock");
}

fn attempt(
    harness: &Harness,
    product: ProductId,
    unit: UnitId,
    over: Option<ManagerOverride>,
) -> Result<String, ApplicationError> {
    let mut input = checkout_input(harness, product, unit);
    input.command_id = Uuid::new_v4().to_string();
    input.below_cost_override = over;
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
        &harness.users,
        &harness.hasher,
    )
    .execute(
        &session_with(Role::Vendeur, VENDEUR_UUID),
        input,
        &harness.terminal_id,
    )
    .map(|receipt| receipt.document_number)
}

#[test]
fn gerant_pin_override_unlocks_this_sale_only() {
    let harness = setup();
    open_session(&harness);
    let (product, unit) = seed_below_cost_product(&harness);
    let before = document_count(&harness);
    let number = attempt(
        &harness,
        product,
        unit,
        Some(ManagerOverride {
            username: "gerant1".to_string(),
            pin: "2468".to_string(),
        }),
    )
    .expect("gerant override unlocks");
    assert!(!number.is_empty());
    assert_eq!(
        document_count(&harness),
        before + 1,
        "exactly this sale commits"
    );
}

#[test]
fn vendeur_pin_wrong_pin_and_unknown_user_reject_loudly() {
    let harness = setup();
    open_session(&harness);
    let (product, unit) = seed_below_cost_product(&harness);
    for (label, over) in [
        (
            "vendeur pin",
            ManagerOverride {
                username: "vendeur1".to_string(),
                pin: "1357".to_string(),
            },
        ),
        (
            "wrong pin",
            ManagerOverride {
                username: "gerant1".to_string(),
                pin: "0000".to_string(),
            },
        ),
        (
            "unknown user",
            ManagerOverride {
                username: "ghost".to_string(),
                pin: "2468".to_string(),
            },
        ),
        (
            "empty pin",
            ManagerOverride {
                username: "gerant1".to_string(),
                pin: String::new(),
            },
        ),
    ] {
        let before = document_count(&harness);
        let rejected = attempt(&harness, product, unit, Some(over)).expect_err(label);
        assert_code(rejected, "MARGIN_GUARD_OVERRIDE_REJECTED");
        assert_eq!(
            document_count(&harness),
            before,
            "nothing persists on {label}"
        );
    }
}
