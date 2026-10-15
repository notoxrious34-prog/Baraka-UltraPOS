//! TASK-REL-055 forensic end-to-end drill: the 10 operational pillars.
//!
//! An exhaustive, automated walkthrough of every merchant-facing module over
//! one real SQLite database per pillar (tempdir scratch files; the live
//! `%APPDATA%` database is never opened). Each pillar test drives the real
//! use cases through the real infrastructure adapters and asserts the
//! observable business outcome — money, stock, ledger, audit — with `i64`
//! centimes and fixed-point 10^4 micros only; no float exists in this file.
//!
//! Pillars:
//!  1. Caisse (POS)        2. Stock & Dépôts       3. Clients & Crédit
//!  4. Documents & Factures 5. Fournisseurs         6. Rapports
//!  7. Catalogue            8. Caisse (session)     9. Utilisateurs & Accès
//! 10. Paramètres & Sauvegarde

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    DocumentRepository, NewCustomer, NewSupplier, PasswordHasher, PurchaseLineInput,
    SessionContext, SessionToken, StockRepository,
};
use baraka_application::use_cases::backup::{CreateBackupUseCase, VerifyBackupIntegrityUseCase};
use baraka_application::use_cases::cash::{
    BeginClosingUseCase, ConfirmClosingUseCase, GetActiveSessionUseCase, GetSessionReportUseCase,
    OpenCashSessionUseCase, RecordExpenseUseCase,
};
use baraka_application::use_cases::catalog::{
    CreateProductInput, CreateProductUseCase, ImportBatchInput, ImportDuplicatePolicy,
    ImportProductRow, ImportProductsUseCase, NewUnitInput, SearchCatalogUseCase,
};
use baraka_application::use_cases::customers::{
    CreateCustomerUseCase, GetCustomerStatementUseCase, RecordCreditPaymentUseCase,
};
use baraka_application::use_cases::inventory::{
    AdjustStockInput, AdjustStockUseCase, GetStockLevelUseCase, ListWarehousesUseCase,
    StockDirection,
};
use baraka_application::use_cases::printing::{
    GetPrintStatusUseCase, GetStoreIdentityUseCase, ReprintDocumentUseCase,
    SaveStoreIdentityUseCase,
};
use baraka_application::use_cases::purchases::RecordSupplierInvoiceUseCase;
use baraka_application::use_cases::reports::{
    GetCreditOutstandingReportUseCase, GetDailySalesSummaryUseCase, GetSalesByProductUseCase,
};
use baraka_application::use_cases::returns::{
    ProcessReturnCommand, ProcessReturnUseCase, RefundDestination, ReturnItemInput,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput,
};
use baraka_application::use_cases::suppliers::{
    CreateSupplierUseCase, GetSupplierStatementUseCase, GetSupplierUseCase,
};
use baraka_application::use_cases::users::{ListUsersUseCase, UpdateUserUseCase};
use baraka_application::use_cases::VerifyPinUseCase;
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::inventory::WarehouseId;
use baraka_domain::primitives::QtyMicro;
use baraka_domain::printing::StoreIdentity;
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CustomerId, CustomerTier, DocumentId, DocumentLanguage, DocumentType,
    PaymentMethod,
};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    Argon2PasswordHasher, SqliteBackupManager, SqliteCashSessionRepository,
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteDocumentRepository, SqlitePrintJobRepository, SqlitePrinterSettings,
    SqlitePurchaseRepository, SqliteReportingRepository, SqliteReturnRepository,
    SqliteStockRepository, SqliteSupplierRepository, SqliteUserRepository,
    SqliteWarehouseRepository, SystemClock,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use chrono::{Datelike, TimeDelta, Utc};
use rusqlite::Connection;
use uuid::Uuid;

const VENDEUR_UUID: &str = "11111111-1111-1111-1111-111111111111";
const CAISSIER_UUID: &str = "22222222-2222-2222-2222-222222222222";
const GERANT_UUID: &str = "33333333-3333-3333-3333-333333333333";
const ADMIN_UUID: &str = "44444444-4444-4444-4444-444444444444";
const TEST_TERMINAL: &str = "TEST-TERMINAL-FORENSIC";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

