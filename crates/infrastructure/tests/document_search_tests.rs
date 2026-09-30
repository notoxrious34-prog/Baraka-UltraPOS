//! Read-only document search tests over a real SQLite backend (TASK-BRK-042).
//!
//! Seeds a mixed ledger (FINALIZED/CONVERTED DEVIS, FINALIZED BL with a
//! customer party, FINALIZED TICKET) and proves type/status filtering,
//! partial number and customer-name matching, limit clamping, strict filter
//! validation, and the CreateSale role gate — all without mutating state.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    DocumentRepository, DocumentSearchFilter, NewCustomer, ProductRepository, SessionContext,
    SessionToken, WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_application::use_cases::sales::SearchDocumentsUseCase;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, Document, DocumentId, DocumentItem, DocumentLanguage, DocumentStatus,
    DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCustomerRepository, SqliteDocumentRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const GERANT_UUID: &str = "55555555-5555-5555-5555-555555555555";
const VENDEUR_UUID: &str = "44444444-4444-4444-4444-444444444444";
const TEST_TERMINAL: &str = "TEST-TERMINAL-SEARCH";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    warehouses: SqliteWarehouseRepository,
    documents: SqliteDocumentRepository,
    customers: SqliteCustomerRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("search.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    for (id, name, role) in [
        (GERANT_UUID, "gerant1", "GERANT"),
        (VENDEUR_UUID, "vendeur1", "VENDEUR"),
    ] {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![id, name, role],
        )
        .expect("seed user");
    }
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
    }
}

fn user_id(value: &str) -> UserId {
    UserId::from(Uuid::parse_str(value).expect("uuid"))
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: user_id(GERANT_UUID),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn seed_warehouse(harness: &Harness, code: &str) -> WarehouseId {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new(code).expect("code"),
        name: code.to_string(),
        is_active: true,
    };
    harness
        .warehouses
        .create_warehouse(&warehouse, TEST_TERMINAL)
        .expect("warehouse");
    warehouse.id
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
                name_ar: "منتج".to_string(),
                name_fr: format!("Produit {sku}"),
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
                barcode: Some(Barcode::new(&format!("611125{sku}")).expect("barcode")),
                sale_price: Money::from_centimes(price),
                purchase_price: Money::from_centimes(price - 2_000),
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("product");
    (id, unit_id)
}

fn seed_customer(harness: &Harness, name: &str) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &session_with(Role::Gerant),
            NewCustomer {
                name: name.to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
            },
            TEST_TERMINAL,
        )
        .expect("customer")
}

fn line(
    document_id: DocumentId,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    unit_price: i64,
    sort_order: i64,
) -> DocumentItem {
    let micros = qty_units * 10_000;
    DocumentItem {
        id: Uuid::new_v4(),
        document_id,
        product_id: product,
        unit_id: unit,
        quantity_micro: micros,
        quantity_base_micro: micros,
        unit_price_centimes: unit_price,
        discount_centimes: 0,
        tva_rate_micro: 0,
        line_ht_centimes: unit_price * qty_units,
        line_tva_centimes: 0,
        line_ttc_centimes: unit_price * qty_units,
        cost_price_centimes: unit_price - 2_000,
        sort_order,
    }
}

#[allow(clippy::too_many_arguments)]
fn seed_document(
    harness: &Harness,
    warehouse: WarehouseId,
    doc_type: DocumentType,
    status: DocumentStatus,
    number: &str,
    ttc: i64,
    items: Vec<DocumentItem>,
    party: Option<CustomerId>,
) -> DocumentId {
    let id = DocumentId::new();
    let mut fixed = Vec::with_capacity(items.len());
    for mut item in items {
        item.document_id = id;
        fixed.push(item);
    }
    harness
        .documents
        .finalize_document(
            &Document {
                id,
                language: DocumentLanguage::Fr,
                number: number.to_string(),
                fiscal_year: chrono::Utc::now()
                    .format("%Y")
                    .to_string()
                    .parse()
                    .unwrap_or(1970),
                doc_type,
                status,
                warehouse_id: warehouse,
                party_id: party.map(|id| id.to_string()),
                subtotal_ht_centimes: ttc,
                vat_total_centimes: 0,
                global_discount_centimes: 0,
                timbre_centimes: 0,
                ttc_centimes: ttc,
                paid_amount_centimes: 0,
                change_amount_centimes: 0,
                payment_method: "UNPAID".to_string(),
                notes: None,
                created_at: chrono::Utc::now(),
                created_by: user_id(GERANT_UUID),
            },
            &fixed,
            &[],
            TEST_TERMINAL,
        )
        .expect("seed document");
    id
}

