//! Managed operator administration (RBAC-gated, last-admin protected).
//!
//! All four use cases require `Permission::UserManage` (Admin only).
//! Wire codes surfacing through `error_code` are stable UPPER_SNAKE
//! (`INVALID_USERNAME`, `USERNAME_TAKEN`, `USER_NOT_FOUND`,
//! `LAST_ADMIN_PROTECTED`, `NOTHING_TO_RESET`).

use baraka_domain::errors::DomainError;
use baraka_domain::role::{Permission, Role};
use baraka_domain::user::UserId;

use crate::errors::ApplicationError;
use crate::ports::{
    Clock, CreateUserInput, PasswordHasher, SessionContext, UpdateUserInput, UserRecord,
    UserRepository,
};

/// Stable wire code for a rejected username shape.
const INVALID_USERNAME: &str = "INVALID_USERNAME";
/// Stable wire code for a rejected password shape.
const WEAK_PASSWORD: &str = "WEAK_PASSWORD";
/// Stable wire code for a rejected PIN shape.
const INVALID_PIN: &str = "INVALID_PIN";
/// Stable wire code for a duplicate username.
const USERNAME_TAKEN: &str = "USERNAME_TAKEN";
/// Stable wire code for last-admin demotion/deactivation attempts.
const LAST_ADMIN_PROTECTED: &str = "LAST_ADMIN_PROTECTED";
/// Stable wire code for a credential reset carrying nothing to change.
const NOTHING_TO_RESET: &str = "NOTHING_TO_RESET";

fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

fn require_user_manage(session: &SessionContext) -> Result<(), ApplicationError> {
    if !session.role.has_permission(Permission::UserManage) {
        return Err(ApplicationError::Unauthorized);
    }
    Ok(())
}

/// Validate a login name: trimmed, 3..=30 chars, alphanumeric + underscores.
fn validate_username(raw: &str) -> Result<String, ApplicationError> {
    let trimmed = raw.trim();
    let shaped = trimmed.len() >= 3
        && trimmed.len() <= 30
        && trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    if shaped {
        Ok(trimmed.to_string())
    } else {
        Err(coded(INVALID_USERNAME))
    }
}

/// Validate a password: at least 6 characters.
fn validate_password(raw: &str) -> Result<(), ApplicationError> {
    if raw.chars().count() >= 6 {
        Ok(())
    } else {
        Err(coded(WEAK_PASSWORD))
    }
}

/// Validate an optional PIN: 4..=6 ASCII digits.
fn validate_pin(raw: &str) -> Result<(), ApplicationError> {
    let shaped = (4..=6).contains(&raw.len()) && raw.bytes().all(|byte| byte.is_ascii_digit());
    if shaped {
        Ok(())
    } else {
        Err(coded(INVALID_PIN))
    }
}

/// List every operator (RBAC-gated read).
pub struct ListUsersUseCase<'a, R>
where
    R: ?Sized,
{
    users: &'a R,
}

impl<'a, R> ListUsersUseCase<'a, R>
where
    R: UserRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(users: &'a R) -> Self {
        Self { users }
    }

    /// Return every managed operator ordered by username.
    pub fn execute(&self, session: &SessionContext) -> Result<Vec<UserRecord>, ApplicationError> {
        require_user_manage(session)?;
        self.users.list_all_users()
    }
}

/// Create a validated, hashed operator (RBAC-gated write).
pub struct CreateUserUseCase<'a, H, R, C>
where
    H: ?Sized,
    R: ?Sized,
    C: ?Sized,
{
    hasher: &'a H,
    users: &'a R,
    clock: &'a C,
}

