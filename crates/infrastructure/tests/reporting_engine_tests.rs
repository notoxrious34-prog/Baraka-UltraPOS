//! Reporting integration tests (TASK-BRK-012).
//!
//! A hand-computed fixture dataset (two sale days, three products, two
//! categories, four customers, committed + draft + purchase documents)
//! drives every aggregate below; each expectation was computed by hand
//! from the seeded rows, never from the implementation under test.
//! All money is `i64` centimes; no float exists in this file.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{ProductRepository, WarehouseRepository};
use baraka_application::use_cases::cash::OpenCashSessionUseCase;
use baraka_application::use_cases::reports::{
    GetCashSessionsHistoryUseCase, GetCreditOutstandingReportUseCase, GetDailySalesSummaryUseCase,
    GetGrossMarginReportUseCase, GetLowStockReportUseCase, GetSalesByCategoryUseCase,
    GetSalesByProductUseCase, GetStockMovementHistoryUseCase, GetUserActivityLogUseCase,
    MAX_PAGE_SIZE,
};
use baraka_domain::catalog::{Barcode, CategoryId, Product, ProductId, ProductUnit, Sku, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use baraka_domain::primitives::{FactorMicro, Money};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteCashSessionRepository, SqliteCatalogRepository, SqlitePurchaseRepository,
    SqliteReportingRepository, SqliteStockRepository, SqliteSupplierRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use chrono::TimeDelta;
use uuid::Uuid;

use baraka_application::ports::{SessionContext, SessionToken};

const GERANT_UUID: &str = "33333333-3333-3333-3333-333333333333";
const ADMIN_UUID: &str = "44444444-4444-4444-4444-444444444444";
const VENDEUR_UUID: &str = "11111111-1111-1111-1111-111111111111";
const CAISSIER_UUID: &str = "22222222-2222-2222-2222-222222222222";

const DAY1: &str = "2026-09-20";
const DAY2: &str = "2026-09-21";

struct Fixture {
    p1: (ProductId, UnitId),
    p2: (ProductId, UnitId),
    cat_a: CategoryId,
    _cat_b: CategoryId,
    c1: String,
    _c4: String,
}

struct Harness {
    _dir: tempfile::TempDir,
    db: Arc<Mutex<rusqlite::Connection>>,
    _catalog: SqliteCatalogRepository,
    _stock: SqliteStockRepository,
    warehouses: SqliteWarehouseRepository,
    cash: SqliteCashSessionRepository,
    suppliers: SqliteSupplierRepository,
    _purchases: SqlitePurchaseRepository,
    reports: SqliteReportingRepository,
    terminal_id: String,
    warehouse_id: WarehouseId,
}

fn session_with(role: Role, uuid: &str) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(uuid).expect("uuid")),
        role,
        expires_at: chrono::Utc::now() + TimeDelta::hours(1),
    }
}

fn gerant() -> SessionContext {
    session_with(Role::Gerant, GERANT_UUID)
}

fn exec(conn: &rusqlite::Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) {
    conn.execute(sql, rusqlite::params_from_iter(params.iter()))
        .expect("fixture insert");
}

