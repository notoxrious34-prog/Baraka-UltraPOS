//! Authentication DTOs: login, logout, first-run bootstrap.
use serde::{Deserialize, Serialize};

/// Username + password login request.
///
/// Never `Debug`-printable with the secret: the manual `Debug` impl below
/// redacts the password field.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequestDto {
    /// Raw login name as typed by the operator.
    pub username: String,
    /// Plaintext password (memory only, never logged or persisted).
    pub password: String,
}

impl std::fmt::Debug for LoginRequestDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginRequestDto")
            .field("username", &self.username)
            .field("password", &"***")
            .finish()
    }
}

/// Successful authentication response (all plain strings per IPC contract).
///
/// The manual `Debug` impl redacts the session token.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginResponseDto {
    /// Opaque volatile session token.
    pub session_token: String,
    /// Authenticated user id (UUID hyphenated string).
    pub user_id: String,
    /// Login name that authenticated.
    pub username: String,
    /// Role code (`VENDEUR`, `CAISSIER`, `GERANT`, `ADMIN`).
    pub role: String,
    /// Effective permission set for the role (ADR-100): sorted
    /// SCREAMING_SNAKE_CASE codes from `Role::permissions()` (serde form).
    pub permissions: Vec<String>,
    /// Fixed session expiry (RFC3339).
    pub expires_at: String,
}

impl std::fmt::Debug for LoginResponseDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginResponseDto")
            .field("session_token", &"***")
            .field("user_id", &self.user_id)
            .field("username", &self.username)
            .field("role", &self.role)
            .field("permissions", &self.permissions)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Session revocation request.
///
/// The manual `Debug` impl redacts the session token.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutRequestDto {
    /// Token to revoke (unknown tokens are a silent no-op).
    pub session_token: String,
}

impl std::fmt::Debug for LogoutRequestDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogoutRequestDto")
            .field("session_token", &"***")
            .finish()
    }
}

// First-run bootstrap wire types (TASK-BRK-016, ADR-048) -------------------
// Unauthenticated by definition: no session token travels here. The Admin
// sets their own credentials; no default password exists anywhere.

/// First-run probe response: true only while the `users` table is empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFirstRunStatusResponse {
    /// True when the setup screen must be shown instead of the login form.
    pub requires_setup: bool,
}

/// First-admin creation request.
///
/// Never `Debug`-printable with the secret: the manual `Debug` impl below
/// redacts the password field, mirroring `LoginRequestDto`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapFirstAdminRequest {
    /// Client idempotency key (UUID v4 string; informational for this flow).
    pub command_id: String,
    /// Requested Admin login name.
    pub username: String,
    /// Requested Admin password (memory only, never logged or persisted).
    pub password: String,
}

impl std::fmt::Debug for BootstrapFirstAdminRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BootstrapFirstAdminRequest")
            .field("command_id", &self.command_id)
            .field("username", &self.username)
            .field("password", &"***")
            .finish()
    }
}

/// First-admin creation response (no session token: the frontend logs in
/// with the freshly created credentials through the standard login flow).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapFirstAdminResponse {
    /// Fresh operator id of the created Admin (UUID hyphenated string).
    pub user_id: String,
    /// Stored login name (trimmed).
    pub username: String,
}
