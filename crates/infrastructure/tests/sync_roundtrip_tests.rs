//! Two-till sync round-trip over real SQLite files (TASK-BRK-124).
//!
//! Origin till ("TILL-A") holds two finalized local sales (one cash
//! ticket, one credit BL); the importer ("TILL-B") merges the exported
//! package atomically: local gapless renumber, origin identity preserved,
//! stock applied to the importer's warehouse, ledger legs recomputed, and
//! imported payments kept out of every cash session. Re-import aborts on
//! the sync-identity index (zero double-count); tampered bytes and unknown
//! SKUs abort with zero writes.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    DocumentRepository, NewCustomer, ProductRepository, StoredPayment, SyncExportPort,
    SyncImportPort, WarehouseRepository,
};
use baraka_application::use_cases::customers::CreateCustomerUseCase;
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, CustomerTier, Document, DocumentId, DocumentItem, DocumentLanguage,
    DocumentStatus, DocumentType,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCustomerRepository, SqliteDocumentRepository,
    SqliteSyncRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::{TimeDelta, Utc};
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const ORIGIN_TERMINAL: &str = "TILL-A";
const IMPORTER_TERMINAL: &str = "TILL-B";

struct Till {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    warehouses: SqliteWarehouseRepository,
    catalog: SqliteCatalogRepository,
    documents: SqliteDocumentRepository,
    customers: SqliteCustomerRepository,
    sync: SqliteSyncRepository,
}

fn open_till() -> Till {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("till.db");
    let mut conn = open_connection(&path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('77777777-7777-7777-7777-777777777777', 'gerant1', 'hash', 'GERANT',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Till {
        _dir: dir,
        db: Arc::clone(&shared),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        sync: SqliteSyncRepository::new(Arc::clone(&shared)),
    }
}

fn session_with(role: Role) -> baraka_application::ports::SessionContext {
    use baraka_application::ports::{SessionContext, SessionToken};
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        role,
        expires_at: Utc::now() + TimeDelta::hours(1),
    }
}

/// Seed one warehouse, one base-unit product with stock, one customer.
/// Returns (warehouse, product, unit, customer code).
fn seed_retail(till: &Till) -> (WarehouseId, ProductId, UnitId, String) {
    let warehouse = Warehouse {
        id: WarehouseId::new(),
        code: WarehouseCode::new("MAIN").expect("code"),
        name: "Main".to_string(),
        is_active: true,
    };
    till.warehouses
        .create_warehouse(&warehouse, ORIGIN_TERMINAL)
        .expect("warehouse");
    let product_id = ProductId::new();
    let unit_id = UnitId::new();
    till.catalog
        .create_product(
            &Product {
                id: product_id,
                sku: Sku::new("BK1").expect("sku"),
                barcode: None,
                name_ar: "منتج".to_string(),
                name_fr: "Produit".to_string(),
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
                barcode: Some(Barcode::new("611125BK1001").expect("barcode")),
                sale_price: Money::from_centimes(10_000),
                purchase_price: Money::from_centimes(8_000),
                wholesale_price: None,
                super_gros_price: None,
                is_base_unit: true,
                is_default_selling: true,
            }],
            ORIGIN_TERMINAL,
        )
        .expect("product");
    till.db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO stock_levels (warehouse_id, product_id, quantity_micro, quantity_base_micro)
             VALUES (?1, ?2, 100000, 100000)",
            rusqlite::params![warehouse.id.to_string(), product_id.to_string()],
        )
        .expect("stock level");
    let customer: CustomerId = CreateCustomerUseCase::new(&till.customers)
        .execute(
            &session_with(Role::Gerant),
            NewCustomer {
                name: "Débiteur".to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 100_000,
                customer_tier: CustomerTier::Retail,
            },
            ORIGIN_TERMINAL,
        )
        .expect("customer");
    let code: String = till
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT code FROM customers WHERE id = ?1",
            [customer.to_string()],
            |row| row.get(0),
        )
        .expect("customer code");
    (warehouse.id, product_id, unit_id, code)
}

