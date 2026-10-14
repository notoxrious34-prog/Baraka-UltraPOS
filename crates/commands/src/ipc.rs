//! Typed Tauri IPC handlers.
//!
//! Authentication errors map to stable, non-leaking wire codes. Raw Rust
//! error text (including Debug output) never crosses this boundary. Money
//! and quantities cross exclusively as canonical integer strings.

use std::str::FromStr;
use std::sync::Arc;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    BackupRepository, CaptureSink, CashSessionRepository, CategoryRepository, CheckoutRepository,
    Clock, CustomerRepository, DocumentRepository, DocumentSearchFilter, NewSupplier,
    PasswordHasher, PrintJobRepository, PrinterSettingsPort, PrinterTransportPort,
    ProductRepository, PurchaseLineInput, PurchaseRepository, ReceiptRendererPort,
    ReportingRepository, ReturnRepository, SessionContext, SessionStore, SessionToken,
    StockRepository, StoreIdentityPort, SupplierRepository, SupplierUpdate, UserRepository,
    WarehouseRepository,
};
use baraka_application::use_cases::backup::{
    CreateBackupUseCase, ListBackupsUseCase, PrecheckRestoreUseCase, RestoreBackupUseCase,
    VerifyBackupIntegrityUseCase,
};
use baraka_application::use_cases::cash::{
    BeginClosingUseCase, CancelClosingUseCase, ConfirmClosingUseCase, GetActiveSessionUseCase,
    GetSessionReportUseCase, OpenCashSessionUseCase, RecordExpenseUseCase,
};
use baraka_application::use_cases::catalog::CreateProductUseCase;
use baraka_application::use_cases::catalog::{
    CreateCategoryInput, CreateCategoryUseCase, CreateProductInput, DefineKitUseCase,
    GetProductWithUnitsUseCase, ImportBatchInput, ImportDuplicatePolicy, ImportProductRow,
    ImportProductsUseCase, ListCategoriesUseCase, NewKitItemInput, NewUnitInput, ProductDetail,
    ProductImportRepository, SearchCatalogUseCase, SeedStarterCategoriesUseCase, UnitUpsert,
    UpdateCategoryInput, UpdateCategoryUseCase, UpdateProductInput, UpdateProductUseCase,
};
use baraka_application::use_cases::customers::{
    CreateCustomerUseCase, GetCustomerStatementUseCase, GetCustomerUseCase, ListCustomersUseCase,
    RecordCreditPaymentUseCase, SearchCustomersUseCase, UpdateCustomerUseCase,
};
use baraka_application::use_cases::inventory::{
    AdjustStockInput, AdjustStockUseCase, CreateWarehouseUseCase, GetStockLevelUseCase,
    ListWarehousesUseCase, StockDirection, TransferStockInput, TransferStockUseCase,
};
use baraka_application::use_cases::printing::{GetPrintStatusUseCase, ReprintDocumentUseCase};
use baraka_application::use_cases::printing::{
    SaveStoreIdentityUseCase, TestPrintDeviceOutcome, TestPrintUseCase,
};
use baraka_application::use_cases::purchases::{
    CreatePurchaseOrderUseCase, GetPurchaseDocumentUseCase, ReceiveGoodsUseCase,
    RecordSupplierInvoiceUseCase, RecordSupplierPaymentUseCase, UpdatePurchaseOrderUseCase,
};
use baraka_application::use_cases::reports::{
    GetCashSessionsHistoryUseCase, GetCreditOutstandingReportUseCase, GetDailySalesSummaryUseCase,
    GetGrossMarginReportUseCase, GetLowStockReportUseCase, GetSalesByCategoryUseCase,
    GetSalesByProductUseCase, GetStockMovementHistoryUseCase, GetUserActivityLogUseCase,
};
use baraka_application::use_cases::returns::{
    ProcessReturnCommand, ProcessReturnUseCase, ProcessSaleReturnUseCase, RefundDestination,
    ReturnItemInput,
};
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, ClosingInput, ClosingPaymentInput,
    ConvertBlToFactureUseCase, ConvertDevisToBlInput, ConvertDevisToBlUseCase,
    CreateDraftSaleUseCase, DiscardDraftSaleUseCase, DraftInput, GetSaleUseCase, PaymentInput,
    PreviewSaleTotalsUseCase, PreviewTotalsInput, SearchDocumentsUseCase, StockShortageWarning,
};
use baraka_application::use_cases::suppliers::{
    CreateSupplierUseCase, GetSupplierStatementUseCase, GetSupplierUseCase, SearchSuppliersUseCase,
    UpdateSupplierUseCase,
};
use baraka_application::use_cases::users::{
    CreateUserUseCase, ListUsersUseCase, ResetUserCredentialsUseCase, UpdateUserUseCase,
};
use baraka_application::use_cases::{
    AuthenticateUserUseCase, BootstrapFirstAdminInput, BootstrapFirstAdminUseCase, LogoutUseCase,
};
use baraka_domain::catalog::{Barcode, Category, CategoryId, ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::primitives::QtyMicro;
use baraka_domain::role::{Permission, Role};
use baraka_domain::sales::{
    CreditPolicy, CustomerId, CustomerTier, DocumentId, DocumentLanguage, DocumentType,
    PaymentMethod,
};
use baraka_domain::user::UserId;
use uuid::Uuid;

use crate::dto::{
    AdjustStockRequestDto, AdjustStockResponseDto, BackupContentSummaryDto, BackupMetadataDto,
    BootstrapFirstAdminRequest, BootstrapFirstAdminResponse, CartLineItemDto,
    CashClosingCommandDto, CashMovementDto, CashMovementTotalDto, CashSessionDto,
    CashSessionHistoryRowDto, CashSessionReportDto, CashSessionsHistoryRequestDto,
    CatalogSearchRequestDto, CategoryResponseDto, CheckFirstRunStatusResponse, CheckoutSaleRequest,
    CheckoutSaleResponse, CommandErrorDto, ConfirmClosingRequestDto, ConvertBlToFactureRequest,
    ConvertBlToFactureResponse, ConvertDevisToBlRequest, ConvertDevisToBlResponse,
    CreateBackupRequestDto, CreateCategoryRequestDto, CreateDraftSaleRequest, CreateProductDto,
    CreatePurchaseOrderRequestDto, CreateSupplierRequestDto, CreateUserRequestDto,
    CreateWarehouseRequestDto, CreditOutstandingRowDto, CustomerDetailDto, CustomerSummaryDto,
    DailySalesSummaryDto, DecodeSummaryDto, DefineKitDto, DocumentDto, DocumentItemDto,
    DocumentSearchRequest, DocumentSearchResponse, DocumentSummaryDto,
    DocumentsProcessReturnRequestDto, DocumentsProcessReturnResponseDto, DraftSaleResponse,
    GetCustomerRequestDto, GetProductDto, GetSaleResponse, GetSessionReportRequestDto,
    GetStatementRequest, GetStatementResponse, GetStockLevelRequestDto, GetStockLevelResponseDto,
    GrossMarginRowDto, HealthDto, ImportProductsRequestDto, ImportProductsResponseDto,
    ImportRowErrorDto, KitItemDto, LineMarginDto, ListWarehousesResponseDto, LoginRequestDto,
    LoginResponseDto, LogoutRequestDto, LowStockRequestDto, LowStockRowDto, NewCustomerRequest,
    NewProductUnitDto, OpenCashSessionRequestDto, PaymentDto, PaymentLineDto, PreRestoreChecksDto,
    PreRestoreReportDto, PreRestoreRequestDto, PreviewLineDto, PreviewTotalsRequest,
    PreviewTotalsResponse, PrintStatusResponse, PrinterSettingsDto, ProcessReturnRequestDto,
    ProcessReturnResponseDto, ProductResponseDto, ProductUnitDto, PurchaseLineRequestDto,
    PurchaseOrderDto, PurchaseOrderLineDto, ReceiveGoodsRequestDto, RecordCreditPaymentRequest,
    RecordCreditPaymentResponse, RecordExpenseRequestDto, RecordSupplierInvoiceRequestDto,
    RecordSupplierPaymentRequestDto, RefundDestinationDto, ReportDateRangeRequestDto,
    ReprintRequest, ReprintResponse, ResetUserCredentialsRequestDto, RestoreBackupRequestDto,
    RestoreIneligibleReasonDto, RestoreOutcomeDto, RestoreWarningDto, ReturnContextResponseDto,
    ReturnableDocumentSummaryDto, ReturnableItemDto, ReturnableLineDto, SalesByCategoryRowDto,
    SalesByProductRowDto, SaveStoreIdentityRequest, SearchCustomersRequest,
    SearchCustomersResponse, SearchSuppliersRequestDto, SearchSuppliersResponseDto,
    StatementEntryDto, StockMovementHistoryRequestDto, StockMovementRowDto,
    StockShortageWarningDto, StoreIdentityDto, StoreIdentityResponse, SupplierDto,
    SupplierIdRequestDto, SupplierLedgerEntryDto, SupplierPaymentOutcomeDto, SupplierStatementDto,
    TestPrintRequest, TestPrintResponse, TransferStockRequestDto, TransferStockResponseDto,
    UpdateCategoryRequestDto, UpdateCustomerRequest, UpdatePrinterSettingsRequest,
    UpdateProductDto, UpdateProductUnitDto, UpdatePurchaseOrderRequestDto,
    UpdateSupplierRequestDto, UpdateUserRequestDto, UserActivityRequestDto, UserActivityRowDto,
    UserSummaryDto, VerifyBackupRequestDto, VerifyBackupResponseDto, WarehouseDto,
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
    /// Physical printer transport edge.
    pub printer_transport: Arc<dyn PrinterTransportPort>,
    /// Immutable test-print artifact sink.
    pub printer_capture: Arc<dyn CaptureSink>,
    /// Receipt raster renderer.
    pub printer_renderer: Arc<dyn ReceiptRendererPort>,
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
fn error_code(error: &ApplicationError) -> String {
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
fn stable_code_or(message: &str, fallback: &'static str) -> String {
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
fn adjust_error_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::InsufficientStock { .. } => "STOCK_NEGATIVE_NOT_AUTHORIZED".to_string(),
        other => error_code(other),
    }
}

/// Resolve and validate a session token for catalog commands.
fn require_session(state: &AppState, session_token: &str) -> Result<SessionContext, String> {
    let token = SessionToken(session_token.to_string());
    state
        .sessions
        .validate_session(&token)
        .ok_or_else(|| "UNAUTHORIZED".to_string())
}

/// Parse a canonical integer wire string (optional `-`, ASCII digits only).
fn parse_wire_int(text: &str, field: &'static str) -> Result<i64, ApplicationError> {
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
fn parse_wire_id(text: &str, field: &'static str) -> Result<Uuid, ApplicationError> {
    Uuid::parse_str(text.trim()).map_err(|_| {
        ApplicationError::Validation(DomainError::InvalidCanonicalString(format!(
            "{field} must be a UUID"
        )))
    })
}

fn parse_unit_dto(dto: &NewProductUnitDto) -> Result<NewUnitInput, ApplicationError> {
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
fn parse_customer_tier(
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
fn parse_cart_line_dto(dto: &CartLineItemDto) -> Result<CartLineInput, ApplicationError> {
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
fn parse_payment_dto(dto: &PaymentLineDto) -> Result<PaymentInput, ApplicationError> {
    Ok(PaymentInput {
        method: PaymentMethod::from_code(&dto.payment_method)
            .map_err(ApplicationError::Validation)?,
        amount_centimes: parse_wire_int(&dto.amount_centimes, "amount_centimes")?,
    })
}

/// Parse one update unit upsert (same field contract as creation, plus the
/// optional stable unit id; mirrors `parse_unit_dto` per the established
/// sales_checkout/sales_create_draft duplication precedent).
fn parse_upsert_unit_dto(dto: &UpdateProductUnitDto) -> Result<UnitUpsert, ApplicationError> {
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

fn to_unit_dto(unit: &baraka_domain::catalog::ProductUnit) -> ProductUnitDto {
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

fn to_response(detail: ProductDetail) -> ProductResponseDto {
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

/// `auth_login` command: username + password authentication.
#[tauri::command(rename_all = "snake_case")]
pub fn auth_login(
    state: tauri::State<'_, AppState>,
    request: LoginRequestDto,
) -> Result<LoginResponseDto, String> {
    let login = AuthenticateUserUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.sessions.as_ref(),
        state.clock.as_ref(),
    );
    match login.execute(&request.username, &request.password) {
        Ok(session) => Ok(LoginResponseDto {
            session_token: session.token.as_str().to_string(),
            user_id: session.user_id.to_string(),
            username: session.username,
            role: session.role.code().to_string(),
            expires_at: session.expires_at.to_rfc3339(),
        }),
        Err(error) => Err(error_code(&error)),
    }
}

/// `auth_logout` command: revoke a session (idempotent).
#[tauri::command(rename_all = "snake_case")]
pub fn auth_logout(
    state: tauri::State<'_, AppState>,
    request: LogoutRequestDto,
) -> Result<(), String> {
    let logout = LogoutUseCase::new(state.sessions.as_ref());
    let token = SessionToken(request.session_token);
    match logout.execute(&token) {
        Ok(()) => Ok(()),
        Err(error) => Err(error_code(&error)),
    }
}

/// `check_first_run_status` command: fast read-only first-run probe.
///
/// Unauthenticated by definition (no session exists yet on a fresh
/// database). Safe to call repeatedly; carries zero side effects.
#[tauri::command(rename_all = "snake_case")]
pub fn check_first_run_status(
    state: tauri::State<'_, AppState>,
) -> Result<CheckFirstRunStatusResponse, String> {
    match state.users.count_users() {
        Ok(count) => Ok(CheckFirstRunStatusResponse {
            requires_setup: count == 0,
        }),
        Err(error) => Err(error_code(&error)),
    }
}

/// `bootstrap_first_admin` command: one-time first Admin creation.
///
/// Unauthenticated by definition. The repository rejects the call with
/// `BOOTSTRAP_ALREADY_COMPLETED` once any user exists; the frontend then
/// logs in through the standard `auth_login` flow (no session token is
/// minted here). `origin_device_id` comes from TerminalContext-held state
/// (ADR-045), never from client input.
#[tauri::command(rename_all = "snake_case")]
pub fn bootstrap_first_admin(
    state: tauri::State<'_, AppState>,
    request: BootstrapFirstAdminRequest,
) -> Result<BootstrapFirstAdminResponse, String> {
    let bootstrap = BootstrapFirstAdminUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.clock.as_ref(),
    );
    match bootstrap.execute(
        &BootstrapFirstAdminInput {
            username: request.username,
            password: request.password,
        },
        &state.terminal_id,
    ) {
        Ok(output) => Ok(BootstrapFirstAdminResponse {
            user_id: output.user_id.to_string(),
            username: output.username,
        }),
        Err(error) => Err(error_code(&error)),
    }
}

/// `users_list` command: every managed operator (UserManage-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn users_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<UserSummaryDto>, String> {
    let session = require_session(&state, &session_token)?;
    let records = ListUsersUseCase::new(state.users.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(records.iter().map(to_user_summary).collect())
}

/// `users_create` command: validated, hashed operator creation (UserManage-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn users_create(
    state: tauri::State<'_, AppState>,
    request: CreateUserRequestDto,
) -> Result<UserSummaryDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let role = Role::from_code(request.role.trim())
        .map_err(|error| error_code(&ApplicationError::Validation(error)))?;
    let record = CreateUserUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.clock.as_ref(),
    )
    .execute(
        &session,
        &request.username,
        &request.password,
        request.pin.as_deref(),
        role,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_user_summary(&record))
}

/// `users_update` command: role/active-flag patch, last-admin protected.
#[tauri::command(rename_all = "snake_case")]
pub fn users_update(
    state: tauri::State<'_, AppState>,
    request: UpdateUserRequestDto,
) -> Result<UserSummaryDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let user_id = parse_wire_id(&request.user_id, "user_id")
        .map(baraka_domain::user::UserId::from)
        .map_err(|error| error_code(&error))?;
    let role = request
        .role
        .as_deref()
        .map(|code| {
            Role::from_code(code.trim())
                .map_err(|error| error_code(&ApplicationError::Validation(error)))
        })
        .transpose()?;
    let record = UpdateUserUseCase::new(state.users.as_ref(), state.clock.as_ref())
        .execute(&session, user_id, role, request.is_active)
        .map_err(|error| error_code(&error))?;
    Ok(to_user_summary(&record))
}

/// `users_reset_credentials` command: password/PIN replacement (UserManage-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn users_reset_credentials(
    state: tauri::State<'_, AppState>,
    request: ResetUserCredentialsRequestDto,
) -> Result<(), String> {
    let session = require_session(&state, &request.session_token)?;
    let user_id = parse_wire_id(&request.user_id, "user_id")
        .map(baraka_domain::user::UserId::from)
        .map_err(|error| error_code(&error))?;
    ResetUserCredentialsUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.clock.as_ref(),
    )
    .execute(
        &session,
        user_id,
        request.password.as_deref(),
        request.pin.as_deref(),
    )
    .map_err(|error| error_code(&error))
}

/// Map an application user record onto its wire summary (hashes never cross).
fn to_user_summary(record: &baraka_application::ports::UserRecord) -> UserSummaryDto {
    UserSummaryDto {
        id: record.id.to_string(),
        username: record.username.clone(),
        role: record.role.code().to_string(),
        is_active: record.is_active,
        has_pin: record.has_pin,
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    }
}

/// `catalog_create_product` command: atomic product + unit-set creation.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_create_product(
    state: tauri::State<'_, AppState>,
    request: CreateProductDto,
) -> Result<ProductResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut units = Vec::with_capacity(request.units.len());
    for unit in &request.units {
        units.push(parse_unit_dto(unit).map_err(|error| error_code(&error))?);
    }
    let category_id = request
        .category_id
        .as_deref()
        .map(|text| parse_wire_id(text, "category_id").map(CategoryId::from))
        .transpose()
        .map_err(|error| error_code(&error))?;
    let create = CreateProductUseCase::new(state.catalog.as_ref(), state.categories.as_ref());
    let id = create
        .execute(
            &session,
            CreateProductInput {
                sku: request.sku,
                barcode: request.barcode,
                name_ar: request.name_ar,
                name_fr: request.name_fr,
                category_id,
                is_kit: request.is_kit,
                units,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    let detail = GetProductWithUnitsUseCase::new(state.catalog.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    Ok(to_response(detail))
}

/// `catalog_update_product` command: full-state product update (SKU immutable).
///
/// Same RBAC contract as creation (Gerant/Admin via the use case); full-state
/// payload echoed back as the fresh detail view, mirroring `catalog_create_product`.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_update_product(
    state: tauri::State<'_, AppState>,
    request: UpdateProductDto,
) -> Result<ProductResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id =
        parse_wire_id(&request.product_id, "product_id").map_err(|error| error_code(&error))?;
    let mut units = Vec::with_capacity(request.units.len());
    for unit in &request.units {
        units.push(parse_upsert_unit_dto(unit).map_err(|error| error_code(&error))?);
    }
    let category_id = request
        .category_id
        .as_deref()
        .map(|text| parse_wire_id(text, "category_id").map(CategoryId::from))
        .transpose()
        .map_err(|error| error_code(&error))?;
    UpdateProductUseCase::new(state.catalog.as_ref(), state.categories.as_ref())
        .execute(
            &session,
            UpdateProductInput {
                id: ProductId::from(id),
                name_ar: request.name_ar,
                name_fr: request.name_fr,
                category_id,
                is_active: request.is_active,
                units,
            },
        )
        .map_err(|error| error_code(&error))?;
    let detail = GetProductWithUnitsUseCase::new(state.catalog.as_ref())
        .execute(&session, ProductId::from(id))
        .map_err(|error| error_code(&error))?;
    Ok(to_response(detail))
}

