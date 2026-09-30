//! Application-level errors (use-case failures, not infra failures).
//!
//! Authentication surface contract: domain [`DomainError`] auth variants
//! ([`InvalidCredentials`], [`AccountInactive`], [`AccountLocked`],
//! [`PinNotSet`]) flow through [`ApplicationError::Validation`] unchanged,
//! so the IPC layer can match on the exact variant and emit stable,
//! non-leaking wire codes (`INVALID_CREDENTIALS`, `ACCOUNT_LOCKED:<secs>`,
//! `ACCOUNT_INACTIVE`, `PIN_NOT_SET`). No other variant may be exposed
//! for an authentication failure.
//!
//! [`DomainError`]: baraka_domain::errors::DomainError
//! [`InvalidCredentials`]: baraka_domain::errors::DomainError::InvalidCredentials
//! [`AccountInactive`]: baraka_domain::errors::DomainError::AccountInactive
//! [`AccountLocked`]: baraka_domain::errors::DomainError::AccountLocked
//! [`PinNotSet`]: baraka_domain::errors::DomainError::PinNotSet
//!
//! Print surface contract: domain [`PrintJobNotFound`],
//! [`PrintRenderingFailed`], and [`PrinterTransportUnavailable`] flow
//! through [`ApplicationError::Validation`] unchanged, so the IPC layer
//! emits stable codes without leaking driver internals.
//!
//! [`PrintJobNotFound`]: baraka_domain::errors::DomainError::PrintJobNotFound
//! [`PrintRenderingFailed`]: baraka_domain::errors::DomainError::PrintRenderingFailed
//! [`PrinterTransportUnavailable`]: baraka_domain::errors::DomainError::PrinterTransportUnavailable

use baraka_domain::errors::DomainError;
use thiserror::Error;

/// Failures that a use case can report to its caller.
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// Input rejected by domain validation (including all auth failures).
    #[error("invalid input: {0}")]
    Validation(#[from] DomainError),
    /// Requested entity was not found.
    #[error("not found: {0}")]
    NotFound(String),
    /// Business rule conflict (e.g. duplicate reference).
    #[error("conflict: {0}")]
    Conflict(String),
    /// Dependency port failed without a narrower mapping.
    #[error("port failure: {0}")]
    Port(String),
    /// Caller session lacks the required permission.
    #[error("unauthorized")]
    Unauthorized,
    /// Quantity failed validation (must be a positive magnitude).
    #[error("invalid quantity")]
    InvalidQuantity,
    /// Stock engines only move physical leaf products, never kit headers.
    #[error("cannot move stock for a kit product")]
    CannotMoveStockForKitProduct,
    /// A movement would drive stock below zero without authorization.
    #[error("insufficient stock")]
    InsufficientStock {
        /// On-hand base micros at decision time.
        available_base_micro: i64,
        /// Requested base micros.
        requested_base_micro: i64,
    },
    /// A privileged sale would drive stock below zero without the
    /// operator's explicit in-session confirmation (TASK-BRK-033).
    #[error("negative stock confirmation required")]
    NegativeStockConfirmationRequired {
        /// Short product id (UUID string).
        product_id: String,
        /// Commercial designation (FR) for the confirmation banner.
        product_name: String,
        /// On-hand base micros at decision time.
        available_micro: i64,
        /// Requested base micros.
        requested_micro: i64,
    },
    /// A concurrency invariant was violated (non-retryable bug signal).
    #[error("concurrency invariant violated")]
    ConcurrencyInvariantViolated,
    /// A credit sale would exceed the authorized limit.
    #[error("credit limit exceeded")]
    CreditLimitExceeded {
        /// Configured limit in centimes.
        limit_centimes: i64,
        /// Proposed balance in centimes.
        proposed_centimes: i64,
    },
    /// Same command id submitted with a different payload.
    #[error("idempotency conflict")]
    IdempotencyConflict,
    /// Customer row not found.
    #[error("customer not found")]
    CustomerNotFound,
    /// A phone number is already registered to another customer.
    #[error("duplicate customer phone")]
    DuplicatePhone,
    /// A credit payment exceeds the outstanding balance.
    #[error("payment exceeds balance")]
    PaymentExceedsBalance {
        /// Current balance in centimes.
        balance_centimes: i64,
        /// Attempted payment in centimes.
        attempted_centimes: i64,
    },
    /// Overpayment requires explicit credit-management authority.
    #[error("overpayment unauthorized")]
    OverpaymentUnauthorized,
}