fn setup() -> (Harness, Fixture) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("reports.db");
    let mut conn = open_connection(&path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    for (id, name, role) in [
        (VENDEUR_UUID, "vendeur1", "VENDEUR"),
        (CAISSIER_UUID, "caissier1", "CAISSIER"),
        (GERANT_UUID, "gerant1", "GERANT"),
        (ADMIN_UUID, "admin1", "ADMIN"),
    ] {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'hash', ?3, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            rusqlite::params![id, name, role],
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
        .create_warehouse(&warehouse, &terminal_id)
        .expect("warehouse");
    let catalog = SqliteCatalogRepository::new(Arc::clone(&shared));
    // Categories via direct SQL (master-data write path is out of scope).
    let cat_a = CategoryId::new();
    let cat_b = CategoryId::new();
    {
        let guard = shared.lock().expect("lock");
        for (id, code, ar, fr) in [
            (cat_a.to_string(), "QUIN", "خردوات", "Quincaillerie"),
            (cat_b.to_string(), "ELEC", "كهرباء", "Électricité"),
        ] {
            exec(
                &guard,
                "INSERT INTO categories (id, code, name_ar, name_fr, parent_id, is_active)
                 VALUES (?1, ?2, ?3, ?4, NULL, 1)",
                &[&id, &code.to_string(), &ar.to_string(), &fr.to_string()],
            );
        }
    }
    let seed_product =
        |sku: &str, category: Option<CategoryId>, sale: i64, cost: i64| -> (ProductId, UnitId) {
            let id = ProductId::new();
            let unit_id = UnitId::new();
            catalog
                .create_product(
                    &Product {
                        id,
                        sku: Sku::new(sku).expect("sku"),
                        barcode: None,
                        name_ar: "منتج".to_string(),
                        name_fr: format!("Produit {sku}"),
                        category_id: category,
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
                        sale_price: Money::from_centimes(sale),
                        purchase_price: Money::from_centimes(cost),
                        wholesale_price: None,
                        is_base_unit: true,
                        is_default_selling: true,
                    }],
                    &terminal_id,
                )
                .expect("product");
            (id, unit_id)
        };
    let p1 = seed_product("REP-P1", Some(cat_a), 10_000, 8_000);
    let p2 = seed_product("REP-P2", Some(cat_b), 20_000, 15_000);
    let _p3 = seed_product("REP-P3", None, 5_000, 5_000);

    // Committed sale documents with fixed timestamps (hand-computed below).
    {
        let guard = shared.lock().expect("lock");
        let mut seq = 0;
        let mut doc = |number: &str,
                       doc_type: &str,
                       status: &str,
                       day: &str,
                       time: &str,
                       subtotal: i64,
                       ttc: i64,
                       party: Option<&str>| {
            seq += 1;
            let id = format!("00000000-0000-4000-8000-{seq:012}");
            let created = format!("{day}T{time}");
            exec(
                &guard,
                "INSERT INTO documents
                 (id, number, fiscal_year, type, party_type, party_id,
                  source_document_id, warehouse_id, document_language, language,
                  subtotal_ht_centimes, vat_total_centimes, global_discount_centimes,
                  timbre_centimes, rounding_centimes, ttc_centimes,
                  paid_amount_centimes, change_amount_centimes, payment_method,
                  status, created_at, created_by, revision, origin_device_id, notes)
                 VALUES (?1, ?2, 2026, ?3, ?4, ?5, NULL, ?6, 'FR', 'FR', ?7, 0, 0,
                         0, 0, ?8, ?8, 0, 'CASH', ?9, ?10, ?11, 1, ?12, NULL)",
                &[
                    &id,
                    &number.to_string(),
                    &doc_type.to_string(),
                    &party.map(|_| "CUSTOMER").unwrap_or("NONE").to_string(),
                    &party.map(str::to_string).unwrap_or_default(),
                    &warehouse.id.to_string(),
                    &subtotal,
                    &ttc,
                    &status.to_string(),
                    &created,
                    &GERANT_UUID.to_string(),
                    &terminal_id,
                ],
            );
            id
        };
        let mut item_seq = 0;
        let mut item = |doc: &str,
                        product: ProductId,
                        unit: UnitId,
                        qty_units: i64,
                        price: i64,
                        cost: i64,
                        sort: i64| {
            item_seq += 1;
            let id = format!("11111111-1111-4000-8000-{item_seq:012}");
            let micros = qty_units * 10_000;
            let line_total = price * qty_units;
            exec(
                &guard,
                "INSERT INTO document_items
                 (id, document_id, product_id, unit_id, source_item_id,
                  quantity_micro, quantity_base_micro, unit_price_centimes,
                  discount_centimes, tva_rate_micro, tax_exemption_code,
                  line_ht_centimes, line_tva_centimes, line_ttc_centimes,
                  cost_price_centimes, sort_order)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5, ?6, 0, 0, NULL, ?7, 0, ?7, ?8, ?9)",
                &[
                    &id,
                    &doc.to_string(),
                    &product.to_string(),
                    &unit.to_string(),
                    &micros,
                    &price,
                    &line_total,
                    &cost,
                    &sort,
                ],
            );
        };
        // DAY1: TICKET (P1 x2 @10000, P2 x1 @20000) => ttc 40000.
        let d1 = doc(
            "TICK-2026-000101",
            "TICKET",
            "FINALIZED",
            DAY1,
            "10:00:00.000Z",
            40_000,
            40_000,
            None,
        );
        item(&d1, p1.0, p1.1, 2, 10_000, 8_000, 0);
        item(&d1, p2.0, p2.1, 1, 20_000, 15_000, 1);
        // DAY1: FACTURE (P1 x1 @10000) => ttc 10000.
        let d2 = doc(
            "FAC-2026-000101",
            "FACTURE",
            "FINALIZED",
            DAY1,
            "15:30:00.000Z",
            10_000,
            10_000,
            None,
        );
        item(&d2, p1.0, p1.1, 1, 10_000, 8_000, 0);
        // DAY2: TICKET (P2 x3 @20000) => ttc 60000.
        let d3 = doc(
            "TICK-2026-000102",
            "TICKET",
            "FINALIZED",
            DAY2,
            "09:00:00.000Z",
            60_000,
            60_000,
            None,
        );
        item(&d3, p2.0, p2.1, 3, 20_000, 15_000, 0);
        // DAY2: parked DRAFT (must be excluded from every sales aggregate).
        let d4 = doc(
            "DRAFT-XYZ",
            "TICKET",
            "DRAFT",
            DAY2,
            "10:00:00.000Z",
            99_999,
            99_999,
            None,
        );
        item(&d4, p1.0, p1.1, 9, 10_000, 8_000, 0);
        // DAY2: purchase BC (must be excluded by the sale-type filter).
        doc(
            "BC-2026-000101",
            "BC",
            "DRAFT",
            DAY2,
            "11:00:00.000Z",
            1_000,
            1_000,
            None,
        );
    }

    // Customers: included debtors plus excluded DISABLED / zero-balance rows.
    let customer = |name: &str, policy: &str, limit: i64, balance: i64| -> String {
        let guard = shared.lock().expect("lock");
        let id = Uuid::new_v4().to_string();
        exec(
            &guard,
            "INSERT INTO customers
             (id, code, name, credit_policy, credit_limit_centimes,
              current_balance_centimes, is_active, created_at, updated_at,
              revision, origin_device_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1,
                     '2026-09-20T00:00:00.000Z', '2026-09-20T00:00:00.000Z', 1, ?7)",
            &[
                &id,
                &format!("C-{}", &id[..8]),
                &name.to_string(),
                &policy.to_string(),
                &limit,
                &balance,
                &terminal_id,
            ],
        );
        id
    };
    let c1 = customer("Débiteur Alpha", "LIMITED", 100_000, 15_000);
    let _c2 = customer("Désactivé Bêta", "DISABLED", 0, 0);
    let _c3 = customer("Soldé Gamma", "LIMITED", 50_000, 0);
    let c4 = customer("Illimité Delta", "UNLIMITED", 0, 7_000);
    {
        let guard = shared.lock().expect("lock");
        let ledger = |cid: &str, entry: &str, amount: i64, running: i64, seq: i64| {
            let id = Uuid::new_v4().to_string();
            exec(
                &guard,
                "INSERT INTO customer_ledger
                 (id, sequence_no, customer_id, entry_type, amount_centimes,
                  running_balance_centimes, document_type, document_id,
                  created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'SALE', 'seed', ?7, '2026-09-20T00:00:00.000Z')",
                &[
                    &id,
                    &seq,
                    &cid.to_string(),
                    &entry.to_string(),
                    &amount,
                    &running,
                    &GERANT_UUID.to_string(),
                ],
            );
        };
        ledger(&c1, "DEBIT", 20_000, 20_000, 1);
        ledger(&c1, "CREDIT", 5_000, 15_000, 2);
        ledger(&c4, "DEBIT", 7_000, 7_000, 3);
    }

    // Stock movements: 205 SALE_OUT rows (pagination fixture) + 3 PURCHASE_IN.
    {
        let guard = shared.lock().expect("lock");
        for index in 0..205 {
            let day = if index % 2 == 0 { DAY1 } else { DAY2 };
            let id = Uuid::new_v4().to_string();
            exec(
                &guard,
                "INSERT INTO stock_movements
                 (id, sequence_no, warehouse_id, product_id, movement_type,
                  unit_id, quantity_micro, quantity_base_micro, reference_type,
                  reference_id, transfer_batch_id, authorized_negative_stock,
                  reason, created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'SALE_OUT', ?5, 10000, 10000, 'SALE',
                         NULL, NULL, 0, 'seed', ?6, ?7)",
                &[
                    &id,
                    &(index as i64 + 1),
                    &warehouse.id.to_string(),
                    &p1.0.to_string(),
                    &p1.1.to_string(),
                    &GERANT_UUID.to_string(),
                    &format!("{day}T12:00:00.000Z"),
                ],
            );
        }
        for (index, day) in [DAY1, DAY2, DAY2].into_iter().enumerate() {
            let id = Uuid::new_v4().to_string();
            exec(
                &guard,
                "INSERT INTO stock_movements
                 (id, sequence_no, warehouse_id, product_id, movement_type,
                  unit_id, quantity_micro, quantity_base_micro, reference_type,
                  reference_id, transfer_batch_id, authorized_negative_stock,
                  reason, created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'PURCHASE_IN', ?5, 50000, 50000, 'BR',
                         NULL, NULL, 0, 'seed', ?6, ?7)",
                &[
                    &id,
                    &(1000 + index as i64),
                    &warehouse.id.to_string(),
                    &p2.0.to_string(),
                    &p2.1.to_string(),
                    &GERANT_UUID.to_string(),
                    &format!("{day}T08:00:00.000Z"),
                ],
            );
        }
    }

    // Audit rows across both days (generic payload, never decoded per type).
    {
        let guard = shared.lock().expect("lock");
        for (action, entity, day, user) in [
            ("CASH_SESSION_OPENED", "cash_sessions", DAY1, GERANT_UUID),
            ("PURCHASE_ORDER_CREATED", "documents", DAY2, GERANT_UUID),
            ("STOCK_ADJUSTED", "stock_levels", DAY2, ADMIN_UUID),
        ] {
            exec(
                &guard,
                "INSERT INTO audit_log
                 (prev_hash, record_hash, timestamp, user_id, action,
                  entity_type, entity_id, old_value_json, new_value_json)
                 VALUES ('GENESIS', 'seed-hash', ?1, ?2, ?3, ?4, 'seed-entity', NULL, '{}')",
                &[
                    &format!("{day}T10:00:00.000Z"),
                    &user.to_string(),
                    &action.to_string(),
                    &entity.to_string(),
                ],
            );
        }
    }

    let harness = Harness {
        _dir: dir,
        db: Arc::clone(&shared),
        _catalog: SqliteCatalogRepository::new(Arc::clone(&shared)),
        _stock: SqliteStockRepository::new(Arc::clone(&shared)),
        warehouses,
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
        suppliers: baraka_infrastructure::adapters::SqliteSupplierRepository::new(Arc::clone(
            &shared,
        )),
        _purchases: baraka_infrastructure::adapters::SqlitePurchaseRepository::new(Arc::clone(
            &shared,
        )),
        reports: SqliteReportingRepository::new(Arc::clone(&shared)),
        terminal_id,
        warehouse_id: warehouse.id,
    };
    // One OPEN cash session for the history report (stamped now).
    OpenCashSessionUseCase::new(&harness.cash, &harness.warehouses)
        .execute(
            &session_with(Role::Caissier, CAISSIER_UUID),
            harness.warehouse_id,
            50_000,
            &harness.terminal_id,
            &Uuid::new_v4().to_string(),
        )
        .expect("open session");
    (
        harness,
        Fixture {
            p1,
            p2,
            cat_a,
            _cat_b: cat_b,
            c1,
            _c4: c4,
        },
    )
}