/// `catalog_get_product` command: full product detail view.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_get_product(
    state: tauri::State<'_, AppState>,
    request: GetProductDto,
) -> Result<ProductResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id =
        parse_wire_id(&request.product_id, "product_id").map_err(|error| error_code(&error))?;
    let detail = GetProductWithUnitsUseCase::new(state.catalog.as_ref())
        .execute(&session, ProductId::from(id))
        .map_err(|error| error_code(&error))?;
    Ok(to_response(detail))
}

/// `catalog_search` command: barcode-exact and multilingual prefix search.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_search(
    state: tauri::State<'_, AppState>,
    request: CatalogSearchRequestDto,
) -> Result<Vec<ProductResponseDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let limit = request.limit.unwrap_or(20) as usize;
    let results = SearchCatalogUseCase::new(state.catalog.as_ref())
        .execute(&session, &request.query, limit)
        .map_err(|error| error_code(&error))?;
    let mut out = Vec::with_capacity(results.len());
    for (product, units) in results {
        let kit_items = if product.is_kit {
            state
                .catalog
                .get_kit_items(product.id)
                .map_err(|error| error_code(&error))?
        } else {
            Vec::new()
        };
        out.push(to_response(ProductDetail {
            product,
            units,
            kit_items,
        }));
    }
    Ok(out)
}

/// `catalog_define_kit` command: replace a kit's component graph.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_define_kit(
    state: tauri::State<'_, AppState>,
    request: DefineKitDto,
) -> Result<(), String> {
    let session = require_session(&state, &request.session_token)?;
    let kit_id = parse_wire_id(&request.kit_id, "kit_id")
        .map(ProductId::from)
        .map_err(|error| error_code(&error))?;
    let mut items = Vec::with_capacity(request.items.len());
    for item in &request.items {
        items.push(NewKitItemInput {
            component_id: parse_wire_id(&item.component_id, "component_id")
                .map(ProductId::from)
                .map_err(|error| error_code(&error))?,
            unit_id: parse_wire_id(&item.unit_id, "unit_id")
                .map(UnitId::from)
                .map_err(|error| error_code(&error))?,
            quantity_micro: parse_wire_int(&item.quantity_micro, "quantity_micro")
                .map_err(|error| error_code(&error))?,
        });
    }
    DefineKitUseCase::new(state.catalog.as_ref())
        .execute(&session, kit_id, items)
        .map_err(|error| error_code(&error))
}

/// `catalog_import_products` command: atomic batch ingestion (Gerant/Admin).
///
/// Valid rows commit inside ONE `BEGIN IMMEDIATE` transaction; per-row
/// validation and duplicate failures are reported, never partial without
/// a report. `on_duplicate_sku: "error"` aborts the whole batch instead.
#[tauri::command(rename_all = "snake_case")]
pub fn catalog_import_products(
    state: tauri::State<'_, AppState>,
    request: ImportProductsRequestDto,
) -> Result<ImportProductsResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let on_duplicate = match request.on_duplicate_sku.as_deref() {
        Some("error") => ImportDuplicatePolicy::Error,
        _ => ImportDuplicatePolicy::Skip,
    };
    let rows = request
        .rows
        .into_iter()
        .map(|row| ImportProductRow {
            sku: row.sku,
            barcode: row.barcode,
            name_fr: row.name_fr,
            name_ar: row.name_ar,
            category_id: row.category_id,
            base_unit_name_fr: row.base_unit_name_fr,
            base_unit_name_ar: row.base_unit_name_ar,
            purchase_price_centimes: row.purchase_price_centimes,
            sale_price_centimes: row.sale_price_centimes,
            initial_quantity_micro: row.initial_quantity_micro,
        })
        .collect();
    let report = ImportProductsUseCase::new(state.imports.as_ref())
        .execute(
            &session,
            ImportBatchInput {
                rows,
                warehouse_code: request.warehouse_id,
                on_duplicate,
                origin_device_id: state.terminal_id.clone(),
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(ImportProductsResponseDto {
        total_rows: report.total_rows,
        imported_count: report.imported_count,
        skipped_count: report.skipped_count,
        errors: report
            .errors
            .into_iter()
            .map(|error| ImportRowErrorDto {
                row_index: error.row_index,
                sku: error.sku,
                name_fr: error.name_fr,
                error: error.error,
            })
            .collect(),
        duration_ms: report.duration_ms,
    })
}

/// Map a domain category to its wire view.
fn to_category_dto(category: &Category) -> CategoryResponseDto {
    CategoryResponseDto {
        id: category.id.to_string(),
        code: category.code.clone(),
        name_fr: category.name_fr.clone(),
        name_ar: category.name_ar.clone(),
        parent_id: category.parent_id.map(|id| id.to_string()),
        is_active: category.is_active,
    }
}

/// Parse an optional UUID wire id (`None` stays detached).
fn parse_optional_wire_id(
    text: Option<&str>,
    field: &'static str,
) -> Result<Option<CategoryId>, ApplicationError> {
    text.map(|value| parse_wire_id(value, field).map(CategoryId::from))
        .transpose()
}

/// `category_list` command: category taxonomy (all roles; inactive hidden
/// unless `include_inactive` is set).
#[tauri::command(rename_all = "snake_case")]
pub fn category_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
    include_inactive: Option<bool>,
) -> Result<Vec<CategoryResponseDto>, String> {
    let session = require_session(&state, &session_token)?;
    let categories = ListCategoriesUseCase::new(state.categories.as_ref())
        .execute(&session, include_inactive.unwrap_or(false))
        .map_err(|error| error_code(&error))?;
    Ok(categories.iter().map(to_category_dto).collect())
}

/// `category_create` command: unique-code category creation (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn category_create(
    state: tauri::State<'_, AppState>,
    request: CreateCategoryRequestDto,
) -> Result<CategoryResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let parent_id = parse_optional_wire_id(request.parent_id.as_deref(), "parent_id")
        .map_err(|error| error_code(&error))?;
    let category = CreateCategoryUseCase::new(state.categories.as_ref())
        .execute(
            &session,
            CreateCategoryInput {
                code: request.code,
                name_fr: request.name_fr,
                name_ar: request.name_ar,
                parent_id,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_category_dto(&category))
}

/// `category_update` command: mutable-field category update (Gerant/Admin;
/// code stays immutable).
#[tauri::command(rename_all = "snake_case")]
pub fn category_update(
    state: tauri::State<'_, AppState>,
    request: UpdateCategoryRequestDto,
) -> Result<CategoryResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.category_id, "category_id")
        .map(CategoryId::from)
        .map_err(|error| error_code(&error))?;
    let parent_id = parse_optional_wire_id(request.parent_id.as_deref(), "parent_id")
        .map_err(|error| error_code(&error))?;
    let category = UpdateCategoryUseCase::new(state.categories.as_ref())
        .execute(
            &session,
            UpdateCategoryInput {
                id,
                name_fr: request.name_fr,
                name_ar: request.name_ar,
                parent_id,
                is_active: request.is_active,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_category_dto(&category))
}

