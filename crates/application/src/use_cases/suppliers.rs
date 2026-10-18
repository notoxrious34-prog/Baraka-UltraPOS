//! Supplier management use cases: master data and statements.
//!
//! RBAC is enforced before any repository contact: all six operations need
//! `SupplierManage` (Gerant/Admin only). Search, get, and statement mirror
//! the exact established customer-module pattern (TASK-BRK-009), including
//! its LIKE-based search.

use baraka_domain::errors::DomainError;
use baraka_domain::role::Permission;
use baraka_domain::suppliers::{validate_supplier_name, validate_supplier_optional_text};

use crate::errors::ApplicationError;
use crate::ports::{
    NewSupplier, SessionContext, SupplierRepository, SupplierStatement, SupplierUpdate,
    SupplierView,
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

/// Validate one optional sparse field (`Some(None)` clears; blank clears).
fn optional_field(
    field: &'static str,
    max_chars: usize,
    slot: &Option<Option<String>>,
) -> Result<Option<Option<String>>, ApplicationError> {
    match slot {
        None => Ok(None),
        Some(value) => Ok(Some(
            validate_supplier_optional_text(field, value.as_deref(), max_chars)
                .map_err(ApplicationError::Validation)?,
        )),
    }
}

/// Register a supplier.
pub struct CreateSupplierUseCase<'a, S>
where
    S: ?Sized,
{
    suppliers: &'a S,
}

impl<'a, S> CreateSupplierUseCase<'a, S>
where
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(suppliers: &'a S) -> Self {
        Self { suppliers }
    }

    /// Validate and persist a supplier atomically.
    /// `terminal_id` is TerminalContext-resolved (ADR-028, never client input).
    pub fn execute(
        &self,
        session: &SessionContext,
        input: NewSupplier,
        terminal_id: &str,
    ) -> Result<SupplierView, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        let name = validate_supplier_name(&input.name).map_err(ApplicationError::Validation)?;
        let phone = validate_supplier_optional_text("phone", input.phone.as_deref(), 20)
            .map_err(ApplicationError::Validation)?;
        let address = validate_supplier_optional_text("address", input.address.as_deref(), 200)
            .map_err(ApplicationError::Validation)?;
        let nif = validate_supplier_optional_text("nif", input.nif.as_deref(), 20)
            .map_err(ApplicationError::Validation)?;
        let rc = validate_supplier_optional_text("rc", input.rc.as_deref(), 20)
            .map_err(ApplicationError::Validation)?;
        let notes = validate_supplier_optional_text("notes", input.notes.as_deref(), 500)
            .map_err(ApplicationError::Validation)?;
        self.suppliers.create_supplier(
            &NewSupplier {
                name,
                phone,
                address,
                nif,
                rc,
                notes,
            },
            terminal_id.trim(),
        )
    }
}

/// Update mutable supplier fields.
pub struct UpdateSupplierUseCase<'a, S>
where
    S: ?Sized,
{
    suppliers: &'a S,
}

impl<'a, S> UpdateSupplierUseCase<'a, S>
where
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(suppliers: &'a S) -> Self {
        Self { suppliers }
    }

    /// Validate and persist the sparse update.
    pub fn execute(
        &self,
        session: &SessionContext,
        supplier_id: &str,
        input: SupplierUpdate,
    ) -> Result<SupplierView, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        if supplier_id.trim().is_empty() {
            return Err(ApplicationError::Validation(DomainError::SupplierNotFound));
        }
        let mut update = SupplierUpdate::default();
        if let Some(name) = input.name.as_deref() {
            update.name = Some(validate_supplier_name(name).map_err(ApplicationError::Validation)?);
        }
        update.phone = optional_field("phone", 20, &input.phone)?;
        update.address = optional_field("address", 200, &input.address)?;
        update.nif = optional_field("nif", 20, &input.nif)?;
        update.rc = optional_field("rc", 20, &input.rc)?;
        update.notes = optional_field("notes", 500, &input.notes)?;
        if let Some(is_active) = input.is_active {
            update.is_active = Some(is_active);
        }
        self.suppliers.update_supplier(supplier_id.trim(), &update)
    }
}

/// Search the supplier directory.
pub struct SearchSuppliersUseCase<'a, S>
where
    S: ?Sized,
{
    suppliers: &'a S,
}

impl<'a, S> SearchSuppliersUseCase<'a, S>
where
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(suppliers: &'a S) -> Self {
        Self { suppliers }
    }

    /// Prefix/multilingual search (blank queries return no rows).
    pub fn execute(
        &self,
        session: &SessionContext,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SupplierView>, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        self.suppliers
            .search_suppliers(trimmed, limit.clamp(1, 100))
    }
}

/// List the supplier directory ordered by name.
pub struct ListSuppliersUseCase<'a, S>
where
    S: ?Sized,
{
    suppliers: &'a S,
}

impl<'a, S> ListSuppliersUseCase<'a, S>
where
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(suppliers: &'a S) -> Self {
        Self { suppliers }
    }

    /// List suppliers ordered by name.
    pub fn execute(
        &self,
        session: &SessionContext,
        active_only: bool,
    ) -> Result<Vec<SupplierView>, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        self.suppliers.list_suppliers(active_only)
    }
}

/// Fetch one full supplier view.
pub struct GetSupplierUseCase<'a, S>
where
    S: ?Sized,
{
    suppliers: &'a S,
}

impl<'a, S> GetSupplierUseCase<'a, S>
where
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(suppliers: &'a S) -> Self {
        Self { suppliers }
    }

    /// Load the detail view or fail with `SupplierNotFound`.
    pub fn execute(
        &self,
        session: &SessionContext,
        supplier_id: &str,
    ) -> Result<SupplierView, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        self.suppliers
            .get_supplier(supplier_id.trim())?
            .ok_or(ApplicationError::Validation(DomainError::SupplierNotFound))
    }
}

/// Read a supplier statement with server-side running balances.
pub struct GetSupplierStatementUseCase<'a, S>
where
    S: ?Sized,
{
    suppliers: &'a S,
}

impl<'a, S> GetSupplierStatementUseCase<'a, S>
where
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(suppliers: &'a S) -> Self {
        Self { suppliers }
    }

    /// Load ledger rows in ledger order.
    pub fn execute(
        &self,
        session: &SessionContext,
        supplier_id: &str,
    ) -> Result<SupplierStatement, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        self.suppliers
            .get_statement(supplier_id.trim())?
            .ok_or(ApplicationError::Validation(DomainError::SupplierNotFound))
    }
}