#[test]
fn daily_sales_aggregates_match_hand_computed_values() {
    let (harness, _) = setup();
    let rows = GetDailySalesSummaryUseCase::new(&harness.reports)
        .execute(&gerant(), DAY1, DAY2)
        .expect("daily");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].day, DAY1);
    assert_eq!(rows[0].total_ttc_centimes, 50_000);
    assert_eq!(rows[0].total_ht_centimes, 50_000);
    assert_eq!(rows[0].total_tva_centimes, 0);
    assert_eq!(rows[0].document_count, 2);
    assert_eq!(rows[0].average_basket_centimes, 25_000);
    assert_eq!(rows[1].day, DAY2);
    assert_eq!(rows[1].total_ttc_centimes, 60_000);
    assert_eq!(rows[1].document_count, 1);
    assert_eq!(rows[1].average_basket_centimes, 60_000);
    // Single-day window isolates DAY2 only.
    let single = GetDailySalesSummaryUseCase::new(&harness.reports)
        .execute(&gerant(), DAY2, DAY2)
        .expect("single");
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].document_count, 1);
}

#[test]
fn sales_by_product_aggregates_and_sorts_by_revenue_desc() {
    let (harness, fixture) = setup();
    let rows = GetSalesByProductUseCase::new(&harness.reports)
        .execute(&gerant(), DAY1, DAY2)
        .expect("by product");
    assert_eq!(rows.len(), 2);
    // P2 earned 80_000 (20_000 + 60_000) vs P1's 30_000: revenue desc.
    assert_eq!(rows[0].product_id, fixture.p2.0.to_string());
    assert_eq!(rows[0].revenue_ttc_centimes, 80_000);
    assert_eq!(rows[0].quantity_sold_base_micro, 40_000);
    assert_eq!(rows[0].line_count, 2);
    assert_eq!(rows[1].product_id, fixture.p1.0.to_string());
    assert_eq!(rows[1].revenue_ttc_centimes, 30_000);
    assert_eq!(rows[1].quantity_sold_base_micro, 30_000);
    assert_eq!(rows[1].line_count, 2);
    assert_eq!(rows[1].sku, "REP-P1");
}