struct Harness {
    _dir: tempfile::TempDir,
    _live_path: std::path::PathBuf,
    db: Arc<Mutex<Connection>>,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
    documents: SqliteDocumentRepository,
    jobs: SqlitePrintJobRepository,
    cash: SqliteCashSessionRepository,
    suppliers: SqliteSupplierRepository,
    purchases: SqlitePurchaseRepository,
    returns: SqliteReturnRepository,
    reports: SqliteReportingRepository,
    backups: SqliteBackupManager,
    printers: SqlitePrinterSettings,
    users: SqliteUserRepository,
    hasher: Argon2PasswordHasher,
    clock: Arc<SystemClock>,
    terminal_id: String,
    warehouse_id: WarehouseId,
}

fn session_with(role: Role, uuid: &str) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(uuid).expect("uuid")),
        role,
        expires_at: Utc::now() + TimeDelta::hours(1),
    }
}

fn vendeur() -> SessionContext {
    session_with(Role::Vendeur, VENDEUR_UUID)
}
fn caissier() -> SessionContext {
    session_with(Role::Caissier, CAISSIER_UUID)
}
fn gerant() -> SessionContext {
    session_with(Role::Gerant, GERANT_UUID)
}
fn admin() -> SessionContext {
    session_with(Role::Admin, ADMIN_UUID)
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let live_path = dir.path().join("forensic.db");
    let mut conn = open_connection(&live_path).expect("open");
    run_migrations(&mut conn).expect("migrate");

    // Operators: one per role, plus an Argon2id-hashed PIN for the Admin.
    let hasher = Argon2PasswordHasher;
    let admin_pin = hasher.hash("1234").expect("hash pin");
    for (id, name, role) in [
        (VENDEUR_UUID, "vendeur1", "VENDEUR"),
        (CAISSIER_UUID, "caissier1", "CAISSIER"),
        (GERANT_UUID, "gerant1", "GERANT"),
        (ADMIN_UUID, "admin1", "ADMIN"),
    ] {
        conn.execute(
            "INSERT INTO users
             (id, username, password_hash, pin_hash, role, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, ?4, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![
                id,
                name,
                if role == "ADMIN" {
                    Some(admin_pin.clone())
                } else {
                    None
                },
                role
            ],
        )
        .expect("seed user");
    }

    let terminal_id = resolve_device_id(&conn).expect("device id");
    let shared = Arc::new(Mutex::new(conn));
    let warehouses = SqliteWarehouseRepository::new(Arc::clone(&shared));
    // The canonical DEP-01 depot seeded by migration 0012 is the operating one.
    let warehouse_id = {
        let guard = shared.lock().expect("lock");
        guard
            .query_row(
                "SELECT id FROM warehouses WHERE code = 'DEP-01'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("DEP-01")
    };
    let warehouse_id = WarehouseId::from(Uuid::parse_str(&warehouse_id).expect("depot uuid"));

    Harness {
        _dir: dir,
        _live_path: live_path.clone(),
        db: Arc::clone(&shared),
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses,
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
        documents: SqliteDocumentRepository::new(Arc::clone(&shared)),
        jobs: SqlitePrintJobRepository::new(Arc::clone(&shared)),
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
        suppliers: SqliteSupplierRepository::new(Arc::clone(&shared)),
        purchases: SqlitePurchaseRepository::new(Arc::clone(&shared)),
        returns: SqliteReturnRepository::new(Arc::clone(&shared)),
        reports: SqliteReportingRepository::new(Arc::clone(&shared)),
        backups: SqliteBackupManager::new(Arc::clone(&shared), live_path),
        printers: SqlitePrinterSettings::new(Arc::clone(&shared)),
        users: SqliteUserRepository::new(Arc::clone(&shared)),
        hasher,
        clock: Arc::new(SystemClock),
        terminal_id,
        warehouse_id,
    }
}

/// Create a product with one base unit and opening stock; returns ids.
fn seed_product(harness: &Harness, sku: &str, sale: i64, purchase: i64) -> (ProductId, UnitId) {
    let session = gerant();
    let id = ProductId::new();
    let unit_id = UnitId::new();
    CreateProductUseCase::new(&harness.catalog, &harness.catalog)
        .execute(
            &session,
            CreateProductInput {
                sku: sku.to_string(),
                barcode: None,
                name_ar: "منتج".to_string(),
                name_fr: format!("Produit {sku}"),
                category_id: None,
                is_kit: false,
                units: vec![NewUnitInput {
                    unit_name_ar: "قطعة".to_string(),
                    unit_name_fr: "Pièce".to_string(),
                    factor_micro: 10_000,
                    barcode: None,
                    sale_price_centimes: sale,
                    purchase_price_centimes: purchase,
                    wholesale_price_centimes: None,
                    is_base_unit: true,
                    is_default_selling: true,
                }],
            },
            TEST_TERMINAL,
        )
        .expect("create product");
    let created = SearchCatalogUseCase::new(&harness.catalog)
        .execute(&session, sku, 10)
        .expect("search");
    let (product, units) = created
        .into_iter()
        .find(|(p, _)| p.sku.as_str() == sku)
        .expect("seeded product visible");
    let product_id = product.id;
    let unit = units.first().expect("base unit").id;
    let _ = (id, unit_id);
    harness
        .stock
        .upsert_stock_level_with_delta(product_id, harness.warehouse_id, 1_000_000)
        .expect("opening stock");
    (product_id, unit)
}

fn seed_customer(harness: &Harness, name: &str, tier: CustomerTier) -> CustomerId {
    CreateCustomerUseCase::new(&harness.customers)
        .execute(
            &gerant(),
            NewCustomer {
                name: name.to_string(),
                phone: None,
                address: None,
                credit_policy: CreditPolicy::Limited,
                credit_limit_centimes: 1_000_000,
                customer_tier: tier,
            },
            TEST_TERMINAL,
        )
        .expect("create customer")
}

fn open_session(harness: &Harness, float: i64) -> String {
    OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &caissier(),
            harness.warehouse_id,
            float,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("open session")
        .id
}

fn checkout_input(
    harness: &Harness,
    product: ProductId,
    unit: UnitId,
    qty_units: i64,
    cash: i64,
    customer: Option<CustomerId>,
    doc_type: DocumentType,
) -> CheckoutInput {
    CheckoutInput {
        command_id: Uuid::new_v4().to_string(),
        warehouse_id: harness.warehouse_id,
        customer_id: customer,
        doc_type,
        language: DocumentLanguage::Fr,
        lines: vec![CartLineInput {
            product_id: product,
            unit_id: unit,
            quantity_micro: qty_units * 10_000,
            unit_price_override: None,
            line_discount_centimes: 0,
        }],
        global_discount_centimes: 0,
        payments: vec![PaymentInput {
            method: PaymentMethod::Cash,
            amount_centimes: cash,
        }],
        allow_negative_stock: false,
        notes: None,
    }
}

fn count(harness: &Harness, table: &str) -> i64 {
    let guard = harness.db.lock().expect("lock");
    guard
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

// ---------------------------------------------------------------- pillar 1

#[test]
fn pillar_01_caisse_pos_sale_pipeline() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "FOR-001", 10_000, 8_000);
    open_session(&harness, 50_000);

    // Catalog search reaches the seeded row.
    let found = SearchCatalogUseCase::new(&harness.catalog)
        .execute(&caissier(), "FOR-001", 10)
        .expect("search");
    assert_eq!(found.len(), 1, "POS search finds the product");

    let before = harness
        .stock
        .get_stock_level(product, harness.warehouse_id)
        .expect("level before");
    assert_eq!(before, 1_000_000, "opening stock is 100 base units");

    // Atomic checkout: TICKET of 2 units at 10 000 centimes; engine adds the
    // 500-centime timbre floor (TICKET + cash tender), TVA 0 unseeded.
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &caissier(),
        checkout_input(
            &harness,
            product,
            unit,
            2,
            23_800,
            None,
            DocumentType::Ticket,
        ),
        &harness.terminal_id,
    )
    .expect("checkout");
    assert_eq!(receipt.total_ttc_centimes, "20500", "HT 20000 + timbre 500");
    assert_eq!(receipt.change_amount_centimes, "3300", "23800 - 20500");

    let after = harness
        .stock
        .get_stock_level(product, harness.warehouse_id)
        .expect("level after");
    assert_eq!(after, 980_000, "2 base units deducted (100 -> 98)");
    assert_eq!(count(&harness, "documents"), 1, "one document");
    assert_eq!(count(&harness, "document_items"), 1, "one item");
    assert_eq!(count(&harness, "payments"), 1, "one payment");
    assert_eq!(count(&harness, "stock_movements"), 1, "one movement");

    // Print-job queue creation is part of the same atomic sale.
    let doc_id =
        baraka_domain::sales::DocumentId(Uuid::parse_str(&receipt.document_id).expect("doc uuid"));
    assert_eq!(count(&harness, "print_jobs"), 1, "PENDING print job queued");
    let status = GetPrintStatusUseCase::new(&harness.jobs)
        .execute(&caissier(), doc_id)
        .expect("print status");
    assert_eq!(status, "PENDING");
}