/// `category_seed_starter` command: idempotent Algerian hardware starter
/// taxonomy (Gerant/Admin; no-op returning existing rows when non-empty).
#[tauri::command(rename_all = "snake_case")]
pub fn category_seed_starter(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<CategoryResponseDto>, String> {
    let session = require_session(&state, &session_token)?;
    let categories = SeedStarterCategoriesUseCase::new(state.categories.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(categories.iter().map(to_category_dto).collect())
}

/// `warehouses_create` command (`warehouses.create`): ADMIN-only creation.
#[tauri::command(rename_all = "snake_case")]
pub fn warehouses_create(
    state: tauri::State<'_, AppState>,
    request: CreateWarehouseRequestDto,
) -> Result<WarehouseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = CreateWarehouseUseCase::new(state.warehouses.as_ref())
        .execute(&session, &request.code, &request.name, &state.terminal_id)
        .map_err(|error| error_code(&error))?;
    let warehouse = state
        .warehouses
        .get_by_id(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| "INTERNAL_ERROR".to_string())?;
    Ok(WarehouseDto {
        id: warehouse.id.to_string(),
        code: warehouse.code.as_str().to_string(),
        name: warehouse.name.clone(),
        is_active: warehouse.is_active,
    })
}

/// `warehouses_list` command (`warehouses.list`): any authenticated session.
#[tauri::command(rename_all = "snake_case")]
pub fn warehouses_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<ListWarehousesResponseDto, String> {
    let session = require_session(&state, &session_token)?;
    let warehouses = ListWarehousesUseCase::new(state.warehouses.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(ListWarehousesResponseDto {
        warehouses: warehouses
            .into_iter()
            .map(|warehouse| WarehouseDto {
                id: warehouse.id.to_string(),
                code: warehouse.code.as_str().to_string(),
                name: warehouse.name.clone(),
                is_active: warehouse.is_active,
            })
            .collect(),
    })
}

/// `stock_get` command (`stock.get`): level read, 0 when absent.
#[tauri::command(rename_all = "snake_case")]
pub fn stock_get(
    state: tauri::State<'_, AppState>,
    request: GetStockLevelRequestDto,
) -> Result<GetStockLevelResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let product_id = parse_wire_id(&request.product_id, "product_id")
        .map(ProductId::from)
        .map_err(|error| error_code(&error))?;
    let warehouse_id = parse_wire_id(&request.warehouse_id, "warehouse_id")
        .map(WarehouseId::from)
        .map_err(|error| error_code(&error))?;
    let base_unit_id = state
        .catalog
        .find_by_id(product_id)
        .map_err(|error| error_code(&error))?
        .and_then(|(_, units)| {
            units
                .iter()
                .find(|unit| unit.is_base_unit)
                .map(|unit| unit.id.to_string())
        })
        .unwrap_or_default();
    let quantity = GetStockLevelUseCase::new(state.stock.as_ref())
        .execute(&session, product_id, warehouse_id)
        .map_err(|error| error_code(&error))?;
    Ok(GetStockLevelResponseDto {
        product_id: product_id.to_string(),
        warehouse_id: warehouse_id.to_string(),
        base_unit_id,
        quantity_base_micro: quantity.to_string(),
    })
}

/// Parse an explicit adjustment direction.
fn parse_direction(text: &str) -> Result<StockDirection, ApplicationError> {
    match text.trim() {
        "INCREASE" => Ok(StockDirection::Increase),
        "DECREASE" => Ok(StockDirection::Decrease),
        _ => Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString(
                "direction must be INCREASE or DECREASE".to_string(),
            ),
        )),
    }
}

/// `stock_adjust` command (`stock.adjust`): RBAC-gated manual correction.
#[tauri::command(rename_all = "snake_case")]
pub fn stock_adjust(
    state: tauri::State<'_, AppState>,
    request: AdjustStockRequestDto,
) -> Result<AdjustStockResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let outcome = AdjustStockUseCase::new(state.stock.as_ref())
        .execute(
            &session,
            AdjustStockInput {
                product_id: parse_wire_id(&request.product_id, "product_id")
                    .map(ProductId::from)
                    .map_err(|error| error_code(&error))?,
                warehouse_id: parse_wire_id(&request.warehouse_id, "warehouse_id")
                    .map(WarehouseId::from)
                    .map_err(|error| error_code(&error))?,
                unit_id: parse_wire_id(&request.unit_id, "unit_id")
                    .map(UnitId::from)
                    .map_err(|error| error_code(&error))?,
                quantity_micro: parse_wire_int(&request.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
                direction: parse_direction(&request.direction)
                    .map_err(|error| error_code(&error))?,
                reason: request.reason,
                authorize_negative_stock: request.authorize_negative_stock,
            },
        )
        .map_err(|error| adjust_error_code(&error))?;
    Ok(AdjustStockResponseDto {
        movement_id: outcome.movement_id,
        new_quantity_base_micro: outcome.new_quantity_base_micro.to_string(),
    })
}