#[test]
fn sales_by_category_groups_and_sorts_by_revenue_desc() {
    let (harness, fixture) = setup();
    let rows = GetSalesByCategoryUseCase::new(&harness.reports)
        .execute(&gerant(), DAY1, DAY2)
        .expect("by category");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].category_id.as_deref(),
        Some(fixture._cat_b.to_string())
            .as_deref()
            .map(|_| fixture._cat_b.to_string())
            .as_deref()
            .or(rows[0].category_id.as_deref())
    );
    assert_eq!(rows[0].revenue_ttc_centimes, 80_000);
    assert_eq!(rows[0].line_count, 2);
    assert_eq!(rows[1].category_id, Some(fixture.cat_a.to_string()));
    assert_eq!(rows[1].revenue_ttc_centimes, 30_000);
    // Uncategorized P3 never sold: no NULL bucket appears.
    assert!(rows.iter().all(|row| row.category_id.is_some()));
}

#[test]
fn gross_margin_uses_engine_cost_and_rounding() {
    let (harness, _) = setup();
    let rows = GetGrossMarginReportUseCase::new(&harness.reports)
        .execute(&gerant(), DAY1, DAY2)
        .expect("margin");
    assert_eq!(rows.len(), 2);
    // P2: revenue 80_000, cost 15_000*4 = 60_000, margin 20_000 (first).
    assert_eq!(rows[0].revenue_ht_centimes, 80_000);
    assert_eq!(rows[0].cost_total_centimes, 60_000);
    assert_eq!(rows[0].margin_centimes, 20_000);
    // P1: revenue 30_000, cost 8_000*3 = 24_000, margin 6_000.
    assert_eq!(rows[1].revenue_ht_centimes, 30_000);
    assert_eq!(rows[1].cost_total_centimes, 24_000);
    assert_eq!(rows[1].margin_centimes, 6_000);
}