#[test]
fn pillar_01b_tier_pricing_and_margin_guard() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "FOR-002", 10_000, 8_000);
    open_session(&harness, 50_000);
    // Wholesale tier customer: retail 10 000 vs wholesale NULL -> falls back
    // to the retail price (no fabricated wholesale price). BL carries no
    // timbre, so TTC is exactly 10 000.
    let wholesale_customer = seed_customer(&harness, "Grossiste SARL", CustomerTier::Wholesale);

    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &caissier(),
        checkout_input(
            &harness,
            product,
            unit,
            1,
            10_000,
            Some(wholesale_customer),
            DocumentType::Bl,
        ),
        &harness.terminal_id,
    )
    .expect("checkout");
    assert_eq!(
        receipt.total_ttc_centimes, "10000",
        "tier fallback keeps retail price"
    );
    assert_eq!(receipt.change_amount_centimes, "0", "tendered exactly");

    // Margin guard: the same line priced BELOW cost is flagged, never
    // silently accepted as profit.
    let guard = harness.db.lock().expect("lock");
    let (cost, price): (i64, i64) = guard
        .query_row(
            "SELECT cost_price_centimes, unit_price_centimes FROM document_items LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("item costs");
    drop(guard);
    assert_eq!(cost, 8_000, "cost snapshot is the purchase price");
    assert_eq!(price, 10_000, "price above cost -> no below-cost flag");
    let margin = baraka_domain::sales::pricing::calculate_margin_basis_points(10_000, 8_000);
    assert_eq!(margin, (2_000, false), "margin +2 000 bps on price");
    // Engine-pinned loss vector: below-cost is always flagged, never silent.
    let loss = baraka_domain::sales::pricing::calculate_margin_basis_points(8_000, 10_000);
    assert_eq!(loss, (-2_500, true), "below-cost flagged");
}

