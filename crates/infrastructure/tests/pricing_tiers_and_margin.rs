//! Tier pricing & Margin Guard integration tests over real SQLite.
//!
//! Pillar 4 (`TASK-BRK-068`): migration 0015 adds `customers.customer_tier`
//! and `product_units.wholesale_price_centimes`; the domain resolves tier
//! prices via `ProductUnit::price_for_tier` and integer-exact margins via
//! `calculate_margin_basis_points`. These tests pin the full stack —
//! migration, repository round-trips, tier resolution, and margin vectors.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    CustomerRepository, NewCustomer, ProductRepository, SessionContext, SessionToken,
};
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, NewUnitInput,
};
use baraka_application::use_cases::customers::{CreateCustomerUseCase, GetCustomerUseCase};
use baraka_domain::catalog::ProductId;
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::pricing::calculate_margin_basis_points;
use baraka_domain::sales::{CreditPolicy, CustomerId, CustomerTier};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{SqliteCatalogRepository, SqliteCustomerRepository};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

const TEST_TERMINAL: &str = "TEST-TERMINAL-TIER";

struct Harness {
    _dir: tempfile::TempDir,
    db_path: PathBuf,
    customers: SqliteCustomerRepository,
    catalog: SqliteCatalogRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("tiers.db");
    let conn = open_connection(&db_path).expect("open");
    let shared = Arc::new(Mutex::new(conn));
    {
        let mut guard = shared.lock().expect("lock");
        run_migrations(&mut guard).expect("migrate");
    }
    Harness {
        _dir: dir,
        db_path,
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        catalog: SqliteCatalogRepository::new(shared),
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

#[test]
fn test_migration_0015_applies_and_adds_tier_columns() {
    let harness = setup();
    // Fresh read connection: the harness repos keep the write handle locked
    // behind their own mutex, so schema assertions use a second handle.
    let conn = open_connection(&harness.db_path).expect("reopen");
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("version");
    assert!(version >= 15, "migration 0015 must be applied");
    for (table, column) in [
        ("customers", "customer_tier"),
        ("product_units", "wholesale_price_centimes"),
    ] {
        let present: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info(?) WHERE name = ?",
                rusqlite::params![table, column],
                |row| row.get(0),
            )
            .expect("pragma");
        assert!(present, "{table}.{column} must exist after 0015");
    }
}

fn create_customer(harness: &Harness, name: &str, tier: CustomerTier) -> CustomerId {
    let session = session_with(Role::Gerant);
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session,
            NewCustomer {
                name: name.to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Disabled,
                credit_limit_centimes: 0,
                customer_tier: tier,
            },
            TEST_TERMINAL,
        )
        .expect("create customer")
}

#[test]
fn test_migration_0015_persists_customer_tiers() {
    let harness = setup();
    let session = session_with(Role::Gerant);
    let get = GetCustomerUseCase::new(&harness.customers);

    let cases = [
        (
            create_customer(&harness, "Walk-in", CustomerTier::Retail),
            CustomerTier::Retail,
        ),
        (
            create_customer(&harness, "Plombier Karim", CustomerTier::Artisan),
            CustomerTier::Artisan,
        ),
        (
            create_customer(&harness, "Grossiste Oran", CustomerTier::Wholesale),
            CustomerTier::Wholesale,
        ),
    ];
    for (id, expected) in cases {
        let customer = get.execute(&session, id).expect("get customer");
        assert_eq!(customer.customer_tier, expected);
        assert_eq!(customer.customer_tier.as_str(), expected.as_str());
    }

    // Directory summaries carry the tier too.
    let summaries = harness
        .customers
        .list_customers(false)
        .expect("list customers");
    assert_eq!(summaries.len(), 3);
    let tiers: Vec<CustomerTier> = summaries.iter().map(|row| row.customer_tier).collect();
    assert!(tiers.contains(&CustomerTier::Retail));
    assert!(tiers.contains(&CustomerTier::Artisan));
    assert!(tiers.contains(&CustomerTier::Wholesale));
}

fn create_product_with_wholesale(
    harness: &Harness,
    sku: &str,
    wholesale: Option<i64>,
) -> ProductId {
    let session = session_with(Role::Gerant);
    CreateProductUseCase::new(&harness.catalog, &harness.catalog)
        .execute(
            &session,
            CreateProductInput {
                sku: sku.to_string(),
                barcode: None,
                name_ar: "سلعة".to_string(),
                name_fr: format!("Produit {sku}"),
                category_id: None,
                is_kit: false,
                units: vec![NewUnitInput {
                    unit_name_ar: "قطعة".to_string(),
                    unit_name_fr: "Pièce".to_string(),
                    factor_micro: 10_000,
                    barcode: None,
                    sale_price_centimes: 10_000,
                    purchase_price_centimes: 7_000,
                    wholesale_price_centimes: wholesale,
                    is_base_unit: true,
                    is_default_selling: true,
                }],
            },
            TEST_TERMINAL,
        )
        .expect("create product")
}

fn stored_unit(harness: &Harness, product_id: ProductId) -> baraka_domain::catalog::ProductUnit {
    let (_, units) = harness
        .catalog
        .find_by_id(product_id)
        .expect("find")
        .expect("product must exist");
    assert_eq!(units.len(), 1);
    units.into_iter().next().expect("single unit")
}

#[test]
fn test_wholesale_price_unit_persistence() {
    let harness = setup();
    let with_id = create_product_with_wholesale(&harness, "TIER-W-1", Some(8_000));
    let unit = stored_unit(&harness, with_id);
    assert_eq!(unit.wholesale_price, Some(Money::from_centimes(8_000)));

    let bare_id = create_product_with_wholesale(&harness, "TIER-W-2", None);
    let bare = stored_unit(&harness, bare_id);
    assert_eq!(bare.wholesale_price, None);
}

#[test]
fn test_pricing_tier_resolution_and_margin_guard() {
    let harness = setup();
    let product_id = create_product_with_wholesale(&harness, "TIER-W-3", Some(8_500));
    let unit = stored_unit(&harness, product_id);
    assert_eq!(unit.sale_price, Money::from_centimes(10_000));
    assert_eq!(unit.purchase_price, Money::from_centimes(7_000));

    // Tier resolution: retail pays sale price; artisan/wholesale pay wholesale.
    assert_eq!(
        unit.price_for_tier(CustomerTier::Retail),
        Money::from_centimes(10_000)
    );
    assert_eq!(
        unit.price_for_tier(CustomerTier::Artisan),
        Money::from_centimes(8_500)
    );
    assert_eq!(
        unit.price_for_tier(CustomerTier::Wholesale),
        Money::from_centimes(8_500)
    );

    // Margin Guard: (8500-7000)*10000/8500 = 1764 bps, not below cost.
    assert_eq!(calculate_margin_basis_points(8_500, 7_000), (1764, false));
    // Margin Guard: (6000-7000)*10000/6000 = -1666 bps, below cost.
    assert_eq!(calculate_margin_basis_points(6_000, 7_000), (-1666, true));

    // Factor sanity: the stored base unit converts 1:1.
    assert_eq!(unit.factor_micro, FactorMicro::from_micros(10_000));
}