#[test]
fn credit_outstanding_excludes_disabled_and_zero_balances() {
    let (harness, fixture) = setup();
    let rows = GetCreditOutstandingReportUseCase::new(&harness.reports)
        .execute(&gerant())
        .expect("outstanding");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].customer_id, fixture.c1);
    assert_eq!(rows[0].current_balance_centimes, 15_000);
    assert_eq!(rows[0].credit_policy, "LIMITED");
    assert_eq!(rows[0].credit_limit_centimes, Some(100_000));
    assert_eq!(rows[1].credit_policy, "UNLIMITED");
    assert_eq!(rows[1].credit_limit_centimes, None);
    assert_eq!(rows[1].current_balance_centimes, 7_000);
}

#[test]
fn stock_movement_history_filters_and_rejects_oversized_pages() {
    let (harness, fixture) = setup();
    // TASK-BRK-012A: limit > 200 is rejected, never silently clamped.
    let rejected = GetStockMovementHistoryUseCase::new(&harness.reports).execute(
        &gerant(),
        None,
        None,
        Some("SALE_OUT"),
        DAY1,
        DAY2,
        500,
        0,
    );
    assert!(
        matches!(
            rejected,
            Err(ApplicationError::Validation(
                DomainError::ReportPaginationLimitExceeded
            ))
        ),
        "unexpected: {rejected:?}"
    );
    // The boundary value itself is accepted (200 of 205 SALE_OUT rows).
    let page = GetStockMovementHistoryUseCase::new(&harness.reports)
        .execute(&gerant(), None, None, Some("SALE_OUT"), DAY1, DAY2, 200, 0)
        .expect("page");
    assert_eq!(page.len(), MAX_PAGE_SIZE);
    assert!(page.iter().all(|row| row.movement_type == "SALE_OUT"));
    // Newest-first ordering within the same day prefix.
    assert!(page
        .windows(2)
        .all(|pair| pair[0].created_at >= pair[1].created_at));
    // Product + type filter isolates the 3 PURCHASE_IN rows exactly.
    let filtered = GetStockMovementHistoryUseCase::new(&harness.reports)
        .execute(
            &gerant(),
            Some(&harness.warehouse_id.to_string()),
            Some(&fixture.p2.0.to_string()),
            Some("PURCHASE_IN"),
            DAY1,
            DAY2,
            50,
            0,
        )
        .expect("filtered");
    assert_eq!(filtered.len(), 3);
    // Signed deltas: SALE_OUT negative, PURCHASE_IN positive.
    assert_eq!(filtered[0].quantity_base_micro, 50_000);
    let sale = GetStockMovementHistoryUseCase::new(&harness.reports)
        .execute(
            &gerant(),
            None,
            Some(&fixture.p1.0.to_string()),
            Some("SALE_OUT"),
            DAY1,
            DAY1,
            50,
            0,
        )
        .expect("sale day1");
    assert!(!sale.is_empty());
    assert!(sale.iter().all(|row| row.quantity_base_micro == -10_000));
}

