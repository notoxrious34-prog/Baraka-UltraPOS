//! Shared DTO primitives: IPC errors and the health view.
use baraka_domain::models::HealthSnapshot;
use serde::{Deserialize, Serialize};

/// Command failure serialized to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandErrorDto {
    /// Stable machine-readable code, e.g. `VALIDATION`.
    pub code: String,
    /// French user-facing message.
    pub message: String,
}

impl CommandErrorDto {
    /// Build a DTO from a code and a French message.
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

/// Health response served to the cockpit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthDto {
    /// Workspace version.
    pub version: String,
    /// RFC3339 capture time.
    pub captured_at: String,
    /// Offline-first readiness flag.
    pub offline_ready: bool,
}

impl From<HealthSnapshot> for HealthDto {
    fn from(snapshot: HealthSnapshot) -> Self {
        Self {
            version: snapshot.version,
            captured_at: snapshot.captured_at.to_rfc3339(),
            offline_ready: snapshot.offline_ready,
        }
    }
}