/// DEVIS FINALIZED (2 lines, 40_000) + DEVIS CONVERTED (1 line, 10_000) +
/// BL FINALIZED (2 lines, 40_000, customer party) + TICKET FINALIZED
/// (1 line, 5_000).
fn seed_ledger(harness: &Harness, warehouse: WarehouseId) {
    let (p1, u1) = seed_product(harness, "S1", 10_000);
    let (p2, u2) = seed_product(harness, "S2", 20_000);
    let customer = seed_customer(harness, "Cherif Benz");
    let draft = DocumentId::new();
    seed_document(
        harness,
        warehouse,
        DocumentType::Devis,
        DocumentStatus::Finalized,
        "DEV-2026-000001",
        40_000,
        vec![
            line(draft, p1, u1, 2, 10_000, 0),
            line(draft, p2, u2, 1, 20_000, 1),
        ],
        None,
    );
    let draft = DocumentId::new();
    seed_document(
        harness,
        warehouse,
        DocumentType::Devis,
        DocumentStatus::Converted,
        "DEV-2026-000002",
        10_000,
        vec![line(draft, p1, u1, 1, 10_000, 0)],
        None,
    );
    let draft = DocumentId::new();
    seed_document(
        harness,
        warehouse,
        DocumentType::Bl,
        DocumentStatus::Finalized,
        "BL-2026-000001",
        40_000,
        vec![
            line(draft, p1, u1, 2, 10_000, 0),
            line(draft, p2, u2, 1, 20_000, 1),
        ],
        Some(customer),
    );
    let draft = DocumentId::new();
    seed_document(
        harness,
        warehouse,
        DocumentType::Ticket,
        DocumentStatus::Finalized,
        "TICK-2026-000001",
        5_000,
        vec![line(draft, p1, u1, 1, 5_000, 0)],
        None,
    );
}

fn search(
    harness: &Harness,
    role: Role,
    doc_type: Option<&str>,
    status: Option<&str>,
    query: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<baraka_application::ports::DocumentSummary>, ApplicationError> {
    SearchDocumentsUseCase::new(&harness.documents).execute(
        &session_with(role),
        DocumentSearchFilter {
            doc_type: doc_type.map(str::to_string),
            status: status.map(str::to_string),
            query: query.map(str::to_string),
            limit,
        },
    )
}

#[test]
fn filter_by_type_and_status_returns_only_convertible_devis() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    seed_ledger(&harness, warehouse);
    let rows = search(
        &harness,
        Role::Gerant,
        Some("DEVIS"),
        Some("FINALIZED"),
        None,
        None,
    )
    .expect("search");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.number, "DEV-2026-000001");
    assert_eq!(row.doc_type, "DEVIS");
    assert_eq!(row.status, "FINALIZED");
    assert_eq!(row.ttc_centimes, 40_000);
    assert_eq!(row.items_count, 2);
    assert_eq!(row.party_id, None);
    assert_eq!(row.party_name, None);
}

#[test]
fn partial_number_search_matches_across_types() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    seed_ledger(&harness, warehouse);
    let rows = search(&harness, Role::Gerant, None, None, Some("000001"), None).expect("search");
    assert_eq!(rows.len(), 3);
    for row in &rows {
        assert!(
            row.number.contains("000001"),
            "unexpected number: {}",
            row.number
        );
    }
}

#[test]
fn customer_name_search_resolves_party_join() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    seed_ledger(&harness, warehouse);
    let rows = search(&harness, Role::Gerant, None, None, Some("cherif"), None).expect("search");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].number, "BL-2026-000001");
    assert_eq!(rows[0].party_name.as_deref(), Some("Cherif Benz"));
    assert!(rows[0].party_id.is_some());
    assert_eq!(rows[0].items_count, 2);
}

#[test]
fn limit_clamping_bounds_result_size() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    seed_ledger(&harness, warehouse);
    // Absurd request: served without error, bounded by the 100 cap.
    let rows = search(&harness, Role::Gerant, None, None, None, Some(200)).expect("search");
    assert!(rows.len() <= 100);
    assert_eq!(rows.len(), 4);
    // Small explicit limit honored exactly.
    let rows = search(&harness, Role::Gerant, None, None, None, Some(2)).expect("search");
    assert_eq!(rows.len(), 2);
    // Unset and zero limits fall back to the 20 default (all 4 rows served).
    for limit in [None, Some(0)] {
        let rows = search(&harness, Role::Gerant, None, None, None, limit).expect("search");
        assert_eq!(rows.len(), 4);
    }
}

#[test]
fn invalid_filter_codes_rejected_before_any_read() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    seed_ledger(&harness, warehouse);
    assert!(
        search(&harness, Role::Gerant, Some("NOPE"), None, None, None).is_err(),
        "unknown type must fail"
    );
    assert!(
        search(&harness, Role::Gerant, None, Some("NOPE"), None, None).is_err(),
        "unknown status must fail"
    );
}

#[test]
fn converted_status_filter_lists_sealed_sources() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    seed_ledger(&harness, warehouse);
    let rows = search(&harness, Role::Gerant, None, Some("CONVERTED"), None, None).expect("search");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].number, "DEV-2026-000002");
}

#[test]
fn vendeur_may_search_via_create_sale_grant() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    seed_ledger(&harness, warehouse);
    let rows = search(&harness, Role::Vendeur, None, None, None, None).expect("search");
    assert_eq!(rows.len(), 4);
}
