//! SQLite connection factory with mandatory offline-first PRAGMAs.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use baraka_domain::catalog::search_normalization::normalize_for_search;

use super::InfrastructureError;

/// PRAGMAs applied to every connection at open time (offline-first law).
pub const OPEN_PRAGMAS: &str = "\
PRAGMA journal_mode = WAL;\n\
PRAGMA synchronous = FULL;\n\
PRAGMA foreign_keys = ON;\n\
PRAGMA busy_timeout = 5000;\n\
PRAGMA wal_autocheckpoint = 1000;";

/// Resolve the database file name for the desktop runtime.
#[must_use]
pub fn default_db_file_name() -> &'static str {
    "baraka.db"
}

/// Resolve the production database file inside the OS-standard per-user
/// application data directory (ADR-049).
///
/// The caller supplies the directory resolved via Tauri's official path
/// API (`app.path().app_data_dir()`); this pure join keeps the policy
/// testable without a Tauri runtime.
#[must_use]
pub fn resolve_database_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(default_db_file_name())
}

/// Ensure the database parent directory exists (idempotent).
///
/// An existing directory is a silent no-op success. Any I/O failure is
/// mapped to [`InfrastructureError::Config`] with a clear message so the
/// desktop harness can fail startup loudly instead of panicking.
pub fn ensure_database_dir(app_data_dir: &Path) -> Result<(), InfrastructureError> {
    std::fs::create_dir_all(app_data_dir).map_err(|error| {
        InfrastructureError::Config(format!(
            "cannot create application data directory '{}': {error}",
            app_data_dir.display()
        ))
    })
}

/// Open (or create) the SQLite file with the enforced PRAGMAs applied.
pub fn open_connection(path: &Path) -> Result<Connection, InfrastructureError> {
    let conn = Connection::open(path)?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

/// Open an in-memory database with the same PRAGMAs (pure schema tests only).
///
/// Production and concurrency/crash tests must use file paths.
pub fn open_in_memory() -> Result<Connection, InfrastructureError> {
    let conn = Connection::open_in_memory()?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

fn apply_pragmas(conn: &Connection) -> Result<(), InfrastructureError> {
    conn.busy_timeout(Duration::from_millis(5_000))?;
    conn.execute_batch(OPEN_PRAGMAS)?;
    register_search_norm(conn)?;
    Ok(())
}

/// Register the deterministic `baraka_search_norm(text)` scalar on every
/// connection (TASK-BRK-076): migration-0016 triggers normalize names and
/// barcodes in SQL, so the function must exist wherever writes happen —
/// not only on the repository search path (where re-registration stays
/// idempotent). Pure domain logic; zero query-path change.
fn register_search_norm(conn: &Connection) -> Result<(), InfrastructureError> {
    conn.create_scalar_function(
        "baraka_search_norm",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8
            | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            use rusqlite::types::ValueRef;
            let value = ctx.get_raw(0);
            if value == ValueRef::Null {
                return Ok(String::new());
            }
            let text = value.as_str()?;
            Ok(normalize_for_search(text))
        },
    )?;
    Ok(())
}
