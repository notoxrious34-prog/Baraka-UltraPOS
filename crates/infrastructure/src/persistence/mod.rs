//! SQLite persistence: connections, transactions, embedded migrations.

pub mod connection;
pub mod data_dir;
pub mod migration;
pub mod sha256;
pub mod terminal_context;
pub mod transaction;

use thiserror::Error;

/// Failures surfaced by the persistence layer.
#[derive(Debug, Error)]
pub enum InfrastructureError {
    /// Underlying SQLite failure.
    #[error("sqlite failure: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Configuration or path failure.
    #[error("configuration failure: {0}")]
    Config(String),
    /// Migration execution failure.
    #[error("migration failure: {0}")]
    Migration(String),
    /// A previously applied migration no longer matches its checksum.
    #[error(
        "migration checksum mismatch for version {version}: expected {expected}, found {found}"
    )]
    ChecksumMismatch {
        /// Applied migration version.
        version: i64,
        /// Checksum recorded at apply time.
        expected: String,
        /// Checksum of the currently embedded script.
        found: String,
    },
    /// Transient busy/locked retries exhausted without success.
    #[error("transient lock retries exhausted after {0} attempts")]
    BusyTimeout(u32),
}
