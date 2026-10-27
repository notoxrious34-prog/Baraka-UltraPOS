//! Preview/commit totals parity regression (TASK-BRK-031).
//!
//! Reproduces the forensic TICK-2026-000002 scenario exactly — a single
//! 150.50 DZD line settled with mixed cash (150.00) + card (150.50) — and
//! proves the read-only preview totals are byte-for-byte identical, in
//! exact centimes, to what the subsequent atomic commit persists
//! (including the 500-centime timbre the pre-fix preview never showed).

use std::sync::{Arc, Mutex};

use baraka_application::ports::{
    ProductRepository, SessionContext, SessionToken, StockRepository, WarehouseRepository,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, PaymentInput, PreviewSaleTotalsUseCase,
    PreviewTotalsInput,
};
use baraka_domain::catalog::{Barcode, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::sales::{DocumentLanguage, DocumentType, PaymentMethod};
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCatalogRepository, SqliteCheckoutRepository, SqliteCustomerRepository,
    SqliteStockRepository, SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;
use uuid::Uuid;

const ACTOR_UUID: &str = "66666666-6666-6666-6666-666666666666";
const TEST_TERMINAL: &str = "TEST-TERMINAL-001";

struct Harness {
    _dir: tempfile::TempDir,
    catalog: SqliteCatalogRepository,
    stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    customers: SqliteCustomerRepository,
    checkout: SqliteCheckoutRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("parity.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('66666666-6666-6666-6666-666666666666', 'admin', 'hash', 'ADMIN',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
        customers: SqliteCustomerRepository::new(Arc::clone(&shared)),
        checkout: SqliteCheckoutRepository::new(Arc::clone(&shared)),
    }
}

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn ensure_cash_session(harness: &Harness, warehouse: WarehouseId) {
    let conn = rusqlite::Connection::open(harness._dir.path().join("parity.db")).expect("reopen");
    let session_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO cash_sessions
         (id, warehouse_id, terminal_id, status, opened_by, opened_at,
          opening_float_centimes, revision, origin_device_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'OPEN', ?4, '2026-01-01T00:00:00Z', 0, 1, ?3,
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![session_id, warehouse.to_string(), TEST_TERMINAL, ACTOR_UUID,],
    )
    .expect("open session");
    conn.execute(
        "INSERT INTO cash_movements
         (id, cash_session_id, movement_type, amount_centimes, description, created_at, created_by)
         VALUES (?1, ?2, 'OPENING_FLOAT', 0, 'Opening float', '2026-01-01T00:00:00Z', ?3)",
        rusqlite::params![Uuid::new_v4().to_string(), session_id, ACTOR_UUID,],
    )
    .expect("opening float");
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

fn seed_product(
    harness: &Harness,
    sku: &str,
    sale_price: i64,
    cost_price: i64,
) -> (ProductId, UnitId) {
    let id = ProductId::new();
    let unit_id = UnitId::new();
    let product = Product {
        id,
        sku: Sku::new(sku).expect("sku"),
        barcode: None,
        name_ar: "منتج".to_string(),
        name_fr: "Produit".to_string(),
        category_id: None,
        is_kit: false,
        is_active: true,
    };
    let unit = ProductUnit {
        id: unit_id,
        product_id: id,
        unit_name_ar: "قطعة".to_string(),
        unit_name_fr: "Pièce".to_string(),
        factor_micro: FactorMicro::from_micros(10_000),
        barcode: Some(Barcode::new(&format!("611125{sku}")).expect("barcode")),
        sale_price: Money::from_centimes(sale_price),
        purchase_price: Money::from_centimes(cost_price),
        wholesale_price: None,
        is_base_unit: true,
        is_default_selling: true,
    };
    harness
        .catalog
        .create_product(&product, &[unit], TEST_TERMINAL)
        .expect("product");
    (id, unit_id)
}

fn seed_stock(harness: &Harness, product: ProductId, warehouse: WarehouseId, qty: i64) {
    harness
        .stock
        .upsert_stock_level_with_delta(product, warehouse, qty)
        .expect("stock");
}

fn read_conn(harness: &Harness) -> rusqlite::Connection {
    rusqlite::Connection::open(harness._dir.path().join("parity.db")).expect("reopen")
}

fn count(harness: &Harness, table: &str) -> i64 {
    read_conn(harness)
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("count")
}

/// Stored header totals of a document, in exact centimes.
fn stored_document_totals(
    harness: &Harness,
    document_id: &str,
) -> (i64, i64, i64, i64, i64, i64, i64) {
    read_conn(harness)
        .query_row(
            "SELECT subtotal_ht_centimes, vat_total_centimes, global_discount_centimes,
                    timbre_centimes, ttc_centimes, paid_amount_centimes, change_amount_centimes
             FROM documents WHERE id = ?1",
            [document_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("stored document")
}

#[test]
fn preview_equals_commit_tick_2026_000002_scenario() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    // The live product shape: VIS-8X40 at 150.50 DZD sale (100.00 cost).
    let (product, unit) = seed_product(&harness, "VIS-8X40", 15_050, 10_000);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let session = session_with(Role::Admin);
    // The exact live tender: 150.00 cash + 150.50 card = 300.50 DZD.
    let lines = vec![CartLineInput {
        product_id: product,
        unit_id: unit,
        quantity_micro: 10_000,
        unit_price_override: None,
        line_discount_centimes: 0,
    }];
    let payments = vec![
        PaymentInput {
            method: PaymentMethod::Cash,
            amount_centimes: 15_000,
        },
        PaymentInput {
            method: PaymentMethod::Card,
            amount_centimes: 15_050,
        },
    ];
    // 1. PREVIEW before any commit: must already carry the timbre.
    let preview = PreviewSaleTotalsUseCase::new(
        &harness.catalog,
        &harness.stock,
        &harness.warehouses,
        &harness.customers,
    )
    .execute(
        &session,
        PreviewTotalsInput {
            warehouse_id: warehouse,
            doc_type: DocumentType::Ticket,
            lines: lines.clone(),
            customer_id: None,
            global_discount_centimes: 0,
            payments: payments.clone(),

            timbre_enabled: true,
        },
    )
    .expect("preview");
    assert_eq!(
        (
            preview.subtotal_ht,
            preview.total_tva,
            preview.global_discount,
            preview.timbre,
            preview.total_ttc,
            preview.paid_total,
            preview.change,
            preview.remaining_due,
        ),
        (15_050, 0, 0, 500, 15_550, 30_050, 14_500, 0)
    );
    // The preview is strictly read-only: nothing is persisted.
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "document_sequences"), 0);
    // 2. COMMIT the same cart state atomically.
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &session,
        CheckoutInput {
            command_id: Uuid::new_v4().to_string(),
            warehouse_id: warehouse,
            customer_id: None,
            doc_type: DocumentType::Ticket,
            language: DocumentLanguage::Fr,
            lines,
            global_discount_centimes: 0,
            payments,
            allow_negative_stock: false,
            notes: None,

            timbre_enabled: true,
        },
        TEST_TERMINAL,
    )
    .expect("commit");
    // 3. Preview equals commit byte-for-byte, in exact centimes.
    assert_eq!(
        preview.subtotal_ht.to_string(),
        receipt.subtotal_ht_centimes
    );
    assert_eq!(preview.total_tva.to_string(), receipt.tva_total_centimes);
    assert_eq!(
        preview.global_discount.to_string(),
        receipt.discount_total_centimes
    );
    assert_eq!(preview.timbre.to_string(), receipt.timbre_centimes);
    assert_eq!(preview.total_ttc.to_string(), receipt.total_ttc_centimes);
    assert_eq!(preview.paid_total.to_string(), receipt.paid_amount_centimes);
    assert_eq!(preview.change.to_string(), receipt.change_amount_centimes);
    // 4. The persisted books equal the previewed figures too.
    assert_eq!(
        stored_document_totals(&harness, &receipt.document_id),
        (15_050, 0, 0, 500, 15_550, 30_050, 14_500)
    );
}