// ---------------------------------------------------------------- pillar 2

#[test]
fn pillar_02_stock_depots_adjustment() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "FOR-101", 5_000, 4_000);

    let depots = ListWarehousesUseCase::new(&harness.warehouses)
        .execute(&gerant())
        .expect("warehouses");
    assert!(
        depots.iter().any(|w| w.code.as_str() == "DEP-01"),
        "DEP-01 listed"
    );

    let level = GetStockLevelUseCase::new(&harness.stock)
        .execute(&caissier(), product, harness.warehouse_id)
        .expect("level");
    assert_eq!(level, 1_000_000, "opening 100 base units");

    // Manual adjustment with mandatory reason: +25 units.
    let outcome = AdjustStockUseCase::new(&harness.stock)
        .execute(
            &gerant(),
            AdjustStockInput {
                product_id: product,
                warehouse_id: harness.warehouse_id,
                unit_id: unit,
                quantity_micro: 250_000,
                direction: StockDirection::Increase,
                reason: "Regularisation".to_string(),
                authorize_negative_stock: false,
            },
        )
        .expect("adjust");
    assert_eq!(
        outcome.new_quantity_base_micro, 1_250_000,
        "125 base units after +25"
    );
    assert!(!outcome.movement_id.is_empty(), "movement recorded");

    // The reason is persisted on the movement row.
    let guard = harness.db.lock().expect("lock");
    let reason: String = guard
        .query_row(
            "SELECT reason FROM stock_movements WHERE id = ?1",
            [&outcome.movement_id],
            |row| row.get(0),
        )
        .expect("reason");
    assert_eq!(reason, "Regularisation");
}

// ---------------------------------------------------------------- pillar 3

