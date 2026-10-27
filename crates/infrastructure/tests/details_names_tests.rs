//! Document-details display names over a real SQLite backend (TASK-BRK-088).
//!
//! FAIL-before: `get_items_with_names` / `DocumentItemView` do not exist yet —
//! the details wire carries IDs only (AUD-041 verdict B). After the fix, every
//! details item carries product designation (FR + AR), sku and unit label via
//! a single JOIN query; a missing product row degrades to nullable fields +
//! `product_missing` without dropping the line.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{DocumentRepository, ProductRepository, WarehouseRepository};
use baraka_domain::catalog::{Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::sales::{
    Document, DocumentId, DocumentItem, DocumentLanguage, DocumentStatus, DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteDocumentRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use uuid::Uuid;

const GERANT_UUID: &str = "55555555-5555-5555-5555-555555555555";
const TEST_TERMINAL: &str = "TEST-TERMINAL-DETAILS-NAMES";

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    catalog: SqliteCatalogRepository,
    documents: SqliteDocumentRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("details_names.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('55555555-5555-5555-5555-555555555555', 'gerant1', 'hash', 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
    }
}

fn seed_warehouse(harness: &Harness, code: &str) -> WarehouseId {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new(code).expect("code"),
        name: code.to_string(),
        is_active: true,
    };
    let repos = SqliteWarehouseRepository::new(Arc::clone(&harness.db));
    repos
        .create_warehouse(&warehouse, TEST_TERMINAL)
        .expect("warehouse");
    warehouse.id
}

fn seed_product(
    harness: &Harness,
    sku: &str,
    name_fr: &str,
    name_ar: &str,
    unit_fr: &str,
    unit_ar: &str,
    price: i64,
) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id,
                sku: Sku::new(sku).expect("sku"),
                barcode: None,
                name_ar: name_ar.to_string(),
                name_fr: name_fr.to_string(),
                category_id: None,
                is_kit: false,
                is_active: true,
            },
            &[ProductUnit {
                id: unit_id,
                product_id: id,
                unit_name_ar: unit_ar.to_string(),
                unit_name_fr: unit_fr.to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: None,
                sale_price: Money::from_centimes(price),
                purchase_price: Money::from_centimes(price - 2_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("product");
    (id, unit_id)
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
        discount_per_unit_centimes: None,
        tva_rate_micro: 0,
        line_ht_centimes: unit_price * qty_units,
        line_tva_centimes: 0,
        line_ttc_centimes: unit_price * qty_units,
        cost_price_centimes: unit_price - 2_000,
        sort_order,
    }
}

fn seed_document(
    harness: &Harness,
    warehouse: WarehouseId,
    number: &str,
    items: Vec<DocumentItem>,
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
                fiscal_year: 2026,
                doc_type: DocumentType::Ticket,
                status: DocumentStatus::Finalized,
                warehouse_id: warehouse,
                party_id: None,
                subtotal_ht_centimes: 40_000,
                vat_total_centimes: 0,
                global_discount_centimes: 0,
                timbre_centimes: 0,
                ttc_centimes: 40_000,
                paid_amount_centimes: 0,
                change_amount_centimes: 0,
                payment_method: "UNPAID".to_string(),
                notes: None,
                created_at: chrono::Utc::now(),
                created_by: UserId::from(Uuid::parse_str(GERANT_UUID).expect("uuid")),
            },
            &fixed,
            &[],
            TEST_TERMINAL,
        )
        .expect("seed document");
    id
}

/// 2-line sale: every details item carries FR + AR designation, sku and unit
/// labels from a single JOIN query (no per-item N+1 — one call, all rows).
#[test]
fn details_items_carry_designation_in_both_locales() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "DET-1");
    let draft = DocumentId::new();
    let (p1, u1) = seed_product(
        &harness,
        "DET-S1",
        "Café Arabica 250g",
        "قهوة عربية 250غ",
        "Paquet",
        "علبة",
        10_000,
    );
    let (p2, u2) = seed_product(
        &harness,
        "DET-S2",
        "Thé Vert 100g",
        "شاي أخضر 100غ",
        "Boîte",
        "علبة معدنية",
        20_000,
    );
    let id = seed_document(
        &harness,
        warehouse,
        "T-2026-000001",
        vec![
            line(draft, p1, u1, 2, 10_000, 0),
            line(draft, p2, u2, 1, 20_000, 1),
        ],
    );

    let views = harness.documents.get_items_with_names(id).expect("views");

    assert_eq!(views.len(), 2, "both lines returned from one call");
    assert_eq!(views[0].item.product_id, p1);
    assert_eq!(
        views[0].product_name_fr.as_deref(),
        Some("Café Arabica 250g")
    );
    assert_eq!(views[0].product_name_ar.as_deref(), Some("قهوة عربية 250غ"));
    assert_eq!(views[0].sku.as_deref(), Some("DET-S1"));
    assert_eq!(views[0].unit_name_fr.as_deref(), Some("Paquet"));
    assert_eq!(views[0].unit_name_ar.as_deref(), Some("علبة"));
    assert!(!views[0].product_missing);
    assert_eq!(views[1].item.product_id, p2);
    assert_eq!(views[1].product_name_fr.as_deref(), Some("Thé Vert 100g"));
    assert_eq!(views[1].product_name_ar.as_deref(), Some("شاي أخضر 100غ"));
    assert_eq!(views[1].sku.as_deref(), Some("DET-S2"));
    assert_eq!(views[1].unit_name_fr.as_deref(), Some("Boîte"));
    assert_eq!(views[1].unit_name_ar.as_deref(), Some("علبة معدنية"));
    assert!(!views[1].product_missing);
}

/// Missing product row (orphaned line): the line still returns, name fields
/// are null, `product_missing` is set — totals/row-count unaffected.
#[test]
fn details_missing_product_degrades_gracefully() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "DET-2");
    let draft = DocumentId::new();
    let (p1, u1) = seed_product(
        &harness,
        "DET-M1",
        "Sucre 1kg",
        "سكر 1كغ",
        "Sachet",
        "كيس",
        5_000,
    );
    let (p2, u2) = seed_product(
        &harness,
        "DET-M2",
        "Lait 1L",
        "حليب 1ل",
        "Bouteille",
        "قارورة",
        8_000,
    );
    let id = seed_document(
        &harness,
        warehouse,
        "T-2026-000002",
        vec![
            line(draft, p1, u1, 1, 5_000, 0),
            line(draft, p2, u2, 1, 8_000, 1),
        ],
    );
    // Simulate a legacy orphan row: FK-off delete of the second product
    // (live deletes are RESTRICT-blocked + BRK-080 deactivates instead).
    {
        let guard = harness.db.lock().expect("lock");
        guard
            .execute_batch("PRAGMA foreign_keys=OFF;")
            .expect("pragma off");
        guard
            .execute("DELETE FROM products WHERE id = ?1", [p2.to_string()])
            .expect("orphan product");
        guard
            .execute_batch("PRAGMA foreign_keys=ON;")
            .expect("pragma on");
    }

    let views = harness.documents.get_items_with_names(id).expect("views");

    assert_eq!(views.len(), 2, "orphaned line is not dropped");
    assert!(!views[0].product_missing);
    assert!(views[1].product_missing);
    assert!(views[1].product_name_fr.is_none());
    assert!(views[1].product_name_ar.is_none());
    assert!(views[1].sku.is_none());
}