#[allow(clippy::too_many_arguments)]
fn seed_finalized_doc(
    till: &Till,
    number: &str,
    doc_type: DocumentType,
    party: Option<CustomerId>,
    paid: i64,
    pays: Vec<StoredPayment>,
) {
    use baraka_domain::printing::TSPL_TICKET_TEMPLATE_VERSION;
    let id = DocumentId::new();
    let line_id = Uuid::new_v4();
    let product_id: ProductId = till
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT id FROM products WHERE sku = 'BK1'", [], |row| {
            row.get::<_, String>(0)
        })
        .map(|raw| ProductId::from(Uuid::parse_str(&raw).expect("uuid")))
        .expect("product id");
    let unit_id: UnitId = till
        .db
        .lock()
        .expect("lock")
        .query_row(
            "SELECT id FROM product_units WHERE product_id = ?1 AND is_default_selling = 1",
            [product_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .map(|raw| UnitId::from(Uuid::parse_str(&raw).expect("uuid")))
        .expect("unit id");
    let warehouse_id: WarehouseId = till
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT id FROM warehouses WHERE code = 'MAIN'", [], |row| {
            row.get::<_, String>(0)
        })
        .map(|raw| WarehouseId::from(Uuid::parse_str(&raw).expect("uuid")))
        .expect("warehouse id");
    let doc_pays: Vec<StoredPayment> = pays
        .into_iter()
        .map(|pay| StoredPayment {
            document_id: id,
            ..pay
        })
        .collect();
    till.documents
        .finalize_document(
            &Document {
                id,
                language: DocumentLanguage::Fr,
                number: number.to_string(),
                fiscal_year: 2026,
                doc_type,
                status: DocumentStatus::Finalized,
                warehouse_id,
                party_id: party.map(|customer| customer.to_string()),
                subtotal_ht_centimes: 10_000,
                vat_total_centimes: 0,
                global_discount_centimes: 0,
                timbre_centimes: 0,
                ttc_centimes: 10_000,
                paid_amount_centimes: paid,
                change_amount_centimes: 0,
                payment_method: "CASH".to_string(),
                template_version: TSPL_TICKET_TEMPLATE_VERSION,
                credit_solde_before_centimes: None,
                credit_solde_after_centimes: None,
                credit_delta_centimes: None,
                notes: None,
                created_at: Utc::now(),
                created_by: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
            },
            &[DocumentItem {
                id: line_id,
                document_id: id,
                product_id,
                unit_id,
                quantity_micro: 10_000,
                quantity_base_micro: 10_000,
                unit_price_centimes: 10_000,
                discount_centimes: 0,
                discount_per_unit_centimes: None,
                tva_rate_micro: 0,
                line_ht_centimes: 10_000,
                line_tva_centimes: 0,
                line_ttc_centimes: 10_000,
                cost_price_centimes: 8_000,
                sort_order: 0,
            }],
            &doc_pays,
            ORIGIN_TERMINAL,
        )
        .expect("seed document");
}

fn count(conn: &rusqlite::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .expect("count")
}

fn fresh_payment(amount: i64, method: &str, customer: Option<CustomerId>) -> StoredPayment {
    StoredPayment {
        id: Uuid::new_v4().to_string(),
        document_id: DocumentId::new(),
        customer_id: customer,
        amount_centimes: amount,
        method_code: method.to_string(),
        cash_session_id: None,
    }
}

