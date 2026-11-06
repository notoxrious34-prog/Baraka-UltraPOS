//! Shared IPC kernel: AppState, session guard, error codes, wire parsers.
//!
//! Every domain module builds on this kernel. All helpers are `pub(crate)`
//! (used across modules); the hub re-exports them for handler paths.
use std::str::FromStr;
use std::sync::Arc;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    BackupHardeningPort, BackupRepository, CaptureSink, CashSessionRepository, CategoryRepository,
    CheckoutRepository, Clock, CustomerRepository, DocumentRepository, FactoryResetRepository,
    FiscalSettingsPort, PasswordHasher, PrintJobRepository, PrinterSettingsPort,
    PrinterTransportPort, ProductRepository, PurchaseRepository, ReceiptRendererPort,
    ReportingRepository, ReturnRepository, SessionContext, SessionStore, SessionToken,
    StockRepository, StoreIdentityPort, SupplierRepository, TsplLabelRasterPort, UserRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::catalog::{
    NewUnitInput, ProductDetail, ProductImportRepository, UnitUpsert,
};
use baraka_application::use_cases::sales::{CartLineInput, PaymentInput};
use baraka_domain::catalog::{Barcode, ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Role;
use baraka_domain::sales::{CustomerTier, PaymentMethod};
use uuid::Uuid;

use crate::dto::{
    CartLineItemDto, CommandErrorDto, HealthDto, KitItemDto, NewProductUnitDto, PaymentLineDto,
    ProductResponseDto, ProductUnitDto, UpdateProductUnitDto,
};

/// Shared backend handles for command handlers.
///
/// All members are trait objects over application ports, so this state
/// depends only on `baraka-application` - never on infrastructure or SQL.
pub struct AppState {
    /// Operator lookup and lockout persistence.
    pub users: Arc<dyn UserRepository>,
    /// Volatile session registry.
    pub sessions: Arc<dyn SessionStore>,
    /// Single source of time.
    pub clock: Arc<dyn Clock>,
    /// Secret verifier.
    pub hasher: Arc<dyn PasswordHasher>,
    /// Catalog persistence.
    pub catalog: Arc<dyn ProductRepository>,
    /// Atomic batch-import engine (same SQLite store, import port).
    pub imports: Arc<dyn ProductImportRepository>,
    /// Category persistence.
    pub categories: Arc<dyn CategoryRepository>,
    /// Warehouse persistence.
    pub warehouses: Arc<dyn WarehouseRepository>,
    /// Stock engine persistence.
    pub stock: Arc<dyn StockRepository>,
    /// Customer credit persistence.
    pub customers: Arc<dyn CustomerRepository>,
    /// Atomic checkout engine.
    pub checkout: Arc<dyn CheckoutRepository>,
    /// Document reads and drafts.
    pub documents: Arc<dyn DocumentRepository>,
    /// Print outbox state machine.
    pub print_jobs: Arc<dyn PrintJobRepository>,
    /// Serial printer configuration.
    pub printer_settings: Arc<dyn PrinterSettingsPort>,
    /// Store identity + diagnostics settings (same table, identity port).
    pub store_identity: Arc<dyn StoreIdentityPort>,
    /// Fiscal stamp settings (`fiscal.*` keys, ADR-092).
    pub fiscal: Arc<dyn FiscalSettingsPort>,
    /// Physical printer transport edge.
    pub printer_transport: Arc<dyn PrinterTransportPort>,
    /// Immutable test-print artifact sink.
    pub printer_capture: Arc<dyn CaptureSink>,
    /// Receipt raster renderer.
    pub printer_renderer: Arc<dyn ReceiptRendererPort>,
    /// Arabic-capable label line rasterer (PRT-02).
    pub label_raster: Arc<dyn TsplLabelRasterPort>,
    /// Cash session engine.
    pub cash: Arc<dyn CashSessionRepository>,
    /// Supplier master-data and statement persistence.
    pub suppliers: Arc<dyn SupplierRepository>,
    /// Atomic purchase engine (BC/BR/FF + ledger + payments).
    pub purchases: Arc<dyn PurchaseRepository>,
    /// Read-only reporting views over transactional tables.
    pub reports: Arc<dyn ReportingRepository>,
    /// Return bundle persistence (cumulative reads + atomic writes).
    pub returns: Arc<dyn ReturnRepository>,
    /// Backup snapshot persistence and audit rows.
    pub backups: Arc<dyn BackupRepository>,
    /// Automatic-backup schedule + USB mirror persistence (TASK-BRK-092).
    pub backup_hardening: Arc<dyn BackupHardeningPort>,
    /// Factory-reset execution edge (same SQLite manager, second handle).
    pub factory_reset: Arc<dyn FactoryResetRepository>,
    /// Directory holding backup snapshot files (created on demand).
    pub backup_dir: std::path::PathBuf,
    /// Sovereign terminal identifier (managed state, never client input).
    pub terminal_id: String,
}

/// `app_health` command: returns the baseline health snapshot.
///
/// The frontend calls this on startup to prove the Rust core is alive.
#[tauri::command(rename_all = "snake_case")]
pub fn app_health() -> Result<HealthDto, CommandErrorDto> {
    let snapshot = baraka_application::use_cases::health_snapshot();
    Ok(HealthDto::from(snapshot))
}

/// Map an application failure to a stable wire code.
///
/// Unknown-user and wrong-password both yield `INVALID_CREDENTIALS`;
/// nothing secret-bearing ever leaves this function.
pub(crate) fn error_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::Validation(DomainError::InvalidCredentials) => {
            "INVALID_CREDENTIALS".to_string()
        }
        ApplicationError::Validation(DomainError::AccountInactive) => {
            "ACCOUNT_INACTIVE".to_string()
        }
        ApplicationError::Validation(DomainError::AccountLocked { remaining_seconds }) => {
            format!("ACCOUNT_LOCKED:{remaining_seconds}")
        }
        ApplicationError::Validation(DomainError::PinNotSet) => "PIN_NOT_SET".to_string(),
        ApplicationError::Validation(DomainError::BootstrapAlreadyCompleted) => {
            "BOOTSTRAP_ALREADY_COMPLETED".to_string()
        }
        ApplicationError::Validation(DomainError::DuplicateSku) => "DUPLICATE_SKU".to_string(),
        ApplicationError::Validation(DomainError::DuplicateBarcode) => {
            "DUPLICATE_BARCODE".to_string()
        }
        ApplicationError::Validation(DomainError::BaseUnitMismatch) => {
            "BASE_UNIT_MISMATCH".to_string()
        }
        ApplicationError::Validation(DomainError::KitCycleDetected) => "KIT_CYCLE".to_string(),
        ApplicationError::Validation(DomainError::InvalidKitComponent) => "INVALID_KIT".to_string(),
        ApplicationError::Validation(DomainError::SelfReferencingKit) => {
            "SELF_REFERENCING_KIT".to_string()
        }
        ApplicationError::Validation(DomainError::InactiveKitComponent) => {
            "KIT_COMPONENT_INACTIVE".to_string()
        }
        ApplicationError::Validation(DomainError::EmptyCart) => "EMPTY_CART".to_string(),
        ApplicationError::Validation(DomainError::InsufficientPayment) => {
            "INSUFFICIENT_PAYMENT".to_string()
        }
        ApplicationError::Validation(DomainError::ChangeNotPermittedForNonCash) => {
            "CHANGE_NOT_PERMITTED_FOR_NON_CASH".to_string()
        }
        ApplicationError::Validation(DomainError::CreditDisabled) => "CREDIT_DISABLED".to_string(),
        ApplicationError::Validation(DomainError::GaplessSequenceOverflow) => {
            "GAPLESS_SEQUENCE_OVERFLOW".to_string()
        }
        ApplicationError::Validation(DomainError::InvalidDocumentLanguage) => {
            "INVALID_DOCUMENT_LANGUAGE".to_string()
        }
        ApplicationError::Validation(DomainError::CustomerNotFound) => {
            "CUSTOMER_NOT_FOUND".to_string()
        }
        ApplicationError::Validation(DomainError::DuplicateCustomerPhone) => {
            "CUSTOMER_PHONE_DUPLICATE".to_string()
        }
        ApplicationError::Validation(DomainError::InvalidCreditLimit) => {
            "INVALID_CREDIT_LIMIT".to_string()
        }
        ApplicationError::Validation(DomainError::PaymentExceedsBalance) => {
            "PAYMENT_EXCEEDS_BALANCE".to_string()
        }
        ApplicationError::CreditLimitExceeded { .. } => "CREDIT_LIMIT_EXCEEDED".to_string(),
        ApplicationError::IdempotencyConflict => "IDEMPOTENCY_CONFLICT".to_string(),
        ApplicationError::CustomerNotFound => "CUSTOMER_NOT_FOUND".to_string(),
        ApplicationError::DuplicatePhone => "CUSTOMER_PHONE_DUPLICATE".to_string(),
        ApplicationError::PaymentExceedsBalance { .. } => "PAYMENT_EXCEEDS_BALANCE".to_string(),
        ApplicationError::OverpaymentUnauthorized => "OVERPAYMENT_UNAUTHORIZED".to_string(),
        ApplicationError::Validation(DomainError::KitExpansionDepthExceeded) => {
            "KIT_EXPANSION_DEPTH_EXCEEDED".to_string()
        }
        ApplicationError::Validation(DomainError::PrintJobNotFound) => {
            "PRINT_JOB_NOT_FOUND".to_string()
        }
        ApplicationError::Validation(DomainError::PrintRenderingFailed) => {
            "PRINT_RENDERING_FAILED".to_string()
        }
        ApplicationError::Validation(DomainError::PrinterTransportUnavailable) => {
            "PRINTER_UNAVAILABLE".to_string()
        }
        ApplicationError::Validation(DomainError::TsplTransportRequiresSpooler) => {
            "TSPL_REQUIRES_SPOOLER".to_string()
        }
        ApplicationError::Validation(DomainError::TsplBarcodeUnencodable) => {
            "TSPL_BARCODE_UNENCODABLE".to_string()
        }
        ApplicationError::Validation(DomainError::TsplBarcodeTooWide) => {
            "TSPL_BARCODE_TOO_WIDE".to_string()
        }
        ApplicationError::Validation(DomainError::TsplCopiesOutOfRange) => {
            "TSPL_COPIES_OUT_OF_RANGE".to_string()
        }
        ApplicationError::Validation(DomainError::CashSessionAlreadyOpenForTerminal) => {
            "CASH_SESSION_ALREADY_OPEN_FOR_TERMINAL".to_string()
        }
        ApplicationError::Validation(DomainError::CashSessionNotOpen) => {
            "CASH_SESSION_NOT_OPEN".to_string()
        }
        ApplicationError::Validation(DomainError::CashSessionNotInClosing) => {
            "CASH_SESSION_NOT_IN_CLOSING".to_string()
        }
        ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal) => {
            "NO_OPEN_CASH_SESSION_FOR_TERMINAL".to_string()
        }
        ApplicationError::Validation(DomainError::CashSessionInvalidCountedAmount) => {
            "CASH_SESSION_INVALID_COUNTED_AMOUNT".to_string()
        }
        ApplicationError::Validation(DomainError::CashSessionInvalidOpeningFloat) => {
            "CASH_SESSION_INVALID_OPENING_FLOAT".to_string()
        }
        ApplicationError::Validation(DomainError::CashExpenseInvalidAmount) => {
            "CASH_EXPENSE_INVALID_AMOUNT".to_string()
        }
        ApplicationError::Validation(DomainError::CashExpenseDescriptionRequired) => {
            "CASH_EXPENSE_DESCRIPTION_REQUIRED".to_string()
        }
        ApplicationError::Validation(DomainError::CashSessionNotFound) => {
            "CASH_SESSION_NOT_FOUND".to_string()
        }
        ApplicationError::Validation(DomainError::InvalidPaymentState) => {
            "INVALID_PAYMENT_STATE".to_string()
        }
        ApplicationError::Validation(DomainError::SupplierNotFound) => {
            "SUPPLIER_NOT_FOUND".to_string()
        }
        ApplicationError::Validation(DomainError::SupplierNameRequired) => {
            "SUPPLIER_NAME_REQUIRED".to_string()
        }
        ApplicationError::Validation(DomainError::SupplierInactive) => {
            "SUPPLIER_INACTIVE".to_string()
        }
        ApplicationError::Validation(DomainError::PurchaseOrderNotFound) => {
            "PURCHASE_ORDER_NOT_FOUND".to_string()
        }
        ApplicationError::Validation(DomainError::PurchaseOrderNotDraft) => {
            "PURCHASE_ORDER_NOT_DRAFT".to_string()
        }
        ApplicationError::Validation(DomainError::PurchaseReceiptQuantityExceedsOrdered) => {
            "PURCHASE_RECEIPT_QUANTITY_EXCEEDS_ORDERED".to_string()
        }
        ApplicationError::Validation(DomainError::PurchaseInvoiceAlreadySourced) => {
            "PURCHASE_INVOICE_ALREADY_SOURCED".to_string()
        }
        ApplicationError::Validation(DomainError::DocumentAlreadyConverted) => {
            "DOCUMENT_ALREADY_CONVERTED".to_string()
        }
        ApplicationError::Validation(DomainError::InvalidConversionSourceStatus) => {
            "INVALID_CONVERSION_SOURCE_STATUS".to_string()
        }
        ApplicationError::Validation(DomainError::InvalidConversionSourceType) => {
            "INVALID_CONVERSION_SOURCE_TYPE".to_string()
        }
        ApplicationError::Validation(DomainError::SourceDocumentNotFound) => {
            "SOURCE_DOCUMENT_NOT_FOUND".to_string()
        }
        ApplicationError::Validation(DomainError::EmptySourceDocument) => {
            "EMPTY_SOURCE_DOCUMENT".to_string()
        }
        ApplicationError::Validation(DomainError::PurchaseLineQuantityInvalid) => {
            "PURCHASE_LINE_QUANTITY_INVALID".to_string()
        }
        ApplicationError::Validation(DomainError::SupplierPaymentInvalidAmount) => {
            "SUPPLIER_PAYMENT_INVALID_AMOUNT".to_string()
        }
        ApplicationError::Validation(DomainError::SupplierPaymentMethodInvalid) => {
            "SUPPLIER_PAYMENT_METHOD_INVALID".to_string()
        }
        ApplicationError::Validation(DomainError::InvalidDateRange) => {
            "INVALID_DATE_RANGE".to_string()
        }
        ApplicationError::Validation(DomainError::ReportPaginationLimitExceeded) => {
            "REPORT_PAGINATION_LIMIT_EXCEEDED".to_string()
        }
        ApplicationError::Validation(DomainError::InvalidCanonicalString(message)) => {
            stable_code_or(message, "INVALID_INPUT")
        }
        ApplicationError::Validation(_) => "INVALID_INPUT".to_string(),
        ApplicationError::NotFound(entity) if entity == "warehouse" => {
            "WAREHOUSE_NOT_FOUND".to_string()
        }
        ApplicationError::NotFound(entity) if entity == "document" => {
            "DOCUMENT_NOT_FOUND".to_string()
        }
        ApplicationError::NotFound(_) => "NOT_FOUND".to_string(),
        ApplicationError::Unauthorized => "UNAUTHORIZED".to_string(),
        ApplicationError::InvalidQuantity => "INVALID_QUANTITY".to_string(),
        ApplicationError::CannotMoveStockForKitProduct => {
            "PRODUCT_IS_KIT_NO_DIRECT_STOCK".to_string()
        }
        ApplicationError::InsufficientStock { .. } => "STOCK_INSUFFICIENT".to_string(),
        ApplicationError::NegativeStockConfirmationRequired { .. } => {
            "STOCK_NEGATIVE_CONFIRMATION_REQUIRED".to_string()
        }
        ApplicationError::ConcurrencyInvariantViolated => {
            "CONCURRENCY_INVARIANT_VIOLATED".to_string()
        }
        ApplicationError::Conflict(message) => stable_code_or(message, "INTERNAL_ERROR"),
        ApplicationError::Port(_) => "INTERNAL_ERROR".to_string(),
    }
}

