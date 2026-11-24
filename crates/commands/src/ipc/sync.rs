//! Two-till sync commands (TASK-BRK-124, ADR-123 subset).
//!
//! `sync_export_package` assembles the canonical v1 package (read-only).
//! `sync_import_package` merges a verified package in ONE atomic
//! transaction (fail-closed). Both are Gérant/Admin only
//! (`BackupRestoreManage`); the terminal id and clock come from managed
//! state, never client input.

use super::{error_code, require_session, AppState};
use crate::dto::{
    SyncExportRequestDto, SyncExportResponseDto, SyncImportRequestDto, SyncImportResponseDto,
    SyncRenumberDto, SyncStockWarningDto,
};

/// `sync_export_package` command: assemble the canonical v1 package of
/// locally created sale documents (Gérant/Admin, read-only).
#[tauri::command(rename_all = "snake_case")]
pub fn sync_export_package(
    state: tauri::State<'_, AppState>,
    request: SyncExportRequestDto,
) -> Result<SyncExportResponseDto, String> {
    use baraka_application::use_cases::sync::ExportSyncPackageUseCase;
    let session = require_session(&state, &request.session_token)?;
    let limit = usize::try_from(request.limit.unwrap_or(500)).unwrap_or(500);
    let output = ExportSyncPackageUseCase::new(state.sync.as_ref())
        .execute(
            &session,
            &state.terminal_id,
            &request.since_created_at,
            limit,
        )
        .map_err(|error| error_code(&error))?;
    let exported_docs = u32::try_from(output.exported_docs).unwrap_or(u32::MAX);
    Ok(SyncExportResponseDto {
        package_text: output.package_text,
        exported_docs,
    })
}

/// `sync_import_package` command: merge a verified package atomically
/// (Gérant/Admin, all-or-nothing, fail-closed on any identity gap).
#[tauri::command(rename_all = "snake_case")]
pub fn sync_import_package(
    state: tauri::State<'_, AppState>,
    request: SyncImportRequestDto,
) -> Result<SyncImportResponseDto, String> {
    use baraka_application::use_cases::sync::ImportSyncPackageUseCase;
    let session = require_session(&state, &request.session_token)?;
    let now_rfc3339 = state.clock.now_rfc3339();
    let output = ImportSyncPackageUseCase::new(state.sync.as_ref())
        .execute(
            &session,
            &request.package_text,
            &request.warehouse_id,
            &state.terminal_id,
            &now_rfc3339,
        )
        .map_err(|error| error_code(&error))?;
    let imported_docs = u32::try_from(output.imported_docs).unwrap_or(u32::MAX);
    Ok(SyncImportResponseDto {
        imported_docs,
        renumbered: output
            .renumbered
            .into_iter()
            .map(|row| SyncRenumberDto {
                origin_number: row.origin_number,
                new_number: row.new_number,
            })
            .collect(),
        negative_stock_warnings: output
            .negative_stock_warnings
            .into_iter()
            .map(|(sku, new_number)| SyncStockWarningDto { sku, new_number })
            .collect(),
    })
}
