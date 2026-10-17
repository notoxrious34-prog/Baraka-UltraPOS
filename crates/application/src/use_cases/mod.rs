//! Use cases: deterministic orchestration over domain primitives.

pub mod backup;
pub mod cash;
pub mod catalog;
pub mod customers;
pub mod deletion;
pub mod inventory;
pub mod printing;
pub mod purchases;
pub mod reports;
pub mod returns;
pub mod sales;
pub mod suppliers;
pub mod users;

use baraka_domain::errors::DomainError;
use baraka_domain::models::{HealthSnapshot, PaymentMode, SaleLine};
use baraka_domain::primitives::Money;
use baraka_domain::role::Role;
use baraka_domain::user::{UserId, Username};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::ApplicationError;
use crate::ports::{
    BootstrapFirstAdminCommand, Clock, PasswordHasher, SessionStore, SessionToken, UserRepository,
};

/// Application version baked into the health snapshot.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Fixed session lifetime: 12 hours, never sliding or refreshable.
pub const SESSION_TTL_SECONDS: i64 = 43_200;

/// Failed password logins tolerated before a lockout is recorded.
pub const MAX_LOGIN_ATTEMPTS: u8 = 5;

/// Lockout duration after the threshold is reached: 5 minutes.
pub const LOCKOUT_SECONDS: i64 = 300;

/// Input for computing a sale total (centimes arithmetic only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeSaleTotalInput {
    /// Validated sale lines.
    pub lines: Vec<SaleLine>,
    /// Global discount in centimes (>= 0, <= subtotal).
    pub global_discount_centimes: i64,
    /// Declared payment mode (informational at this stage).
    pub payment_mode: PaymentMode,
}

/// Deterministic sale total: `max(subtotal - discount, 0)`.
pub fn compute_sale_total(input: &ComputeSaleTotalInput) -> Result<Money, ApplicationError> {
    if input.global_discount_centimes < 0 {
        return Err(ApplicationError::Validation(
            DomainError::NegativeMoneyForbidden,
        ));
    }
    let mut subtotal = Money::zero();
    for line in &input.lines {
        let line_total = line.line_total()?;
        subtotal = subtotal.checked_add(line_total)?;
    }
    let total = subtotal
        .centimes()
        .saturating_sub(input.global_discount_centimes);
    if total < 0 {
        return Err(ApplicationError::Validation(
            DomainError::NegativeMoneyForbidden,
        ));
    }
    Ok(Money::from_centimes(total))
}

/// Build the baseline health snapshot served to the cockpit.
#[must_use]
pub fn health_snapshot() -> HealthSnapshot {
    HealthSnapshot {
        captured_at: Utc::now(),
        version: APP_VERSION.to_string(),
        offline_ready: true,
    }
}

/// Successful password authentication (token + attribution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    /// Opaque volatile token for subsequent calls.
    pub token: SessionToken,
    /// Authenticated operator.
    pub user_id: UserId,
    /// Login name that authenticated.
    pub username: String,
    /// Operator role at login time.
    pub role: Role,
    /// Fixed expiry instant (`now + SESSION_TTL_SECONDS`).
    pub expires_at: DateTime<Utc>,
}

/// Username + password login with lockout enforcement.
///
/// Algorithm: unknown-user and wrong-password both yield
/// `InvalidCredentials`; inactive accounts yield `AccountInactive`; attempts
/// inside a live lock yield `AccountLocked` without touching the password
/// verifier; the attempt that reaches the failure threshold yields
/// `AccountLocked`; a success resets the failure counter.
pub struct AuthenticateUserUseCase<'a, H, R, S, C>
where
    H: ?Sized,
    R: ?Sized,
    S: ?Sized,
    C: ?Sized,
{
    hasher: &'a H,
    users: &'a R,
    sessions: &'a S,
    clock: &'a C,
}