/// Pass through stable UPPER_SNAKE codes carried inside generic errors;
/// anything else collapses to the safe fallback.
pub(crate) fn stable_code_or(message: &str, fallback: &'static str) -> String {
    if !message.is_empty()
        && message
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte == b'_' || byte.is_ascii_digit())
    {
        message.to_string()
    } else {
        fallback.to_string()
    }
}

/// Adjust-path mapping: insufficient stock without authorization surfaces
/// the dedicated negative-stock code instead of the transfer-oriented one.
pub(crate) fn adjust_error_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::InsufficientStock { .. } => "STOCK_NEGATIVE_NOT_AUTHORIZED".to_string(),
        other => error_code(other),
    }
}

/// Resolve and validate a session token for catalog commands.
pub(crate) fn require_session(
    state: &AppState,
    session_token: &str,
) -> Result<SessionContext, String> {
    let token = SessionToken(session_token.to_string());
    state
        .sessions
        .validate_session(&token)
        .ok_or_else(|| "UNAUTHORIZED".to_string())
}

/// Parse a canonical integer wire string (optional `-`, ASCII digits only).
pub(crate) fn parse_wire_int(text: &str, field: &'static str) -> Result<i64, ApplicationError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString(format!("{field} must not be empty")),
        ));
    }
    let digits = trimmed.strip_prefix('-').unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString(format!("{field} must be an integer string")),
        ));
    }
    trimmed.parse::<i64>().map_err(|_| {
        ApplicationError::Validation(DomainError::InvalidCanonicalString(format!(
            "{field} out of range"
        )))
    })
}

