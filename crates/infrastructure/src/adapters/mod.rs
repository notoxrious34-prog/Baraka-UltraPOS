//! Concrete adapters: auth (Argon2id, SQLite users, memory sessions,
//! system clock) plus hardware peripheral status contracts.

pub mod argon2_hasher;
pub mod in_memory_session_store;
pub mod sqlite_audit_log;
pub mod sqlite_backup_manager;
pub mod sqlite_cash_session_repository;
pub mod sqlite_catalog_repository;
pub mod sqlite_checkout_repository;
pub mod sqlite_command_log_repository;
pub mod sqlite_conversion_repository;
pub mod sqlite_customer_repository;
pub mod sqlite_document_repository;
pub mod sqlite_document_sequence_repository;
pub mod sqlite_fiscal_settings;
pub mod sqlite_print_job_repository;
pub mod sqlite_printer_settings;
pub mod sqlite_purchase_repository;
pub mod sqlite_reporting_repository;
pub mod sqlite_return_repository;
pub mod sqlite_stock_repository;
pub mod sqlite_supplier_repository;
pub mod sqlite_user_repository;
pub mod sqlite_warehouse_repository;
pub mod system_clock;

pub use argon2_hasher::Argon2PasswordHasher;
pub use in_memory_session_store::InMemorySessionStore;
pub use sqlite_audit_log::SqliteAuditLog;
pub use sqlite_backup_manager::SqliteBackupManager;
pub use sqlite_cash_session_repository::SqliteCashSessionRepository;
pub use sqlite_catalog_repository::SqliteCatalogRepository;
pub use sqlite_checkout_repository::SqliteCheckoutRepository;
pub use sqlite_command_log_repository::SqliteCommandLogRepository;
pub use sqlite_customer_repository::SqliteCustomerRepository;
pub use sqlite_document_repository::SqliteDocumentRepository;
pub use sqlite_document_sequence_repository::SqliteDocumentSequenceRepository;
pub use sqlite_fiscal_settings::SqliteFiscalSettings;
pub use sqlite_print_job_repository::SqlitePrintJobRepository;
pub use sqlite_printer_settings::SqlitePrinterSettings;
pub use sqlite_purchase_repository::SqlitePurchaseRepository;
pub use sqlite_reporting_repository::SqliteReportingRepository;
pub use sqlite_return_repository::SqliteReturnRepository;
pub use sqlite_stock_repository::SqliteStockRepository;
pub use sqlite_supplier_repository::SqliteSupplierRepository;
pub use sqlite_user_repository::SqliteUserRepository;
pub use sqlite_warehouse_repository::SqliteWarehouseRepository;
pub use system_clock::SystemClock;

use serde::{Deserialize, Serialize};

/// Readiness of one local peripheral.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeripheralStatus {
    /// Stable peripheral key, e.g. `receipt_printer`.
    pub key: String,
    /// Human-readable label (French).
    pub label: String,
    /// True when the device answered its last probe.
    pub ready: bool,
}

impl PeripheralStatus {
    /// Build an "absent but known" peripheral entry (no I/O performed).
    pub fn unknown(key: &str, label: &str) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            ready: false,
        }
    }
}

/// Baseline peripheral inventory (no probing yet).
#[must_use]
pub fn baseline_peripherals() -> Vec<PeripheralStatus> {
    vec![
        PeripheralStatus::unknown("receipt_printer", "Imprimante ticket"),
        PeripheralStatus::unknown("barcode_scanner", "Douchette code-barres"),
        PeripheralStatus::unknown("cash_drawer", "Tiroir-caisse"),
    ]
}