impl<'a, H, R, S, C> AuthenticateUserUseCase<'a, H, R, S, C>
where
    H: PasswordHasher + ?Sized,
    R: UserRepository + ?Sized,
    S: SessionStore + ?Sized,
    C: Clock + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(hasher: &'a H, users: &'a R, sessions: &'a S, clock: &'a C) -> Self {
        Self {
            hasher,
            users,
            sessions,
            clock,
        }
    }

    /// Authenticate `username` with `password`.
    pub fn execute(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthenticatedSession, ApplicationError> {
        let trimmed = username.trim();
        if trimmed.is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCredentials,
            ));
        }
        let user = match self.users.find_by_username(trimmed)? {
            Some(user) => user,
            None => {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidCredentials,
                ));
            }
        };
        if !user.is_active {
            return Err(ApplicationError::Validation(DomainError::AccountInactive));
        }
        let now = self.clock.now();
        if let Some(locked_until) = user.locked_until {
            if now < locked_until {
                let remaining = (locked_until - now).num_seconds();
                return Err(ApplicationError::Validation(DomainError::AccountLocked {
                    remaining_seconds: remaining,
                }));
            }
        }
        let stored = self.users.get_password_hash(user.id)?;
        if !self.hasher.verify(password, &stored)? {
            let attempts = user.failed_login_attempts.saturating_add(1);
            if attempts >= MAX_LOGIN_ATTEMPTS {
                let locked_until = now + TimeDelta::seconds(LOCKOUT_SECONDS);
                self.users
                    .record_login_failure(user.id, Some(locked_until))?;
                return Err(ApplicationError::Validation(DomainError::AccountLocked {
                    remaining_seconds: LOCKOUT_SECONDS,
                }));
            }
            self.users.record_login_failure(user.id, None)?;
            return Err(ApplicationError::Validation(
                DomainError::InvalidCredentials,
            ));
        }
        self.users.reset_login_failures(user.id)?;
        let token = self
            .sessions
            .create_session(user.id, user.role, SESSION_TTL_SECONDS);
        Ok(AuthenticatedSession {
            token,
            user_id: user.id,
            username: user.username.as_str().to_string(),
            role: user.role,
            expires_at: now + TimeDelta::seconds(SESSION_TTL_SECONDS),
        })
    }
}

/// Fast user-switch PIN check (no lockout interaction in this task).
pub struct VerifyPinUseCase<'a, H, R>
where
    H: ?Sized,
    R: ?Sized,
{
    hasher: &'a H,
    users: &'a R,
}

impl<'a, H, R> VerifyPinUseCase<'a, H, R>
where
    H: PasswordHasher + ?Sized,
    R: UserRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(hasher: &'a H, users: &'a R) -> Self {
        Self { hasher, users }
    }

    /// Verify `pin` for `user_id`; wrong PINs yield `InvalidCredentials`.
    pub fn execute(&self, user_id: UserId, pin: &str) -> Result<(), ApplicationError> {
        let stored = match self.users.get_pin_hash(user_id)? {
            Some(hash) => hash,
            None => return Err(ApplicationError::Validation(DomainError::PinNotSet)),
        };
        if self.hasher.verify(pin, &stored)? {
            Ok(())
        } else {
            Err(ApplicationError::Validation(
                DomainError::InvalidCredentials,
            ))
        }
    }
}

/// Session revocation (idempotent by contract).
pub struct LogoutUseCase<'a, S>
where
    S: ?Sized,
{
    sessions: &'a S,
}

impl<'a, S> LogoutUseCase<'a, S>
where
    S: SessionStore + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(sessions: &'a S) -> Self {
        Self { sessions }
    }

    /// Revoke `token`; unknown tokens are a silent no-op success.
    pub fn execute(&self, token: &SessionToken) -> Result<(), ApplicationError> {
        self.sessions.invalidate_session(token);
        Ok(())
    }
}

/// First-run administrator bootstrap input (raw operator typing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapFirstAdminInput {
    /// Requested login name (trimmed and validated by the use case).
    pub username: String,
    /// Requested password (presence-checked; hashed by the shared port).
    pub password: String,
}

/// First-run administrator bootstrap outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapFirstAdminOutput {
    /// Fresh operator id of the created Admin.
    pub user_id: UserId,
    /// Stored login name (trimmed).
    pub username: String,
}