#[test]
fn pillar_03_clients_credit_lifecycle() {
    let harness = setup();
    open_session(&harness, 50_000);
    let customer = seed_customer(&harness, "Client Forensic", CustomerTier::Retail);

    // Record an opening debt of 10 000 centimes through a credit BL
    // (BL carries no timbre; CREDIT legs must tender exactly).
    let (product, unit) = seed_product(&harness, "FOR-102", 10_000, 8_000);
    let mut credit_input = checkout_input(
        &harness,
        product,
        unit,
        1,
        0,
        Some(customer),
        DocumentType::Bl,
    );
    credit_input.payments = vec![PaymentInput {
        method: PaymentMethod::Credit,
        amount_centimes: 10_000,
    }];
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(&caissier(), credit_input, &harness.terminal_id)
    .expect("credit sale");

    let balance: i64 = {
        let guard = harness.db.lock().expect("lock");
        guard
            .query_row(
                "SELECT current_balance_centimes FROM customers WHERE id = ?1",
                [customer.to_string()],
                |row| row.get(0),
            )
            .expect("balance")
    };
    assert_eq!(balance, 10_000, "debt posted to cached balance");

    // Partial debt payment of 1 900 decreases the balance. The payment is
    // collected into the OPEN session resolved by terminal id.
    let payment = RecordCreditPaymentUseCase::new(&harness.customers)
        .execute(
            &caissier(),
            customer,
            1_900,
            Some("REC-FORENSIC".to_string()),
            false,
            &harness.terminal_id,
        )
        .expect("payment");
    assert_eq!(payment.new_balance_centimes, 8_100);

    // Overpayment without carry-forward flag is rejected with zero writes.
    let before = count(&harness, "customer_ledger");
    let rejected = RecordCreditPaymentUseCase::new(&harness.customers)
        .execute(
            &gerant(),
            customer,
            99_999,
            None,
            false,
            &harness.terminal_id,
        )
        .expect_err("overpay");
    assert!(
        matches!(
            rejected,
            ApplicationError::PaymentExceedsBalance {
                balance_centimes: 8_100,
                attempted_centimes: 99_999,
            }
        ),
        "unexpected: {rejected:?}"
    );
    assert_eq!(count(&harness, "customer_ledger"), before);

    // Statement generation over the ledger.
    let statement = GetCustomerStatementUseCase::new(&harness.customers)
        .execute(&gerant(), customer, None, None)
        .expect("statement");
    assert!(
        statement.len() >= 2,
        "debt + payment entries visible, got {}",
        statement.len()
    );
}

// ---------------------------------------------------------------- pillar 4

#[test]
fn pillar_04_documents_factures_and_return() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "FOR-103", 10_000, 8_000);
    let session_id = open_session(&harness, 50_000);
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &caissier(),
        checkout_input(
            &harness,
            product,
            unit,
            2,
            23_800,
            None,
            DocumentType::Ticket,
        ),
        &harness.terminal_id,
    )
    .expect("checkout");
    let doc_id = DocumentId(Uuid::parse_str(&receipt.document_id).expect("doc uuid"));
    let year = Utc::now().year();

    // Finalized document lookup by (type, year, number).
    let header = harness
        .documents
        .get_by_number("TICKET", year, &receipt.document_number)
        .expect("lookup")
        .expect("finalized row");
    assert_eq!(header.id, doc_id, "same document");

    // Duplicate print status: the atomic sale queued exactly one PENDING job.
    let status = GetPrintStatusUseCase::new(&harness.jobs)
        .execute(&caissier(), doc_id)
        .expect("status");
    assert_eq!(status, "PENDING");
    let reprint = ReprintDocumentUseCase::new(&harness.jobs, &harness.documents)
        .execute(&gerant(), doc_id)
        .expect("reprint");
    assert!(!reprint.is_empty(), "DUPLICATA job id returned");
    assert_eq!(count(&harness, "print_jobs"), 2, "original + duplicata");

    // Return context check: a real 1-unit return against the finalized
    // document produces an AVOIR and restores stock, no cash leg.
    let items = harness.documents.get_items(doc_id).expect("items");
    assert_eq!(items.len(), 1);
    let source_item = items[0].id;
    let stock_before = harness
        .stock
        .get_stock_level(product, harness.warehouse_id)
        .expect("stock before return");
    let outcome = ProcessReturnUseCase::new(
        &harness.documents,
        &harness.returns,
        &harness.customers,
        &harness.cash,
    )
    .execute(
        &gerant(),
        ProcessReturnCommand {
            operator_id: gerant().user_id,
            source_document_id: doc_id,
            items: vec![ReturnItemInput {
                source_item_id: source_item,
                quantity_micro: QtyMicro::from_micros(10_000),
                reason: Some("Defaut".to_string()),
            }],
            refund: RefundDestination::NoRefund,
            idempotency_key: Uuid::new_v4().to_string(),
            terminal_id: harness.terminal_id.clone(),
            notes: None,
        },
    )
    .expect("return");
    assert_eq!(outcome.items_returned_count, 1);
    // The fiscal timbre is not refunded: 1 unit at 10 000 HT, TVA 0.
    assert_eq!(outcome.total_refund_ttc.centimes(), 10_000);
    let stock_after = harness
        .stock
        .get_stock_level(product, harness.warehouse_id)
        .expect("stock after return");
    assert_eq!(stock_after, stock_before + 10_000, "one base unit restored");
    let _ = session_id;
}

