//! Warehouse repository + use-case tests over real SQLite.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{SessionContext, SessionToken, WarehouseRepository};
use baraka_application::use_cases::inventory::{CreateWarehouseUseCase, ListWarehousesUseCase};
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