/// Parse a UUID wire string.
pub(crate) fn parse_wire_id(text: &str, field: &'static str) -> Result<Uuid, ApplicationError> {
    Uuid::parse_str(text.trim()).map_err(|_| {
        ApplicationError::Validation(DomainError::InvalidCanonicalString(format!(
            "{field} must be a UUID"
        )))
    })
}

pub(crate) fn parse_unit_dto(dto: &NewProductUnitDto) -> Result<NewUnitInput, ApplicationError> {
    Ok(NewUnitInput {
        unit_name_ar: dto.unit_name_ar.clone(),
        unit_name_fr: dto.unit_name_fr.clone(),
        factor_micro: parse_wire_int(&dto.factor_micro, "factor_micro")?,
        barcode: dto.barcode.clone(),
        sale_price_centimes: parse_wire_int(&dto.sale_price_centimes, "sale_price_centimes")?,
        purchase_price_centimes: parse_wire_int(
            &dto.purchase_price_centimes,
            "purchase_price_centimes",
        )?,
        wholesale_price_centimes: dto
            .wholesale_price_centimes
            .as_deref()
            .map(|text| parse_wire_int(text, "wholesale_price_centimes"))
            .transpose()?,
        is_base_unit: dto.is_base_unit,
        is_default_selling: dto.is_default_selling,
    })
}