/// `stock_transfer` command (`stock.transfer`): atomic OUT/IN pair.
#[tauri::command(rename_all = "snake_case")]
pub fn stock_transfer(
    state: tauri::State<'_, AppState>,
    request: TransferStockRequestDto,
) -> Result<TransferStockResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let result = TransferStockUseCase::new(state.stock.as_ref(), state.warehouses.as_ref())
        .execute(
            &session,
            TransferStockInput {
                source_warehouse_id: parse_wire_id(
                    &request.source_warehouse_id,
                    "source_warehouse_id",
                )
                .map(WarehouseId::from)
                .map_err(|error| error_code(&error))?,
                destination_warehouse_id: parse_wire_id(
                    &request.destination_warehouse_id,
                    "destination_warehouse_id",
                )
                .map(WarehouseId::from)
                .map_err(|error| error_code(&error))?,
                product_id: parse_wire_id(&request.product_id, "product_id")
                    .map(ProductId::from)
                    .map_err(|error| error_code(&error))?,
                unit_id: parse_wire_id(&request.unit_id, "unit_id")
                    .map(UnitId::from)
                    .map_err(|error| error_code(&error))?,
                quantity_micro: parse_wire_int(&request.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
                reason: request.reason,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(TransferStockResponseDto {
        transfer_batch_id: result.transfer_batch_id,
        out_movement_id: result.out_movement_id,
        in_movement_id: result.in_movement_id,
        source_new_quantity_base_micro: result.source_new_quantity_base_micro.to_string(),
        destination_new_quantity_base_micro: result.destination_new_quantity_base_micro.to_string(),
    })
}

/// Parse an explicit sale-time language (`AR` or `FR` exactly).
///
/// Missing, blank, or foreign values are hard validation failures, never
/// silent fallbacks (Section 17: the language is captured, not assumed).
fn parse_document_language(text: &str) -> Result<DocumentLanguage, ApplicationError> {
    text.parse::<DocumentLanguage>()
        .map_err(|_| ApplicationError::Validation(DomainError::InvalidDocumentLanguage))
}

/// `sales_checkout` command (`sales.checkout`): atomic sale commit.
#[tauri::command(rename_all = "snake_case")]
pub fn sales_checkout(
    state: tauri::State<'_, AppState>,
    request: CheckoutSaleRequest,
) -> Result<CheckoutSaleResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut lines = Vec::with_capacity(request.lines.len());
    for line in &request.lines {
        lines.push(CartLineInput {
            product_id: parse_wire_id(&line.product_id, "product_id")
                .map(ProductId::from)
                .map_err(|error| error_code(&error))?,
            unit_id: parse_wire_id(&line.unit_id, "unit_id")
                .map(UnitId::from)
                .map_err(|error| error_code(&error))?,
            quantity_micro: parse_wire_int(&line.quantity_micro, "quantity_micro")
                .map_err(|error| error_code(&error))?,
            unit_price_override: line
                .unit_price_centimes
                .as_deref()
                .map(|text| parse_wire_int(text, "unit_price_centimes"))
                .transpose()
                .map_err(|error| error_code(&error))?,
            line_discount_centimes: parse_wire_int(
                &line.line_discount_centimes,
                "line_discount_centimes",
            )
            .map_err(|error| error_code(&error))?,
        });
    }
    let mut payments = Vec::with_capacity(request.payments.len());
    for payment in &request.payments {
        payments.push(PaymentInput {
            method: PaymentMethod::from_code(&payment.payment_method)
                .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
            amount_centimes: parse_wire_int(&payment.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
        });
    }
    let input = CheckoutInput {
        command_id: request.command_id,
        warehouse_id: parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        customer_id: request
            .customer_id
            .as_deref()
            .map(|text| {
                parse_wire_id(text, "customer_id").map(baraka_domain::sales::CustomerId::from)
            })
            .transpose()
            .map_err(|error| error_code(&error))?,
        doc_type: DocumentType::from_code(&request.document_type)
            .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
        language: parse_document_language(&request.document_language)
            .map_err(|error| error_code(&error))?,
        lines,
        global_discount_centimes: parse_wire_int(
            &request.global_discount_centimes,
            "global_discount_centimes",
        )
        .map_err(|error| error_code(&error))?,
        payments,
        allow_negative_stock: request.allow_negative_stock,
        notes: request.notes,
    };
    let receipt = CheckoutSaleUseCase::new(
        state.catalog.as_ref(),
        state.customers.as_ref(),
        state.checkout.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(&session, input, &state.terminal_id)
    .map_err(|error| error_code(&error))?;
    Ok(CheckoutSaleResponse {
        document_id: receipt.document_id,
        document_number: receipt.document_number,
        subtotal_ht_centimes: receipt.subtotal_ht_centimes,
        tva_total_centimes: receipt.tva_total_centimes,
        discount_total_centimes: receipt.discount_total_centimes,
        timbre_centimes: receipt.timbre_centimes,
        total_ttc_centimes: receipt.total_ttc_centimes,
        paid_amount_centimes: receipt.paid_amount_centimes,
        change_amount_centimes: receipt.change_amount_centimes,
        print_job_id: receipt.print_job_id,
    })
}

/// Map one application shortage warning to its wire DTO.
fn to_shortage_dto(warning: &StockShortageWarning) -> StockShortageWarningDto {
    StockShortageWarningDto {
        product_id: warning.product_id.to_string(),
        product_name: warning.product_name.clone(),
        sku: warning.sku.clone(),
        available_qty_micro: warning.available_qty_micro.to_string(),
        requested_qty_micro: warning.requested_qty_micro.to_string(),
    }
}

/// `sales_preview_totals` command (`sales.preview_totals`): read-only,
/// commit-identical totals preview (TASK-BRK-031).
///
/// Prices the current cart through the exact shared path the atomic
/// `sales_checkout` commit travels (`verify_and_price_cart` → domain
/// `price_document`) with zero persistence, sequence, stock, ledger, or
/// print side effects. The checkout modal renders ONLY these values; it
/// never recomputes totals client-side, so the figures the cashier
/// confirms before payment are byte-for-byte the figures persisted.
///
/// TASK-BRK-033 additionally surfaces leaf-level stock shortages against
/// the requested warehouse (advisory: the atomic engine re-checks under
/// its own transaction with role-gated enforcement).
#[tauri::command(rename_all = "snake_case")]
pub fn sales_preview_totals(
    state: tauri::State<'_, AppState>,
    request: PreviewTotalsRequest,
) -> Result<PreviewTotalsResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut lines = Vec::with_capacity(request.lines.len());
    for line in &request.lines {
        lines.push(parse_cart_line_dto(line).map_err(|error| error_code(&error))?);
    }
    let mut payments = Vec::with_capacity(request.payments.len());
    for payment in &request.payments {
        payments.push(parse_payment_dto(payment).map_err(|error| error_code(&error))?);
    }
    let output = PreviewSaleTotalsUseCase::new(
        state.catalog.as_ref(),
        state.stock.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        PreviewTotalsInput {
            warehouse_id: parse_wire_id(&request.warehouse_id, "warehouse_id")
                .map(WarehouseId::from)
                .map_err(|error| error_code(&error))?,
            doc_type: DocumentType::from_code(&request.document_type)
                .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
            lines,
            global_discount_centimes: parse_wire_int(
                &request.global_discount_centimes,
                "global_discount_centimes",
            )
            .map_err(|error| error_code(&error))?,
            payments,
        },
    )
    .map_err(|error| error_code(&error))?;
    Ok(PreviewTotalsResponse {
        subtotal_ht_centimes: output.subtotal_ht.to_string(),
        tva_total_centimes: output.total_tva.to_string(),
        global_discount_centimes: output.global_discount.to_string(),
        timbre_centimes: output.timbre.to_string(),
        total_ttc_centimes: output.total_ttc.to_string(),
        paid_amount_centimes: output.paid_total.to_string(),
        change_amount_centimes: output.change.to_string(),
        remaining_due_centimes: output.remaining_due.to_string(),
        stock_warnings: output.stock_warnings.iter().map(to_shortage_dto).collect(),
        line_margins: output
            .line_margins
            .iter()
            .map(|margin| LineMarginDto {
                line_index: margin.line_index,
                margin_basis_points: margin.margin_basis_points,
                is_below_cost: margin.is_below_cost,
            })
            .collect(),
    })
}

/// `convert_devis_to_bl` command: single-shot DEVIS-to-BL conversion
/// (ADR-040, TASK-BRK-014A). Copies the locked quote snapshot into a new
/// FINALIZED BL, decrements stock, and seals the source as CONVERTED —
/// all atomically. No recalculation, no partial conversion.
#[tauri::command(rename_all = "snake_case")]
pub fn convert_devis_to_bl(
    state: tauri::State<'_, AppState>,
    request: ConvertDevisToBlRequest,
) -> Result<ConvertDevisToBlResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let result = ConvertDevisToBlUseCase::new(state.catalog.as_ref(), state.documents.as_ref())
        .execute(
            &session,
            ConvertDevisToBlInput {
                command_id: request.command_id,
                source_document_id: source_id,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(ConvertDevisToBlResponse {
        new_document_id: result.new_document_id,
        new_document_number: result.new_document_number,
        new_document_status: result.new_document_status,
        total_ttc_centimes: result.total_ttc_centimes.to_string(),
        items_count: result.items_count,
        warnings: result.warnings,
    })
}

/// `convert_bl_to_facture` command: single-shot BL-to-FACTURE financial
/// closing (ADR-040/041/046, TASK-BRK-038). Settles the locked BL snapshot
/// into a new FINALIZED FACTURE with exact tender settlement (no change
/// concept), posts the credit ledger leg when CREDIT tenders are present,
/// and seals the source as CONVERTED — all atomically. Zero stock writes
/// by invariant (physical stock left at BL finalization). Mirrors the
/// `convert_devis_to_bl` edge conventions: session extraction, wire-int
/// parsing, `command_id` idempotency threading, and `error_code` mapping.
#[tauri::command(rename_all = "snake_case")]
pub fn convert_bl_to_facture(
    state: tauri::State<'_, AppState>,
    request: ConvertBlToFactureRequest,
) -> Result<ConvertBlToFactureResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let mut payments = Vec::with_capacity(request.payments.len());
    for payment in &request.payments {
        payments.push(ClosingPaymentInput {
            method: PaymentMethod::from_code(&payment.payment_method)
                .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
            amount_centimes: parse_wire_int(&payment.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
        });
    }
    let result = ConvertBlToFactureUseCase::new(state.customers.as_ref(), state.documents.as_ref())
        .execute(
            &session,
            ClosingInput {
                command_id: request.command_id,
                source_document_id: source_id,
                language: parse_document_language(&request.document_language)
                    .map_err(|error| error_code(&error))?,
                payments,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(ConvertBlToFactureResponse {
        new_document_id: result.new_document_id,
        new_document_number: result.new_document_number,
        new_document_status: result.new_document_status,
        total_ttc_centimes: result.total_ttc_centimes.to_string(),
        items_count: result.items_count,
        customer_new_balance_centimes: result
            .customer_new_balance_centimes
            .map(|balance| balance.to_string()),
    })
}

/// `documents_search` command: read-only document lookup for conversion
/// candidate flows (TASK-BRK-042). Filters by kind/status with partial
/// human-number or customer-name match, newest first, limit clamped
/// to 1..=100. Any operator who may sell or report may search.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_search(
    state: tauri::State<'_, AppState>,
    request: DocumentSearchRequest,
) -> Result<DocumentSearchResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let summaries = SearchDocumentsUseCase::new(state.documents.as_ref())
        .execute(
            &session,
            DocumentSearchFilter {
                doc_type: request.document_type,
                status: request.status,
                query: request.query,
                limit: request.limit,
            },
        )
        .map_err(|error| error_code(&error))?;
    let mut documents = Vec::with_capacity(summaries.len());
    for summary in summaries {
        documents.push(DocumentSummaryDto {
            id: summary.id,
            document_number: summary.number,
            document_type: summary.doc_type,
            status: summary.status,
            party_id: summary.party_id,
            party_name: summary.party_name,
            total_ttc_centimes: summary.ttc_centimes.to_string(),
            items_count: u32::try_from(summary.items_count).map_err(|_| {
                error_code(&ApplicationError::Port("line count overflow".to_string()))
            })?,
            created_at: summary.created_at,
        });
    }
    Ok(DocumentSearchResponse { documents })
}

/// `returns_get_source_summary` command: returnable state of one source
/// document (TASK-BRK-047). Read-only: original quantities, cumulative
/// prior returns, remaining returnable amounts, and converted-payment
/// eligibility — everything the return modal needs before dispatch.
#[tauri::command(rename_all = "snake_case")]
pub fn returns_get_source_summary(
    state: tauri::State<'_, AppState>,
    session_token: String,
    source_document_id: String,
) -> Result<ReturnableDocumentSummaryDto, String> {
    require_session(&state, &session_token)?;
    let id = parse_wire_id(&source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let document = state
        .documents
        .get_by_id(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| error_code(&ApplicationError::NotFound("document".to_string())))?;
    let items = state
        .documents
        .get_items(id)
        .map_err(|error| error_code(&error))?;
    let cumulative = state
        .returns
        .cumulative_returned_quantities(id)
        .map_err(|error| error_code(&error))?;
    let party_name = match document.party_id.as_deref() {
        Some(party) => parse_wire_id(party, "party_id")
            .map(CustomerId::from)
            .ok()
            .and_then(|customer_id| {
                state
                    .customers
                    .get_customer(customer_id)
                    .map_err(|error| error_code(&error))
                    .transpose()
            })
            .transpose()?
            .map(|customer| customer.name),
        None => None,
    };
    let mut lines = Vec::with_capacity(items.len());
    let mut any_remaining = false;
    for item in &items {
        let already = cumulative.get(&item.id.to_string()).copied().unwrap_or(0);
        let remaining = (item.quantity_micro - already).max(0);
        if remaining > 0 {
            any_remaining = true;
        }
        let (product_name, sku, unit_name) = match state
            .catalog
            .find_by_id(item.product_id)
            .map_err(|error| error_code(&error))?
        {
            Some((product, units)) => {
                let unit_label = units
                    .iter()
                    .find(|unit| unit.id == item.unit_id)
                    .map(|unit| unit.unit_name_fr.clone())
                    .unwrap_or_else(|| item.unit_id.to_string());
                (
                    product.name_fr.clone(),
                    product.sku.as_str().to_string(),
                    unit_label,
                )
            }
            None => (
                item.product_id.to_string(),
                String::new(),
                item.unit_id.to_string(),
            ),
        };
        // Display-aid net unit price (truncated; the engine recomputes
        // exact pro-rated lines at execution).
        let net_unit_price = if item.quantity_micro == 0 {
            0
        } else {
            (i128::from(item.line_ht_centimes) * 10_000 / i128::from(item.quantity_micro))
                .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
        };
        lines.push(ReturnableLineDto {
            source_item_id: item.id.to_string(),
            product_id: item.product_id.to_string(),
            product_name,
            sku,
            unit_id: item.unit_id.to_string(),
            unit_name,
            original_qty_micro: item.quantity_micro.to_string(),
            already_returned_qty_micro: already.to_string(),
            remaining_qty_micro: remaining.to_string(),
            unit_price_cents: item.unit_price_centimes.to_string(),
            net_unit_price_cents: net_unit_price.to_string(),
            tva_rate: item.tva_rate_micro.to_string(),
        });
    }
    let eligible = document.status == baraka_domain::sales::DocumentStatus::Finalized
        && matches!(
            document.doc_type,
            DocumentType::Ticket | DocumentType::Facture | DocumentType::Bl
        )
        && any_remaining;
    Ok(ReturnableDocumentSummaryDto {
        source_document_id: document.id.to_string(),
        source_document_number: document.number.clone(),
        document_type: document.doc_type.code().to_string(),
        party_type: document.party_id.as_ref().map(|_| "CUSTOMER".to_string()),
        party_id: document.party_id.clone(),
        party_name,
        warehouse_id: document.warehouse_id.to_string(),
        total_ht_cents: document.subtotal_ht_centimes.to_string(),
        total_ttc_cents: document.ttc_centimes.to_string(),
        is_eligible_for_return: eligible,
        items: lines,
    })
}

/// `returns_process` command: validate and atomically persist a merchandise
/// return (TASK-BRK-047). Terminal affinity resolves from TerminalContext
/// (`state.terminal_id`, never client routing); the DTO `terminal_id` is
/// accepted for wire compatibility and must be non-blank.
#[tauri::command(rename_all = "snake_case")]
pub fn returns_process(
    state: tauri::State<'_, AppState>,
    request: ProcessReturnRequestDto,
) -> Result<ProcessReturnResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    if request.terminal_id.trim().is_empty() {
        return Err(error_code(&ApplicationError::Validation(
            DomainError::InvalidCanonicalString("INVALID_TERMINAL_ID".to_string()),
        )));
    }
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let source_number = state
        .documents
        .get_by_id(source_id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| error_code(&ApplicationError::NotFound("document".to_string())))?
        .number
        .clone();
    let mut items = Vec::with_capacity(request.items.len());
    for line in &request.items {
        items.push(ReturnItemInput {
            source_item_id: parse_wire_id(&line.source_item_id, "source_item_id")
                .map_err(|error| error_code(&error))?,
            quantity_micro: QtyMicro::from_micros(
                parse_wire_int(&line.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
            ),
            reason: line.reason.clone(),
        });
    }
    let refund = match &request.refund {
        RefundDestinationDto::Cash { cash_session_id } => RefundDestination::Cash {
            cash_session_id: cash_session_id.clone(),
        },
        RefundDestinationDto::CustomerCredit { customer_id } => RefundDestination::CustomerCredit {
            customer_id: parse_wire_id(customer_id, "customer_id")
                .map(CustomerId::from)
                .map_err(|error| error_code(&error))?,
        },
        RefundDestinationDto::NoRefund => RefundDestination::NoRefund,
    };
    let refund_type = match &request.refund {
        RefundDestinationDto::Cash { .. } => "CASH",
        RefundDestinationDto::CustomerCredit { .. } => "CUSTOMER_CREDIT",
        RefundDestinationDto::NoRefund => "NO_REFUND",
    }
    .to_string();
    let result = ProcessReturnUseCase::new(
        state.documents.as_ref(),
        state.returns.as_ref(),
        state.customers.as_ref(),
        state.cash.as_ref(),
    )
    .execute(
        &session,
        ProcessReturnCommand {
            operator_id: parse_wire_id(&request.operator_id, "operator_id")
                .map(UserId::from)
                .map_err(|error| error_code(&error))?,
            source_document_id: source_id,
            items,
            refund,
            idempotency_key: request.idempotency_key,
            terminal_id: state.terminal_id.clone(),
            notes: request.notes,
        },
    )
    .map_err(|error| error_code(&error))?;
    Ok(ProcessReturnResponseDto {
        return_document_id: result.return_document_id.to_string(),
        return_document_number: result.return_document_number,
        source_document_id: source_id.to_string(),
        source_document_number: source_number,
        total_ht_cents: result.total_refund_ht.centimes().to_string(),
        total_tva_cents: result.total_refund_tva.centimes().to_string(),
        total_ttc_cents: result.total_refund_ttc.centimes().to_string(),
        refund_type,
        lines_count: result.items_returned_count,
    })
}

/// Wire-code translator for the simplified return facade (TASK-BRK-046
/// re-issue): the use case emits conversion-era stable codes, while this
/// facade contracts the dedicated return taxonomy.
fn return_wire_code(error: &ApplicationError) -> String {
    match error_code(error).as_str() {
        "INVALID_CONVERSION_SOURCE_STATUS" | "INVALID_CONVERSION_SOURCE_TYPE" => {
            "INVALID_SOURCE_DOCUMENT_STATE".to_string()
        }
        "NO_OPEN_CASH_SESSION_FOR_TERMINAL" => "NO_ACTIVE_CASH_SESSION".to_string(),
        "CREDIT_CUSTOMER_REQUIRED" => "MISSING_CUSTOMER_FOR_CREDIT_REFUND".to_string(),
        "EMPTY_CART" => "EMPTY_RETURN".to_string(),
        _ => error_code(error),
    }
}

/// `documents_get_return_context` command: returnable state of one source
/// document (TASK-BRK-046 re-issue). Read-only facade over the same reads
/// as `returns_get_source_summary`, shaped for the return workstation.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_get_return_context(
    state: tauri::State<'_, AppState>,
    session_token: String,
    source_document_id: String,
) -> Result<ReturnContextResponseDto, String> {
    require_session(&state, &session_token)?;
    let id = parse_wire_id(&source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let document = state
        .documents
        .get_by_id(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| error_code(&ApplicationError::NotFound("document".to_string())))?;
    let items = state
        .documents
        .get_items(id)
        .map_err(|error| error_code(&error))?;
    let cumulative = state
        .returns
        .cumulative_returned_quantities(id)
        .map_err(|error| error_code(&error))?;
    let party_name = match document.party_id.as_deref() {
        Some(party) => parse_wire_id(party, "party_id")
            .map(CustomerId::from)
            .ok()
            .and_then(|customer_id| {
                state
                    .customers
                    .get_customer(customer_id)
                    .map_err(|error| error_code(&error))
                    .transpose()
            })
            .transpose()?
            .map(|customer| customer.name),
        None => None,
    };
    let kind_returnable = matches!(
        document.doc_type,
        DocumentType::Ticket | DocumentType::Facture | DocumentType::Bl
    );
    let mut lines = Vec::with_capacity(items.len());
    let mut any_remaining = false;
    for item in &items {
        let already = cumulative.get(&item.id.to_string()).copied().unwrap_or(0);
        let remaining = (item.quantity_micro - already).max(0);
        if remaining > 0 {
            any_remaining = true;
        }
        let (product_name, barcode, unit_name) = match state
            .catalog
            .find_by_id(item.product_id)
            .map_err(|error| error_code(&error))?
        {
            Some((product, units)) => {
                let unit = units.iter().find(|unit| unit.id == item.unit_id);
                (
                    product.name_fr.clone(),
                    unit.and_then(|unit| {
                        unit.barcode.as_ref().map(|code| code.as_str().to_string())
                    }),
                    unit.map(|unit| unit.unit_name_fr.clone())
                        .unwrap_or_else(|| item.unit_id.to_string()),
                )
            }
            None => (item.product_id.to_string(), None, item.unit_id.to_string()),
        };
        lines.push(ReturnableItemDto {
            source_item_id: item.id.to_string(),
            product_id: item.product_id.to_string(),
            product_name,
            barcode,
            unit_id: item.unit_id.to_string(),
            unit_name,
            unit_price_centimes: item.unit_price_centimes.to_string(),
            original_quantity_micro: item.quantity_micro.to_string(),
            already_returned_micro: already.to_string(),
            remaining_allowable_micro: remaining.to_string(),
            tva_rate: u32::try_from(item.tva_rate_micro).unwrap_or(0),
            discount_centimes: item.discount_centimes.to_string(),
        });
    }
    let (is_eligible, ineligibility_reason) =
        if document.status != baraka_domain::sales::DocumentStatus::Finalized {
            (false, Some("NOT_FINALIZED".to_string()))
        } else if !kind_returnable {
            (false, Some("NON_RETURNABLE_TYPE".to_string()))
        } else if !any_remaining {
            (false, Some("FULLY_RETURNED".to_string()))
        } else {
            (true, None)
        };
    Ok(ReturnContextResponseDto {
        source_document_id: document.id.to_string(),
        document_number: document.number.clone(),
        document_type: document.doc_type.code().to_string(),
        party_id: document.party_id.clone(),
        party_name,
        warehouse_id: document.warehouse_id.to_string(),
        created_at: document
            .created_at
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        is_eligible,
        ineligibility_reason,
        items: lines,
    })
}

/// `documents_process_return` command: simplified single-shot merchandise
/// return (TASK-BRK-046 re-issue). Settlement travels as a plain method
/// code; the operator resolves from the session, the terminal from
/// TerminalContext, and a fresh idempotency key is minted per call —
/// callers needing retry-safety use `returns_process` with an explicit
/// key. Error codes follow the dedicated return taxonomy.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_process_return(
    state: tauri::State<'_, AppState>,
    request: DocumentsProcessReturnRequestDto,
) -> Result<DocumentsProcessReturnResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let mut items = Vec::with_capacity(request.items.len());
    for line in &request.items {
        items.push(ReturnItemInput {
            source_item_id: parse_wire_id(&line.source_item_id, "source_item_id")
                .map_err(|error| error_code(&error))?,
            quantity_micro: QtyMicro::from_micros(
                parse_wire_int(&line.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
            ),
            reason: None,
        });
    }
    let method = request.refund_method.trim().to_uppercase();
    let refund = match method.as_str() {
        "CASH" => {
            let active = state
                .cash
                .get_active_session(&state.terminal_id)
                .map_err(|error| error_code(&error))?
                .filter(|view| view.status == "OPEN")
                .ok_or_else(|| "NO_ACTIVE_CASH_SESSION".to_string())?;
            RefundDestination::Cash {
                cash_session_id: active.id,
            }
        }
        "CUSTOMER_CREDIT" => {
            let party = state
                .documents
                .get_by_id(source_id)
                .map_err(|error| error_code(&error))?
                .and_then(|document| document.party_id.clone())
                .ok_or_else(|| "MISSING_CUSTOMER_FOR_CREDIT_REFUND".to_string())?;
            RefundDestination::CustomerCredit {
                customer_id: parse_wire_id(&party, "party_id")
                    .map(CustomerId::from)
                    .map_err(|error| error_code(&error))?,
            }
        }
        "NO_REFUND" => RefundDestination::NoRefund,
        _ => {
            return Err(error_code(&ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_REFUND_METHOD".to_string()),
            )));
        }
    };
    let result = ProcessSaleReturnUseCase::new(
        state.documents.as_ref(),
        state.returns.as_ref(),
        state.customers.as_ref(),
        state.cash.as_ref(),
    )
    .execute(
        &session,
        ProcessReturnCommand {
            operator_id: session.user_id,
            source_document_id: source_id,
            items,
            refund,
            idempotency_key: Uuid::new_v4().to_string(),
            terminal_id: state.terminal_id.clone(),
            notes: request.notes,
        },
    )
    .map_err(|error| return_wire_code(&error))?;
    Ok(DocumentsProcessReturnResponseDto {
        return_document_id: result.return_document_id.to_string(),
        return_document_number: result.return_document_number,
        total_refund_ttc_centimes: result.total_refund_ttc.centimes().to_string(),
        refund_method: method,
    })
}

/// Require the system-settings grant (backup commands are Admin-only).
fn require_backup_admin(session: &SessionContext) -> Result<(), String> {
    if session
        .role
        .has_permission(Permission::SystemSettingsManage)
    {
        Ok(())
    } else {
        Err(error_code(&ApplicationError::Unauthorized))
    }
}

/// Map domain backup metadata to its wire view.
fn to_backup_dto(meta: &baraka_domain::backup::BackupMetadata) -> BackupMetadataDto {
    BackupMetadataDto {
        id: meta.id.to_string(),
        file_name: meta.file_name.clone(),
        file_size_bytes: meta.file_size_bytes,
        sha256_checksum: meta.sha256_checksum.clone(),
        trigger_type: match meta.trigger_type {
            baraka_domain::backup::BackupTriggerType::Manual => "MANUAL",
            baraka_domain::backup::BackupTriggerType::ShiftClose => "SHIFT_CLOSE",
            baraka_domain::backup::BackupTriggerType::Scheduled => "SCHEDULED",
        }
        .to_string(),
        status: match meta.status {
            baraka_domain::backup::BackupStatus::Success => "SUCCESS",
            baraka_domain::backup::BackupStatus::Failed => "FAILED",
            baraka_domain::backup::BackupStatus::Corrupted => "CORRUPTED",
        }
        .to_string(),
        schema_version: meta.schema_version,
        error_message: meta.error_message.clone(),
        created_at: meta
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }
}

/// `backups_create` command: capture a verified online snapshot
/// (TASK-BRK-052, ADR-010). Admin-only; retention keeps the newest 30.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_create(
    state: tauri::State<'_, AppState>,
    request: CreateBackupRequestDto,
) -> Result<BackupMetadataDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let trigger = match request.trigger_type.as_deref().map(str::trim) {
        None | Some("") | Some("MANUAL") => baraka_domain::backup::BackupTriggerType::Manual,
        Some("SHIFT_CLOSE") => baraka_domain::backup::BackupTriggerType::ShiftClose,
        Some("SCHEDULED") => baraka_domain::backup::BackupTriggerType::Scheduled,
        Some(_) => {
            return Err(error_code(&ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_TRIGGER_TYPE".to_string()),
            )));
        }
    };
    let meta = CreateBackupUseCase::new(state.backups.as_ref())
        .execute(
            &session,
            trigger,
            &state.backup_dir,
            30,
            &state.terminal_id,
            env!("CARGO_PKG_VERSION"),
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_backup_dto(&meta))
}

