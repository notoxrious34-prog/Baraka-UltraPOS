//! Warehouse repository + use-case tests over real SQLite.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{SessionContext, SessionToken, WarehouseRepository};
use baraka_application::use_cases::inventory::{
    CreateWarehouseUseCase, ListWarehousesUseCase, WarehouseStockPresenceUseCase,
};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::SqliteWarehouseRepository;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::TimeDelta;

struct Harness {
    _dir: tempfile::TempDir,
    repo: SqliteWarehouseRepository,
}

const TEST_TERMINAL: &str = "TEST-TERMINAL-WH";

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("wh.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    Harness {
        _dir: dir,
        repo: SqliteWarehouseRepository::new(Arc::new(Mutex::new(conn))),
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
fn admin_creates_and_lists_while_vendeur_is_rejected() {
    let harness = setup();
    let admin = session_with(Role::Admin);
    let create = CreateWarehouseUseCase::new(&harness.repo);
    let id = create
        .execute(&admin, "MAIN", "Principal", TEST_TERMINAL)
        .expect("create");
    let fetched = harness.repo.get_by_id(id).expect("lookup").expect("row");
    assert_eq!(fetched.code.as_str(), "MAIN");
    assert!(fetched.is_active);

    let list = ListWarehousesUseCase::new(&harness.repo)
        .execute(&admin)
        .expect("list");
    // Migration 0012 seeds DEP-01, so the fresh database already holds one row.
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|w| w.code.as_str() == "MAIN"));

    let vendeur = session_with(Role::Vendeur);
    assert!(matches!(
        create.execute(&vendeur, "EAST", "Est", TEST_TERMINAL),
        Err(ApplicationError::Unauthorized)
    ));
    // Reads stay open to every authenticated role.
    assert_eq!(
        ListWarehousesUseCase::new(&harness.repo)
            .execute(&vendeur)
            .expect("vendeur list")
            .len(),
        2
    );
}

#[test]
fn duplicate_code_and_validation_rules() {
    let harness = setup();
    let admin = session_with(Role::Admin);
    let create = CreateWarehouseUseCase::new(&harness.repo);
    create
        .execute(&admin, "MAIN", "Principal", TEST_TERMINAL)
        .expect("first");
    let dup = create
        .execute(&admin, "MAIN", "Doublon", TEST_TERMINAL)
        .expect_err("dup");
    assert!(matches!(
        dup,
        ApplicationError::Conflict(message) if message == "WAREHOUSE_CODE_EXISTS"
    ));
    for (code, name) in [("", "Ok"), ("MAIN2", ""), ("   ", "Ok"), ("MAIN3", "  ")] {
        assert!(
            create.execute(&admin, code, name, TEST_TERMINAL).is_err(),
            "must reject code={code:?} name={name:?}"
        );
    }
    assert!(create
        .execute(&admin, &"C".repeat(21), "Ok", TEST_TERMINAL)
        .is_err());
    assert!(create
        .execute(&admin, "OK", &"N".repeat(101), TEST_TERMINAL)
        .is_err());
}

#[test]
fn list_active_hides_deactivated() {
    let harness = setup();
    let admin = session_with(Role::Admin);
    let create = CreateWarehouseUseCase::new(&harness.repo);
    create
        .execute(&admin, "MAIN", "Principal", TEST_TERMINAL)
        .expect("main");
    create
        .execute(&admin, "EAST", "Est", TEST_TERMINAL)
        .expect("east");
    // No deactivate port exists in this task; flip state directly.
    let db_path = harness._dir.path().join("wh.db");
    let conn = rusqlite::Connection::open(db_path).expect("reopen");
    conn.execute(
        "UPDATE warehouses SET is_active = 0 WHERE code = 'EAST'",
        [],
    )
    .expect("deactivate");
    let listed = ListWarehousesUseCase::new(&harness.repo)
        .execute(&admin)
        .expect("list");
    // Migration 0012 seeds DEP-01 (active); EAST is deactivated above, so the
    // active set ordered by code is exactly [DEP-01, MAIN].
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].code.as_str(), "DEP-01");
    assert_eq!(listed[1].code.as_str(), "MAIN");
}