/// Parse a pricing-tier wire code (`None`/blank = Retail default).
pub(crate) fn parse_customer_tier(
    raw: Option<&str>,
    field: &'static str,
) -> Result<CustomerTier, ApplicationError> {
    match raw.map(str::trim) {
        None | Some("") => Ok(CustomerTier::Retail),
        Some(code) => CustomerTier::from_str(code).map_err(|_| {
            ApplicationError::Validation(DomainError::InvalidCanonicalString(format!(
                "{field} must be retail, artisan, or wholesale"
            )))
        }),
    }
}

/// Parse one cart line DTO (same field contract as `sales_checkout` and
/// `sales_create_draft`, per the established sales edge-parse precedent).
/// `line_discount_centimes` is the PER-UNIT F5 discount (BRK-096); the
/// shape is unchanged, only the meaning moved from line gross to unit price.
pub(crate) fn parse_cart_line_dto(
    dto: &CartLineItemDto,
) -> Result<CartLineInput, ApplicationError> {
    Ok(CartLineInput {
        product_id: parse_wire_id(&dto.product_id, "product_id").map(ProductId::from)?,
        unit_id: parse_wire_id(&dto.unit_id, "unit_id").map(UnitId::from)?,
        quantity_micro: parse_wire_int(&dto.quantity_micro, "quantity_micro")?,
        unit_price_override: dto
            .unit_price_centimes
            .as_deref()
            .map(|text| parse_wire_int(text, "unit_price_centimes"))
            .transpose()?,
        line_discount_centimes: parse_wire_int(
            &dto.line_discount_centimes,
            "line_discount_centimes",
        )?,
    })
}

