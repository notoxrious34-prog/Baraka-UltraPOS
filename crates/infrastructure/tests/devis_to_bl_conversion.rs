//! End-to-end DEVIS-to-BL conversion tests over a real SQLite backend.
//!
//! File-based temporary database (WAL/crash standards): every test opens
//! `conversion.db` in a fresh tempdir, runs all migrations, and asserts
//! atomicity by counting rows before and after each rejected attempt.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    DocumentRepository, ProductRepository, SessionContext, SessionToken, StockRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::sales::{ConvertDevisToBlInput, ConvertDevisToBlUseCase};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    Document, DocumentId, DocumentItem, DocumentLanguage, DocumentStatus, DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteDocumentRepository, SqliteStockRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const GERANT_UUID: &str = "55555555-5555-5555-5555-555555555555";
const VENDEUR_UUID: &str = "44444444-4444-4444-4444-444444444444";
const TEST_TERMINAL: &str = "TEST-TERMINAL-014A";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    documents: SqliteDocumentRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("conversion.db")).expect("open");
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
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
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
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("product");
    (id, unit_id)
}

fn seed_stock(harness: &Harness, product: ProductId, warehouse: WarehouseId, qty: i64) {
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, qty)
        .expect("stock");
}

/// One frozen DEVIS line snapshot (mirrors checkout snapshot semantics).
fn devis_line(
    document_id: DocumentId,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    unit_price: i64,
    sort_order: i64,
) -> DocumentItem {
    let micros = qty_units * 10_000;
    let total = unit_price * qty_units;
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
        line_ht_centimes: total,
        line_tva_centimes: 0,
        line_ttc_centimes: total,
        cost_price_centimes: unit_price - 2_000,
        sort_order,
    }
}

/// Persist a DEVIS with the given status (FINALIZED for the happy path).
fn seed_devis(
    harness: &Harness,
    warehouse: WarehouseId,
    status: DocumentStatus,
    number: &str,
    lines: Vec<DocumentItem>,
) -> DocumentId {
    let id = DocumentId::new();
    let ttc: i64 = lines.iter().map(|line| line.line_ttc_centimes).sum();
    let mut fixed_lines = Vec::with_capacity(lines.len());
    for mut line in lines {
        line.document_id = id;
        fixed_lines.push(line);
    }
    let document = Document {
        id,
        language: DocumentLanguage::Fr,
        number: number.to_string(),
        fiscal_year: current_year(),
        doc_type: DocumentType::Devis,
        status,
        warehouse_id: warehouse,
        party_id: None,
        subtotal_ht_centimes: ttc,
        vat_total_centimes: 0,
        global_discount_centimes: 0,
        timbre_centimes: 0,
        ttc_centimes: ttc,
        paid_amount_centimes: 0,
        change_amount_centimes: 0,
        payment_method: "UNPAID".to_string(),
        notes: Some("Devis test".to_string()),
        created_at: chrono::Utc::now(),
        created_by: user_id(GERANT_UUID),
    };
    harness
        .documents
        .finalize_document(&document, &fixed_lines, &[], TEST_TERMINAL)
        .expect("seed devis");
    id
}

/// Persist a finalized non-DEVIS document (BL) for source-type rejection.
fn seed_finalized_bl(harness: &Harness, warehouse: WarehouseId, number: &str) -> DocumentId {
    let id = DocumentId::new();
    let document = Document {
        id,
        language: DocumentLanguage::Fr,
        number: number.to_string(),
        fiscal_year: current_year(),
        doc_type: DocumentType::Bl,
        status: DocumentStatus::Finalized,
        warehouse_id: warehouse,
        party_id: None,
        subtotal_ht_centimes: 0,
        vat_total_centimes: 0,
        global_discount_centimes: 0,
        timbre_centimes: 0,
        ttc_centimes: 0,
        paid_amount_centimes: 0,
        change_amount_centimes: 0,
        payment_method: "UNPAID".to_string(),
        notes: None,
        created_at: chrono::Utc::now(),
        created_by: user_id(GERANT_UUID),
    };
    harness
        .documents
        .finalize_document(&document, &[], &[], TEST_TERMINAL)
        .expect("seed bl");
    id
}

