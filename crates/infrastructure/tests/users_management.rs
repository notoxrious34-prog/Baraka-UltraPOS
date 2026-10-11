//! End-to-end users-management tests over a real SQLite backend.
//!
//! Covers the TASK-BRK-064 contract: admin list/create, non-admin RBAC
//! rejection, last-admin deactivation/demotion protection, and credential
//! reset with hash replacement.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{PasswordHasher, SessionContext, SessionToken, UserRepository};
use baraka_application::use_cases::users::{
    CreateUserUseCase, ListUsersUseCase, ResetUserCredentialsUseCase, UpdateUserUseCase,
};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{SqliteUserRepository, SystemClock};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::{DateTime, Utc};

const ADMIN_UUID: &str = "11111111-1111-1111-1111-111111111111";
const VENDEUR_UUID: &str = "22222222-2222-2222-2222-222222222222";

struct StubHasher;

impl PasswordHasher for StubHasher {
    fn hash(&self, plaintext: &str) -> Result<String, ApplicationError> {
        Ok(format!("hash:{plaintext}"))
    }

    fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, ApplicationError> {
        Ok(hash == format!("hash:{plaintext}"))
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    users: SqliteUserRepository,
    hasher: StubHasher,
    clock: SystemClock,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("users.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, pin_hash, role,
                            is_active, created_at, updated_at,
                            failed_login_attempts, locked_until)
         VALUES (?1, 'admin', 'hash:admin', NULL, 'ADMIN',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 0, NULL)",
        [ADMIN_UUID],
    )
    .expect("seed admin");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, pin_hash, role,
                            is_active, created_at, updated_at,
                            failed_login_attempts, locked_until)
         VALUES (?1, 'vendeur1', 'hash:vendeur1', NULL, 'VENDEUR',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 0, NULL)",
        [VENDEUR_UUID],
    )
    .expect("seed vendeur");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        users: SqliteUserRepository::new(Arc::clone(&shared)),
        hasher: StubHasher,
        clock: SystemClock,
    }
}

fn session(user_id: &str, role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken("test-token".to_string()),
        user_id: UserId::from(uuid::Uuid::parse_str(user_id).expect("valid test uuid")),
        role,
        expires_at: DateTime::<Utc>::MAX_UTC,
    }
}

fn admin_session() -> SessionContext {
    session(ADMIN_UUID, Role::Admin)
}

fn vendeur_session() -> SessionContext {
    session(VENDEUR_UUID, Role::Vendeur)
}

#[test]
fn admin_lists_seeded_operators() {
    let harness = setup();
    let rows = ListUsersUseCase::new(&harness.users)
        .execute(&admin_session())
        .expect("admin lists users");
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .any(|row| row.username == "admin" && row.role == Role::Admin));
    assert!(rows.iter().any(|row| row.username == "vendeur1"));
}

#[test]
fn admin_creates_caissier_with_pin() {
    let harness = setup();
    let record = CreateUserUseCase::new(&harness.hasher, &harness.users, &harness.clock)
        .execute(
            &admin_session(),
            "caisse1",
            "secret12",
            Some("1234"),
            Role::Caissier,
        )
        .expect("admin creates user");
    assert_eq!(record.username, "caisse1");
    assert_eq!(record.role, Role::Caissier);
    assert!(record.is_active);
    assert!(record.has_pin);
    let stored = harness
        .users
        .get_password_hash(record.id)
        .expect("password hash stored");
    assert_eq!(stored, "hash:secret12");
}

#[test]
fn duplicate_username_conflicts() {
    let harness = setup();
    let error = CreateUserUseCase::new(&harness.hasher, &harness.users, &harness.clock)
        .execute(
            &admin_session(),
            "vendeur1",
            "secret12",
            None,
            Role::Caissier,
        )
        .expect_err("duplicate username must fail");
    assert!(matches!(error, ApplicationError::Conflict(code) if code == "USERNAME_TAKEN"));
}

#[test]
fn vendeur_cannot_list_or_create() {
    let harness = setup();
    let list_error = ListUsersUseCase::new(&harness.users)
        .execute(&vendeur_session())
        .expect_err("vendeur list must fail");
    assert!(matches!(list_error, ApplicationError::Unauthorized));
    let create_error = CreateUserUseCase::new(&harness.hasher, &harness.users, &harness.clock)
        .execute(
            &vendeur_session(),
            "nope1",
            "secret12",
            None,
            Role::Caissier,
        )
        .expect_err("vendeur create must fail");
    assert!(matches!(create_error, ApplicationError::Unauthorized));
}

#[test]
fn last_admin_deactivation_is_rejected() {
    let harness = setup();
    let admin_id = UserId::from(uuid::Uuid::parse_str(ADMIN_UUID).expect("valid uuid"));
    let error = UpdateUserUseCase::new(&harness.users, &harness.clock)
        .execute(&admin_session(), admin_id, None, Some(false))
        .expect_err("last admin deactivation must fail");
    assert!(matches!(error, ApplicationError::Conflict(code) if code == "LAST_ADMIN_PROTECTED"));
}

#[test]
fn last_admin_demotion_is_rejected() {
    let harness = setup();
    let admin_id = UserId::from(uuid::Uuid::parse_str(ADMIN_UUID).expect("valid uuid"));
    let error = UpdateUserUseCase::new(&harness.users, &harness.clock)
        .execute(&admin_session(), admin_id, Some(Role::Gerant), None)
        .expect_err("last admin demotion must fail");
    assert!(matches!(error, ApplicationError::Conflict(code) if code == "LAST_ADMIN_PROTECTED"));
}

#[test]
fn second_admin_frees_the_first() {
    let harness = setup();
    let second = CreateUserUseCase::new(&harness.hasher, &harness.users, &harness.clock)
        .execute(&admin_session(), "admin2", "secret12", None, Role::Admin)
        .expect("second admin created");
    let admin_id = UserId::from(uuid::Uuid::parse_str(ADMIN_UUID).expect("valid uuid"));
    let updated = UpdateUserUseCase::new(&harness.users, &harness.clock)
        .execute(&admin_session(), admin_id, None, Some(false))
        .expect("deactivation allowed with two admins");
    assert!(!updated.is_active);
    assert_eq!(second.role, Role::Admin);
}

#[test]
fn admin_resets_password_and_pin() {
    let harness = setup();
    let target = UserId::from(uuid::Uuid::parse_str(VENDEUR_UUID).expect("valid uuid"));
    ResetUserCredentialsUseCase::new(&harness.hasher, &harness.users, &harness.clock)
        .execute(&admin_session(), target, Some("newpass1"), Some("9999"))
        .expect("reset succeeds");
    let stored = harness
        .users
        .get_password_hash(target)
        .expect("password hash stored");
    assert_eq!(stored, "hash:newpass1");
    let view = harness
        .users
        .find_user_by_id(target)
        .expect("lookup works")
        .expect("user present");
    assert!(view.has_pin);
}