/// Parse one tender line DTO (same field contract as `sales_checkout`).
pub(crate) fn parse_payment_dto(dto: &PaymentLineDto) -> Result<PaymentInput, ApplicationError> {
    Ok(PaymentInput {
        method: PaymentMethod::from_code(&dto.payment_method)
            .map_err(ApplicationError::Validation)?,
        amount_centimes: parse_wire_int(&dto.amount_centimes, "amount_centimes")?,
    })
}

/// Parse one update unit upsert (same field contract as creation, plus the
/// optional stable unit id; mirrors `parse_unit_dto` per the established
/// sales_checkout/sales_create_draft duplication precedent).
pub(crate) fn parse_upsert_unit_dto(
    dto: &UpdateProductUnitDto,
) -> Result<UnitUpsert, ApplicationError> {
    Ok(UnitUpsert {
        id: dto
            .unit_id
            .as_deref()
            .map(|text| parse_wire_id(text, "unit_id").map(UnitId::from))
            .transpose()?,
        unit: NewUnitInput {
            unit_name_ar: dto.unit_name_ar.clone(),
            unit_name_fr: dto.unit_name_fr.clone(),
            factor_micro: parse_wire_int(&dto.factor_micro, "factor_micro")?,
            barcode: dto.barcode.clone(),
            sale_price_centimes: parse_wire_int(&dto.sale_price_centimes, "sale_price_centimes")?,
            purchase_price_centimes: parse_wire_int(
                &dto.purchase_price_centimes,
                "purchase_price_centimes",
            )?,
            wholesale_price_centimes: dto
                .wholesale_price_centimes
                .as_deref()
                .map(|text| parse_wire_int(text, "wholesale_price_centimes"))
                .transpose()?,
            is_base_unit: dto.is_base_unit,
            is_default_selling: dto.is_default_selling,
        },
    })
}

