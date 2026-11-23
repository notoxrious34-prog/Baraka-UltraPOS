//! Signal-economy reads over a real SQLite backend (TASK-BRK-116).
//!
//! A 2-cell model (M/L x NOIR): M stays at zero stock while 15 L units
//! sell through a committed TICKET, plus one stocked-but-forgotten product.
//! The end-to-end [`ComputeSignalsUseCase`] must flag the broken M run,
//! the dead product, today's velocity, and the model's settled margin —
//! the ledger stays empty so aging reads all-zero.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CategoryRepository, DocumentRepository, NewCustomer, ProductRepository, SessionContext,
    SessionToken, VariantMatrixRepository, VerticalProfilePort, WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::signals::ComputeSignalsUseCase;
use baraka_application::use_cases::variant_matrix::GenerateVariantMatrixInput;
use baraka_application::use_cases::variant_matrix::GenerateVariantMatrixUseCase;
use baraka_domain::catalog::{Category, CategoryId, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::vertical::VerticalProfile;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, CustomerTier, Document, DocumentId, DocumentItem, DocumentLanguage,
    DocumentStatus, DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCustomerRepository, SqliteDocumentRepository,
    SqliteSignalRepository, SqliteVariantMatrixRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const TERMINAL: &str = "TEST-TERMINAL-SIGNALS";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    warehouses: SqliteWarehouseRepository,
    documents: SqliteDocumentRepository,
    customers: SqliteCustomerRepository,
    matrix: SqliteVariantMatrixRepository,
    signals: SqliteSignalRepository,
}

fn session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role: Role::Admin,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

struct NoCategories;

impl CategoryRepository for NoCategories {
    fn create_category(&self, _: &Category) -> Result<(), ApplicationError> {
        Ok(())
    }

    fn find_category_by_id(&self, _: CategoryId) -> Result<Option<Category>, ApplicationError> {
        Ok(None)
    }

    fn list_categories(&self) -> Result<Vec<Category>, ApplicationError> {
        Ok(vec![])
    }

    fn update_category(&self, _: &Category) -> Result<(), ApplicationError> {
        Ok(())
    }
}

struct TextileProfile;

impl VerticalProfilePort for TextileProfile {
    fn load_profile(&self) -> Result<VerticalProfile, ApplicationError> {
        Ok(VerticalProfile::ApparelWholesaleRetail)
    }

    fn save_profile(&self, _: &VerticalProfile, _: &str) -> Result<(), ApplicationError> {
        Ok(())
    }
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("signals.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, pin_hash, role, is_active, created_at, updated_at)
         VALUES ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'tester', 'hash', NULL, 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        matrix: SqliteVariantMatrixRepository::new(Arc::clone(&shared)),
        signals: SqliteSignalRepository::new(Arc::clone(&shared)),
    }
}

fn seed_warehouse(harness: &Harness) -> WarehouseId {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("MAIN").expect("code"),
        name: "Main".to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, TERMINAL)
        .expect("warehouse");
    warehouse.id
}

fn seed_template(harness: &Harness) -> ProductId {
    let id = ProductId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new("SERIE-116").expect("sku"),
                barcode: None,
                name_ar: "سيرية".to_string(),
                name_fr: "SERIE".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: UnitId::new(),
                product_id: id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: None,
                sale_price: Money::from_centimes(25_000),
                purchase_price: Money::from_centimes(15_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TERMINAL,
        )
        .expect("template");
    id
}

fn seed_product(harness: &Harness, sku: &str, price: i64) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new(sku).expect("sku"),
                barcode: None,
                name_ar: "منسي".to_string(),
                name_fr: "OUBLIE".to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: unit_id,
                product_id: id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: None,
                sale_price: Money::from_centimes(price),
                purchase_price: Money::from_centimes(price - 5_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TERMINAL,
        )
        .expect("product");
    (id, unit_id)
}

fn seed_customer(harness: &Harness) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session(),
            NewCustomer {
                name: "Zaboun".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
                customer_tier: CustomerTier::Retail,
            },
            TERMINAL,
        )
        .expect("customer")
}

fn set_stock(harness: &Harness, product: ProductId, warehouse: WarehouseId, qty: i64) {
    harness
        .db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO stock_levels (warehouse_id, product_id, quantity_micro, quantity_base_micro)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(warehouse_id, product_id) DO UPDATE SET quantity_micro = ?3, quantity_base_micro = ?3",
            rusqlite::params![warehouse.to_string(), product.to_string(), qty],
        )
        .expect("stock level");
}

