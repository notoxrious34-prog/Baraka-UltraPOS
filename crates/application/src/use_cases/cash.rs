//! Cash session lifecycle use cases (ADR-025/026/027).
//!
//! RBAC and pure validation happen here BEFORE any repository contact.
//! All SQL writes (including idempotency and audit) happen inside the
//! repository's single `run_transaction`; this module opens no
//! transaction itself.

use baraka_domain::cash::cash_session::{
    validate_counted_amount, validate_expense_amount, validate_expense_description,
    validate_opening_float,
};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::role::Permission;
use baraka_domain::user::UserId;

use crate::errors::ApplicationError;
use crate::ports::{
    CashMovementView, CashSessionReport, CashSessionRepository, CashSessionView, SessionContext,
    WarehouseRepository,
};

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

fn require(session: &SessionContext, permission: Permission) -> Result<(), ApplicationError> {
    if session.role.has_permission(permission) {
        Ok(())
    } else {
        Err(ApplicationError::Unauthorized)
    }
}

fn canonical_open(
    command_id: &str,
    warehouse_id: WarehouseId,
    opening_float: i64,
    terminal_id: &str,
    actor: UserId,
) -> String {
    format!("v1|cash.open|{command_id}|{warehouse_id}|{opening_float}|{terminal_id}|{actor}")
}

fn canonical_simple(
    command_name: &str,
    command_id: &str,
    terminal_id: &str,
    actor: UserId,
) -> String {
    format!("v1|{command_name}|{command_id}|{terminal_id}|{actor}")
}

/// Open a cash session on this terminal.
pub struct OpenCashSessionUseCase<'a, C, W>
where
    C: ?Sized,
    W: ?Sized,
{
    cash: &'a C,
    warehouses: &'a W,
}