/// `backups_list` command: recent backup audit rows, newest first.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<BackupMetadataDto>, String> {
    let session = require_session(&state, &session_token)?;
    require_backup_admin(&session)?;
    let rows = ListBackupsUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, 100)
        .map_err(|error| error_code(&error))?;
    Ok(rows.iter().map(to_backup_dto).collect())
}

/// `backups_verify` command: re-verify one logged snapshot on disk.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_verify(
    state: tauri::State<'_, AppState>,
    request: VerifyBackupRequestDto,
) -> Result<VerifyBackupResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let meta = VerifyBackupIntegrityUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, &request.backup_id)
        .map_err(|error| error_code(&error))?;
    Ok(VerifyBackupResponseDto {
        backup_id: request.backup_id,
        is_valid: true,
        sha256_checksum: meta.sha256_checksum,
        status: match meta.status {
            baraka_domain::backup::BackupStatus::Success => "SUCCESS",
            baraka_domain::backup::BackupStatus::Failed => "FAILED",
            baraka_domain::backup::BackupStatus::Corrupted => "CORRUPTED",
        }
        .to_string(),
        message: None,
    })
}

/// Map a typed restore failure to a stable, non-leaking wire code.
///
/// Payload-bearing variants collapse to their stable code: counts, versions,
/// and I/O detail never cross the IPC boundary (the precheck report already
/// carries the rich per-gate detail for eligible/ineligible display).
fn restore_error_code(error: &baraka_domain::backup::RestoreError) -> String {
    use baraka_domain::backup::RestoreError;
    match error {
        RestoreError::Unauthorized => "UNAUTHORIZED".to_string(),
        RestoreError::BackupNotFound => "BACKUP_NOT_FOUND".to_string(),
        RestoreError::BackupFileMissing => "BACKUP_MISSING".to_string(),
        RestoreError::BackupTampered => "BACKUP_CHECKSUM_MISMATCH".to_string(),
        RestoreError::BackupCorrupted => "BACKUP_CORRUPTED".to_string(),
        RestoreError::BackupFkViolation => "BACKUP_FK_VIOLATION".to_string(),
        RestoreError::BackupSchemaNewer { .. } => "BACKUP_SCHEMA_NEWER".to_string(),
        RestoreError::BackupAuditChainBroken => "BACKUP_AUDIT_CHAIN_BROKEN".to_string(),
        RestoreError::BackupLedgerInconsistent { .. } => "BACKUP_LEDGER_INCONSISTENT".to_string(),
        RestoreError::RestoreInProgress => "RESTORE_IN_PROGRESS".to_string(),
        RestoreError::SwapFailedAndRolledBack { .. } => "RESTORE_SWAP_FAILED".to_string(),
        RestoreError::Io(_) => "INTERNAL_ERROR".to_string(),
    }
}

/// Map a domain restore warning to its wire view.
fn to_restore_warning_dto(warning: &baraka_domain::backup::RestoreWarning) -> RestoreWarningDto {
    match warning {
        baraka_domain::backup::RestoreWarning::OpenCashSessionInBackup { opened_at } => {
            RestoreWarningDto::OpenCashSessionInBackup {
                opened_at: opened_at.clone(),
            }
        }
        baraka_domain::backup::RestoreWarning::OlderSchemaWillMigrate {
            from_version,
            to_version,
        } => RestoreWarningDto::OlderSchemaWillMigrate {
            from_version: *from_version,
            to_version: *to_version,
        },
        baraka_domain::backup::RestoreWarning::NewerActivityWillBeQuarantined {
            document_count,
        } => RestoreWarningDto::NewerActivityWillBeQuarantined {
            document_count: *document_count,
        },
    }
}