#[test]
fn cash_sessions_history_lists_sessions_with_variance_fields() {
    let (harness, _) = setup();
    let rows = GetCashSessionsHistoryUseCase::new(&harness.reports)
        .execute(&gerant(), None, "2026-01-01", "2099-01-01")
        .expect("history");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "OPEN");
    assert_eq!(rows[0].terminal_id, harness.terminal_id);
    assert_eq!(rows[0].opening_float_centimes, 50_000);
    assert_eq!(rows[0].closed_at, None);
    // Unknown terminal filters to zero rows (no silent fallback).
    let empty = GetCashSessionsHistoryUseCase::new(&harness.reports)
        .execute(&gerant(), Some("NOPE-TERMINAL"), "2026-01-01", "2099-01-01")
        .expect("empty");
    assert!(empty.is_empty());
}

#[test]
fn user_activity_filters_paginates_and_keeps_payload_generic() {
    let (harness, _) = setup();
    let all = GetUserActivityLogUseCase::new(&harness.reports)
        .execute(&gerant(), None, None, "2026-01-01", "2099-01-01", 50, 0)
        .expect("all");
    // Three seeded fixture rows plus the CASH_SESSION_OPENED row written by
    // the setup's own cash-session open (all part of the audit truth).
    assert_eq!(all.len(), 4);
    // Newest first by monotonic sequence number.
    assert!(all
        .windows(2)
        .all(|pair| pair[0].sequence_no > pair[1].sequence_no));
    // Raw payload is preserved verbatim, never decoded per entity type.
    // (The newest row is the setup's own cash-session open; the '{}'
    // fixture payload sits somewhere below it.)
    assert_eq!(all[0].action, "CASH_SESSION_OPENED");
    assert!(
        all.iter()
            .any(|row| row.new_value_json.as_deref() == Some("{}")),
        "seeded generic payload must survive verbatim"
    );
    let filtered = GetUserActivityLogUseCase::new(&harness.reports)
        .execute(
            &gerant(),
            Some(GERANT_UUID),
            Some("PURCHASE_ORDER_CREATED"),
            "2026-01-01",
            "2099-01-01",
            50,
            0,
        )
        .expect("filtered");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].action, "PURCHASE_ORDER_CREATED");
    // Oversized page is rejected (TASK-BRK-012A); offset pages the remainder.
    let big = GetUserActivityLogUseCase::new(&harness.reports).execute(
        &gerant(),
        None,
        None,
        "2026-01-01",
        "2099-01-01",
        500,
        0,
    );
    assert!(
        matches!(
            big,
            Err(ApplicationError::Validation(
                DomainError::ReportPaginationLimitExceeded
            ))
        ),
        "unexpected: {big:?}"
    );
    let tail = GetUserActivityLogUseCase::new(&harness.reports)
        .execute(&gerant(), None, None, "2026-01-01", "2099-01-01", 50, 3)
        .expect("tail");
    assert_eq!(tail.len(), 1);
}

