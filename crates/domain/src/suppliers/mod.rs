//! Supplier domain: master data and the supplier ledger.
//!
//! Pure business logic only. Money is integer DZD centimes (`Money`).
//! Supplier balances are derived from the append-only ledger; the cached
//! `current_balance_centimes` column must always equal
//! `SUM(DEBIT) - SUM(CREDIT)` (reconciliation invariant).

pub mod supplier;
pub mod supplier_ledger;

pub use supplier::{
    validate_supplier_name, validate_supplier_optional_text, PurchaseOrderStatus, Supplier,
    SupplierId,
};
pub use supplier_ledger::{LedgerEntryType, LedgerReferenceType, SupplierLedgerEntry};