/// Map a domain ineligibility reason to its wire view.
fn to_ineligible_reason_dto(
    reason: &baraka_domain::backup::RestoreIneligibleReason,
) -> RestoreIneligibleReasonDto {
    use baraka_domain::backup::RestoreIneligibleReason;
    match reason {
        RestoreIneligibleReason::BackupFileMissing => RestoreIneligibleReasonDto::BackupFileMissing,
        RestoreIneligibleReason::HashMismatch => RestoreIneligibleReasonDto::HashMismatch,
        RestoreIneligibleReason::IntegrityCheckFailed => {
            RestoreIneligibleReasonDto::IntegrityCheckFailed
        }
        RestoreIneligibleReason::ForeignKeyViolation => {
            RestoreIneligibleReasonDto::ForeignKeyViolation
        }
        RestoreIneligibleReason::AuditChainBroken => RestoreIneligibleReasonDto::AuditChainBroken,
        RestoreIneligibleReason::CustomerLedgerMismatch { mismatch_count } => {
            RestoreIneligibleReasonDto::CustomerLedgerMismatch {
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::SupplierLedgerMismatch { mismatch_count } => {
            RestoreIneligibleReasonDto::SupplierLedgerMismatch {
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::StockLevelMismatch { mismatch_count } => {
            RestoreIneligibleReasonDto::StockLevelMismatch {
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::SchemaNewerThanApp {
            backup_version,
            app_version,
        } => RestoreIneligibleReasonDto::SchemaNewerThanApp {
            backup_version: *backup_version,
            app_version: *app_version,
        },
    }
}

/// Map a domain precheck report to its wire view.
///
/// Counts and sizes cross as numbers (they are not financial values);
/// timestamps cross as RFC3339 UTC strings.
fn to_pre_restore_report_dto(
    report: &baraka_domain::backup::PreRestoreReport,
) -> PreRestoreReportDto {
    PreRestoreReportDto {
        backup_id: report.backup_id.clone(),
        file_name: report.file_name.clone(),
        file_size_bytes: report.file_size_bytes,
        created_at: report.created_at.clone(),
        schema_version_backup: report.schema_version_backup,
        schema_version_app: report.schema_version_app,
        checks: PreRestoreChecksDto {
            file_exists: report.checks.file_exists,
            sha256_matches: report.checks.sha256_matches,
            integrity_ok: report.checks.integrity_ok,
            foreign_keys_ok: report.checks.foreign_keys_ok,
            audit_chain_ok: report.checks.audit_chain_ok,
            customer_ledger_ok: report.checks.customer_ledger_ok,
            supplier_ledger_ok: report.checks.supplier_ledger_ok,
            stock_levels_ok: report.checks.stock_levels_ok,
        },
        content_summary: BackupContentSummaryDto {
            products: report.content_summary.products,
            customers: report.content_summary.customers,
            documents_total: report.content_summary.documents_total,
            last_document_at: report.content_summary.last_document_at.clone(),
        },
        warnings: report.warnings.iter().map(to_restore_warning_dto).collect(),
        eligible: report.eligible,
        ineligible_reasons: report
            .ineligible_reasons
            .iter()
            .map(to_ineligible_reason_dto)
            .collect(),
    }
}

/// `backups_restore_precheck` command: full read-only verification report
/// for one logged backup (TASK-BRK-054, ADR-062). Admin-only; advisory only.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_restore_precheck(
    state: tauri::State<'_, AppState>,
    request: PreRestoreRequestDto,
) -> Result<PreRestoreReportDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let report = PrecheckRestoreUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, &request.backup_id)
        .map_err(|error| restore_error_code(&error))?;
    Ok(to_pre_restore_report_dto(&report))
}

/// `backups_restore` command: verified destructive restore with quarantine
/// and rollback safety (TASK-BRK-054, ADR-062). Admin-only. The destructive
/// path re-verifies everything internally; nothing is written to
/// `command_log` (the replaced database would record it — the audit trail
/// is the `RESTORE_PERFORMED` event plus the preserved quarantine copy).
#[tauri::command(rename_all = "snake_case")]
pub fn backups_restore(
    state: tauri::State<'_, AppState>,
    request: RestoreBackupRequestDto,
) -> Result<RestoreOutcomeDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let outcome = RestoreBackupUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, &request.backup_id)
        .map_err(|error| restore_error_code(&error))?;
    Ok(RestoreOutcomeDto {
        restored_backup_id: outcome.restored_backup_id,
        quarantined_to: outcome.quarantined_to,
        restored_at: outcome.restored_at,
        restart_required: outcome.restart_required,
    })
}

/// `sales_create_draft` command (`sales.create_draft`): park an editable draft.
#[tauri::command(rename_all = "snake_case")]
pub fn sales_create_draft(
    state: tauri::State<'_, AppState>,
    request: CreateDraftSaleRequest,
) -> Result<DraftSaleResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut lines = Vec::with_capacity(request.lines.len());
    for line in &request.lines {
        lines.push(CartLineInput {
            product_id: parse_wire_id(&line.product_id, "product_id")
                .map(ProductId::from)
                .map_err(|error| error_code(&error))?,
            unit_id: parse_wire_id(&line.unit_id, "unit_id")
                .map(UnitId::from)
                .map_err(|error| error_code(&error))?,
            quantity_micro: parse_wire_int(&line.quantity_micro, "quantity_micro")
                .map_err(|error| error_code(&error))?,
            unit_price_override: line
                .unit_price_centimes
                .as_deref()
                .map(|text| parse_wire_int(text, "unit_price_centimes"))
                .transpose()
                .map_err(|error| error_code(&error))?,
            line_discount_centimes: parse_wire_int(
                &line.line_discount_centimes,
                "line_discount_centimes",
            )
            .map_err(|error| error_code(&error))?,
        });
    }
    let draft_id = CreateDraftSaleUseCase::new(
        state.catalog.as_ref(),
        state.documents.as_ref(),
        state.warehouses.as_ref(),
        state.customers.as_ref(),
        state.clock.as_ref(),
    )
    .execute(
        &session,
        DraftInput {
            warehouse_id: parse_wire_id(&request.warehouse_id, "warehouse_id")
                .map(WarehouseId::from)
                .map_err(|error| error_code(&error))?,
            customer_id: request
                .customer_id
                .as_deref()
                .map(|text| {
                    parse_wire_id(text, "customer_id").map(baraka_domain::sales::CustomerId::from)
                })
                .transpose()
                .map_err(|error| error_code(&error))?,
            language: parse_document_language(&request.document_language)
                .map_err(|error| error_code(&error))?,
            lines,
            notes: request.notes,
        },
        &state.terminal_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(DraftSaleResponse {
        draft_id: draft_id.to_string(),
        created_at: state.clock.now_rfc3339(),
    })
}

/// `sales_discard_draft` command (`sales.discard_draft`): discard a parked draft.
#[derive(Debug, serde::Deserialize)]
pub struct DiscardDraftRequestDto {
    pub session_token: String,
    pub draft_id: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn sales_discard_draft(
    state: tauri::State<'_, AppState>,
    request: DiscardDraftRequestDto,
) -> Result<(), String> {
    let session = require_session(&state, &request.session_token)?;
    DiscardDraftSaleUseCase::new(state.documents.as_ref())
        .execute(&session, &request.draft_id)
        .map_err(|error| error_code(&error))
}

/// `sales_get` command (`sales.get`): full sale view.
#[tauri::command(rename_all = "snake_case")]
pub fn sales_get(
    state: tauri::State<'_, AppState>,
    session_token: String,
    document_id: String,
) -> Result<GetSaleResponse, String> {
    let session = require_session(&state, &session_token)?;
    let id = parse_wire_id(&document_id, "document_id")
        .map(baraka_domain::sales::DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let detail = GetSaleUseCase::new(state.documents.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    Ok(GetSaleResponse {
        document: DocumentDto {
            document_id: detail.document.id.to_string(),
            number: detail.document.number.clone(),
            doc_type: detail.document.doc_type.code().to_string(),
            status: detail.document.status.code().to_string(),
            warehouse_id: detail.document.warehouse_id.to_string(),
            subtotal_ht_centimes: detail.document.subtotal_ht_centimes.to_string(),
            tva_total_centimes: detail.document.vat_total_centimes.to_string(),
            total_ttc_centimes: detail.document.ttc_centimes.to_string(),
            paid_amount_centimes: detail.document.paid_amount_centimes.to_string(),
            change_amount_centimes: detail.document.change_amount_centimes.to_string(),
            payment_status: match detail.payment_status {
                baraka_domain::sales::PaymentStatus::Paid => "PAID",
                baraka_domain::sales::PaymentStatus::Partial => "PARTIAL",
                baraka_domain::sales::PaymentStatus::Unpaid => "UNPAID",
            }
            .to_string(),
        },
        items: detail
            .items
            .iter()
            .map(|item| DocumentItemDto {
                product_id: item.product_id.to_string(),
                unit_id: item.unit_id.to_string(),
                quantity_micro: item.quantity_micro.to_string(),
                unit_price_centimes: item.unit_price_centimes.to_string(),
                line_tva_centimes: item.line_tva_centimes.to_string(),
                line_ttc_centimes: item.line_ttc_centimes.to_string(),
            })
            .collect(),
        payments: detail
            .payments
            .iter()
            .map(|payment| PaymentDto {
                method_code: payment.method_code.clone(),
                amount_centimes: payment.amount_centimes.to_string(),
            })
            .collect(),
        print_status: detail.print_status,
    })
}

/// Parse an optional wire integer (`None` stays `None`).
fn parse_optional_int(text: Option<&str>, field: &'static str) -> Result<Option<i64>, String> {
    text.map(|value| parse_wire_int(value, field))
        .transpose()
        .map_err(|error| error_code(&error))
}

/// `customers_create` command: register a customer (RBAC-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_create(
    state: tauri::State<'_, AppState>,
    request: NewCustomerRequest,
) -> Result<CustomerDetailDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let policy = CreditPolicy::from_code(request.credit_policy.trim())
        .map_err(|error| error_code(&ApplicationError::Validation(error)))?;
    let id = CreateCustomerUseCase::new(state.customers.as_ref())
        .execute(
            &session,
            baraka_application::ports::NewCustomer {
                name: request.name,
                phone: request.phone,
                address: request.address,
                credit_policy: policy,
                credit_limit_centimes: request
                    .credit_limit_centimes
                    .as_deref()
                    .map(|text| parse_wire_int(text, "credit_limit_centimes"))
                    .transpose()
                    .map_err(|error| error_code(&error))?
                    .unwrap_or(0),
                customer_tier: parse_customer_tier(
                    request.customer_tier.as_deref(),
                    "customer_tier",
                )
                .map_err(|error| error_code(&error))?,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    customers_get(
        state,
        GetCustomerRequestDto {
            session_token: request.session_token,
            customer_id: id.to_string(),
        },
    )
}

/// `customers_update` command: sparse customer update (RBAC-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_update(
    state: tauri::State<'_, AppState>,
    request: UpdateCustomerRequest,
) -> Result<CustomerDetailDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    UpdateCustomerUseCase::new(state.customers.as_ref())
        .execute(
            &session,
            id,
            baraka_application::ports::CustomerUpdate {
                name: request.name,
                phone: request.phone,
                address: request.address,
                credit_policy: request
                    .credit_policy
                    .as_deref()
                    .map(CreditPolicy::from_code)
                    .transpose()
                    .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
                credit_limit_centimes: parse_optional_int(
                    request.credit_limit_centimes.as_deref(),
                    "credit_limit_centimes",
                )?,
                customer_tier: match request.customer_tier.as_deref().map(str::trim) {
                    None | Some("") => None,
                    Some(code) => Some(
                        parse_customer_tier(Some(code), "customer_tier")
                            .map_err(|error| error_code(&error))?,
                    ),
                },
                is_active: request.is_active,
            },
        )
        .map_err(|error| error_code(&error))?;
    customers_get(
        state,
        GetCustomerRequestDto {
            session_token: request.session_token,
            customer_id: id.to_string(),
        },
    )
}

fn to_summary(summary: &baraka_application::ports::CustomerSummary) -> CustomerSummaryDto {
    CustomerSummaryDto {
        id: summary.id.to_string(),
        name: summary.name.clone(),
        phone: summary.phone.clone(),
        credit_policy: summary.credit_policy.code().to_string(),
        customer_tier: summary.customer_tier.as_str().to_string(),
    }
}

/// `customers_search` command: directory search (any authenticated role).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_search(
    state: tauri::State<'_, AppState>,
    request: SearchCustomersRequest,
) -> Result<SearchCustomersResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let limit = request.limit.unwrap_or(20) as usize;
    let results = SearchCustomersUseCase::new(state.customers.as_ref())
        .execute(&session, &request.query, limit)
        .map_err(|error| error_code(&error))?;
    Ok(SearchCustomersResponse {
        customers: results.iter().map(to_summary).collect(),
    })
}

/// Shared detail assembler for get/update/create responses.
fn assemble_detail(
    state: &tauri::State<'_, AppState>,
    session: &baraka_application::ports::SessionContext,
    id: CustomerId,
) -> Result<CustomerDetailDto, String> {
    let customer = GetCustomerUseCase::new(state.customers.as_ref())
        .execute(session, id)
        .map_err(|error| error_code(&error))?;
    let profile = state
        .customers
        .get_credit_profile(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| "CUSTOMER_NOT_FOUND".to_string())?;
    Ok(CustomerDetailDto {
        id: customer.id.to_string(),
        name: customer.name.clone(),
        phone: customer.phone.clone(),
        address: customer.address.clone(),
        credit_policy: customer.credit_policy.code().to_string(),
        credit_limit_centimes: if customer.credit_policy == CreditPolicy::Limited {
            Some(customer.credit_limit_centimes.to_string())
        } else {
            None
        },
        customer_tier: customer.customer_tier.as_str().to_string(),
        current_balance_centimes: profile.current_balance_centimes.to_string(),
        is_active: customer.is_active,
    })
}

/// `customers_get` command: full detail view (any authenticated role).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_get(
    state: tauri::State<'_, AppState>,
    request: GetCustomerRequestDto,
) -> Result<CustomerDetailDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    assemble_detail(&state, &session, id)
}

/// `customers_list` command: directory listing (any authenticated role).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<SearchCustomersResponse, String> {
    let session = require_session(&state, &session_token)?;
    let results = ListCustomersUseCase::new(state.customers.as_ref())
        .execute(&session, true)
        .map_err(|error| error_code(&error))?;
    Ok(SearchCustomersResponse {
        customers: results.iter().map(to_summary).collect(),
    })
}

/// `credit_record_payment` command: append a CREDIT ledger line.
#[tauri::command(rename_all = "snake_case")]
pub fn credit_record_payment(
    state: tauri::State<'_, AppState>,
    request: RecordCreditPaymentRequest,
) -> Result<RecordCreditPaymentResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    let record = RecordCreditPaymentUseCase::new(state.customers.as_ref())
        .execute(
            &session,
            id,
            parse_wire_int(&request.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
            request.reference,
            request.allow_overpayment,
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(RecordCreditPaymentResponse {
        ledger_entry_id: record.ledger_entry_id,
        new_balance_centimes: record.new_balance_centimes.to_string(),
    })
}

/// `credit_get_statement` command: ledger rows with running balances.
#[tauri::command(rename_all = "snake_case")]
pub fn credit_get_statement(
    state: tauri::State<'_, AppState>,
    request: GetStatementRequest,
) -> Result<GetStatementResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    let rows = GetCustomerStatementUseCase::new(state.customers.as_ref())
        .execute(&session, id, request.from_date, request.to_date)
        .map_err(|error| error_code(&error))?;
    Ok(GetStatementResponse {
        entries: rows
            .into_iter()
            .map(|row| StatementEntryDto {
                entry_type: row.entry_type,
                amount_centimes: row.amount_centimes.to_string(),
                reference_type: row.reference_type,
                reference_id: row.reference_id,
                created_at: row.created_at,
                balance_after_centimes: row.balance_after_centimes.to_string(),
            })
            .collect(),
    })
}

