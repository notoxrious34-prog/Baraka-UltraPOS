//! First-run admin bootstrap tests over real file databases.
//!
//! File-based temporary databases (WAL/crash standards): every test opens
//! its own file in a fresh tempdir and runs all migrations. Coverage:
//! first-run probe semantics, single-Admin creation with the production
//! Argon2id hasher, post-bootstrap rejection, a two-connection race proving
//! exactly one winner, the audit event, and the app-data-dir path policy
//! (ADR-049).

use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{PasswordHasher, UserRepository};
use baraka_application::use_cases::{BootstrapFirstAdminInput, BootstrapFirstAdminUseCase};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Role;
use baraka_infrastructure::adapters::{Argon2PasswordHasher, SqliteUserRepository, SystemClock};
use baraka_infrastructure::persistence::connection::{
    default_db_file_name, ensure_database_dir, open_connection, resolve_database_path,
};
use baraka_infrastructure::persistence::migration::run_migrations;

const TEST_TERMINAL: &str = "TEST-TERMINAL-016";

struct Harness {
    _dir: tempfile::TempDir,
    users: SqliteUserRepository,
    db: Arc<Mutex<rusqlite::Connection>>,
}

fn setup(file_name: &str) -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(file_name);
    let mut conn = open_connection(&db_path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let db = Arc::new(Mutex::new(conn));
    let users = SqliteUserRepository::new(Arc::clone(&db));
    Harness {
        _dir: dir,
        users,
        db,
    }
}

fn bootstrap(
    users: &SqliteUserRepository,
    username: &str,
    password: &str,
) -> Result<String, ApplicationError> {
    let hasher = Argon2PasswordHasher;
    let clock = SystemClock;
    BootstrapFirstAdminUseCase::new(&hasher, users, &clock)
        .execute(
            &BootstrapFirstAdminInput {
                username: username.to_string(),
                password: password.to_string(),
            },
            TEST_TERMINAL,
        )
        .map(|output| output.user_id.to_string())
}

fn user_count(harness: &Harness) -> i64 {
    harness
        .db
        .lock()
        .expect("db lock")
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .expect("count")
}

fn is_bootstrap_rejection(error: &ApplicationError) -> bool {
    matches!(
        error,
        ApplicationError::Validation(DomainError::BootstrapAlreadyCompleted)
    )
}

#[test]
fn fresh_database_requires_setup_with_zero_users() {
    let harness = setup("fresh.db");
    assert_eq!(harness.users.count_users().expect("count"), 0);
    assert_eq!(user_count(&harness), 0);
}