#[test]
fn created_warehouse_persists_terminal_origin_device_id() {
    let harness = setup();
    let admin = session_with(Role::Admin);
    let create = CreateWarehouseUseCase::new(&harness.repo);
    let terminal = "TERMINAL-ORIGIN-003";
    let id = create
        .execute(&admin, "ORIGIN", "Origine", terminal)
        .expect("create");
    let stored: String = rusqlite::Connection::open(harness._dir.path().join("wh.db"))
        .expect("reopen")
        .query_row(
            "SELECT origin_device_id FROM warehouses WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .expect("origin");
    assert_eq!(stored, terminal);
}

/// Seed one product row (FK target for `stock_levels` inserts).
fn seed_product(conn: &rusqlite::Connection, sku: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO products (id, sku, name_ar, name_fr, created_at, updated_at, origin_device_id) \
         VALUES (?1, ?2, 'صنف', 'Article', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 'TEST')",
        rusqlite::params![id, sku],
    )
    .expect("seed product");
    id
}

/// Snapshot mutable-table row counts (zero-write proof baseline).
fn row_counts(conn: &rusqlite::Connection) -> Vec<(String, i64)> {
    ["warehouses", "stock_levels", "stock_movements", "audit_log"]
        .iter()
        .map(|table| {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count");
            (table.to_string(), count)
        })
        .collect()
}

#[test]
fn presence_empty_depot_reports_zero() {
    let harness = setup();
    let admin = session_with(Role::Admin);
    CreateWarehouseUseCase::new(&harness.repo)
        .execute(&admin, "MAIN", "Principal", TEST_TERMINAL)
        .expect("create");
    let presence = WarehouseStockPresenceUseCase::new(&harness.repo)
        .execute(&admin)
        .expect("presence");
    // DEP-01 (seeded) + MAIN, code-ordered, both untouched by stock.
    assert_eq!(presence.len(), 2);
    assert!(presence.iter().all(|row| row.nonzero_levels == 0));
}

#[test]
fn presence_stocked_depot_reports_counts_code_ordered() {
    let harness = setup();
    let admin = session_with(Role::Admin);
    let create = CreateWarehouseUseCase::new(&harness.repo);
    let main = create
        .execute(&admin, "MAIN", "Principal", TEST_TERMINAL)
        .expect("main");
    let east = create
        .execute(&admin, "EAST", "Est", TEST_TERMINAL)
        .expect("east");
    let db = open_connection(&harness._dir.path().join("wh.db")).expect("reopen");
    let sku_a = seed_product(&db, "SKU-A");
    let sku_b = seed_product(&db, "SKU-B");
    // Two non-zero rows in MAIN, one zero-quantity row in EAST (must not count).
    for (warehouse, product, qty) in [
        (main.to_string(), sku_a.clone(), 5_000),
        (main.to_string(), sku_b.clone(), 2_000),
        (east.to_string(), sku_a.clone(), 0),
    ] {
        db.execute(
            "INSERT INTO stock_levels (warehouse_id, product_id, quantity_base_micro) VALUES (?1, ?2, ?3)",
            rusqlite::params![warehouse, product, qty],
        )
        .expect("seed level");
    }
    let presence = WarehouseStockPresenceUseCase::new(&harness.repo)
        .execute(&admin)
        .expect("presence");
    // DEP-01, EAST, MAIN — code order, DEP-01 truthfully empty here.
    let codes: Vec<u64> = presence.iter().map(|row| row.nonzero_levels).collect();
    assert_eq!(codes, vec![0, 0, 2]);
}

#[test]
fn presence_reports_dep01_stock_and_writes_nothing() {
    let harness = setup();
    let admin = session_with(Role::Admin);
    let db = open_connection(&harness._dir.path().join("wh.db")).expect("reopen");
    let dep01: String = db
        .query_row(
            "SELECT id FROM warehouses WHERE code = 'DEP-01'",
            [],
            |row| row.get(0),
        )
        .expect("dep01");
    let sku = seed_product(&db, "SKU-DEP");
    db.execute(
        "INSERT INTO stock_levels (warehouse_id, product_id, quantity_base_micro) VALUES (?1, ?2, 100)",
        rusqlite::params![dep01, sku],
    )
    .expect("seed level");
    let before = row_counts(&db);
    let presence = WarehouseStockPresenceUseCase::new(&harness.repo)
        .execute(&admin)
        .expect("presence");
    // The wire does not hide DEP-01: the lock badge stays a UI decision.
    let dep = presence.first().expect("dep01 row");
    assert_eq!(dep.nonzero_levels, 1);
    let after = row_counts(&open_connection(&harness._dir.path().join("wh.db")).expect("reopen"));
    assert_eq!(before, after, "presence read must not write");
}

#[test]
fn presence_rejects_vendeur_like_deletion_path() {
    let harness = setup();
    let vendeur = session_with(Role::Vendeur);
    assert!(matches!(
        WarehouseStockPresenceUseCase::new(&harness.repo).execute(&vendeur),
        Err(ApplicationError::Unauthorized)
    ));
}