pub(crate) fn to_unit_dto(unit: &baraka_domain::catalog::ProductUnit) -> ProductUnitDto {
    ProductUnitDto {
        unit_id: unit.id.to_string(),
        product_id: unit.product_id.to_string(),
        unit_name_ar: unit.unit_name_ar.clone(),
        unit_name_fr: unit.unit_name_fr.clone(),
        factor_micro: unit.factor_micro.micros().to_string(),
        barcode: unit
            .barcode
            .as_ref()
            .map(Barcode::as_str)
            .map(str::to_string),
        sale_price_centimes: unit.sale_price.centimes().to_string(),
        purchase_price_centimes: unit.purchase_price.centimes().to_string(),
        wholesale_price_centimes: unit
            .wholesale_price
            .map(|price| price.centimes().to_string()),
        is_base_unit: unit.is_base_unit,
        is_default_selling: unit.is_default_selling,
    }
}

pub(crate) fn to_response(detail: ProductDetail) -> ProductResponseDto {
    ProductResponseDto {
        product_id: detail.product.id.to_string(),
        sku: detail.product.sku.as_str().to_string(),
        barcode: detail
            .product
            .barcode
            .as_ref()
            .map(Barcode::as_str)
            .map(str::to_string),
        name_ar: detail.product.name_ar.clone(),
        name_fr: detail.product.name_fr.clone(),
        category_id: detail.product.category_id.map(|id| id.to_string()),
        is_kit: detail.product.is_kit,
        is_active: detail.product.is_active,
        units: detail.units.iter().map(to_unit_dto).collect(),
        kit_items: detail
            .kit_items
            .iter()
            .map(|item| KitItemDto {
                component_id: item.component_product_id.to_string(),
                unit_id: item.unit_id.to_string(),
                quantity_micro: item.quantity_micro.micros().to_string(),
            })
            .collect(),
    }
}

/// Sorted effective permission codes for `role` (ADR-100).
///
/// Codes are the serde `SCREAMING_SNAKE_CASE` form of `Permission`
/// (e.g. `USER_MANAGE`), sorted lexicographically for deterministic
/// serialization. Serialization of a unit variant is infallible; the
/// `Debug` fallback keeps this total without panicking.
pub(crate) fn sorted_permission_codes(role: Role) -> Vec<String> {
    let mut codes: Vec<String> = role
        .permissions()
        .iter()
        .map(|permission| {
            serde_json::to_value(permission)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_else(|| format!("{permission:?}"))
        })
        .collect();
    codes.sort();
    codes
}