#[test]
fn inverted_date_ranges_reject_with_invalid_date_range() {
    let (harness, fixture) = setup();
    let wid = harness.warehouse_id.to_string();
    let pid = fixture.p1.0.to_string();
    assert!(matches!(
        GetDailySalesSummaryUseCase::new(&harness.reports).execute(&gerant(), DAY2, DAY1),
        Err(ApplicationError::Validation(DomainError::InvalidDateRange))
    ));
    assert!(matches!(
        GetSalesByProductUseCase::new(&harness.reports).execute(&gerant(), DAY2, DAY1),
        Err(ApplicationError::Validation(DomainError::InvalidDateRange))
    ));
    assert!(matches!(
        GetSalesByCategoryUseCase::new(&harness.reports).execute(&gerant(), DAY2, DAY1),
        Err(ApplicationError::Validation(DomainError::InvalidDateRange))
    ));
    assert!(matches!(
        GetGrossMarginReportUseCase::new(&harness.reports).execute(&gerant(), DAY2, DAY1),
        Err(ApplicationError::Validation(DomainError::InvalidDateRange))
    ));
    assert!(matches!(
        GetStockMovementHistoryUseCase::new(&harness.reports).execute(
            &gerant(),
            Some(&wid),
            Some(&pid),
            None,
            DAY2,
            DAY1,
            50,
            0
        ),
        Err(ApplicationError::Validation(DomainError::InvalidDateRange))
    ));
    assert!(matches!(
        GetCashSessionsHistoryUseCase::new(&harness.reports).execute(&gerant(), None, DAY2, DAY1),
        Err(ApplicationError::Validation(DomainError::InvalidDateRange))
    ));
    assert!(matches!(
        GetUserActivityLogUseCase::new(&harness.reports).execute(
            &gerant(),
            None,
            None,
            DAY2,
            DAY1,
            50,
            0
        ),
        Err(ApplicationError::Validation(DomainError::InvalidDateRange))
    ));
}

#[test]
fn rbac_matrix_denies_vendeur_caissier_permits_gerant_admin() {
    let (harness, fixture) = setup();
    let wid = harness.warehouse_id.to_string();
    let pid = fixture.p1.0.to_string();
    for denied in [
        session_with(Role::Vendeur, VENDEUR_UUID),
        session_with(Role::Caissier, CAISSIER_UUID),
    ] {
        assert!(matches!(
            GetDailySalesSummaryUseCase::new(&harness.reports).execute(&denied, DAY1, DAY2),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            GetSalesByProductUseCase::new(&harness.reports).execute(&denied, DAY1, DAY2),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            GetSalesByCategoryUseCase::new(&harness.reports).execute(&denied, DAY1, DAY2),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            GetGrossMarginReportUseCase::new(&harness.reports).execute(&denied, DAY1, DAY2),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            GetCreditOutstandingReportUseCase::new(&harness.reports).execute(&denied),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            GetStockMovementHistoryUseCase::new(&harness.reports).execute(
                &denied,
                Some(&wid),
                Some(&pid),
                None,
                DAY1,
                DAY2,
                50,
                0
            ),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            GetCashSessionsHistoryUseCase::new(&harness.reports).execute(&denied, None, DAY1, DAY2),
            Err(ApplicationError::Unauthorized)
        ));
        assert!(matches!(
            GetUserActivityLogUseCase::new(&harness.reports)
                .execute(&denied, None, None, DAY1, DAY2, 50, 0),
            Err(ApplicationError::Unauthorized)
        ));
    }
    for permitted in [
        session_with(Role::Gerant, GERANT_UUID),
        session_with(Role::Admin, ADMIN_UUID),
    ] {
        assert!(GetDailySalesSummaryUseCase::new(&harness.reports)
            .execute(&permitted, DAY1, DAY2)
            .is_ok());
        assert!(GetSalesByProductUseCase::new(&harness.reports)
            .execute(&permitted, DAY1, DAY2)
            .is_ok());
        assert!(GetSalesByCategoryUseCase::new(&harness.reports)
            .execute(&permitted, DAY1, DAY2)
            .is_ok());
        assert!(GetGrossMarginReportUseCase::new(&harness.reports)
            .execute(&permitted, DAY1, DAY2)
            .is_ok());
        assert!(GetCreditOutstandingReportUseCase::new(&harness.reports)
            .execute(&permitted)
            .is_ok());
        assert!(GetStockMovementHistoryUseCase::new(&harness.reports)
            .execute(&permitted, None, None, None, DAY1, DAY2, 50, 0)
            .is_ok());
        assert!(GetCashSessionsHistoryUseCase::new(&harness.reports)
            .execute(&permitted, None, DAY1, DAY2)
            .is_ok());
        assert!(GetUserActivityLogUseCase::new(&harness.reports)
            .execute(&permitted, None, None, DAY1, DAY2, 50, 0)
            .is_ok());
    }
}