fn convert_uc<'a>(
    harness: &'a Harness,
) -> ConvertDevisToBlUseCase<'a, SqliteCatalogRepository, SqliteDocumentRepository> {
    ConvertDevisToBlUseCase::new(&harness.catalog, &harness.documents)
}

fn convert_input(source: DocumentId) -> ConvertDevisToBlInput {
    ConvertDevisToBlInput {
        command_id: Uuid::new_v4().to_string(),
        source_document_id: source,
    }
}

fn read_conn(harness: &Harness) -> rusqlite::Connection {
    rusqlite::Connection::open(harness._dir.path().join("conversion.db")).expect("reopen")
}

fn count(harness: &Harness, table: &str) -> i64 {
    read_conn(harness)
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn current_year() -> i32 {
    chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .unwrap_or(1970)
}

fn doc_status(harness: &Harness, id: DocumentId) -> String {
    read_conn(harness)
        .query_row(
            "SELECT status FROM documents WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .expect("status")
}

fn stock_level(harness: &Harness, product: ProductId, warehouse: WarehouseId) -> i64 {
    harness
        .stock
        .get_stock_level(product, warehouse)
        .expect("level")
}

#[test]
fn convert_finalized_devis_creates_bl_and_seals_source() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    let (p2, u2) = seed_product(&harness, "P2", 20_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    seed_stock(&harness, p2, warehouse, 100_000);
    let devis = DocumentId::new();
    let lines = vec![
        devis_line(devis, p1, u1, 2, 10_000, 0),
        devis_line(devis, p2, u2, 1, 20_000, 1),
    ];
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        lines,
    );

    let result = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect("convert");
    let year = current_year();
    assert_eq!(result.new_document_number, format!("BL-{year}-000001"));
    assert_eq!(result.new_document_status, "FINALIZED");
    assert_eq!(result.total_ttc_centimes, 40_000);
    assert_eq!(result.items_count, 2);
    assert!(result.warnings.is_empty());

    // BL header: source link, copied warehouse, copied totals.
    let conn = read_conn(&harness);
    let header: (String, String, String, String, i64, String) = conn
        .query_row(
            "SELECT type, status, source_document_id, warehouse_id, ttc_centimes, origin_device_id
             FROM documents WHERE id = ?1",
            [result.new_document_id.clone()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("bl header");
    assert_eq!(header.0, "BL");
    assert_eq!(header.1, "FINALIZED");
    assert_eq!(header.2, devis.to_string());
    assert_eq!(header.3, warehouse.to_string());
    assert_eq!(header.4, 40_000);
    assert_eq!(header.5, TEST_TERMINAL);

    // BL items: byte-for-byte copies of the source lines.
    let source_items = harness.documents.get_items(devis).expect("source items");
    let bl_items = harness
        .documents
        .get_items(DocumentId::from(
            Uuid::parse_str(&result.new_document_id).expect("uuid"),
        ))
        .expect("bl items");
    assert_eq!(bl_items.len(), source_items.len());
    for (actual, expected) in bl_items.iter().zip(source_items.iter()) {
        assert_eq!(actual.document_id.to_string(), result.new_document_id);
        assert_ne!(actual.id, expected.id);
        assert_eq!(actual.product_id, expected.product_id);
        assert_eq!(actual.unit_id, expected.unit_id);
        assert_eq!(actual.quantity_micro, expected.quantity_micro);
        assert_eq!(actual.quantity_base_micro, expected.quantity_base_micro);
        assert_eq!(actual.unit_price_centimes, expected.unit_price_centimes);
        assert_eq!(actual.discount_centimes, expected.discount_centimes);
        assert_eq!(actual.tva_rate_micro, expected.tva_rate_micro);
        assert_eq!(actual.line_ht_centimes, expected.line_ht_centimes);
        assert_eq!(actual.line_tva_centimes, expected.line_tva_centimes);
        assert_eq!(actual.line_ttc_centimes, expected.line_ttc_centimes);
        assert_eq!(actual.cost_price_centimes, expected.cost_price_centimes);
        assert_eq!(actual.sort_order, expected.sort_order);
    }

    // Source sealed.
    assert_eq!(doc_status(&harness, devis), "CONVERTED");

    // Stock decremented exactly.
    assert_eq!(stock_level(&harness, p1, warehouse), 80_000);
    assert_eq!(stock_level(&harness, p2, warehouse), 90_000);

    // One SALE_OUT movement per line, referencing the new BL.
    let movements: Vec<(String, String)> = {
        let mut statement = conn
            .prepare("SELECT movement_type, reference_id FROM stock_movements ORDER BY rowid")
            .expect("movements");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect")
    };
    assert_eq!(movements.len(), 2);
    for (movement_type, reference_id) in &movements {
        assert_eq!(movement_type, "SALE_OUT");
        assert_eq!(reference_id, &result.new_document_id);
    }

    // Print outbox + audit trail.
    let job: (String, String) = conn
        .query_row(
            "SELECT status, job_type FROM print_jobs WHERE document_id = ?1",
            [result.new_document_id.clone()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("print job");
    assert_eq!(job, ("PENDING".to_string(), "RECEIPT".to_string()));
    let audit: (String, String) = conn
        .query_row(
            "SELECT action, new_value_json FROM audit_log WHERE action = 'DOCUMENT_CONVERTED'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("audit");
    assert_eq!(audit.0, "DOCUMENT_CONVERTED");
    assert!(audit.1.contains(&devis.to_string()));
    assert!(audit.1.contains(&result.new_document_id));
    assert!(audit.1.contains(TEST_TERMINAL));
}

#[test]
fn second_conversion_returns_already_converted_with_zero_side_effects() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        vec![devis_line(devis, p1, u1, 1, 10_000, 0)],
    );
    convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect("first");
    let before = (
        count(&harness, "documents"),
        count(&harness, "stock_movements"),
        count(&harness, "print_jobs"),
        count(&harness, "command_log"),
    );
    let stock_before = stock_level(&harness, p1, warehouse);

    let mut second = convert_input(devis);
    second.command_id = Uuid::new_v4().to_string();
    let error = convert_uc(&harness)
        .execute(&session_with(Role::Gerant), second, TEST_TERMINAL)
        .expect_err("second conversion");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::DocumentAlreadyConverted)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(
        (
            count(&harness, "documents"),
            count(&harness, "stock_movements"),
            count(&harness, "print_jobs"),
            count(&harness, "command_log"),
        ),
        before
    );
    assert_eq!(stock_level(&harness, p1, warehouse), stock_before);
}

#[test]
fn draft_devis_is_rejected_with_zero_side_effects() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Draft,
        "DEV-1",
        vec![devis_line(devis, p1, u1, 1, 10_000, 0)],
    );
    let error = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect_err("draft");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidConversionSourceStatus)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "command_log"), 0);
    assert_eq!(stock_level(&harness, p1, warehouse), 100_000);
}

