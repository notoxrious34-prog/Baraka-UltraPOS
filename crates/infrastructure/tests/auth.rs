//! End-to-end authentication tests over real backends (Argon2id, SQLite,
//! in-memory sessions, system clock). Fake-clock time travel lives in the
//! application unit tests; these tests prove the real wiring.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{PasswordHasher, SessionStore, UserRepository};
use baraka_application::use_cases::{AuthenticateUserUseCase, LogoutUseCase, VerifyPinUseCase};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    Argon2PasswordHasher, InMemorySessionStore, SqliteUserRepository, SystemClock,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use rusqlite::Connection;
use uuid::Uuid;

struct Harness {
    _dir: tempfile::TempDir,
    users: SqliteUserRepository,
    sessions: InMemorySessionStore,
    clock: Arc<SystemClock>,
    hasher: Argon2PasswordHasher,
}

fn setup_with_pin(pin: Option<&str>) -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("auth.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let hasher = Argon2PasswordHasher;
    let password_hash = hasher.hash("secret").expect("hash password");
    let pin_hash: Option<String> = pin.map(|p| hasher.hash(p).expect("hash pin"));
    conn.execute(
        "INSERT INTO users
         (id, username, password_hash, pin_hash, role, is_active, created_at, updated_at)
         VALUES ('11111111-1111-1111-1111-111111111111', 'caissier1', ?1, ?2,
                 'CAISSIER', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![password_hash, pin_hash],
    )
    .expect("seed user");
    let shared = Arc::new(Mutex::new(conn));
    let clock = Arc::new(SystemClock);
    Harness {
        _dir: dir,
        users: SqliteUserRepository::new(Arc::clone(&shared)),
        sessions: InMemorySessionStore::new(clock.clone()),
        clock,
        hasher,
    }
}

fn login<'a>(
    harness: &'a Harness,
) -> AuthenticateUserUseCase<
    'a,
    Argon2PasswordHasher,
    SqliteUserRepository,
    InMemorySessionStore,
    SystemClock,
> {
    AuthenticateUserUseCase::new(
        &harness.hasher,
        &harness.users,
        &harness.sessions,
        harness.clock.as_ref(),
    )
}

fn operator_id() -> UserId {
    UserId::from(Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("seed uuid"))
}

fn attempts(harness: &Harness) -> i64 {
    // Read-only probe of the lockout counter (test introspection only).
    harness
        .users
        .find_by_username("caissier1")
        .expect("lookup")
        .expect("user")
        .failed_login_attempts as i64
}

#[test]
fn argon2_round_trip_and_wrong_password_is_false() {
    let hasher = Argon2PasswordHasher;
    let hash = hasher.hash("secret").expect("hash");
    assert!(hash.starts_with("$argon2id$"), "PHC format: {hash}");
    assert!(hasher.verify("secret", &hash).expect("verify"));
    assert!(!hasher.verify("wrong", &hash).expect("wrong is false"));
}

#[test]
fn login_success_returns_matching_session() {
    let harness = setup_with_pin(Some("1234"));
    let session = login(&harness)
        .execute("caissier1", "secret")
        .expect("login");
    assert_eq!(session.username, "caissier1");
    assert_eq!(session.role, Role::Caissier);
    assert_eq!(session.user_id, operator_id());
    let context = harness
        .sessions
        .validate_session(&session.token)
        .expect("live");
    assert_eq!(context.user_id, operator_id());
    assert_eq!(context.role, Role::Caissier);
    assert_eq!(attempts(&harness), 0);
}

#[test]
fn five_failures_lock_and_sixth_correct_is_rejected() {
    let harness = setup_with_pin(None);
    for _ in 0..4 {
        let failure = login(&harness)
            .execute("caissier1", "wrong")
            .expect_err("fail");
        assert!(matches!(
            failure,
            ApplicationError::Validation(DomainError::InvalidCredentials)
        ));
    }
    let fifth = login(&harness)
        .execute("caissier1", "wrong")
        .expect_err("fifth");
    assert!(
        matches!(
            fifth,
            ApplicationError::Validation(DomainError::AccountLocked { .. })
        ),
        "fifth must lock: {fifth:?}"
    );
    let sixth = login(&harness)
        .execute("caissier1", "secret")
        .expect_err("sixth");
    assert!(
        matches!(
            sixth,
            ApplicationError::Validation(DomainError::AccountLocked { .. })
        ),
        "correct password while locked must stay locked: {sixth:?}"
    );
    assert_eq!(attempts(&harness), 5);
}

#[test]
fn pin_paths_cover_set_unset_and_wrong() {
    let harness = setup_with_pin(Some("1234"));
    let pins = VerifyPinUseCase::new(&harness.hasher, &harness.users);
    pins.execute(operator_id(), "1234").expect("correct pin");
    let wrong = pins.execute(operator_id(), "0000").expect_err("wrong pin");
    assert!(matches!(
        wrong,
        ApplicationError::Validation(DomainError::InvalidCredentials)
    ));

    let bare = setup_with_pin(None);
    let missing = VerifyPinUseCase::new(&bare.hasher, &bare.users)
        .execute(operator_id(), "1234")
        .expect_err("no pin");
    assert!(matches!(
        missing,
        ApplicationError::Validation(DomainError::PinNotSet)
    ));
}

#[test]
fn logout_is_idempotent_and_revokes() {
    let harness = setup_with_pin(None);
    let session = login(&harness)
        .execute("caissier1", "secret")
        .expect("login");
    let logout = LogoutUseCase::new(&harness.sessions);
    logout.execute(&session.token).expect("first logout");
    assert!(harness.sessions.validate_session(&session.token).is_none());
    logout
        .execute(&session.token)
        .expect("second logout is a no-op");
}

#[test]
fn expired_session_validates_to_none() {
    let harness = setup_with_pin(None);
    let token = harness
        .sessions
        .create_session(operator_id(), Role::Caissier, -1);
    assert!(harness.sessions.validate_session(&token).is_none());
}

#[test]
fn inactive_account_is_rejected() {
    let harness = setup_with_pin(None);
    // Direct state flip (no user-CRUD port exists in this task): reopen the
    // same file with a second handle while the repository is idle.
    let db_path = harness._dir.path().join("auth.db");
    let conn = Connection::open(db_path).expect("reopen");
    conn.execute(
        "UPDATE users SET is_active = 0 WHERE username = 'caissier1'",
        [],
    )
    .expect("deactivate");
    let rejected = login(&harness)
        .execute("caissier1", "secret")
        .expect_err("inactive");
    assert!(matches!(
        rejected,
        ApplicationError::Validation(DomainError::AccountInactive)
    ));
}