/// First-run administrator bootstrap (ADR-048).
///
/// Creates exactly one Admin operator when the `users` table is still
/// empty. Username validation reuses `Username::new` (the TASK-BRK-003
/// rule: trimmed, never empty); password hashing reuses the shared
/// [`PasswordHasher`] port (Argon2id in production) — zero new hashing
/// code is introduced here. Atomicity (`BEGIN IMMEDIATE` plus an
/// in-transaction emptiness re-check, so two racing first-run attempts
/// yield exactly one success and one `BootstrapAlreadyCompleted`) lives
/// in the repository implementation, the only layer allowed to own SQL.
/// `terminal_id` is TerminalContext-resolved at the IPC edge (ADR-045).
pub struct BootstrapFirstAdminUseCase<'a, H, R, C>
where
    H: ?Sized,
    R: ?Sized,
    C: ?Sized,
{
    hasher: &'a H,
    users: &'a R,
    clock: &'a C,
}

impl<'a, H, R, C> BootstrapFirstAdminUseCase<'a, H, R, C>
where
    H: PasswordHasher + ?Sized,
    R: UserRepository + ?Sized,
    C: Clock + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(hasher: &'a H, users: &'a R, clock: &'a C) -> Self {
        Self {
            hasher,
            users,
            clock,
        }
    }

    /// Validate, hash, and delegate the atomic first-admin creation.
    pub fn execute(
        &self,
        input: &BootstrapFirstAdminInput,
        terminal_id: &str,
    ) -> Result<BootstrapFirstAdminOutput, ApplicationError> {
        let username = Username::new(&input.username)?;
        if input.password.is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("BOOTSTRAP_PASSWORD_REQUIRED".to_string()),
            ));
        }
        let password_hash = self.hasher.hash(&input.password)?;
        let user_id = UserId::new();
        let stored_name = username.as_str().to_string();
        self.users
            .bootstrap_first_admin(&BootstrapFirstAdminCommand {
                user_id,
                username: stored_name.clone(),
                password_hash,
                created_at: self.clock.now_rfc3339(),
                terminal_id: terminal_id.to_string(),
            })?;
        Ok(BootstrapFirstAdminOutput {
            user_id,
            username: stored_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::SessionContext;
    use baraka_domain::role::Role;
    use baraka_domain::user::{User, Username};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct FakeClock {
        now: Mutex<DateTime<Utc>>,
    }

    impl FakeClock {
        fn new(now: DateTime<Utc>) -> Self {
            Self {
                now: Mutex::new(now),
            }
        }

        fn advance(&self, seconds: i64) {
            let next = *self.now.lock().expect("clock") + TimeDelta::seconds(seconds);
            *self.now.lock().expect("clock") = next;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            *self.now.lock().expect("clock")
        }

        fn now_rfc3339(&self) -> String {
            self.now()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        }
    }

    struct FakeHasher;

    impl PasswordHasher for FakeHasher {
        fn hash(&self, plaintext: &str) -> Result<String, ApplicationError> {
            Ok(format!("hash:{plaintext}"))
        }

        fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, ApplicationError> {
            Ok(hash == format!("hash:{plaintext}"))
        }
    }

    /// Counting wrapper proving whether the verifier ran at all.
    struct CountingHasher {
        calls: Mutex<u32>,
    }

    impl CountingHasher {
        fn new() -> Self {
            Self {
                calls: Mutex::new(0),
            }
        }

        fn calls(&self) -> u32 {
            *self.calls.lock().expect("calls")
        }
    }

    impl PasswordHasher for CountingHasher {
        fn hash(&self, plaintext: &str) -> Result<String, ApplicationError> {
            Ok(format!("hash:{plaintext}"))
        }

        fn verify(&self, plaintext: &str, hash: &str) -> Result<bool, ApplicationError> {
            *self.calls.lock().expect("calls") += 1;
            Ok(hash == format!("hash:{plaintext}"))
        }
    }

    struct UserRecord {
        user: User,
        password_hash: String,
        pin_hash: Option<String>,
    }

    struct FakeUsers {
        by_name: Mutex<HashMap<String, UserRecord>>,
    }

    impl FakeUsers {
        fn with_operator(username: &str, role: Role, pin: Option<&str>) -> Self {
            let id = UserId::new();
            let record = UserRecord {
                user: User {
                    id,
                    username: Username::new(username).expect("valid"),
                    role,
                    is_active: true,
                    failed_login_attempts: 0,
                    locked_until: None,
                },
                password_hash: "hash:secret".to_string(),
                pin_hash: pin.map(|p| format!("hash:{p}")),
            };
            let mut by_name = HashMap::new();
            by_name.insert(username.to_string(), record);
            Self {
                by_name: Mutex::new(by_name),
            }
        }

        fn stored(&self, username: &str) -> User {
            self.by_name
                .lock()
                .expect("users")
                .get(username)
                .unwrap_or_else(|| panic!("missing {username}"))
                .user
                .clone()
        }
    }

    impl UserRepository for FakeUsers {
        fn find_by_username(&self, username: &str) -> Result<Option<User>, ApplicationError> {
            Ok(self
                .by_name
                .lock()
                .expect("users")
                .get(username)
                .map(|r| r.user.clone()))
        }

        fn record_login_failure(
            &self,
            user_id: UserId,
            locked_until: Option<DateTime<Utc>>,
        ) -> Result<(), ApplicationError> {
            for record in self.by_name.lock().expect("users").values_mut() {
                if record.user.id == user_id {
                    record.user.failed_login_attempts =
                        record.user.failed_login_attempts.saturating_add(1);
                    record.user.locked_until = locked_until;
                }
            }
            Ok(())
        }

        fn reset_login_failures(&self, user_id: UserId) -> Result<(), ApplicationError> {
            for record in self.by_name.lock().expect("users").values_mut() {
                if record.user.id == user_id {
                    record.user.failed_login_attempts = 0;
                    record.user.locked_until = None;
                }
            }
            Ok(())
        }

        fn get_password_hash(&self, user_id: UserId) -> Result<String, ApplicationError> {
            for record in self.by_name.lock().expect("users").values() {
                if record.user.id == user_id {
                    return Ok(record.password_hash.clone());
                }
            }
            Err(ApplicationError::NotFound("user".to_string()))
        }

        fn get_pin_hash(&self, user_id: UserId) -> Result<Option<String>, ApplicationError> {
            for record in self.by_name.lock().expect("users").values() {
                if record.user.id == user_id {
                    return Ok(record.pin_hash.clone());
                }
            }
            Err(ApplicationError::NotFound("user".to_string()))
        }

        fn count_users(&self) -> Result<u64, ApplicationError> {
            Ok(self.by_name.lock().expect("users").len() as u64)
        }

        fn bootstrap_first_admin(
            &self,
            command: &BootstrapFirstAdminCommand,
        ) -> Result<UserId, ApplicationError> {
            let mut guard = self.by_name.lock().expect("users");
            if !guard.is_empty() {
                return Err(ApplicationError::Validation(
                    DomainError::BootstrapAlreadyCompleted,
                ));
            }
            let record = UserRecord {
                user: User {
                    id: command.user_id,
                    username: Username::new(&command.username).expect("valid"),
                    role: Role::Admin,
                    is_active: true,
                    failed_login_attempts: 0,
                    locked_until: None,
                },
                password_hash: command.password_hash.clone(),
                pin_hash: None,
            };
            guard.insert(command.username.clone(), record);
            Ok(command.user_id)
        }

        fn list_all_users(&self) -> Result<Vec<crate::ports::UserRecord>, ApplicationError> {
            let guard = self.by_name.lock().expect("users");
            let mut names: Vec<&String> = guard.keys().collect();
            names.sort();
            Ok(names
                .into_iter()
                .map(|name| {
                    let record = &guard[name];
                    crate::ports::UserRecord {
                        id: record.user.id,
                        username: record.user.username.as_str().to_string(),
                        role: record.user.role,
                        is_active: record.user.is_active,
                        has_pin: record.pin_hash.is_some(),
                        created_at: "2026-01-01T00:00:00Z".to_string(),
                        updated_at: "2026-01-01T00:00:00Z".to_string(),
                    }
                })
                .collect())
        }

        fn find_user_by_id(
            &self,
            id: UserId,
        ) -> Result<Option<crate::ports::UserRecord>, ApplicationError> {
            let guard = self.by_name.lock().expect("users");
            Ok(guard
                .values()
                .find(|record| record.user.id == id)
                .map(|record| crate::ports::UserRecord {
                    id: record.user.id,
                    username: record.user.username.as_str().to_string(),
                    role: record.user.role,
                    is_active: record.user.is_active,
                    has_pin: record.pin_hash.is_some(),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                }))
        }

        fn create_user(
            &self,
            input: &crate::ports::CreateUserInput,
        ) -> Result<crate::ports::UserRecord, ApplicationError> {
            let mut guard = self.by_name.lock().expect("users");
            if guard.contains_key(&input.username) {
                return Err(ApplicationError::Conflict("USERNAME_TAKEN".to_string()));
            }
            guard.insert(
                input.username.clone(),
                UserRecord {
                    user: User {
                        id: input.user_id,
                        username: Username::new(&input.username).expect("valid"),
                        role: input.role,
                        is_active: true,
                        failed_login_attempts: 0,
                        locked_until: None,
                    },
                    password_hash: input.password_hash.clone(),
                    pin_hash: input.pin_hash.clone(),
                },
            );
            drop(guard);
            self.find_user_by_id(input.user_id)?
                .ok_or_else(|| ApplicationError::Port("created user unreadable".to_string()))
        }

        fn update_user(
            &self,
            input: &crate::ports::UpdateUserInput,
        ) -> Result<crate::ports::UserRecord, ApplicationError> {
            {
                let mut guard = self.by_name.lock().expect("users");
                let record = guard
                    .values_mut()
                    .find(|record| record.user.id == input.user_id)
                    .ok_or_else(|| ApplicationError::NotFound("user".to_string()))?;
                if let Some(role) = input.role {
                    record.user.role = role;
                }
                if let Some(is_active) = input.is_active {
                    record.user.is_active = is_active;
                }
            }
            self.find_user_by_id(input.user_id)?
                .ok_or_else(|| ApplicationError::Port("updated user unreadable".to_string()))
        }

        fn update_credentials(
            &self,
            user_id: UserId,
            password_hash: Option<&str>,
            pin_hash: Option<&str>,
            _updated_at: &str,
        ) -> Result<(), ApplicationError> {
            let mut guard = self.by_name.lock().expect("users");
            let record = guard
                .values_mut()
                .find(|record| record.user.id == user_id)
                .ok_or_else(|| ApplicationError::NotFound("user".to_string()))?;
            if let Some(hash) = password_hash {
                record.password_hash = hash.to_string();
            }
            if let Some(hash) = pin_hash {
                record.pin_hash = Some(hash.to_string());
            }
            Ok(())
        }

        fn count_active_admins(&self) -> Result<u64, ApplicationError> {
            Ok(self
                .by_name
                .lock()
                .expect("users")
                .values()
                .filter(|record| record.user.role == Role::Admin && record.user.is_active)
                .count() as u64)
        }
    }

    struct FakeSessions {
        clock: FakeClock,
        live: Mutex<HashMap<String, SessionContext>>,
    }

    impl FakeSessions {
        fn new(now: DateTime<Utc>) -> Self {
            Self {
                clock: FakeClock::new(now),
                live: Mutex::new(HashMap::new()),
            }
        }
    }

    impl SessionStore for FakeSessions {
        fn create_session(&self, user_id: UserId, role: Role, ttl_seconds: i64) -> SessionToken {
            let token = SessionToken::new_random();
            let context = SessionContext {
                token: token.clone(),
                user_id,
                role,
                expires_at: self.clock.now() + TimeDelta::seconds(ttl_seconds),
            };
            self.live
                .lock()
                .expect("sessions")
                .insert(token.0.clone(), context);
            token
        }

        fn validate_session(&self, token: &SessionToken) -> Option<SessionContext> {
            let mut live = self.live.lock().expect("sessions");
            match live.get(&token.0) {
                Some(context) if context.expires_at > self.clock.now() => Some(context.clone()),
                _ => {
                    live.remove(&token.0);
                    None
                }
            }
        }

        fn invalidate_session(&self, token: &SessionToken) {
            self.live.lock().expect("sessions").remove(&token.0);
        }
    }

    fn harness() -> (FakeHasher, FakeUsers, FakeSessions, FakeClock) {
        let now = Utc::now();
        (
            FakeHasher,
            FakeUsers::with_operator("caissier1", Role::Caissier, Some("1234")),
            FakeSessions::new(now),
            FakeClock::new(now),
        )
    }

    #[test]
    fn totals_subtract_discount() {
        use baraka_domain::primitives::{Money, QtyMicro};
        let input = ComputeSaleTotalInput {
            lines: vec![SaleLine::new(
                Uuid::nil(),
                QtyMicro::from_micros(20_000),
                Money::from_centimes(1000),
            )],
            global_discount_centimes: 500,
            payment_mode: PaymentMode::Especes,
        };
        let total = compute_sale_total(&input).expect("total");
        assert_eq!(total.centimes(), 1500);
    }

    #[test]
    fn login_success_returns_valid_session() {
        let (hasher, users, sessions, clock) = harness();
        let login = AuthenticateUserUseCase::new(&hasher, &users, &sessions, &clock);
        let session = login.execute("caissier1", "secret").expect("login");
        assert_eq!(session.username, "caissier1");
        assert_eq!(session.role, Role::Caissier);
        let context = sessions
            .validate_session(&session.token)
            .expect("session live");
        assert_eq!(context.user_id, session.user_id);
        assert_eq!(context.role, Role::Caissier);
        assert_eq!(users.stored("caissier1").failed_login_attempts, 0);
    }

    #[test]
    fn unknown_user_and_wrong_password_are_identical() {
        let (hasher, users, sessions, clock) = harness();
        let login = AuthenticateUserUseCase::new(&hasher, &users, &sessions, &clock);
        let ghost = login.execute("nobody", "secret").expect_err("ghost");
        let wrong = login.execute("caissier1", "wrong").expect_err("wrong");
        assert!(matches!(
            ghost,
            ApplicationError::Validation(DomainError::InvalidCredentials)
        ));
        assert!(matches!(
            wrong,
            ApplicationError::Validation(DomainError::InvalidCredentials)
        ));
        assert_eq!(
            format!("{ghost}"),
            format!("{wrong}"),
            "responses must be indistinguishable"
        );
    }

    #[test]
    fn five_failures_lock_and_sixth_skips_verifier() {
        let (hasher, users, sessions, clock) = harness();
        let counting = CountingHasher::new();
        let login = AuthenticateUserUseCase::new(&counting, &users, &sessions, &clock);
        for _ in 0..4 {
            let failure = login.execute("caissier1", "wrong").expect_err("fail");
            assert!(matches!(
                failure,
                ApplicationError::Validation(DomainError::InvalidCredentials)
            ));
        }
        let fifth = login.execute("caissier1", "wrong").expect_err("fifth");
        assert!(
            matches!(
                fifth,
                ApplicationError::Validation(DomainError::AccountLocked { .. })
            ),
            "fifth failure locks: {fifth:?}"
        );
        // Correct password while locked: rejected without verifier contact.
        let before = counting.calls();
        let sixth = login.execute("caissier1", "secret").expect_err("sixth");
        assert!(matches!(
            sixth,
            ApplicationError::Validation(DomainError::AccountLocked { .. })
        ));
        assert_eq!(
            counting.calls(),
            before,
            "verifier must not run while locked"
        );
        let _ = hasher;
    }

    #[test]
    fn lock_expiry_then_success_resets_counter() {
        let (hasher, users, sessions, clock) = harness();
        let login = AuthenticateUserUseCase::new(&hasher, &users, &sessions, &clock);
        for _ in 0..5 {
            let _ = login.execute("caissier1", "wrong");
        }
        assert_eq!(users.stored("caissier1").failed_login_attempts, 5);
        clock.advance(LOCKOUT_SECONDS + 1);
        let session = login
            .execute("caissier1", "secret")
            .expect("post-lock login");
        assert_eq!(session.role, Role::Caissier);
        assert_eq!(users.stored("caissier1").failed_login_attempts, 0);
    }

    #[test]
    fn pin_verification_paths() {
        let (hasher, users, sessions, clock) = harness();
        let pins = VerifyPinUseCase::new(&hasher, &users);
        let id = users.stored("caissier1").id;
        pins.execute(id, "1234").expect("correct pin");
        let wrong = pins.execute(id, "0000").expect_err("wrong pin");
        assert!(matches!(
            wrong,
            ApplicationError::Validation(DomainError::InvalidCredentials)
        ));
        let pinless = FakeUsers::with_operator("vendeur1", Role::Vendeur, None);
        let other_id = pinless.stored("vendeur1").id;
        let missing = VerifyPinUseCase::new(&hasher, &pinless)
            .execute(other_id, "1234")
            .expect_err("no pin");
        assert!(matches!(
            missing,
            ApplicationError::Validation(DomainError::PinNotSet)
        ));
        let _ = (sessions, clock);
    }

    #[test]
    fn logout_is_idempotent_and_session_expiry_enforced() {
        let (hasher, users, sessions, clock) = harness();
        let login = AuthenticateUserUseCase::new(&hasher, &users, &sessions, &clock);
        let logout = LogoutUseCase::new(&sessions);
        let session = login.execute("caissier1", "secret").expect("login");
        logout.execute(&session.token).expect("first logout");
        assert!(sessions.validate_session(&session.token).is_none());
        logout
            .execute(&session.token)
            .expect("second logout is a no-op");

        let id = users.stored("caissier1").id;
        let short = sessions.create_session(id, Role::Caissier, 1);
        sessions.clock.advance(2);
        assert!(sessions.validate_session(&short).is_none());
        let _ = clock;
    }

    fn empty_users() -> (FakeHasher, FakeUsers, FakeClock) {
        let now = Utc::now();
        (
            FakeHasher,
            FakeUsers {
                by_name: Mutex::new(HashMap::new()),
            },
            FakeClock::new(now),
        )
    }

    #[test]
    fn bootstrap_creates_single_admin_and_login_succeeds() {
        let (hasher, users, clock) = empty_users();
        let bootstrap = BootstrapFirstAdminUseCase::new(&hasher, &users, &clock);
        assert_eq!(users.count_users().expect("count"), 0);
        let output = bootstrap
            .execute(
                &BootstrapFirstAdminInput {
                    username: "  admin  ".to_string(),
                    password: "S3cret!".to_string(),
                },
                "TERM-1",
            )
            .expect("bootstrap");
        assert_eq!(output.username, "admin");
        assert_eq!(users.count_users().expect("count"), 1);
        let stored = users.stored("admin");
        assert_eq!(stored.id, output.user_id);
        assert_eq!(stored.role, Role::Admin);
        assert!(stored.is_active);
        // The stored hash round-trips through the same verifier login uses.
        let sessions = FakeSessions::new(clock.now());
        let login = AuthenticateUserUseCase::new(&hasher, &users, &sessions, &clock);
        let session = login.execute("admin", "S3cret!").expect("login");
        assert_eq!(session.role, Role::Admin);
    }

    #[test]
    fn bootstrap_second_attempt_is_rejected_without_side_effects() {
        let (hasher, users, clock) = empty_users();
        let bootstrap = BootstrapFirstAdminUseCase::new(&hasher, &users, &clock);
        bootstrap
            .execute(
                &BootstrapFirstAdminInput {
                    username: "admin".to_string(),
                    password: "S3cret!".to_string(),
                },
                "TERM-1",
            )
            .expect("first");
        let second = bootstrap
            .execute(
                &BootstrapFirstAdminInput {
                    username: "intruder".to_string(),
                    password: "Other!".to_string(),
                },
                "TERM-2",
            )
            .expect_err("second");
        assert!(matches!(
            second,
            ApplicationError::Validation(DomainError::BootstrapAlreadyCompleted)
        ));
        assert_eq!(users.count_users().expect("count"), 1);
        assert!(users
            .find_by_username("intruder")
            .expect("lookup")
            .is_none());
    }

    #[test]
    fn bootstrap_rejects_empty_username_and_empty_password() {
        let (hasher, users, clock) = empty_users();
        let bootstrap = BootstrapFirstAdminUseCase::new(&hasher, &users, &clock);
        let empty_name = bootstrap
            .execute(
                &BootstrapFirstAdminInput {
                    username: "   ".to_string(),
                    password: "S3cret!".to_string(),
                },
                "TERM-1",
            )
            .expect_err("empty name");
        assert!(matches!(
            empty_name,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
        ));
        let empty_secret = bootstrap
            .execute(
                &BootstrapFirstAdminInput {
                    username: "admin".to_string(),
                    password: String::new(),
                },
                "TERM-1",
            )
            .expect_err("empty password");
        assert!(matches!(
            empty_secret,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
        ));
        assert_eq!(users.count_users().expect("count"), 0);
    }
}