#[test]
fn export_import_round_trip_then_reimport_aborts() {
    let origin = open_till();
    let importer = open_till();
    let (_, _, _, origin_customer_code) = seed_retail(&origin);
    let (importer_warehouse, _, _, _) = seed_retail(&importer);
    // Same customer base on both tills (initial clone): align the code.
    let importer_customer: String = importer
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT id FROM customers LIMIT 1", [], |row| {
            row.get::<_, String>(0)
        })
        .expect("importer customer");
    importer
        .db
        .lock()
        .expect("lock")
        .execute(
            "UPDATE customers SET code = ?1 WHERE id = ?2",
            rusqlite::params![origin_customer_code, importer_customer],
        )
        .expect("align customer code");

    // Two finalized local sales on the origin till.
    seed_finalized_doc(
        &origin,
        "T-000001",
        DocumentType::Ticket,
        None,
        10_000,
        vec![fresh_payment(10_000, "CASH", None)],
    );
    let credit_customer: CustomerId = origin
        .db
        .lock()
        .expect("lock")
        .query_row("SELECT id FROM customers LIMIT 1", [], |row| {
            row.get::<_, String>(0)
        })
        .map(|raw: String| CustomerId::from(Uuid::parse_str(&raw).expect("uuid")))
        .expect("credit customer");
    seed_finalized_doc(
        &origin,
        "BL-000001",
        DocumentType::Bl,
        Some(credit_customer),
        4_000,
        vec![
            fresh_payment(4_000, "CASH", None),
            fresh_payment(6_000, "CREDIT", Some(credit_customer)),
        ],
    );

    // Export (read-only): two docs, verifiable package.
    let (package, exported) = origin
        .sync
        .export_package(ORIGIN_TERMINAL, "2020-01-01T00:00:00Z", 500)
        .expect("export");
    assert_eq!(exported, 2);
    assert!(package.starts_with("HDR|SYNC1|TILL-A|"));
    // Independent verification: header hash covers the exact body bytes.
    let (origin_terminal, _, doc_count, hash_hex) =
        baraka_domain::sync::parse_header(package.lines().next().expect("header"))
            .expect("header parses");
    assert_eq!(origin_terminal, ORIGIN_TERMINAL);
    assert_eq!(doc_count, 2);
    let body = package.lines().skip(1).collect::<Vec<_>>().join("\n");
    assert_eq!(
        baraka_infrastructure::persistence::sha256::sha256_hex(body.as_bytes()),
        hash_hex
    );

    // Import: one atomic merge, local gapless renumber.
    let outcome = importer
        .sync
        .import_package(
            &package,
            &importer_warehouse.to_string(),
            ACTOR_UUID,
            IMPORTER_TERMINAL,
            "2026-11-24T10:00:00Z",
        )
        .expect("import");
    assert_eq!(outcome.imported_docs, 2);
    assert_eq!(outcome.renumbered.len(), 2);
    assert!(outcome.negative_stock_warnings.is_empty());
    let guard = importer.db.lock().expect("lock");
    // Local gapless numbers, origin identity preserved verbatim.
    for row in &outcome.renumbered {
        let origin_number: String = guard
            .query_row(
                "SELECT origin_number FROM documents WHERE number = ?1",
                [&row.new_number],
                |r| r.get(0),
            )
            .expect("origin preserved");
        assert_eq!(origin_number, row.origin_number);
        let origin_device: String = guard
            .query_row(
                "SELECT origin_device_id FROM documents WHERE number = ?1",
                [&row.new_number],
                |r| r.get(0),
            )
            .expect("origin device");
        assert_eq!(origin_device, ORIGIN_TERMINAL);
    }
    // Stock applied on the importer's warehouse (2 sales x 1 base unit).
    let stock: i64 = guard
        .query_row(
            "SELECT quantity_base_micro FROM stock_levels WHERE warehouse_id = ?1",
            [importer_warehouse.to_string()],
            |r| r.get(0),
        )
        .expect("stock");
    assert_eq!(stock, 100_000 - 20_000);
    // Imported payments never join a cash session (money stays put).
    let sessioned: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM payments WHERE cash_session_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .expect("sessioned payments");
    assert_eq!(sessioned, 0);
    // Credit leg recomputed locally (6_000 on the aligned customer).
    let balance: i64 = guard
        .query_row(
            "SELECT current_balance_centimes FROM customers WHERE code = ?1",
            [&origin_customer_code],
            |r| r.get(0),
        )
        .expect("balance");
    assert_eq!(balance, 6_000);
    let ledgers = count(&guard, "customer_ledger");
    assert_eq!(ledgers, 1);
    drop(guard);

    // Re-import: the sync-identity index aborts, zero double-count.
    let before_docs = {
        let guard = importer.db.lock().expect("lock");
        count(&guard, "documents")
    };
    let error = importer
        .sync
        .import_package(
            &package,
            &importer_warehouse.to_string(),
            ACTOR_UUID,
            IMPORTER_TERMINAL,
            "2026-11-24T10:00:00Z",
        )
        .expect_err("re-import must abort");
    let message = format!("{error:?}");
    assert!(
        message.contains("SYNC_ALREADY_IMPORTED"),
        "unexpected error: {message}"
    );
    let after_docs = {
        let guard = importer.db.lock().expect("lock");
        count(&guard, "documents")
    };
    assert_eq!(before_docs, after_docs);
}

