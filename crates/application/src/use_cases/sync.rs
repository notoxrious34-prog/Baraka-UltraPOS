//! Two-till sync use cases (TASK-BRK-124, ADR-123 subset).
//!
//! Export and import are BOTH gated on `BackupRestoreManage` (data
//! stewardship: Gérant/Admin only — a cashier never moves another till's
//! documents). Authorization lives here; the UI gate is convenience only.
//!
//! v1 boundary (loud, documented in the UI): sale documents
//! (TICKET/BL/FACTURE) with items + payments. Returns/credit-memos stay on
//! their origin till — their cumulative chains reference source documents
//! the importer cannot resolve. Imported payments carry NO cash session
//! (money stays put); stock moves apply to the importer's warehouse with
//! importer-wins on negative (ADR-123) surfaced as warnings for the radar.

use baraka_domain::errors::DomainError;
use baraka_domain::role::Permission;

use crate::errors::ApplicationError;
use crate::ports::{SessionContext, SyncExportPort, SyncImportOutcome, SyncImportPort};

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// Exported package envelope (transport text + count for the UI summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncExportOutcome {
    /// Canonical v1 package text (header + hashed body, LF-joined).
    pub package_text: String,
    /// Exported document count (0 allowed — empty package, not an error).
    pub exported_docs: usize,
}

/// Assemble a sync package of locally created sale documents.
pub struct ExportSyncPackageUseCase<'a, E: ?Sized> {
    exports: &'a E,
}

impl<'a, E> ExportSyncPackageUseCase<'a, E>
where
    E: SyncExportPort + ?Sized,
{
    /// Wire the use case to its export port.
    pub fn new(exports: &'a E) -> Self {
        Self { exports }
    }

    /// Authorize (Gérant/Admin) then assemble. `since_created_at` is an
    /// RFC-3339 exclusive cursor; `limit` clamps to 1..=5000 upstream.
    pub fn execute(
        &self,
        session: &SessionContext,
        terminal_id: &str,
        since_created_at: &str,
        limit: usize,
    ) -> Result<SyncExportOutcome, ApplicationError> {
        if !session.role.has_permission(Permission::BackupRestoreManage) {
            return Err(ApplicationError::Unauthorized);
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        if !(1..=5_000).contains(&limit) {
            return Err(coded("INVALID_SYNC_LIMIT"));
        }
        if since_created_at.trim().is_empty() {
            return Err(coded("INVALID_SYNC_CURSOR"));
        }
        let (package_text, exported_docs) =
            self.exports
                .export_package(terminal_id.trim(), since_created_at.trim(), limit)?;
        Ok(SyncExportOutcome {
            package_text,
            exported_docs,
        })
    }
}

/// Merge a verified sync package atomically (all-or-nothing).
pub struct ImportSyncPackageUseCase<'a, I: ?Sized> {
    imports: &'a I,
}

impl<'a, I> ImportSyncPackageUseCase<'a, I>
where
    I: SyncImportPort + ?Sized,
{
    /// Wire the use case to its import port.
    pub fn new(imports: &'a I) -> Self {
        Self { imports }
    }

    /// Authorize (Gérant/Admin) then merge into `warehouse_id`. The actor is
    /// the importing session user (imported rows are attributed locally).
    pub fn execute(
        &self,
        session: &SessionContext,
        package_text: &str,
        warehouse_id: &str,
        terminal_id: &str,
        now_rfc3339: &str,
    ) -> Result<SyncImportOutcome, ApplicationError> {
        if !session.role.has_permission(Permission::BackupRestoreManage) {
            return Err(ApplicationError::Unauthorized);
        }
        if package_text.trim().is_empty() {
            return Err(coded("EMPTY_SYNC_PACKAGE"));
        }
        if warehouse_id.trim().is_empty() {
            return Err(coded("INVALID_WAREHOUSE_ID"));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        self.imports.import_package(
            package_text,
            warehouse_id.trim(),
            &session.user_id.to_string(),
            terminal_id.trim(),
            now_rfc3339,
        )
    }
}

/// Re-export for the IPC layer (keeps the outcome shape stable in one place).
pub use crate::ports::SyncImportOutcome as SyncImportResult;
/// Re-export for the IPC layer.
pub use crate::ports::SyncRenumber as SyncRenumberRow;