// ---------------------------------------------------------------- pillar 5

#[test]
fn pillar_05_fournisseurs_achats() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "FOR-104", 10_000, 8_000);

    let supplier = CreateSupplierUseCase::new(&harness.suppliers)
        .execute(
            &gerant(),
            NewSupplier {
                name: "Fournisseur Forensic SARL".to_string(),
                phone: None,
                address: None,
                nif: None,
                rc: None,
                notes: None,
            },
            TEST_TERMINAL,
        )
        .expect("supplier");
    assert!(!supplier.id.is_empty());

    // Purchase invoice: 10 units at 8 000 cost.
    let invoice = RecordSupplierInvoiceUseCase::new(
        &harness.purchases,
        &harness.suppliers,
        &harness.warehouses,
    )
    .execute(
        &gerant(),
        None,
        &supplier.id,
        harness.warehouse_id,
        vec![PurchaseLineInput {
            product_id: product,
            unit_id: unit,
            quantity_micro: 100_000,
            unit_cost_centimes: 8_000,
        }],
        80_000,
        TEST_TERMINAL,
        &Uuid::new_v4().to_string(),
    )
    .expect("invoice");
    assert_eq!(invoice.supplier_id, supplier.id);

    let fetched = GetSupplierUseCase::new(&harness.suppliers)
        .execute(&gerant(), &supplier.id)
        .expect("get supplier");
    assert_eq!(fetched.name, "Fournisseur Forensic SARL");

    let statement = GetSupplierStatementUseCase::new(&harness.suppliers)
        .execute(&gerant(), &supplier.id)
        .expect("statement");
    assert!(
        !statement.entries.is_empty(),
        "invoice posted to supplier ledger"
    );
    assert_eq!(
        statement.current_balance_centimes, 80_000,
        "supplier balance = invoice total"
    );
}

// ---------------------------------------------------------------- pillar 6

#[test]
fn pillar_06_rapports() {
    let harness = setup();
    let (product, unit) = seed_product(&harness, "FOR-105", 10_000, 8_000);
    open_session(&harness, 50_000);
    CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &caissier(),
        checkout_input(
            &harness,
            product,
            unit,
            2,
            23_800,
            None,
            DocumentType::Ticket,
        ),
        &harness.terminal_id,
    )
    .expect("checkout");

    // Daily Z-report aggregation over a wide-open range.
    let days = GetDailySalesSummaryUseCase::new(&harness.reports)
        .execute(&gerant(), "2026-01-01", "2026-12-31")
        .expect("daily summary");
    let revenue: i64 = days.iter().map(|row| row.total_ttc_centimes).sum();
    assert_eq!(
        revenue, 20_500,
        "Z-report revenue exact (HT 20000 + timbre 500)"
    );
    assert_eq!(days.iter().map(|row| row.document_count).sum::<i64>(), 1);

    // Per-product aggregation pins the sold SKU and base micros (line TTC
    // sums exclude the fiscal timbre, which lives on the document header).
    let by_product = GetSalesByProductUseCase::new(&harness.reports)
        .execute(&gerant(), "2026-01-01", "2026-12-31")
        .expect("by product");
    assert_eq!(by_product.len(), 1, "one product sold");
    assert_eq!(by_product[0].revenue_ttc_centimes, 20_000);
    assert_eq!(by_product[0].quantity_sold_base_micro, 20_000);

    // Credit-outstanding report is readable (zero here — cash sale).
    let outstanding = GetCreditOutstandingReportUseCase::new(&harness.reports)
        .execute(&gerant())
        .expect("outstanding");
    let _ = outstanding;
}

