//! IPC hub: domain command modules re-exported on the historic flat paths.
//!
//! Pure move (TASK-BRK-107, mirror of BRK-106): every handler keeps its
//! `baraka_commands::ipc::<command>` paths via hub re-exports, so the
//! `generate_handler!` list in `src-tauri/src/lib.rs` is untouched.
//! The shared kernel (`AppState`, guards, wire parsers) lives in `common`;
//! cross-module display helpers stay `pub(crate)` in their home module and
//! resolve through `super::<module>::<helper>` paths. The five regression
//! tests stay here so `super::*` covers the whole surface.

pub mod auth;
pub mod backups;
pub mod cash;
pub mod catalog;
pub mod common;
pub mod customers;
pub mod drafts;
pub mod inventory;
pub mod printing;
pub mod reports;
pub mod returns;
pub mod sales;
pub mod suppliers;

// Kernel re-export: `AppState` and `app_health` keep their external paths
// (`baraka_commands::ipc::AppState`); the helpers resolve the
// `use super::{...}` imports at the top of every domain module.
pub use common::{__cmd__app_health, __tauri_command_name_app_health, app_health, AppState};
pub(crate) use common::{
    adjust_error_code, error_code, parse_cart_line_dto, parse_customer_tier, parse_payment_dto,
    parse_unit_dto, parse_upsert_unit_dto, parse_wire_id, parse_wire_int, require_session,
    sorted_permission_codes, to_response,
};

// Flat-path re-exports: every domain command plus its `#[tauri::command]`
// generated items (`__cmd__*`), so the `generate_handler!` list in
// `src-tauri/src/lib.rs` resolves on the historic flat paths untouched.
// Glob form is deliberate: new commands flow automatically, while
// `pub(crate)` helpers are never elevated (verified by `cargo check`).
pub use auth::*;
pub use backups::*;
pub use cash::*;
pub use catalog::*;
pub use customers::*;
pub use drafts::*;
pub use inventory::*;
pub use printing::*;
pub use reports::*;
pub use returns::*;
pub use sales::*;
pub use suppliers::*;