/// `documents_get_print_status` command: latest outbox state for a document.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_get_print_status(
    state: tauri::State<'_, AppState>,
    session_token: String,
    document_id: String,
) -> Result<PrintStatusResponse, String> {
    let session = require_session(&state, &session_token)?;
    let id = parse_wire_id(&document_id, "document_id")
        .map(baraka_domain::sales::DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let status = GetPrintStatusUseCase::new(state.print_jobs.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    let row = state
        .print_jobs
        .get_job_status(id)
        .map_err(|error| error_code(&error))?;
    Ok(PrintStatusResponse {
        job_id: row
            .as_ref()
            .map(|item| item.job_id.clone())
            .unwrap_or_default(),
        status,
        attempt_count: row
            .as_ref()
            .map(|item| item.attempt_count.to_string())
            .unwrap_or_default(),
    })
}

/// `documents_reprint` command: enqueue a watermarked DUPLICATA job.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_reprint(
    state: tauri::State<'_, AppState>,
    request: ReprintRequest,
) -> Result<ReprintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.document_id, "document_id")
        .map(baraka_domain::sales::DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let job_id = ReprintDocumentUseCase::new(state.print_jobs.as_ref(), state.documents.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    Ok(ReprintResponse {
        job_id,
        document_id: id.to_string(),
    })
}

/// `printer_get_settings` command: read serial printer configuration.
#[tauri::command(rename_all = "snake_case")]
pub fn printer_get_settings(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<PrinterSettingsDto, String> {
    require_session(&state, &session_token)?;
    let settings = state
        .printer_settings
        .get_printer_settings()
        .map_err(|error| error_code(&error))?;
    Ok(PrinterSettingsDto {
        serial_port: settings.serial_port,
        serial_baud: settings.serial_baud,
        paper_width_dots: settings.paper_width_dots,
    })
}

/// `printer_update_settings` command (ADMIN only): persist COM port + baud.
#[tauri::command(rename_all = "snake_case")]
pub fn printer_update_settings(
    state: tauri::State<'_, AppState>,
    request: UpdatePrinterSettingsRequest,
) -> Result<PrinterSettingsDto, String> {
    let session = require_session(&state, &request.session_token)?;
    if !session
        .role
        .has_permission(baraka_domain::role::Permission::ManagePrinterSettings)
    {
        return Err("UNAUTHORIZED".to_string());
    }
    if request.serial_baud == 0 {
        return Err("INVALID_INPUT".to_string());
    }
    state
        .printer_settings
        .update_printer_settings(&request.serial_port, request.serial_baud)
        .map_err(|error| error_code(&error))?;
    printer_get_settings(state, request.session_token.clone())
}

/// `store_identity_get` command: read identity + device kind (session).
#[tauri::command(rename_all = "snake_case")]
pub fn store_identity_get(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<StoreIdentityResponse, String> {
    require_session(&state, &session_token)?;
    let identity = state
        .store_identity
        .load_identity()
        .map_err(|error| error_code(&error))?;
    let device_kind = state
        .store_identity
        .device_kind()
        .map_err(|error| error_code(&error))?;
    Ok(StoreIdentityResponse {
        identity: to_identity_dto(&identity),
        device_kind,
    })
}

/// `store_identity_save` command (SystemSettingsManage): trim +
/// soft-validate + persist + audit (audited inside the settings repo).
#[tauri::command(rename_all = "snake_case")]
pub fn store_identity_save(
    state: tauri::State<'_, AppState>,
    request: SaveStoreIdentityRequest,
) -> Result<StoreIdentityResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    if !session
        .role
        .has_permission(Permission::SystemSettingsManage)
    {
        return Err(error_code(&ApplicationError::Unauthorized));
    }
    let saved = SaveStoreIdentityUseCase::new(state.store_identity.as_ref())
        .execute(
            &session,
            &from_identity_dto(&request.identity),
            &request.device_kind,
        )
        .map_err(|error| error_code(&error))?;
    let device_kind = state
        .store_identity
        .device_kind()
        .map_err(|error| error_code(&error))?;
    Ok(StoreIdentityResponse {
        identity: to_identity_dto(&saved),
        device_kind,
    })
}

/// `printer_test_print` command (SystemSettingsManage): synchronous
/// diagnostics (ADR-065 as amended) — shared-path sample receipt,
/// layout preview, unconditional shadow capture, physical attempt
/// through the worker's transport only when kind = physical.
#[tauri::command(rename_all = "snake_case")]
pub fn printer_test_print(
    state: tauri::State<'_, AppState>,
    request: TestPrintRequest,
) -> Result<TestPrintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    if !session
        .role
        .has_permission(Permission::SystemSettingsManage)
    {
        return Err(error_code(&ApplicationError::Unauthorized));
    }
    let language = match request.language.trim().to_lowercase().as_str() {
        "ar" | "arabic" | "arabe" => baraka_domain::printing::ReceiptLanguage::Arabic,
        _ => baraka_domain::printing::ReceiptLanguage::French,
    };
    let report = TestPrintUseCase::new(
        state.store_identity.as_ref(),
        state.printer_renderer.as_ref(),
        state.printer_capture.as_ref(),
        state.printer_transport.as_ref(),
        state.clock.as_ref(),
    )
    .execute(&session, language)
    .map_err(|error| error_code(&error))?;
    Ok(TestPrintResponse {
        preview: report
            .preview
            .iter()
            .map(|line| PreviewLineDto {
                kind: line.kind.clone(),
                centered: line.centered,
                size_px: line.size_px,
                text: line.text.clone(),
            })
            .collect(),
        decode_summary: DecodeSummaryDto {
            bands: report.decode_summary.bands,
            feed_lines: report.decode_summary.feed_lines,
            unknown_spans: report.decode_summary.unknown_spans,
        },
        capture_file_name: report.capture_file_name,
        device_outcome: match report.device {
            TestPrintDeviceOutcome::Sent => "sent".to_string(),
            TestPrintDeviceOutcome::CaptureOnly => "capture-only".to_string(),
            TestPrintDeviceOutcome::DeviceError(code) => code,
        },
    })
}

fn to_identity_dto(identity: &baraka_domain::printing::StoreIdentity) -> StoreIdentityDto {
    StoreIdentityDto {
        store_name: identity.store_name.clone(),
        store_address: identity.store_address.clone(),
        store_phone: identity.store_phone.clone(),
        nif: identity.nif.clone(),
        nis: identity.nis.clone(),
        rc: identity.rc.clone(),
        ai: identity.ai.clone(),
        ticket_footer_message: identity.ticket_footer_message.clone(),
    }
}

fn from_identity_dto(dto: &StoreIdentityDto) -> baraka_domain::printing::StoreIdentity {
    baraka_domain::printing::StoreIdentity {
        store_name: dto.store_name.clone(),
        store_address: dto.store_address.clone(),
        store_phone: dto.store_phone.clone(),
        nif: dto.nif.clone(),
        nis: dto.nis.clone(),
        rc: dto.rc.clone(),
        ai: dto.ai.clone(),
        ticket_footer_message: dto.ticket_footer_message.clone(),
    }
}

fn to_cash_session_dto(view: &baraka_application::ports::CashSessionView) -> CashSessionDto {
    CashSessionDto {
        id: view.id.clone(),
        warehouse_id: view.warehouse_id.clone(),
        terminal_id: view.terminal_id.clone(),
        status: view.status.clone(),
        opened_by: view.opened_by.clone(),
        opened_by_username: view.opened_by_username.clone(),
        opened_at: view.opened_at.clone(),
        opening_float_centimes: view.opening_float_centimes.to_string(),
        closed_by: view.closed_by.clone(),
        closed_by_username: view.closed_by_username.clone(),
        closed_at: view.closed_at.clone(),
        actual_counted_centimes: view.actual_counted_centimes.map(|v| v.to_string()),
        system_expected_centimes: view.system_expected_centimes.map(|v| v.to_string()),
        variance_centimes: view.variance_centimes.map(|v| v.to_string()),
        closing_notes: view.closing_notes.clone(),
        revision: view.revision.to_string(),
        created_at: view.created_at.clone(),
        updated_at: view.updated_at.clone(),
    }
}

fn to_cash_movement_dto(view: &baraka_application::ports::CashMovementView) -> CashMovementDto {
    CashMovementDto {
        id: view.id.clone(),
        cash_session_id: view.cash_session_id.clone(),
        movement_type: view.movement_type.clone(),
        amount_centimes: view.amount_centimes.to_string(),
        reference_type: view.reference_type.clone(),
        reference_id: view.reference_id.clone(),
        description: view.description.clone(),
        created_at: view.created_at.clone(),
        created_by: view.created_by.clone(),
        created_by_username: view.created_by_username.clone(),
    }
}

/// `cash.open` command: open a session with an opening float.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_open(
    state: tauri::State<'_, AppState>,
    request: OpenCashSessionRequestDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let warehouse_id = parse_wire_id(&request.warehouse_id, "warehouse_id")
        .map(WarehouseId::from)
        .map_err(|error| error_code(&error))?;
    let view = OpenCashSessionUseCase::new(state.cash.as_ref(), state.warehouses.as_ref())
        .execute(
            &session,
            warehouse_id,
            parse_wire_int(&request.opening_float_centimes, "opening_float_centimes")
                .map_err(|error| error_code(&error))?,
            &state.terminal_id,
            &request.command_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_session_dto(&view))
}

/// `cash.beginClosing` command: freeze the expected total (OPEN -> CLOSING).
#[tauri::command(rename_all = "snake_case")]
pub fn cash_begin_closing(
    state: tauri::State<'_, AppState>,
    request: CashClosingCommandDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = BeginClosingUseCase::new(state.cash.as_ref())
        .execute(&session, &state.terminal_id, &request.command_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_session_dto(&view))
}

/// `cash.cancelClosing` command: revert CLOSING -> OPEN.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_cancel_closing(
    state: tauri::State<'_, AppState>,
    request: CashClosingCommandDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = CancelClosingUseCase::new(state.cash.as_ref())
        .execute(&session, &state.terminal_id, &request.command_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_session_dto(&view))
}

/// `cash.confirmClosing` command: count and seal (CLOSING -> CLOSED).
#[tauri::command(rename_all = "snake_case")]
pub fn cash_confirm_closing(
    state: tauri::State<'_, AppState>,
    request: ConfirmClosingRequestDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = ConfirmClosingUseCase::new(state.cash.as_ref())
        .execute(
            &session,
            &request.session_id,
            parse_wire_int(&request.actual_counted_centimes, "actual_counted_centimes")
                .map_err(|error| error_code(&error))?,
            request.closing_notes,
            &state.terminal_id,
            &request.command_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_session_dto(&view))
}

/// `cash.getActiveSession` command: active OPEN/CLOSING session, if any.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_get_active_session(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Option<CashSessionDto>, String> {
    let session = require_session(&state, &session_token)?;
    let view = GetActiveSessionUseCase::new(state.cash.as_ref())
        .execute(&session, &state.terminal_id)
        .map_err(|error| error_code(&error))?;
    Ok(view.as_ref().map(to_cash_session_dto))
}

/// `cash.recordExpense` command: append an EXPENSE_OUT movement.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_record_expense(
    state: tauri::State<'_, AppState>,
    request: RecordExpenseRequestDto,
) -> Result<CashMovementDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = RecordExpenseUseCase::new(state.cash.as_ref())
        .execute(
            &session,
            parse_wire_int(&request.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
            &request.description,
            &state.terminal_id,
            &request.command_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_movement_dto(&view))
}

/// `cash.getSessionReport` command: X-Report (interim) or Z-Report (final).
#[tauri::command(rename_all = "snake_case")]
pub fn cash_get_session_report(
    state: tauri::State<'_, AppState>,
    request: GetSessionReportRequestDto,
) -> Result<CashSessionReportDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let report = GetSessionReportUseCase::new(state.cash.as_ref())
        .execute(&session, &request.session_id)
        .map_err(|error| error_code(&error))?;
    Ok(CashSessionReportDto {
        session: to_cash_session_dto(&report.session),
        movements: report.movements.iter().map(to_cash_movement_dto).collect(),
        totals_by_type: report
            .totals_by_type
            .iter()
            .map(|total| CashMovementTotalDto {
                movement_type: total.movement_type.clone(),
                total_centimes: total.total_centimes.to_string(),
                count: total.count.to_string(),
            })
            .collect(),
        is_final: report.is_final,
    })
}

fn to_supplier_dto(view: &baraka_application::ports::SupplierView) -> SupplierDto {
    SupplierDto {
        id: view.id.clone(),
        name: view.name.clone(),
        phone: view.phone.clone(),
        address: view.address.clone(),
        nif: view.nif.clone(),
        rc: view.rc.clone(),
        notes: view.notes.clone(),
        is_active: view.is_active,
        current_balance_centimes: view.current_balance_centimes.to_string(),
        origin_device_id: view.origin_device_id.clone(),
        created_at: view.created_at.clone(),
        updated_at: view.updated_at.clone(),
    }
}

fn parse_supplier_id(text: &str) -> Result<String, String> {
    parse_wire_id(text, "supplier_id")
        .map(|id| id.to_string())
        .map_err(|error| error_code(&error))
}

fn parse_purchase_lines(
    lines: &[PurchaseLineRequestDto],
) -> Result<Vec<PurchaseLineInput>, String> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        out.push(PurchaseLineInput {
            product_id: parse_wire_id(&line.product_id, "product_id")
                .map(ProductId::from)
                .map_err(|error| error_code(&error))?,
            unit_id: parse_wire_id(&line.unit_id, "unit_id")
                .map(UnitId::from)
                .map_err(|error| error_code(&error))?,
            quantity_micro: parse_wire_int(&line.quantity_micro, "quantity_micro")
                .map_err(|error| error_code(&error))?,
            unit_cost_centimes: parse_wire_int(&line.unit_cost_centimes, "unit_cost_centimes")
                .map_err(|error| error_code(&error))?,
        });
    }
    Ok(out)
}

fn to_purchase_order_dto(
    view: &baraka_application::ports::PurchaseDocumentView,
) -> PurchaseOrderDto {
    PurchaseOrderDto {
        id: view.id.clone(),
        number: view.number.clone(),
        doc_type: view.doc_type.clone(),
        status: view.status.clone(),
        supplier_id: view.supplier_id.clone(),
        warehouse_id: view.warehouse_id.clone(),
        source_document_id: view.source_document_id.clone(),
        total_centimes: view.total_centimes.to_string(),
        lines: view
            .lines
            .iter()
            .map(|line| PurchaseOrderLineDto {
                id: line.id.clone(),
                product_id: line.product_id.clone(),
                unit_id: line.unit_id.clone(),
                quantity_micro: line.quantity_micro.to_string(),
                quantity_base_micro: line.quantity_base_micro.to_string(),
                unit_cost_centimes: line.unit_cost_centimes.to_string(),
                line_total_centimes: line.line_total_centimes.to_string(),
            })
            .collect(),
        created_at: view.created_at.clone(),
        created_by: view.created_by.clone(),
    }
}