#[test]
fn cancelled_devis_is_rejected_with_zero_side_effects() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Cancelled,
        "DEV-1",
        vec![devis_line(devis, p1, u1, 1, 10_000, 0)],
    );
    let error = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect_err("cancelled");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidConversionSourceStatus)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "stock_movements"), 0);
}

#[test]
fn non_devis_source_is_rejected_with_zero_side_effects() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let bl = seed_finalized_bl(&harness, warehouse, "BL-1");
    let error = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(bl),
            TEST_TERMINAL,
        )
        .expect_err("non-devis");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidConversionSourceType)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "stock_movements"), 0);
    let _ = (p1, u1);
}

#[test]
fn missing_source_returns_not_found() {
    let harness = setup();
    seed_warehouse(&harness, "MAIN");
    let error = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(DocumentId::new()),
            TEST_TERMINAL,
        )
        .expect_err("missing");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::SourceDocumentNotFound)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 0);
}

#[test]
fn empty_devis_is_rejected() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let _placeholder = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        Vec::new(),
    );
    let error = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect_err("empty");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::EmptySourceDocument)
        ),
        "unexpected: {error:?}"
    );
    assert_eq!(count(&harness, "documents"), 1);
}

#[test]
fn insufficient_stock_without_override_aborts_fully_and_names_product() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 5_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        vec![devis_line(devis, p1, u1, 2, 10_000, 0)],
    );
    // Vendeur holds CreateSale but NOT NegativeStockOverride.
    let error = convert_uc(&harness)
        .execute(
            &session_with(Role::Vendeur),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect_err("short stock");
    match &error {
        ApplicationError::Validation(DomainError::InvalidCanonicalString(message)) => {
            assert!(
                message.contains("P1"),
                "error must name the product: {message}"
            );
        }
        other => panic!("unexpected error shape: {other:?}"),
    }
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "print_jobs"), 0);
    assert_eq!(count(&harness, "command_log"), 0);
    assert_eq!(stock_level(&harness, p1, warehouse), 5_000);
    assert_eq!(doc_status(&harness, devis), "FINALIZED");
}