#[test]
fn origin_device_id_matches_live_terminal_on_supplier_rows() {
    let (harness, _) = setup();
    let view =
        baraka_application::use_cases::suppliers::CreateSupplierUseCase::new(&harness.suppliers)
            .execute(
                &gerant(),
                baraka_application::ports::NewSupplier {
                    name: "Origine Exacte".to_string(),
                    phone: None,
                    address: None,
                    nif: None,
                    rc: None,
                    notes: None,
                },
                &harness.terminal_id,
            )
            .expect("supplier");
    assert_eq!(view.origin_device_id, harness.terminal_id);
}

#[test]
fn low_stock_lists_depleted_levels_most_depleted_first() {
    let (harness, fixture) = setup();
    {
        let guard = harness.db.lock().expect("lock");
        let put = |product: ProductId, micros: i64| {
            guard
                .execute(
                    "INSERT INTO stock_levels
                     (warehouse_id, product_id, quantity_micro, quantity_base_micro,
                      revision, updated_at)
                     VALUES (?1, ?2, ?3, ?3, 0, '2026-09-21T00:00:00.000Z')
                     ON CONFLICT(warehouse_id, product_id) DO UPDATE
                     SET quantity_base_micro = excluded.quantity_base_micro",
                    rusqlite::params![
                        harness.warehouse_id.to_string(),
                        product.to_string(),
                        micros,
                    ],
                )
                .expect("level");
        };
        // P1 is stocked out, P2 holds a negative backorder level.
        put(fixture.p1.0, 0);
        put(fixture.p2.0, -25_000);
    }
    // P3 has no level row at all: absent rows are not reported (ADR-037
    // covers stocked pairs only; unknown is not conflated with depleted).
    let rows = GetLowStockReportUseCase::new(&harness.reports)
        .execute(&gerant(), None)
        .expect("low stock");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].product_id, fixture.p2.0.to_string());
    assert_eq!(rows[0].quantity_base_micro, -25_000);
    assert_eq!(rows[0].warehouse_code, "MAIN");
    assert_eq!(rows[1].product_id, fixture.p1.0.to_string());
    assert_eq!(rows[1].quantity_base_micro, 0);
    // Warehouse filter narrows; unknown warehouse yields zero rows.
    let scoped = GetLowStockReportUseCase::new(&harness.reports)
        .execute(&gerant(), Some(&harness.warehouse_id.to_string()))
        .expect("scoped");
    assert_eq!(scoped.len(), 2);
    let empty = GetLowStockReportUseCase::new(&harness.reports)
        .execute(&gerant(), Some("00000000-0000-4000-8000-000000000000"))
        .expect("empty");
    assert!(empty.is_empty());
}

#[test]
fn gross_margin_flags_zero_cost_lines_as_untracked() {
    let (harness, fixture) = setup();
    // A DAY2 line for P1 with zero unit cost: untracked-cost fixture.
    {
        let guard = harness.db.lock().expect("lock");
        guard
            .execute(
                "INSERT INTO document_items
                 (id, document_id, product_id, unit_id, source_item_id,
                  quantity_micro, quantity_base_micro, unit_price_centimes,
                  discount_centimes, tva_rate_micro, tax_exemption_code,
                  line_ht_centimes, line_tva_centimes, line_ttc_centimes,
                  cost_price_centimes, sort_order)
                 VALUES ('22222222-2222-4000-8000-000000000001',
                         (SELECT id FROM documents WHERE number = 'TICK-2026-000102'),
                         ?1, ?2, NULL, 10000, 10000, 10000, 0, 0, NULL,
                         10000, 0, 10000, 0, 9)",
                rusqlite::params![fixture.p1.0.to_string(), fixture.p1.1.to_string()],
            )
            .expect("zero-cost line");
    }
    let rows = GetGrossMarginReportUseCase::new(&harness.reports)
        .execute(&gerant(), DAY1, DAY2)
        .expect("margin");
    let p1 = rows
        .iter()
        .find(|row| row.product_id == fixture.p1.0.to_string())
        .expect("p1 bucket");
    // Revenue grew by the extra 10_000 line; its cost contributes 0 and the
    // bucket is flagged rather than silently absorbing the gap.
    assert_eq!(p1.revenue_ht_centimes, 40_000);
    assert_eq!(p1.cost_total_centimes, 24_000);
    assert_eq!(p1.margin_centimes, 16_000);
    assert!(p1.has_untracked_cost);
    let p2 = rows
        .iter()
        .find(|row| row.product_id == fixture.p2.0.to_string())
        .expect("p2 bucket");
    assert!(!p2.has_untracked_cost);
}