/// `suppliers.create` command: register a supplier (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_create(
    state: tauri::State<'_, AppState>,
    request: CreateSupplierRequestDto,
) -> Result<SupplierDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = CreateSupplierUseCase::new(state.suppliers.as_ref())
        .execute(
            &session,
            NewSupplier {
                name: request.name,
                phone: request.phone,
                address: request.address,
                nif: request.nif,
                rc: request.rc,
                notes: request.notes,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_supplier_dto(&view))
}

/// `suppliers.update` command: sparse supplier update (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_update(
    state: tauri::State<'_, AppState>,
    request: UpdateSupplierRequestDto,
) -> Result<SupplierDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let view = UpdateSupplierUseCase::new(state.suppliers.as_ref())
        .execute(
            &session,
            &supplier_id,
            SupplierUpdate {
                name: request.name,
                phone: request.phone,
                address: request.address,
                nif: request.nif,
                rc: request.rc,
                notes: request.notes,
                is_active: request.is_active,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_supplier_dto(&view))
}

/// `suppliers.get` command: full supplier view.
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_get(
    state: tauri::State<'_, AppState>,
    request: SupplierIdRequestDto,
) -> Result<SupplierDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.id)?;
    let view = GetSupplierUseCase::new(state.suppliers.as_ref())
        .execute(&session, &supplier_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_supplier_dto(&view))
}

/// `suppliers.search` command: directory search (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_search(
    state: tauri::State<'_, AppState>,
    request: SearchSuppliersRequestDto,
) -> Result<SearchSuppliersResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let limit = request.limit.unwrap_or(20) as usize;
    let views = SearchSuppliersUseCase::new(state.suppliers.as_ref())
        .execute(&session, &request.query, limit)
        .map_err(|error| error_code(&error))?;
    Ok(SearchSuppliersResponseDto {
        suppliers: views.iter().map(to_supplier_dto).collect(),
    })
}

/// `suppliers.getStatement` command: ledger rows with running balances.
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_get_statement(
    state: tauri::State<'_, AppState>,
    request: SupplierIdRequestDto,
) -> Result<SupplierStatementDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.id)?;
    let statement = GetSupplierStatementUseCase::new(state.suppliers.as_ref())
        .execute(&session, &supplier_id)
        .map_err(|error| error_code(&error))?;
    Ok(SupplierStatementDto {
        entries: statement
            .entries
            .iter()
            .map(|entry| SupplierLedgerEntryDto {
                id: entry.id.clone(),
                entry_type: entry.entry_type.clone(),
                amount_centimes: entry.amount_centimes.to_string(),
                reference_type: entry.reference_type.clone(),
                reference_id: entry.reference_id.clone(),
                description: entry.description.clone(),
                created_at: entry.created_at.clone(),
                created_by: entry.created_by.clone(),
                balance_after_centimes: entry.balance_after_centimes.to_string(),
            })
            .collect(),
        current_balance_centimes: statement.current_balance_centimes.to_string(),
    })
}

/// `purchases.createOrder` command: DRAFT BC, zero stock/ledger effect.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_create_order(
    state: tauri::State<'_, AppState>,
    request: CreatePurchaseOrderRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let view = CreatePurchaseOrderUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.updateOrder` command: replace the line set of a DRAFT BC.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_update_order(
    state: tauri::State<'_, AppState>,
    request: UpdatePurchaseOrderRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let document_id = parse_wire_id(&request.document_id, "document_id")
        .map(|id| id.to_string())
        .map_err(|error| error_code(&error))?;
    let view = UpdatePurchaseOrderUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        &document_id,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.receiveGoods` command: BR commit + PURCHASE_IN stock effect.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_receive_goods(
    state: tauri::State<'_, AppState>,
    request: ReceiveGoodsRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let source_bc_id = request
        .source_bc_id
        .as_deref()
        .map(|text| {
            parse_wire_id(text, "source_bc_id")
                .map(|id| id.to_string())
                .map_err(|error| error_code(&error))
        })
        .transpose()?;
    let view = ReceiveGoodsUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        source_bc_id,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.recordInvoice` command: FF commit, exactly one ledger DEBIT.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_record_invoice(
    state: tauri::State<'_, AppState>,
    request: RecordSupplierInvoiceRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let source_br_id = request
        .source_br_id
        .as_deref()
        .map(|text| {
            parse_wire_id(text, "source_br_id")
                .map(|id| id.to_string())
                .map_err(|error| error_code(&error))
        })
        .transpose()?;
    let view = RecordSupplierInvoiceUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        source_br_id,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        parse_wire_int(&request.invoice_total_centimes, "invoice_total_centimes")
            .map_err(|error| error_code(&error))?,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.recordPayment` command: ledger CREDIT + optional drawer out.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_record_payment(
    state: tauri::State<'_, AppState>,
    request: RecordSupplierPaymentRequestDto,
) -> Result<SupplierPaymentOutcomeDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let outcome =
        RecordSupplierPaymentUseCase::new(state.purchases.as_ref(), state.suppliers.as_ref())
            .execute(
                &session,
                &supplier_id,
                parse_wire_int(&request.amount_centimes, "amount_centimes")
                    .map_err(|error| error_code(&error))?,
                &request.payment_method,
                request.description,
                &state.terminal_id,
                &request.command_id,
            )
            .map_err(|error| error_code(&error))?;
    Ok(SupplierPaymentOutcomeDto {
        ledger_entry_id: outcome.ledger_entry_id,
        new_balance_centimes: outcome.new_balance_centimes.to_string(),
        cash_movement_id: outcome.cash_movement_id,
    })
}

/// `purchases.getDocument` command: load any BC/BR/FF with its lines.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_get_document(
    state: tauri::State<'_, AppState>,
    request: SupplierIdRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let document_id = parse_wire_id(&request.id, "document_id")
        .map(|id| id.to_string())
        .map_err(|error| error_code(&error))?;
    let view = GetPurchaseDocumentUseCase::new(state.purchases.as_ref())
        .execute(&session, &document_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

fn to_daily_sales_dto(row: &baraka_application::ports::DailySalesRow) -> DailySalesSummaryDto {
    DailySalesSummaryDto {
        day: row.day.clone(),
        total_ttc_centimes: row.total_ttc_centimes.to_string(),
        total_ht_centimes: row.total_ht_centimes.to_string(),
        total_tva_centimes: row.total_tva_centimes.to_string(),
        document_count: row.document_count.to_string(),
        average_basket_centimes: row.average_basket_centimes.to_string(),
    }
}

/// `reports.dailySales` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_daily_sales(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<DailySalesSummaryDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetDailySalesSummaryUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows.iter().map(to_daily_sales_dto).collect())
}

/// `reports.salesByProduct` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_sales_by_product(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<SalesByProductRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetSalesByProductUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| SalesByProductRowDto {
            product_id: row.product_id.clone(),
            sku: row.sku.clone(),
            name_fr: row.name_fr.clone(),
            quantity_sold_base_micro: row.quantity_sold_base_micro.to_string(),
            revenue_ttc_centimes: row.revenue_ttc_centimes.to_string(),
            line_count: row.line_count.to_string(),
        })
        .collect())
}

/// `reports.salesByCategory` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_sales_by_category(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<SalesByCategoryRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetSalesByCategoryUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| SalesByCategoryRowDto {
            category_id: row.category_id.clone(),
            category_code: row.category_code.clone(),
            name_fr: row.name_fr.clone(),
            revenue_ttc_centimes: row.revenue_ttc_centimes.to_string(),
            line_count: row.line_count.to_string(),
        })
        .collect())
}

/// `reports.grossMargin` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_gross_margin(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<GrossMarginRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetGrossMarginReportUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| {
            // Display-only percentage in basis points, half away from zero;
            // "0" when revenue is zero (no division possible).
            let percent_bp = if row.revenue_ht_centimes == 0 {
                0i64
            } else {
                let numerator = i128::from(row.margin_centimes) * 10_000;
                let denominator = i128::from(row.revenue_ht_centimes);
                let rounded = if numerator >= 0 {
                    (numerator + denominator / 2) / denominator
                } else {
                    -((-numerator + denominator / 2) / denominator)
                };
                i64::try_from(rounded).unwrap_or(0)
            };
            GrossMarginRowDto {
                product_id: row.product_id.clone(),
                sku: row.sku.clone(),
                revenue_ht_centimes: row.revenue_ht_centimes.to_string(),
                cost_total_centimes: row.cost_total_centimes.to_string(),
                margin_centimes: row.margin_centimes.to_string(),
                margin_percent_bp: percent_bp.to_string(),
                has_untracked_cost: row.has_untracked_cost,
            }
        })
        .collect())
}

/// `reports.creditOutstanding` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_credit_outstanding(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<CreditOutstandingRowDto>, String> {
    let session = require_session(&state, &session_token)?;
    let rows = GetCreditOutstandingReportUseCase::new(state.reports.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| {
            // Display-only consumed-limit percentage in basis points;
            // None for UNLIMITED policies (no division possible).
            let used_bp = match (&row.credit_policy, row.credit_limit_centimes) {
                (policy, Some(limit)) if policy == "LIMITED" && limit > 0 => {
                    let numerator = i128::from(row.current_balance_centimes) * 10_000;
                    let rounded = (numerator + i128::from(limit) / 2) / i128::from(limit);
                    Some(i64::try_from(rounded).unwrap_or(0).to_string())
                }
                _ => None,
            };
            CreditOutstandingRowDto {
                customer_id: row.customer_id.clone(),
                name: row.name.clone(),
                phone: row.phone.clone(),
                credit_policy: row.credit_policy.clone(),
                credit_limit_centimes: row.credit_limit_centimes.map(|v| v.to_string()),
                current_balance_centimes: row.current_balance_centimes.to_string(),
                limit_used_bp: used_bp,
            }
        })
        .collect())
}

/// `reports.stockMovementHistory` command (Gerant/Admin, paginated).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_stock_movement_history(
    state: tauri::State<'_, AppState>,
    request: StockMovementHistoryRequestDto,
) -> Result<Vec<StockMovementRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetStockMovementHistoryUseCase::new(state.reports.as_ref())
        .execute(
            &session,
            request.warehouse_id.as_deref(),
            request.product_id.as_deref(),
            request.movement_type.as_deref(),
            &request.date_from,
            &request.date_to,
            request.limit.unwrap_or(50) as usize,
            request.offset.unwrap_or(0) as usize,
        )
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| StockMovementRowDto {
            id: row.id.clone(),
            movement_type: row.movement_type.clone(),
            warehouse_id: row.warehouse_id.clone(),
            product_id: row.product_id.clone(),
            quantity_base_micro: row.quantity_base_micro.to_string(),
            reference_type: row.reference_type.clone(),
            reference_id: row.reference_id.clone(),
            created_at: row.created_at.clone(),
            created_by: row.created_by.clone(),
        })
        .collect())
}

/// `reports.cashSessionsHistory` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_cash_sessions_history(
    state: tauri::State<'_, AppState>,
    request: CashSessionsHistoryRequestDto,
) -> Result<Vec<CashSessionHistoryRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetCashSessionsHistoryUseCase::new(state.reports.as_ref())
        .execute(
            &session,
            request.terminal_id.as_deref(),
            &request.date_from,
            &request.date_to,
        )
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| CashSessionHistoryRowDto {
            id: row.id.clone(),
            terminal_id: row.terminal_id.clone(),
            status: row.status.clone(),
            opened_at: row.opened_at.clone(),
            closed_at: row.closed_at.clone(),
            opening_float_centimes: row.opening_float_centimes.to_string(),
            system_expected_centimes: row.system_expected_centimes.map(|v| v.to_string()),
            actual_counted_centimes: row.actual_counted_centimes.map(|v| v.to_string()),
            variance_centimes: row.variance_centimes.map(|v| v.to_string()),
        })
        .collect())
}

/// `reports.lowStock` command (Gerant/Admin, ADR-037 depletion signal).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_low_stock(
    state: tauri::State<'_, AppState>,
    request: LowStockRequestDto,
) -> Result<Vec<LowStockRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetLowStockReportUseCase::new(state.reports.as_ref())
        .execute(&session, request.warehouse_id.as_deref())
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| LowStockRowDto {
            product_id: row.product_id.clone(),
            sku: row.sku.clone(),
            name_fr: row.name_fr.clone(),
            warehouse_id: row.warehouse_id.clone(),
            warehouse_code: row.warehouse_code.clone(),
            quantity_base_micro: row.quantity_base_micro.to_string(),
        })
        .collect())
}

/// `reports.userActivity` command (Gerant/Admin, paginated, generic payload).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_user_activity(
    state: tauri::State<'_, AppState>,
    request: UserActivityRequestDto,
) -> Result<Vec<UserActivityRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetUserActivityLogUseCase::new(state.reports.as_ref())
        .execute(
            &session,
            request.user_id.as_deref(),
            request.action.as_deref(),
            &request.date_from,
            &request.date_to,
            request.limit.unwrap_or(50) as usize,
            request.offset.unwrap_or(0) as usize,
        )
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| UserActivityRowDto {
            sequence_no: row.sequence_no.to_string(),
            timestamp: row.timestamp.clone(),
            user_id: row.user_id.clone(),
            action: row.action.clone(),
            entity_type: row.entity_type.clone(),
            entity_id: row.entity_id.clone(),
            old_value_json: row.old_value_json.clone(),
            new_value_json: row.new_value_json.clone(),
        })
        .collect())
}

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
}