#[test]
fn insufficient_stock_with_override_warns_and_audits() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 5_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        vec![devis_line(devis, p1, u1, 2, 10_000, 0)],
    );
    // Gerant holds NegativeStockOverride.
    let result = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect("override conversion");
    assert_eq!(result.warnings.len(), 1);
    assert!(
        result.warnings[0].contains("P1") && result.warnings[0].contains("unités manquantes"),
        "unexpected warning: {:?}",
        result.warnings
    );
    assert_eq!(stock_level(&harness, p1, warehouse), -15_000);
    let conn = read_conn(&harness);
    let override_flag: i64 = conn
        .query_row(
            "SELECT authorized_negative_stock FROM stock_movements",
            [],
            |row| row.get(0),
        )
        .expect("flag");
    assert_eq!(override_flag, 1);
    let audit_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action = 'NEGATIVE_STOCK_OVERRIDE'",
            [],
            |row| row.get(0),
        )
        .expect("audit count");
    assert_eq!(audit_count, 1);
}

#[test]
fn idempotent_replay_returns_identical_result_without_duplicates() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        vec![devis_line(devis, p1, u1, 1, 10_000, 0)],
    );
    let command_id = Uuid::new_v4().to_string();
    let first = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            ConvertDevisToBlInput {
                command_id: command_id.clone(),
                source_document_id: devis,
            },
            TEST_TERMINAL,
        )
        .expect("first");
    let second = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            ConvertDevisToBlInput {
                command_id: command_id.clone(),
                source_document_id: devis,
            },
            TEST_TERMINAL,
        )
        .expect("replay");
    assert_eq!(first, second);
    assert_eq!(count(&harness, "documents"), 2);
    assert_eq!(count(&harness, "stock_movements"), 1);
    assert_eq!(count(&harness, "print_jobs"), 1);
    // Same command id, different payload (a second, valid DEVIS): conflict,
    // zero writes. The payload must survive use-case pre-validation so the
    // conflict surfaces from the in-transaction idempotency guard.
    let devis2 = DocumentId::new();
    let devis2 = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-2",
        vec![devis_line(devis2, p1, u1, 2, 10_000, 0)],
    );
    let conflict = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            ConvertDevisToBlInput {
                command_id,
                source_document_id: devis2,
            },
            TEST_TERMINAL,
        )
        .expect_err("conflict");
    assert!(
        matches!(conflict, ApplicationError::IdempotencyConflict),
        "unexpected: {conflict:?}"
    );
    assert_eq!(count(&harness, "documents"), 3);
    assert_eq!(count(&harness, "stock_movements"), 1);
    assert_eq!(count(&harness, "print_jobs"), 1);
}

#[test]
fn late_failure_rolls_back_earlier_stock_writes() {
    // Two lines: the first moves stock, the second fails. Atomicity demands
    // zero visible writes — the same guarantee a crash before COMMIT gives.
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    let (p2, u2) = seed_product(&harness, "P2", 20_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    seed_stock(&harness, p2, warehouse, 1_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        vec![
            devis_line(devis, p1, u1, 1, 10_000, 0),
            devis_line(devis, p2, u2, 2, 20_000, 1),
        ],
    );
    convert_uc(&harness)
        .execute(
            &session_with(Role::Vendeur),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect_err("late failure");
    assert_eq!(count(&harness, "documents"), 1);
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "print_jobs"), 0);
    assert_eq!(count(&harness, "command_log"), 0);
    assert_eq!(stock_level(&harness, p1, warehouse), 100_000);
    assert_eq!(stock_level(&harness, p2, warehouse), 1_000);
    assert_eq!(doc_status(&harness, devis), "FINALIZED");
}

