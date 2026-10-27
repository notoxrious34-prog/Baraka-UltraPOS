//! Shared domain errors (pure validation, no I/O).

use thiserror::Error;

/// Failures raised by pure domain primitives and models.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    /// Fixed-point arithmetic exceeded the `i64` centime/micro range.
    #[error("money arithmetic overflow")]
    MoneyOverflow,
    /// A negative amount was supplied where only zero or positive is valid.
    #[error("negative money is forbidden in this context")]
    NegativeMoneyForbidden,
    /// A distribution or conversion was requested over zero shares.
    #[error("division by zero")]
    DivisionByZero,
    /// A rounding step could not be completed deterministically.
    #[error("rounding failure: {0}")]
    RoundingError(String),
    /// A canonical wire string could not be parsed.
    #[error("invalid canonical string: {0}")]
    InvalidCanonicalString(String),
    /// Username unknown OR password mismatch (identical on purpose: never
    /// leak which of the two failed, to prevent username enumeration).
    #[error("invalid credentials")]
    InvalidCredentials,
    /// Account exists but is deactivated.
    #[error("account is inactive")]
    AccountInactive,
    /// Account temporarily locked after repeated login failures.
    #[error("account locked")]
    AccountLocked {
        /// Seconds remaining before the lock naturally expires.
        remaining_seconds: i64,
    },
    /// No PIN is configured for this user.
    #[error("no PIN is set for this user")]
    PinNotSet,
    /// A product SKU already exists in the catalog.
    #[error("duplicate SKU")]
    DuplicateSku,
    /// A unit barcode is already mapped to another product unit.
    #[error("duplicate barcode")]
    DuplicateBarcode,
    /// A unit set violates the base-unit invariant (exactly one base unit
    /// at factor 1.0000, all factors positive, all units of one product).
    #[error("base unit mismatch")]
    BaseUnitMismatch,
    /// A proposed kit edge would close a directed cycle in the BOM graph.
    #[error("kit cycle detected")]
    KitCycleDetected,
    /// Kit expansion exceeded the maximum supported nesting depth.
    #[error("kit expansion depth exceeded")]
    KitExpansionDepthExceeded,
    /// A kit component is unknown, inactive, or bound to a foreign unit.
    #[error("invalid kit component")]
    InvalidKitComponent,
    /// A kit component is deactivated and cannot expand.
    #[error("kit component inactive")]
    InactiveKitComponent,
    /// A kit cannot contain itself as a direct component.
    #[error("self-referencing kit")]
    SelfReferencingKit,
    /// A sale was attempted with no lines.
    #[error("empty cart")]
    EmptyCart,
    /// A global discount exceeds the document gross.
    #[error("discount exceeds total")]
    DiscountExceedsTotal,
    /// A line discount exceeds its unit price.
    #[error("line discount exceeds unit price")]
    LineDiscountExceedsUnitPrice,
    /// A quantity is zero or negative where positive is required.
    #[error("invalid quantity")]
    InvalidQuantity,
    /// Tendered amount is below the total without credit coverage.
    #[error("insufficient payment")]
    InsufficientPayment,
    /// Non-cash tender exceeds the total (change is cash-only).
    #[error("change not permitted for non-cash")]
    ChangeNotPermittedForNonCash,
    /// Credit sale attempted while the customer forbids credit.
    #[error("credit disabled")]
    CreditDisabled,
    /// Gapless sequence display overflow (>= 1,000,000).
    #[error("gapless sequence overflow")]
    GaplessSequenceOverflow,
    /// Document language code outside the AR/FR contract.
    #[error("invalid document language")]
    InvalidDocumentLanguage,
    /// Customer row not found.
    #[error("customer not found")]
    CustomerNotFound,
    /// A phone number is already registered to another customer.
    #[error("duplicate customer phone")]
    DuplicateCustomerPhone,
    /// A LIMITED policy requires a positive limit.
    #[error("invalid credit limit")]
    InvalidCreditLimit,
    /// A credit payment exceeds the outstanding balance.
    #[error("payment exceeds balance")]
    PaymentExceedsBalance,
    /// Print job row not found.
    #[error("print job not found")]
    PrintJobNotFound,
    /// Receipt rasterization failed (malformed content, no panic propagated).
    #[error("print rendering failed")]
    PrintRenderingFailed,
    /// No printer transport is currently reachable.
    #[error("printer transport unavailable")]
    PrinterTransportUnavailable,
    /// A second non-closed cash session was attempted on the same terminal.
    #[error("cash session already open for terminal")]
    CashSessionAlreadyOpenForTerminal,
    /// An operation required an OPEN cash session but none was found.
    #[error("cash session not open")]
    CashSessionNotOpen,
    /// An operation required a CLOSING session but none was found.
    #[error("cash session not in closing")]
    CashSessionNotInClosing,
    /// No OPEN cash session exists for this terminal (blocking precondition).
    #[error("no open cash session for terminal")]
    NoOpenCashSessionForTerminal,
    /// The counted closing amount was negative.
    #[error("invalid counted amount")]
    CashSessionInvalidCountedAmount,
    /// The opening float was negative.
    #[error("invalid opening float")]
    CashSessionInvalidOpeningFloat,
    /// An expense amount was zero or negative.
    #[error("invalid expense amount")]
    CashExpenseInvalidAmount,
    /// An expense description was empty or too long.
    #[error("expense description required")]
    CashExpenseDescriptionRequired,
    /// A cash session row was not found.
    #[error("cash session not found")]
    CashSessionNotFound,
    /// Net cash delta was negative: upstream payment-validation defect.
    #[error("invalid payment state")]
    InvalidPaymentState,
    /// A supplier row was not found.
    #[error("supplier not found")]
    SupplierNotFound,
    /// A supplier name was blank after trim.
    #[error("supplier name required")]
    SupplierNameRequired,
    /// A supplier exists but is inactive.
    #[error("supplier inactive")]
    SupplierInactive,
    /// A purchase document (BC) row was not found.
    #[error("purchase order not found")]
    PurchaseOrderNotFound,
    /// A purchase document mutation was attempted outside DRAFT.
    #[error("purchase order not draft")]
    PurchaseOrderNotDraft,
    /// Received quantity exceeded the remaining ordered quantity.
    #[error("receipt quantity exceeds ordered quantity")]
    PurchaseReceiptQuantityExceedsOrdered,
    /// A BR already has an FF attached (no double-invoicing).
    #[error("purchase invoice already sourced")]
    PurchaseInvoiceAlreadySourced,
    /// A purchase line quantity was non-positive or mismatched.
    #[error("purchase line quantity invalid")]
    PurchaseLineQuantityInvalid,
    /// A supplier payment amount was not strictly positive.
    #[error("supplier payment invalid amount")]
    SupplierPaymentInvalidAmount,
    /// A supplier payment method was neither CASH nor OTHER.
    #[error("supplier payment method invalid")]
    SupplierPaymentMethodInvalid,
    /// A report date range was inverted (from is after to).
    #[error("invalid date range")]
    InvalidDateRange,
    /// A report page size exceeded the maximum of 200 rows.
    #[error("report pagination limit exceeded")]
    ReportPaginationLimitExceeded,
    /// A conversion was attempted on an already-converted document.
    #[error("document already converted")]
    DocumentAlreadyConverted,
    /// A conversion source was in a non-convertible status.
    #[error("invalid conversion source status")]
    InvalidConversionSourceStatus,
    /// A conversion source was of a non-convertible document type.
    #[error("invalid conversion source type")]
    InvalidConversionSourceType,
    /// A conversion source document id matched no row.
    #[error("source document not found")]
    SourceDocumentNotFound,
    /// A conversion source document carried zero lines.
    #[error("empty source document")]
    EmptySourceDocument,
    /// A first-admin bootstrap was attempted after one already exists.
    #[error("first admin already bootstrapped")]
    BootstrapAlreadyCompleted,
    /// A printer paper width was neither 58 nor 80 millimeters.
    #[error("invalid printer paper width (millimeters): {0}")]
    InvalidPaperWidthMillimeters(u32),
}