// ---------------------------------------------------------------- pillar 7

#[test]
fn pillar_07_catalogue_import() {
    let harness = setup();
    let report = ImportProductsUseCase::new(&harness.catalog)
        .execute(
            &gerant(),
            ImportBatchInput {
                rows: vec![
                    ImportProductRow {
                        sku: Some("FOR-IMP-01".to_string()),
                        barcode: None,
                        name_fr: "Tournevis plat".to_string(),
                        name_ar: Some("مفك".to_string()),
                        category_id: None,
                        base_unit_name_fr: None,
                        base_unit_name_ar: None,
                        purchase_price_centimes: Some(4_000),
                        sale_price_centimes: 5_000,
                        initial_quantity_micro: Some(500_000),
                    },
                    ImportProductRow {
                        sku: Some("FOR-IMP-02".to_string()),
                        barcode: None,
                        name_fr: "Pince coupante".to_string(),
                        name_ar: None,
                        category_id: None,
                        base_unit_name_fr: None,
                        base_unit_name_ar: None,
                        purchase_price_centimes: Some(6_000),
                        sale_price_centimes: 7_500,
                        initial_quantity_micro: None,
                    },
                ],
                warehouse_code: Some("DEP-01".to_string()),
                on_duplicate: ImportDuplicatePolicy::Skip,
                origin_device_id: TEST_TERMINAL.to_string(),
            },
        )
        .expect("import");
    assert_eq!(report.imported_count, 2, "both rows ingested");
    assert_eq!(report.total_rows, 2);

    // Both products resolve through the POS search path.
    for sku in ["FOR-IMP-01", "FOR-IMP-02"] {
        let found = SearchCatalogUseCase::new(&harness.catalog)
            .execute(&vendeur(), sku, 10)
            .expect("search");
        assert_eq!(found.len(), 1, "{sku} visible after import");
    }
    // Opening stock provisioned on DEP-01 for the first row only.
    let (product, _) = SearchCatalogUseCase::new(&harness.catalog)
        .execute(&gerant(), "FOR-IMP-01", 10)
        .expect("search")
        .into_iter()
        .map(|(p, u)| (p.id, u))
        .next()
        .expect("row");
    let _ = product;
    let stock_rows: i64 = {
        let guard = harness.db.lock().expect("lock");
        guard
            .query_row("SELECT COUNT(*) FROM stock_levels", [], |row| row.get(0))
            .expect("levels")
    };
    assert_eq!(stock_rows, 1, "only the provisioned row has a level");
}

// ---------------------------------------------------------------- pillar 8