impl<'a, H, R, C> CreateUserUseCase<'a, H, R, C>
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

    /// Validate, hash, and insert a new operator.
    pub fn execute(
        &self,
        session: &SessionContext,
        username: &str,
        password: &str,
        pin: Option<&str>,
        role: Role,
    ) -> Result<UserRecord, ApplicationError> {
        require_user_manage(session)?;
        let validated_name = validate_username(username)?;
        validate_password(password)?;
        if let Some(pin_text) = pin {
            validate_pin(pin_text)?;
        }
        if self.users.find_by_username(&validated_name)?.is_some() {
            return Err(ApplicationError::Conflict(USERNAME_TAKEN.to_string()));
        }
        let password_hash = self.hasher.hash(password)?;
        let pin_hash = pin.map(|pin_text| self.hasher.hash(pin_text)).transpose()?;
        self.users.create_user(&CreateUserInput {
            user_id: UserId::new(),
            username: validated_name,
            password_hash,
            pin_hash,
            role,
            created_at: self.clock.now_rfc3339(),
        })
    }
}

/// Patch an operator's role and/or active flag (last-admin protected).
pub struct UpdateUserUseCase<'a, R, C>
where
    R: ?Sized,
    C: ?Sized,
{
    users: &'a R,
    clock: &'a C,
}

impl<'a, R, C> UpdateUserUseCase<'a, R, C>
where
    R: UserRepository + ?Sized,
    C: Clock + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(users: &'a R, clock: &'a C) -> Self {
        Self { users, clock }
    }

    /// Apply the patch; rejects demotion/deactivation of the last active Admin.
    pub fn execute(
        &self,
        session: &SessionContext,
        user_id: UserId,
        role: Option<Role>,
        is_active: Option<bool>,
    ) -> Result<UserRecord, ApplicationError> {
        require_user_manage(session)?;
        if role.is_none() && is_active.is_none() {
            return Err(coded(NOTHING_TO_RESET));
        }
        let current = self
            .users
            .find_user_by_id(user_id)?
            .ok_or_else(|| ApplicationError::NotFound("user".to_string()))?;
        let effective_active = is_active.unwrap_or(current.is_active);
        let demotes_admin = current.role == Role::Admin
            && current.is_active
            && effective_active
            && role.is_some_and(|next| next != Role::Admin);
        let deactivates_admin =
            current.role == Role::Admin && current.is_active && is_active == Some(false);
        if (demotes_admin || deactivates_admin) && self.users.count_active_admins()? <= 1 {
            return Err(ApplicationError::Conflict(LAST_ADMIN_PROTECTED.to_string()));
        }
        self.users.update_user(&UpdateUserInput {
            user_id,
            role,
            is_active,
            updated_at: self.clock.now_rfc3339(),
        })
    }
}

/// Replace an operator's password and/or PIN hashes (RBAC-gated write).
pub struct ResetUserCredentialsUseCase<'a, H, R, C>
where
    H: ?Sized,
    R: ?Sized,
    C: ?Sized,
{
    hasher: &'a H,
    users: &'a R,
    clock: &'a C,
}

impl<'a, H, R, C> ResetUserCredentialsUseCase<'a, H, R, C>
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

    /// Validate, hash, and store the replacement credentials.
    pub fn execute(
        &self,
        session: &SessionContext,
        user_id: UserId,
        password: Option<&str>,
        pin: Option<&str>,
    ) -> Result<(), ApplicationError> {
        require_user_manage(session)?;
        if password.is_none() && pin.is_none() {
            return Err(coded(NOTHING_TO_RESET));
        }
        if let Some(password_text) = password {
            validate_password(password_text)?;
        }
        if let Some(pin_text) = pin {
            validate_pin(pin_text)?;
        }
        if self.users.find_user_by_id(user_id)?.is_none() {
            return Err(ApplicationError::NotFound("user".to_string()));
        }
        let password_hash = password
            .map(|password_text| self.hasher.hash(password_text))
            .transpose()?;
        let pin_hash = pin.map(|pin_text| self.hasher.hash(pin_text)).transpose()?;
        self.users.update_credentials(
            user_id,
            password_hash.as_deref(),
            pin_hash.as_deref(),
            &self.clock.now_rfc3339(),
        )
    }
}