#[test]
fn preview_equals_commit_with_discounts_bl_no_timbre() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product_a, unit_a) = seed_product(&harness, "A", 10_000, 8_000);
    let (product_b, unit_b) = seed_product(&harness, "B", 5_050, 3_000);
    seed_stock(&harness, product_a, warehouse, 100_000);
    seed_stock(&harness, product_b, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let session = session_with(Role::Vendeur);
    // A: 2 x (100.00 - 10.00) = 180.00 after per-unit discount.
    // B: 1 x 50.50 = 50.50. Global discount 40.50 over bases
    // [180.00, 50.50] -> allocated [31.63, 8.87], HT = 148.37 + 41.63 = 190.00.
    let lines = vec![
        CartLineInput {
            product_id: product_a,
            unit_id: unit_a,
            quantity_micro: 20_000,
            unit_price_override: None,
            line_discount_centimes: 1_000,
        },
        CartLineInput {
            product_id: product_b,
            unit_id: unit_b,
            quantity_micro: 10_000,
            unit_price_override: None,
            line_discount_centimes: 0,
        },
    ];
    // 1. PREVIEW with no tender yet (modal just opened): BL never carries
    //    a timbre, and the global discount allocates proportionally.
    let preview_untendered = PreviewSaleTotalsUseCase::new(
        &harness.catalog,
        &harness.stock,
        &harness.warehouses,
        &harness.customers,
    )
    .execute(
        &session,
        PreviewTotalsInput {
            warehouse_id: warehouse,
            doc_type: DocumentType::Bl,
            lines: lines.clone(),
            customer_id: None,
            global_discount_centimes: 4_050,
            payments: vec![],

            timbre_enabled: true,
        },
    )
    .expect("preview");
    assert_eq!(
        (
            preview_untendered.subtotal_ht,
            preview_untendered.total_tva,
            preview_untendered.global_discount,
            preview_untendered.timbre,
            preview_untendered.total_ttc,
            preview_untendered.paid_total,
            preview_untendered.change,
            preview_untendered.remaining_due,
        ),
        (19_000, 0, 4_050, 0, 19_000, 0, 0, 19_000)
    );
    assert_eq!(count(&harness, "documents"), 0);
    // 2. The cashier tenders exactly the previewed TTC (cheque, no cash).
    let payments = vec![PaymentInput {
        method: PaymentMethod::Cheque,
        amount_centimes: preview_untendered.total_ttc,
    }];
    // 3. The modal re-previews on every tender change: same cart state,
    //    now with the actual tender aboard.
    let preview = PreviewSaleTotalsUseCase::new(
        &harness.catalog,
        &harness.stock,
        &harness.warehouses,
        &harness.customers,
    )
    .execute(
        &session,
        PreviewTotalsInput {
            warehouse_id: warehouse,
            doc_type: DocumentType::Bl,
            lines: lines.clone(),
            customer_id: None,
            global_discount_centimes: 4_050,
            payments: payments.clone(),

            timbre_enabled: true,
        },
    )
    .expect("preview with tender");
    assert_eq!(
        (
            preview.subtotal_ht,
            preview.total_tva,
            preview.global_discount,
            preview.timbre,
            preview.total_ttc,
            preview.paid_total,
            preview.change,
            preview.remaining_due,
        ),
        (19_000, 0, 4_050, 0, 19_000, 19_000, 0, 0)
    );
    // 4. COMMIT the same cart state atomically.
    let receipt = CheckoutSaleUseCase::new(
        &harness.catalog,
        &harness.customers,
        &harness.checkout,
        &harness.warehouses,
    )
    .execute(
        &session,
        CheckoutInput {
            command_id: Uuid::new_v4().to_string(),
            warehouse_id: warehouse,
            customer_id: None,
            doc_type: DocumentType::Bl,
            language: DocumentLanguage::Fr,
            lines,
            global_discount_centimes: 4_050,
            payments,
            allow_negative_stock: false,
            notes: None,

            timbre_enabled: true,
        },
        TEST_TERMINAL,
    )
    .expect("commit");
    // 5. Preview equals commit byte-for-byte, in exact centimes.
    assert_eq!(
        preview.subtotal_ht.to_string(),
        receipt.subtotal_ht_centimes
    );
    assert_eq!(preview.total_tva.to_string(), receipt.tva_total_centimes);
    assert_eq!(
        preview.global_discount.to_string(),
        receipt.discount_total_centimes
    );
    assert_eq!(preview.timbre.to_string(), receipt.timbre_centimes);
    assert_eq!(preview.total_ttc.to_string(), receipt.total_ttc_centimes);
    assert_eq!(preview.paid_total.to_string(), receipt.paid_amount_centimes);
    assert_eq!(preview.change.to_string(), receipt.change_amount_centimes);
    // 6. The persisted books equal the previewed figures too.
    assert_eq!(
        stored_document_totals(&harness, &receipt.document_id),
        (19_000, 0, 4_050, 0, 19_000, 19_000, 0)
    );
    // 7. Per-unit discount persisted exactly (BRK-096): line A stores
    // per-unit 1,000 with lump (20,000 - 18,000) + 3,163 share = 5,163;
    // line B stores per-unit 0 with lump 0 + 887 share = 887.
    let stored: Vec<(i64, Option<i64>, i64, i64)> = read_conn(&harness)
        .prepare(
            "SELECT unit_price_centimes, discount_per_unit_centimes,
                    discount_centimes, line_ht_centimes
             FROM document_items WHERE document_id = ?1 ORDER BY sort_order",
        )
        .expect("lines query")
        .query_map([receipt.document_id.clone()], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("lines")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows");
    assert_eq!(
        stored,
        vec![
            (10_000, Some(1_000), 5_163, 14_837),
            (5_050, Some(0), 887, 4_163)
        ]
    );
}

#[test]
fn preview_is_read_only_and_rejects_empty_cart() {
    let harness = setup();
    let warehouse = seed_warehouse(&harness, "MAIN");
    let (product, unit) = seed_product(&harness, "P1", 10_000, 8_000);
    seed_stock(&harness, product, warehouse, 100_000);
    ensure_cash_session(&harness, warehouse);
    let session = session_with(Role::Vendeur);
    // Empty cart: the kernel rejects exactly as a commit would.
    let rejected = PreviewSaleTotalsUseCase::new(
        &harness.catalog,
        &harness.stock,
        &harness.warehouses,
        &harness.customers,
    )
    .execute(
        &session,
        PreviewTotalsInput {
            warehouse_id: warehouse,
            doc_type: DocumentType::Ticket,
            lines: vec![],
            customer_id: None,
            global_discount_centimes: 0,
            payments: vec![PaymentInput {
                method: PaymentMethod::Cash,
                amount_centimes: 10_000,
            }],

            timbre_enabled: true,
        },
    )
    .expect_err("empty cart");
    assert!(
        matches!(
            rejected,
            baraka_application::errors::ApplicationError::Validation(DomainError::EmptyCart)
        ),
        "unexpected: {rejected:?}"
    );
    // Unsupported document kinds are rejected exactly as a commit would.
    let forbidden = PreviewSaleTotalsUseCase::new(
        &harness.catalog,
        &harness.stock,
        &harness.warehouses,
        &harness.customers,
    )
    .execute(
        &session_with(Role::Vendeur),
        PreviewTotalsInput {
            warehouse_id: warehouse,
            doc_type: DocumentType::Devis,
            lines: vec![CartLineInput {
                product_id: product,
                unit_id: unit,
                quantity_micro: 10_000,
                unit_price_override: None,
                line_discount_centimes: 0,
            }],
            customer_id: None,
            global_discount_centimes: 0,
            payments: vec![],

            timbre_enabled: true,
        },
    )
    .expect_err("unsupported doc type");
    assert!(
        matches!(
            forbidden,
            baraka_application::errors::ApplicationError::Validation(
                DomainError::InvalidCanonicalString(ref code)
            ) if code == "UNSUPPORTED_DOCUMENT_TYPE"
        ),
        "unexpected: {forbidden:?}"
    );
    // Nothing was persisted by either rejected preview.
    assert_eq!(count(&harness, "documents"), 0);
    assert_eq!(count(&harness, "stock_movements"), 0);
    assert_eq!(count(&harness, "print_jobs"), 0);
    assert_eq!(
        harness
            .stock
            .get_stock_level(product, warehouse)
            .expect("level"),
        100_000
    );
}