impl<'a, C, W> OpenCashSessionUseCase<'a, C, W>
where
    C: CashSessionRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(cash: &'a C, warehouses: &'a W) -> Self {
        Self { cash, warehouses }
    }

    /// Validate and open the session atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        warehouse_id: WarehouseId,
        opening_float_centimes: i64,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<CashSessionView, ApplicationError> {
        require(session, Permission::CashSessionOpen)?;
        if command_id.trim().is_empty() {
            return Err(coded("INVALID_COMMAND_ID"));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        validate_opening_float(opening_float_centimes).map_err(ApplicationError::Validation)?;
        match self.warehouses.get_by_id(warehouse_id)? {
            Some(warehouse) if warehouse.is_active => {}
            Some(_) => return Err(coded("WAREHOUSE_INACTIVE")),
            None => return Err(ApplicationError::NotFound("warehouse".to_string())),
        }
        let canonical = canonical_open(
            command_id.trim(),
            warehouse_id,
            opening_float_centimes,
            terminal_id.trim(),
            session.user_id,
        );
        self.cash.open_session(
            warehouse_id,
            opening_float_centimes,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Begin the two-phase closing (OPEN -> CLOSING, freeze expected total).
pub struct BeginClosingUseCase<'a, C>
where
    C: ?Sized,
{
    cash: &'a C,
}

impl<'a, C> BeginClosingUseCase<'a, C>
where
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(cash: &'a C) -> Self {
        Self { cash }
    }

    /// Freeze the expected total and enter CLOSING.
    pub fn execute(
        &self,
        session: &SessionContext,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<CashSessionView, ApplicationError> {
        require(session, Permission::CashSessionClose)?;
        if command_id.trim().is_empty() {
            return Err(coded("INVALID_COMMAND_ID"));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let canonical = canonical_simple(
            "cash.beginClosing",
            command_id.trim(),
            terminal_id.trim(),
            session.user_id,
        );
        self.cash.begin_closing(
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Cancel the closing (CLOSING -> OPEN, clear frozen expected value).
pub struct CancelClosingUseCase<'a, C>
where
    C: ?Sized,
{
    cash: &'a C,
}

impl<'a, C> CancelClosingUseCase<'a, C>
where
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(cash: &'a C) -> Self {
        Self { cash }
    }

    /// Revert to OPEN and re-permit sales.
    pub fn execute(
        &self,
        session: &SessionContext,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<CashSessionView, ApplicationError> {
        require(session, Permission::CashSessionClose)?;
        if command_id.trim().is_empty() {
            return Err(coded("INVALID_COMMAND_ID"));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let canonical = canonical_simple(
            "cash.cancelClosing",
            command_id.trim(),
            terminal_id.trim(),
            session.user_id,
        );
        self.cash.cancel_closing(
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Confirm the closing (CLOSING -> CLOSED with variance).
pub struct ConfirmClosingUseCase<'a, C>
where
    C: ?Sized,
{
    cash: &'a C,
}

impl<'a, C> ConfirmClosingUseCase<'a, C>
where
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(cash: &'a C) -> Self {
        Self { cash }
    }

    /// Count, compute variance, and seal the session immutably.
    pub fn execute(
        &self,
        session: &SessionContext,
        session_id: &str,
        actual_counted_centimes: i64,
        closing_notes: Option<String>,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<CashSessionView, ApplicationError> {
        require(session, Permission::CashSessionClose)?;
        if command_id.trim().is_empty() {
            return Err(coded("INVALID_COMMAND_ID"));
        }
        if session_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::CashSessionNotFound,
            ));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        validate_counted_amount(actual_counted_centimes).map_err(ApplicationError::Validation)?;
        let notes = closing_notes
            .map(|note| note.trim().to_string())
            .filter(|note| !note.is_empty());
        let user = session.user_id;
        let canonical = format!(
            "v1|cash.confirmClosing|{}|{}|{}|{}|{}|{user}",
            command_id.trim(),
            session_id.trim(),
            actual_counted_centimes,
            notes.clone().unwrap_or_default(),
            terminal_id.trim(),
        );
        self.cash.confirm_closing(
            session_id.trim(),
            actual_counted_centimes,
            notes,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Read the active session for this terminal (no idempotency key).
pub struct GetActiveSessionUseCase<'a, C>
where
    C: ?Sized,
{
    cash: &'a C,
}

impl<'a, C> GetActiveSessionUseCase<'a, C>
where
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(cash: &'a C) -> Self {
        Self { cash }
    }

    /// Load the OPEN/CLOSING session, if any.
    pub fn execute(
        &self,
        session: &SessionContext,
        terminal_id: &str,
    ) -> Result<Option<CashSessionView>, ApplicationError> {
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let _ = session;
        self.cash.get_active_session(terminal_id.trim())
    }
}

/// Record a drawer expense on the OPEN session.
pub struct RecordExpenseUseCase<'a, C>
where
    C: ?Sized,
{
    cash: &'a C,
}

impl<'a, C> RecordExpenseUseCase<'a, C>
where
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(cash: &'a C) -> Self {
        Self { cash }
    }

    /// Validate and append the EXPENSE_OUT movement atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        amount_centimes: i64,
        description: &str,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<CashMovementView, ApplicationError> {
        require(session, Permission::CashExpenseRecord)?;
        if command_id.trim().is_empty() {
            return Err(coded("INVALID_COMMAND_ID"));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        validate_expense_amount(amount_centimes).map_err(ApplicationError::Validation)?;
        let clean =
            validate_expense_description(description).map_err(ApplicationError::Validation)?;
        let user = session.user_id;
        let canonical = format!(
            "v1|cash.recordExpense|{}|{amount_centimes}|{clean}|{}|{user}",
            command_id.trim(),
            terminal_id.trim(),
        );
        self.cash.record_expense(
            amount_centimes,
            &clean,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Read a session report (interim X-Report or final Z-Report).
pub struct GetSessionReportUseCase<'a, C>
where
    C: ?Sized,
{
    cash: &'a C,
}

impl<'a, C> GetSessionReportUseCase<'a, C>
where
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(cash: &'a C) -> Self {
        Self { cash }
    }

    /// Load header + movements + grouped totals.
    pub fn execute(
        &self,
        session: &SessionContext,
        session_id: &str,
    ) -> Result<CashSessionReport, ApplicationError> {
        require(session, Permission::CashReportView)?;
        if session_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::CashSessionNotFound,
            ));
        }
        self.cash.get_session_report(session_id.trim())
    }
}