#[test]
fn pillar_08_caisse_session_lifecycle() {
    let harness = setup();
    let session_id = open_session(&harness, 100_000);

    let active = GetActiveSessionUseCase::new(&harness.cash)
        .execute(&caissier(), &harness.terminal_id)
        .expect("active")
        .expect("session open");
    assert_eq!(active.id, session_id);
    assert_eq!(active.status, "OPEN");

    // Small expense: 500 DZD (50 000 centimes) of cleaning supplies.
    let movement = RecordExpenseUseCase::new(&harness.cash)
        .execute(
            &gerant(),
            50_000,
            "Produits de nettoyage",
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("expense");
    assert_eq!(movement.amount_centimes, -50_000, "expense is negative");
    let guard = harness.db.lock().expect("lock");
    let expenses: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM cash_movements WHERE cash_session_id = ?1 AND movement_type = 'EXPENSE_OUT'",
            [&session_id],
            |row| row.get(0),
        )
        .expect("expense rows");
    drop(guard);
    assert_eq!(expenses, 1, "expense movement persisted");

    // Mid-day report (not final) then full close at the exact expectation.
    let mid = GetSessionReportUseCase::new(&harness.cash)
        .execute(&gerant(), &session_id)
        .expect("mid report");
    assert!(!mid.is_final, "open session report is not final");
    assert!(
        mid.movements.len() >= 2,
        "float + expense visible, got {}",
        mid.movements.len()
    );

    // Full close: begin the closing handshake, then confirm at the exact
    // expectation (float 100 000 - expense 50 000 = 50 000).
    BeginClosingUseCase::new(&harness.cash)
        .execute(&gerant(), &harness.terminal_id, &Uuid::new_v4().to_string())
        .expect("begin closing");
    let closing = ConfirmClosingUseCase::new(&harness.cash)
        .execute(
            &gerant(),
            &session_id,
            50_000,
            Some("Cloture forensic".to_string()),
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("close");
    assert_eq!(closing.status, "CLOSED");
    let final_report = GetSessionReportUseCase::new(&harness.cash)
        .execute(&gerant(), &session_id)
        .expect("final report");
    assert!(final_report.is_final, "closed session yields the Z-report");
}

// ---------------------------------------------------------------- pillar 9

#[test]
fn pillar_09_utilisateurs_rbac() {
    let harness = setup();
    let admin_id = UserId::from(Uuid::parse_str(ADMIN_UUID).expect("uuid"));

    // Admin PIN: correct passes, wrong fails.
    VerifyPinUseCase::new(&harness.hasher, &harness.users)
        .execute(admin_id, "1234")
        .expect("correct pin");
    let wrong = VerifyPinUseCase::new(&harness.hasher, &harness.users)
        .execute(admin_id, "0000")
        .expect_err("wrong pin");
    assert!(
        matches!(
            wrong,
            ApplicationError::Validation(baraka_domain::errors::DomainError::InvalidCredentials)
        ),
        "unexpected: {wrong:?}"
    );

    // Directory is readable by management.
    let users = ListUsersUseCase::new(&harness.users)
        .execute(&admin())
        .expect("list");
    assert_eq!(users.len(), 4, "four seeded operators");

    // Vendeur cannot manage suppliers (RBAC gate, before any write).
    let denied = CreateSupplierUseCase::new(&harness.suppliers)
        .execute(
            &vendeur(),
            NewSupplier {
                name: "Blocage".to_string(),
                phone: None,
                address: None,
                nif: None,
                rc: None,
                notes: None,
            },
            TEST_TERMINAL,
        )
        .expect_err("vendeur blocked");
    assert!(
        matches!(denied, ApplicationError::Unauthorized),
        "unexpected: {denied:?}"
    );
    assert_eq!(count(&harness, "suppliers"), 0, "zero writes on denial");

    // Last active admin is protected from deactivation.
    let admins = ListUsersUseCase::new(&harness.users)
        .execute(&admin())
        .expect("admins");
    let active_admins = admins
        .iter()
        .filter(|u| u.role == Role::Admin && u.is_active)
        .count();
    assert_eq!(active_admins, 1, "exactly one active admin seeded");
    let guarded = UpdateUserUseCase::new(&harness.users, harness.clock.as_ref())
        .execute(&admin(), admin_id, None, Some(false))
        .expect_err("last admin");
    assert!(
        matches!(guarded, ApplicationError::Conflict(_)),
        "unexpected: {guarded:?}"
    );
}

// --------------------------------------------------------------- pillar 10

#[test]
fn pillar_10_parametres_sauvegarde() {
    let harness = setup();

    // Store identity round-trip.
    let identity = StoreIdentity {
        store_name: "Quincaillerie Forensic".to_string(),
        ..StoreIdentity::default()
    };
    let saved = SaveStoreIdentityUseCase::new(&harness.printers)
        .execute(&admin(), &identity, TEST_TERMINAL)
        .expect("save identity");
    assert_eq!(saved.store_name, "Quincaillerie Forensic");
    let loaded = GetStoreIdentityUseCase::new(&harness.printers)
        .execute(&admin())
        .expect("load identity");
    assert_eq!(loaded, saved, "identity round-trips byte-identically");

    // SQLite WAL backup of the scratch database, then integrity proof.
    let backup_dir = harness._dir.path().join("backups");
    std::fs::create_dir_all(&backup_dir).expect("backup dir");
    let meta = CreateBackupUseCase::new(&harness.backups)
        .execute(
            &gerant(),
            baraka_domain::backup::BackupTriggerType::Manual,
            &backup_dir,
            5,
            TEST_TERMINAL,
            APP_VERSION,
        )
        .expect("backup");
    let file = backup_dir.join(&meta.file_name);
    assert!(file.is_file(), "backup artifact exists");
    assert!(
        file.metadata().expect("meta").len() > 0,
        "backup file non-empty"
    );
    VerifyBackupIntegrityUseCase::new(&harness.backups)
        .execute(&gerant(), &backup_dir, &meta.file_name)
        .expect("integrity");
}
