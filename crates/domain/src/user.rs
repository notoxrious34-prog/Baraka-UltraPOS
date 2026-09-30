//! Authenticated user entity and identity newtypes.

use std::fmt::{Display, Formatter, Result as FmtResult};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;
use crate::role::Role;

/// Stable user identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Inner UUID value.
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Validated login name (trimmed, never empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Username(String);

impl Username {
    /// Build from raw input; rejects empty or whitespace-only names.
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "username must not be empty".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Authenticated operator (domain view of the `users` table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Stable identifier.
    pub id: UserId,
    /// Validated login name.
    pub username: Username,
    /// Operator role.
    pub role: Role,
    /// Deactivated accounts can never authenticate.
    pub is_active: bool,
    /// Consecutive failed password logins (resets on success).
    pub failed_login_attempts: u8,
    /// Lock expiry time, if a lockout is currently recorded.
    pub locked_until: Option<DateTime<Utc>>,
}

/// Lockout interpretation of the `(failed_login_attempts, locked_until)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockoutState {
    /// No failures recorded.
    Clear,
    /// Failures recorded, but no active lock.
    TrackingFailures(u8),
    /// Lock recorded until the given instant.
    Locked {
        /// Lock expiry instant.
        until: DateTime<Utc>,
    },
}

impl LockoutState {
    /// Derive the state from stored fields.
    #[must_use]
    pub fn from_parts(failed_login_attempts: u8, locked_until: Option<DateTime<Utc>>) -> Self {
        match locked_until {
            Some(until) => Self::Locked { until },
            None if failed_login_attempts == 0 => Self::Clear,
            None => Self::TrackingFailures(failed_login_attempts),
        }
    }

    /// True only while a recorded lock has not yet expired.
    #[must_use]
    pub fn is_locked(&self, now: DateTime<Utc>) -> bool {
        matches!(self, Self::Locked { until } if now < *until)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn user() -> User {
        User {
            id: UserId::new(),
            username: Username::new("  caissier1 ").expect("valid"),
            role: Role::Caissier,
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
        }
    }

    #[test]
    fn username_trims_and_rejects_empty() {
        assert_eq!(user().username.as_str(), "caissier1");
        assert!(Username::new("").is_err());
        assert!(Username::new("   ").is_err());
    }

    #[test]
    fn lockout_state_transitions() {
        let now = Utc::now();
        assert_eq!(LockoutState::from_parts(0, None), LockoutState::Clear);
        assert_eq!(
            LockoutState::from_parts(3, None),
            LockoutState::TrackingFailures(3)
        );
        let future = now + TimeDelta::seconds(60);
        let locked = LockoutState::from_parts(5, Some(future));
        assert!(locked.is_locked(now));
        assert!(!locked.is_locked(future + TimeDelta::seconds(1)));
        assert!(!LockoutState::Clear.is_locked(now));
    }
}