#[test]
fn end_to_end_signals_flag_the_broken_run_and_the_dead_shelf() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness);
    let customer = seed_customer(&harness);
    let template = seed_template(&harness);
    let generate =
        GenerateVariantMatrixUseCase::new(&harness.catalog, &NoCategories, &harness.matrix);
    let output = generate
        .execute(
            &session(),
            GenerateVariantMatrixInput {
                name_ar: "سيرية".to_string(),
                name_fr: "SERIE".to_string(),
                axis1_name: "Taille".to_string(),
                axis2_name: "Couleur".to_string(),
                axis1_values: vec!["M".to_string(), "L".to_string()],
                axis2_values: vec!["NOIR".to_string()],
                base_sku: "SERIE-116".to_string(),
                category_id: None,
                template_product_id: template,
                overrides: vec![],
            },
            TERMINAL,
        )
        .expect("generates");
    assert_eq!(output.cell_count, 2);

    let view = harness.matrix.matrix_view(output.model_id).expect("view");
    assert_eq!(view.len(), 2);
    let _cell_m = view.iter().find(|r| r.axis1_value == "M").expect("M cell");
    let cell_l = view.iter().find(|r| r.axis1_value == "L").expect("L cell");

    // M stays empty; L is stocked then sells 15 units cash today.
    set_stock(&harness, cell_l.product_id, warehouse, 500_000);
    let cell_l_unit = cell_l
        .default_unit
        .as_ref()
        .expect("L default unit")
        .unit_id;
    let ticket_id = DocumentId::new();
    let micros = 15 * 10_000;
    harness
        .documents
        .finalize_document(
            &Document {
                id: ticket_id,
                language: DocumentLanguage::Fr,
                number: "T-SIGNAL-001".to_string(),
                fiscal_year: 2026,
                doc_type: DocumentType::Ticket,
                status: DocumentStatus::Finalized,
                warehouse_id: warehouse,
                party_id: Some(customer.to_string()),
                subtotal_ht_centimes: 375_000,
                vat_total_centimes: 0,
                global_discount_centimes: 0,
                timbre_centimes: 0,
                ttc_centimes: 375_000,
                paid_amount_centimes: 375_000,
                change_amount_centimes: 0,
                payment_method: "CASH".to_string(),
                template_version: baraka_domain::printing::TSPL_TICKET_TEMPLATE_VERSION,
                credit_solde_before_centimes: None,
                credit_solde_after_centimes: None,
                credit_delta_centimes: None,
                notes: None,
                created_at: chrono::Utc::now(),
                created_by: UserId::from(
                    Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").expect("uuid"),
                ),
            },
            &[DocumentItem {
                id: Uuid::new_v4(),
                document_id: ticket_id,
                product_id: cell_l.product_id,
                unit_id: cell_l_unit,
                quantity_micro: micros,
                quantity_base_micro: micros,
                unit_price_centimes: 25_000,
                discount_centimes: 0,
                discount_per_unit_centimes: None,
                tva_rate_micro: 0,
                line_ht_centimes: 375_000,
                line_tva_centimes: 0,
                line_ttc_centimes: 375_000,
                cost_price_centimes: 15_000,
                sort_order: 0,
            }],
            &[],
            TERMINAL,
        )
        .expect("seed ticket");

    // One stocked product nobody ever touches.
    let (old_id, _) = seed_product(&harness, "OLD-116", 30_000);
    set_stock(&harness, old_id, warehouse, 200_000);

    let use_case = ComputeSignalsUseCase::new(&harness.signals, &TextileProfile);
    let out = use_case.compute(&session(), 30, None).expect("compute");

    // Broken M run: 15 sibling units >= textile 12.
    assert_eq!(out.broken_runs.len(), 1);
    assert_eq!(out.broken_runs[0].cells.len(), 1);
    assert_eq!(out.broken_runs[0].cells[0].axis1_value, "M");
    assert_eq!(out.broken_runs[0].cells[0].sibling_units_sold, 15);
    // Dead shelf: stocked, never moved.
    assert!(out
        .dead_stock
        .iter()
        .any(|d| d.product_id == old_id.to_string()));
    // Velocity: 15 units sold inside the window.
    let velocity = out.velocity.expect("velocity");
    assert_eq!(velocity.units_per_day, 0);
    assert!(velocity.is_slow);
    // Margin: model revenue 375_000, cost 15_000 x 15.
    assert_eq!(out.model_margins.len(), 1);
    assert!(out.model_margins[0].margin_bps.is_some());
    // No credit touched: aging empty.
    assert_eq!(out.aging.total(), Money::from_centimes(0));
}