#[test]
fn tampered_package_aborts_with_zero_writes() {
    let importer = open_till();
    let (warehouse, _, _, _) = seed_retail(&importer);
    let before = {
        let guard = importer.db.lock().expect("lock");
        (count(&guard, "documents"), count(&guard, "stock_movements"))
    };
    // Valid header shape but a lying hash: the integrity gate must fire
    // BEFORE any grammar parsing or write.
    let header = baraka_domain::sync::build_header(
        ORIGIN_TERMINAL,
        "2026-11-24T10:00:00Z",
        1,
        &"0".repeat(64),
    );
    let tampered = format!(
        "{header}\nDOC|T-9|TICKET|2026|FR||100|0|0|0|0|100|100|0|CASH|2026-11-24T09:00:00Z"
    );
    let error = importer
        .sync
        .import_package(
            &tampered,
            &warehouse.to_string(),
            ACTOR_UUID,
            IMPORTER_TERMINAL,
            "2026-11-24T10:00:00Z",
        )
        .expect_err("tamper must abort");
    assert!(format!("{error:?}").contains("SYNC_HASH_MISMATCH"));
    let after = {
        let guard = importer.db.lock().expect("lock");
        (count(&guard, "documents"), count(&guard, "stock_movements"))
    };
    assert_eq!(before, after);
}

#[test]
fn unknown_sku_aborts_with_zero_writes() {
    use baraka_domain::sync as sync_format;
    let importer = open_till();
    let (warehouse, _, _, _) = seed_retail(&importer);
    let doc = sync_format::SyncDoc {
        doc_type_code: "TICKET".to_string(),
        fiscal_year: 2026,
        origin_number: "T-000007".to_string(),
        language: "FR".to_string(),
        party_code: None,
        subtotal_ht_centimes: 100,
        vat_total_centimes: 0,
        global_discount_centimes: 0,
        timbre_centimes: 0,
        rounding_centimes: 0,
        ttc_centimes: 100,
        paid_amount_centimes: 100,
        change_amount_centimes: 0,
        payment_method: "CASH".to_string(),
        created_at: "2026-11-24T09:00:00Z".to_string(),
    };
    let item = sync_format::SyncItem {
        doc_origin_number: "T-000007".to_string(),
        sku: "NOPE-404".to_string(),
        quantity_micro: 10_000,
        quantity_base_micro: 10_000,
        unit_price_centimes: 100,
        discount_centimes: 0,
        tva_rate_micro: 0,
        line_ttc_centimes: 100,
        cost_price_centimes: 50,
        unit_barcode: String::new(),
        sort_order: 0,
    };
    let body = format!(
        "{}\n{}",
        sync_format::build_doc_line(&doc),
        sync_format::build_item_line(&item)
    );
    let hash_hex = baraka_infrastructure::persistence::sha256::sha256_hex(body.as_bytes());
    let header = sync_format::build_header(ORIGIN_TERMINAL, "2026-11-24T10:00:00Z", 1, &hash_hex);
    let package = format!("{header}\n{body}");
    let error = importer
        .sync
        .import_package(
            &package,
            &warehouse.to_string(),
            ACTOR_UUID,
            IMPORTER_TERMINAL,
            "2026-11-24T10:00:00Z",
        )
        .expect_err("unknown SKU must abort");
    let message = format!("{error:?}");
    assert!(
        message.contains("SYNC_UNKNOWN_SKU"),
        "unexpected: {message}"
    );
    let guard = importer.db.lock().expect("lock");
    assert_eq!(count(&guard, "documents"), 0);
}