// Test-only names: the hub regression tests below resolve through
// `super::*`, exactly as before the split. Kept `#[cfg(test)]` so normal
// builds carry zero dead imports (G2 `-D warnings`).
#[cfg(test)]
use crate::dto::LoginResponseDto;
#[cfg(test)]
use baraka_application::errors::ApplicationError;
#[cfg(test)]
use baraka_domain::errors::DomainError;
#[cfg(test)]
use baraka_domain::role::Role;
#[cfg(test)]
pub(crate) use sales::parse_document_language;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_language_parses_strictly() {
        use baraka_domain::sales::DocumentLanguage;
        assert_eq!(
            parse_document_language("AR").expect("ar"),
            DocumentLanguage::Ar
        );
        assert_eq!(
            parse_document_language("FR").expect("fr"),
            DocumentLanguage::Fr
        );
        // Missing, blank, foreign, and case variants all reject hard.
        for bad in ["", "   ", "EN", "ar", "Fr", "ARA", "FRENCH"] {
            let error = parse_document_language(bad).expect_err("must reject");
            assert_eq!(error_code(&error), "INVALID_DOCUMENT_LANGUAGE");
        }
    }

    #[test]
    fn language_error_code_is_stable() {
        assert_eq!(
            error_code(&ApplicationError::Validation(
                DomainError::InvalidDocumentLanguage
            )),
            "INVALID_DOCUMENT_LANGUAGE"
        );
    }

    #[test]
    fn catalog_update_error_codes_are_stable() {
        // Every failure the update path can surface maps to a non-empty,
        // frontend-displayable wire code (verified for the new command).
        assert_eq!(
            error_code(&ApplicationError::NotFound("product".to_string())),
            "NOT_FOUND"
        );
        assert_eq!(
            error_code(&ApplicationError::NotFound("category".to_string())),
            "NOT_FOUND"
        );
        assert_eq!(error_code(&ApplicationError::Unauthorized), "UNAUTHORIZED");
        assert_eq!(
            error_code(&ApplicationError::Validation(DomainError::DuplicateSku)),
            "DUPLICATE_SKU"
        );
        assert_eq!(
            error_code(&ApplicationError::Validation(DomainError::DuplicateBarcode)),
            "DUPLICATE_BARCODE"
        );
    }

    #[test]
    fn cash_error_codes_are_stable() {
        let cases = [
            (
                DomainError::CashSessionAlreadyOpenForTerminal,
                "CASH_SESSION_ALREADY_OPEN_FOR_TERMINAL",
            ),
            (DomainError::CashSessionNotOpen, "CASH_SESSION_NOT_OPEN"),
            (
                DomainError::CashSessionNotInClosing,
                "CASH_SESSION_NOT_IN_CLOSING",
            ),
            (
                DomainError::NoOpenCashSessionForTerminal,
                "NO_OPEN_CASH_SESSION_FOR_TERMINAL",
            ),
            (
                DomainError::CashSessionInvalidCountedAmount,
                "CASH_SESSION_INVALID_COUNTED_AMOUNT",
            ),
            (
                DomainError::CashSessionInvalidOpeningFloat,
                "CASH_SESSION_INVALID_OPENING_FLOAT",
            ),
            (
                DomainError::CashExpenseInvalidAmount,
                "CASH_EXPENSE_INVALID_AMOUNT",
            ),
            (
                DomainError::CashExpenseDescriptionRequired,
                "CASH_EXPENSE_DESCRIPTION_REQUIRED",
            ),
            (DomainError::CashSessionNotFound, "CASH_SESSION_NOT_FOUND"),
            (DomainError::InvalidPaymentState, "INVALID_PAYMENT_STATE"),
            (DomainError::SupplierNotFound, "SUPPLIER_NOT_FOUND"),
            (DomainError::SupplierNameRequired, "SUPPLIER_NAME_REQUIRED"),
            (DomainError::SupplierInactive, "SUPPLIER_INACTIVE"),
            (
                DomainError::PurchaseOrderNotFound,
                "PURCHASE_ORDER_NOT_FOUND",
            ),
            (
                DomainError::PurchaseOrderNotDraft,
                "PURCHASE_ORDER_NOT_DRAFT",
            ),
            (
                DomainError::PurchaseReceiptQuantityExceedsOrdered,
                "PURCHASE_RECEIPT_QUANTITY_EXCEEDS_ORDERED",
            ),
            (
                DomainError::PurchaseInvoiceAlreadySourced,
                "PURCHASE_INVOICE_ALREADY_SOURCED",
            ),
            (
                DomainError::PurchaseLineQuantityInvalid,
                "PURCHASE_LINE_QUANTITY_INVALID",
            ),
            (
                DomainError::SupplierPaymentInvalidAmount,
                "SUPPLIER_PAYMENT_INVALID_AMOUNT",
            ),
            (
                DomainError::SupplierPaymentMethodInvalid,
                "SUPPLIER_PAYMENT_METHOD_INVALID",
            ),
            (DomainError::InvalidDateRange, "INVALID_DATE_RANGE"),
        ];
        for (variant, code) in cases {
            assert_eq!(error_code(&ApplicationError::Validation(variant)), code);
        }
    }

    #[test]
    fn login_response_carries_sorted_effective_permissions() {
        // ADR-100: the login wire contract exposes the role's full effective
        // permission set as sorted SCREAMING_SNAKE_CASE codes (serde form).
        let admin = sorted_permission_codes(Role::Admin);
        assert_eq!(admin.len(), 34);
        assert!(admin.contains(&"USER_MANAGE".to_string()));
        assert!(admin.contains(&"CREATE_SALE".to_string()));
        assert!(admin.windows(2).all(|pair| pair[0] <= pair[1]));
        let vendeur = sorted_permission_codes(Role::Vendeur);
        assert_eq!(vendeur.len(), 5);
        assert!(vendeur.contains(&"CREATE_SALE".to_string()));
        assert!(!vendeur.contains(&"USER_MANAGE".to_string()));
        // The DTO itself carries the vector (field asserted by construction).
        let dto = LoginResponseDto {
            session_token: "tok".to_string(),
            user_id: "uid".to_string(),
            username: "admin".to_string(),
            role: Role::Admin.code().to_string(),
            permissions: admin.clone(),
            expires_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(dto.permissions, admin);
    }
}