#[test]
fn kit_devis_expands_to_leaves_only() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (leaf, leaf_unit) = seed_product(&harness, "LEAF", 5_000);
    let kit_id = ProductId::new();
    let kit_unit = UnitId::new();
    harness
        .catalog
        .create_product(
            &Product {
                id: kit_id,
                sku: Sku::new("KIT").expect("sku"),
                barcode: None,
                name_ar: "طقم".to_string(),
                name_fr: "Produit KIT".to_string(),
                category_id: None,
                is_kit: true,
                is_active: true,
            },
            &[ProductUnit {
                id: kit_unit,
                product_id: kit_id,
                unit_name_ar: "قطعة".to_string(),
                unit_name_fr: "Pièce".to_string(),
                factor_micro: FactorMicro::from_micros(10_000),
                barcode: Some(Barcode::new("611125KIT").expect("barcode")),
                sale_price: Money::from_centimes(12_000),
                purchase_price: Money::from_centimes(10_000),
                wholesale_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            TEST_TERMINAL,
        )
        .expect("kit header");
    harness
        .catalog
        .define_kit(
            kit_id,
            &[baraka_domain::catalog::KitItem {
                kit_product_id: kit_id,
                component_product_id: leaf,
                unit_id: leaf_unit,
                quantity_micro: baraka_domain::primitives::QtyMicro::from_micros(20_000),
            }],
        )
        .expect("bom");
    seed_stock(&harness, leaf, warehouse, 100_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        vec![devis_line(devis, kit_id, kit_unit, 3, 12_000, 0)],
    );
    let result = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect("kit conversion");
    // 3 kits x 2 leaves = 6 base units deducted from the leaf only.
    assert_eq!(stock_level(&harness, leaf, warehouse), 40_000);
    assert_eq!(stock_level(&harness, kit_id, warehouse), 0);
    let conn = read_conn(&harness);
    let leaf_movements: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
            [leaf.to_string()],
            |row| row.get(0),
        )
        .expect("leaf movements");
    assert_eq!(leaf_movements, 1);
    // BL items still carry the kit header line (byte copy, not expansion).
    let bl_items = harness
        .documents
        .get_items(DocumentId::from(
            Uuid::parse_str(&result.new_document_id).expect("uuid"),
        ))
        .expect("bl items");
    assert_eq!(bl_items.len(), 1);
    assert_eq!(bl_items[0].product_id, kit_id);
}

#[test]
fn conversion_origin_device_matches_terminal_context() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (p1, u1) = seed_product(&harness, "P1", 10_000);
    seed_stock(&harness, p1, warehouse, 100_000);
    let devis = DocumentId::new();
    let devis = seed_devis(
        &harness,
        warehouse,
        DocumentStatus::Finalized,
        "DEV-1",
        vec![devis_line(devis, p1, u1, 1, 10_000, 0)],
    );
    let result = convert_uc(&harness)
        .execute(
            &session_with(Role::Gerant),
            convert_input(devis),
            TEST_TERMINAL,
        )
        .expect("convert");
    let conn = read_conn(&harness);
    let origin: String = conn
        .query_row(
            "SELECT origin_device_id FROM documents WHERE id = ?1",
            [result.new_document_id.clone()],
            |row| row.get(0),
        )
        .expect("origin");
    assert_eq!(origin, TEST_TERMINAL);
}

#[test]
fn migration_0011_source_document_index_exists() {
    // Authoritative proof that no CHECK rebuild was needed: CONVERTED rows
    // persist, and the new lookup index exists.
    let harness = setup();
    let conn = read_conn(&harness);
    let index: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master
             WHERE type = 'index' AND name = 'idx_documents_source_document'",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .expect("index lookup");
    assert_eq!(index.as_deref(), Some("idx_documents_source_document"));
}