#[test]
fn bootstrap_creates_exactly_one_active_admin_with_argon2id_hash() {
    let harness = setup("create.db");
    let user_id = bootstrap(&harness.users, "  chef  ", "S3cret-Pass!").expect("bootstrap");

    // Exactly one row, trimmed name, Admin role, active.
    assert_eq!(user_count(&harness), 1);
    let guard = harness.db.lock().expect("db lock");
    let (stored_name, role, is_active, stored_hash): (String, String, i64, String) = guard
        .query_row(
            "SELECT username, role, is_active, password_hash FROM users WHERE id = ?1",
            [&user_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("row");
    assert_eq!(stored_name, "chef");
    assert_eq!(role, Role::Admin.code());
    assert_eq!(is_active, 1);
    drop(guard);

    // The stored secret verifies through the production Argon2id
    // implementation (TASK-BRK-003 reuse: no new hashing code exists).
    assert!(stored_hash.starts_with("$argon2id$"));
    assert!(Argon2PasswordHasher
        .verify("S3cret-Pass!", &stored_hash)
        .expect("verify"));
    assert!(!Argon2PasswordHasher
        .verify("wrong-password", &stored_hash)
        .expect("verify"));

    // Probe flips to login mode once a user exists.
    assert_eq!(harness.users.count_users().expect("count"), 1);
}

#[test]
fn bootstrap_writes_first_admin_audit_event_with_terminal_origin() {
    let harness = setup("audit.db");
    let user_id = bootstrap(&harness.users, "chef", "S3cret-Pass!").expect("bootstrap");
    let guard = harness.db.lock().expect("db lock");
    let (action, entity_type, entity_id, actor, new_json): (
        String,
        String,
        String,
        String,
        String,
    ) = guard
        .query_row(
            "SELECT action, entity_type, entity_id, user_id, new_value_json
             FROM audit_log WHERE action = 'FIRST_ADMIN_BOOTSTRAP'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("audit row");
    assert_eq!(action, "FIRST_ADMIN_BOOTSTRAP");
    assert_eq!(entity_type, "users");
    assert_eq!(entity_id, user_id);
    assert_eq!(actor, user_id);
    assert!(new_json.contains("\"role\":\"ADMIN\""));
    assert!(new_json.contains("\"username\":\"chef\""));
    assert!(new_json.contains(TEST_TERMINAL));
}

#[test]
fn second_bootstrap_is_rejected_with_zero_extra_rows() {
    let harness = setup("repeat.db");
    bootstrap(&harness.users, "chef", "S3cret-Pass!").expect("first");
    let rejected = bootstrap(&harness.users, "intruder", "Other-Pass!").expect_err("second");
    assert!(
        is_bootstrap_rejection(&rejected),
        "expected BootstrapAlreadyCompleted, got {rejected:?}"
    );
    assert_eq!(user_count(&harness), 1);
    assert!(harness
        .users
        .find_by_username("intruder")
        .expect("lookup")
        .is_none());
}

#[test]
fn concurrent_bootstrap_race_yields_exactly_one_admin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("race.db");
    let mut conn = open_connection(&db_path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    drop(conn);

    // Two independent connections to the same file: the closest
    // faithful simulation of two first-run windows racing.
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for attempt in ["racer-a", "racer-b"] {
        let path = db_path.clone();
        let gate = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let conn = open_connection(&path).expect("open");
            let users = SqliteUserRepository::new(Arc::new(Mutex::new(conn)));
            gate.wait();
            bootstrap(&users, attempt, "R4cing-Pass!")
        }));
    }
    let mut successes = 0;
    let mut rejections = 0;
    for handle in handles {
        match handle.join().expect("thread") {
            Ok(_) => successes += 1,
            Err(error) => {
                assert!(
                    is_bootstrap_rejection(&error),
                    "loser must see BootstrapAlreadyCompleted, got {error:?}"
                );
                rejections += 1;
            }
        }
    }
    assert_eq!(successes, 1, "exactly one racer must win");
    assert_eq!(rejections, 1, "exactly one racer must be rejected");
    let check = open_connection(&db_path).expect("open");
    let final_count: i64 = check
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .expect("count");
    assert_eq!(final_count, 1);
}

#[test]
fn database_path_resolves_under_app_data_dir_not_cwd() {
    let app_data_dir = std::path::Path::new("/tmp/BarakaTestAppData");
    let resolved = resolve_database_path(app_data_dir);
    assert_eq!(
        resolved,
        app_data_dir.join(default_db_file_name()),
        "path must join the official file name onto the app-data dir"
    );
    assert_eq!(resolved.file_name().expect("file name"), "baraka.db");
    assert_ne!(
        resolved,
        std::path::Path::new("./baraka-dev.sqlite3"),
        "the CWD-relative dev path must be gone"
    );
}

#[test]
fn database_dir_creation_is_idempotent_and_reports_io_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let nested = dir.path().join("level-one").join("level-two");
    assert!(!nested.exists());
    ensure_database_dir(&nested).expect("create");
    assert!(nested.is_dir());
    // Idempotent: existing directories are a silent success.
    ensure_database_dir(&nested).expect("re-create");

    // Simulated I/O failure: a regular file where a directory is required.
    let blocker = dir.path().join("blocker-file");
    std::fs::write(&blocker, b"not a directory").expect("write blocker");
    let impossible = blocker.join("child");
    let failure = ensure_database_dir(&impossible).expect_err("must fail");
    assert!(
        format!("{failure}").contains("cannot create application data directory"),
        "startup must receive a clear message, got {failure:?}"
    );
}
