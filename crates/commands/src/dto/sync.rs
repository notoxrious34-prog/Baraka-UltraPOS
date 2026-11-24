//! Sync data-transfer objects (TASK-BRK-124, two-till merge).
//!
//! The package itself is opaque canonical text (`BARAKA-SYNC-1`, SHA-256
//! over the body) — the frontend copies it verbatim between tills via USB
//! (file round-trip). Money never crosses as floats: the package carries
//! integer centimes/micros; counts cross as `u32`.

use serde::{Deserialize, Serialize};

/// `sync_export_package` request (Gérant/Admin, read-only assembly).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncExportRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Export documents created strictly after this RFC3339 cursor.
    pub since_created_at: String,
    /// Package cap (`None` selects 500, clamped 1..=5000).
    pub limit: Option<u32>,
}

/// `sync_export_package` response (opaque package + doc count).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncExportResponseDto {
    /// Canonical v1 package text (header + hashed body, LF-joined).
    pub package_text: String,
    /// Exported document count (0 allowed — empty package, not an error).
    pub exported_docs: u32,
}

/// `sync_import_package` request (Gérant/Admin, one atomic merge).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncImportRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Canonical v1 package text pasted from the origin till's USB file.
    pub package_text: String,
    /// Importer warehouse receiving the stock moves (chosen locally).
    pub warehouse_id: String,
}

/// One applied renumber (`origin_number` → importer gapless number).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncRenumberDto {
    /// Origin till's local number (identity, never reused locally).
    pub origin_number: String,
    /// Importer's gapless number (displayed, printed, audited).
    pub new_number: String,
}

/// One importer-wins stock line (negative forced, needs radar watch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStockWarningDto {
    /// Product SKU that went negative on the importer's warehouse.
    pub sku: String,
    /// Importer document number that forced it.
    pub new_number: String,
}

/// `sync_import_package` response (atomic outcome, fail-closed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncImportResponseDto {
    /// Imported document count.
    pub imported_docs: u32,
    /// Applied renumbers (origin → local, same order as the package).
    pub renumbered: Vec<SyncRenumberDto>,
    /// Importer-wins negative-stock lines (empty is the healthy case).
    pub negative_stock_warnings: Vec<SyncStockWarningDto>,
}
