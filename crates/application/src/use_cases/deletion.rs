//! Dual-track safe deletion (ADR-092, TASK-BRK-074).
//!
//! Entities with zero transactional references are permanently purged
//! (hard delete); entities with historical documents, movements, or ledger
//! rows are soft-deactivated (`is_active = false`) so append-only ledgers
//! and foreign-key integrity are never violated. Invariant guards:
//! DEP-01 is immutable, and customers carrying a non-zero balance can
//! neither be deleted nor deactivated.

use baraka_domain::inventory::WarehouseId;
use baraka_domain::role::Permission;
use baraka_domain::sales::CustomerId;

use crate::errors::ApplicationError;
use crate::ports::{
    CustomerRepository, DeleteAudit, ProductRepository, SessionContext, SupplierRepository,
    WarehouseRepository,
};
use crate::use_cases::catalog::{require_any, MANAGE_PERMISSIONS};

use baraka_domain::catalog::ProductId;

fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(baraka_domain::errors::DomainError::InvalidCanonicalString(
        code.to_string(),
    ))
}

fn require(session: &SessionContext, permission: Permission) -> Result<(), ApplicationError> {
    if session.role.has_permission(permission) {
        Ok(())
    } else {
        Err(ApplicationError::Unauthorized)
    }
}

/// Purge paths are irreversible: require a non-empty operator reason
/// (min 3 chars). NOTE: there is no `DomainError::ValidationError` variant
/// in this tree — the live convention for empty input is
/// `InvalidCanonicalString("{field} must …")` (`ipc.rs:434`), which the
/// command layer maps to `INVALID_INPUT`.
fn check_purge_reason(reason: &str) -> Result<(), ApplicationError> {
    if reason.trim().len() < 3 {
        return Err(ApplicationError::Validation(
            baraka_domain::errors::DomainError::InvalidCanonicalString(
                "reason must be at least 3 characters".to_string(),
            ),
        ));
    }
    Ok(())
}

fn audit_ctx(session: &SessionContext, reason: &str, old_json: String) -> DeleteAudit {
    DeleteAudit {
        actor: session.user_id.to_string(),
        reason: reason.to_string(),
        old_json,
    }
}

fn json_escape(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Dual-track deletion outcome (mirrors `DeleteEntityResultDto`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteEntityOutcome {
    /// Deleted or deactivated entity id.
    pub id: String,
    /// `"hard_deleted"` or `"deactivated"`.
    pub mode: &'static str,
    /// Stable wire message (French copy resolves in the frontend).
    pub message: &'static str,
}

/// Principal depot code: immutable to deletion and deactivation.
pub const PRINCIPAL_DEPOT_CODE: &str = "DEP-01";

/// Delete a product: hard purge when unreferenced, deactivation otherwise.
pub struct DeleteProductUseCase<'a, P>
where
    P: ?Sized,
{
    products: &'a P,
}

impl<'a, P> DeleteProductUseCase<'a, P>
where
    P: ProductRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(products: &'a P) -> Self {
        Self { products }
    }

    /// Execute the dual-track deletion.
    pub fn execute(
        &self,
        session: &SessionContext,
        id: ProductId,
        reason: &str,
    ) -> Result<DeleteEntityOutcome, ApplicationError> {
        require_any(session, &MANAGE_PERMISSIONS)?;
        if self.products.find_by_id(id)?.is_none() {
            return Err(ApplicationError::NotFound("product".to_string()));
        }
        if self.products.count_product_references(id)? > 0 {
            self.products.set_product_active(id, false)?;
            return Ok(DeleteEntityOutcome {
                id: id.to_string(),
                mode: "deactivated",
                message: "PRODUCT_DEACTIVATED_HISTORY",
            });
        }
        // Purge is irreversible: reason required even though the product
        // adapter cannot yet persist the audit row in-tx (BRK-076 owns
        // `sqlite_catalog_repository.rs` uncommitted; audit emission for
        // products is REMAINING work after it lands).
        check_purge_reason(reason)?;
        self.products.hard_delete_product(id)?;
        Ok(DeleteEntityOutcome {
            id: id.to_string(),
            mode: "hard_deleted",
            message: "PRODUCT_HARD_DELETED",
        })
    }
}

/// Delete a customer: balance guard first, then dual-track deletion.
pub struct DeleteCustomerUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> DeleteCustomerUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// Execute the guarded dual-track deletion.
    pub fn execute(
        &self,
        session: &SessionContext,
        id: CustomerId,
        reason: &str,
    ) -> Result<DeleteEntityOutcome, ApplicationError> {
        require(session, Permission::ManageCustomerCredit)?;
        let profile = self
            .customers
            .get_credit_profile(id)?
            .ok_or_else(|| ApplicationError::NotFound("customer".to_string()))?;
        if profile.current_balance_centimes != 0 {
            return Err(coded("CUSTOMER_BALANCE_NONZERO"));
        }
        let old_json = format!("{{\"id\":\"{}\"}}", json_escape(&id.to_string()));
        let audit = audit_ctx(session, reason, old_json);
        if self.customers.count_customer_references(id)? > 0 {
            self.customers.set_customer_active(id, false, &audit)?;
            return Ok(DeleteEntityOutcome {
                id: id.to_string(),
                mode: "deactivated",
                message: "CUSTOMER_DEACTIVATED_HISTORY",
            });
        }
        check_purge_reason(reason)?;
        self.customers.hard_delete_customer(id, &audit)?;
        Ok(DeleteEntityOutcome {
            id: id.to_string(),
            mode: "hard_deleted",
            message: "CUSTOMER_HARD_DELETED",
        })
    }
}

/// Delete a supplier: hard purge when unreferenced, deactivation otherwise.
pub struct DeleteSupplierUseCase<'a, S>
where
    S: ?Sized,
{
    suppliers: &'a S,
}

impl<'a, S> DeleteSupplierUseCase<'a, S>
where
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(suppliers: &'a S) -> Self {
        Self { suppliers }
    }

    /// Execute the dual-track deletion.
    pub fn execute(
        &self,
        session: &SessionContext,
        id: &str,
        reason: &str,
    ) -> Result<DeleteEntityOutcome, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        let supplier = self
            .suppliers
            .get_supplier(id)?
            .ok_or_else(|| ApplicationError::NotFound("supplier".to_string()))?;
        if supplier.current_balance_centimes != 0 {
            return Err(coded("SUPPLIER_BALANCE_NONZERO"));
        }
        let old_json = format!(
            "{{\"id\":\"{}\",\"name\":\"{}\"}}",
            json_escape(&supplier.id),
            json_escape(&supplier.name)
        );
        let audit = audit_ctx(session, reason, old_json);
        if self.suppliers.count_supplier_references(id)? > 0 {
            self.suppliers.set_supplier_active(id, false, &audit)?;
            return Ok(DeleteEntityOutcome {
                id: id.to_string(),
                mode: "deactivated",
                message: "SUPPLIER_DEACTIVATED_HISTORY",
            });
        }
        check_purge_reason(reason)?;
        self.suppliers.hard_delete_supplier(id, &audit)?;
        Ok(DeleteEntityOutcome {
            id: id.to_string(),
            mode: "hard_deleted",
            message: "SUPPLIER_HARD_DELETED",
        })
    }
}

/// Delete a warehouse: DEP-01 immutable, non-zero stock blocks, then
/// dual-track deletion.
pub struct DeleteWarehouseUseCase<'a, W>
where
    W: ?Sized,
{
    warehouses: &'a W,
}

impl<'a, W> DeleteWarehouseUseCase<'a, W>
where
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(warehouses: &'a W) -> Self {
        Self { warehouses }
    }

    /// Execute the guarded dual-track deletion.
    pub fn execute(
        &self,
        session: &SessionContext,
        id: WarehouseId,
        reason: &str,
    ) -> Result<DeleteEntityOutcome, ApplicationError> {
        require(session, Permission::ManageWarehouses)?;
        let warehouse = self
            .warehouses
            .get_by_id(id)?
            .ok_or_else(|| ApplicationError::NotFound("warehouse".to_string()))?;
        if warehouse.code.as_str() == PRINCIPAL_DEPOT_CODE {
            return Err(coded("DEPOT_PRINCIPAL_IMMUTABLE"));
        }
        if self.warehouses.count_warehouse_nonzero_levels(id)? > 0 {
            return Err(coded("WAREHOUSE_HAS_STOCK"));
        }
        let old_json = format!(
            "{{\"id\":\"{}\",\"name\":\"{}\"}}",
            json_escape(&id.to_string()),
            json_escape(&warehouse.name)
        );
        let audit = audit_ctx(session, reason, old_json);
        if self.warehouses.count_warehouse_references(id)? > 0 {
            self.warehouses.set_warehouse_active(id, false, &audit)?;
            return Ok(DeleteEntityOutcome {
                id: id.to_string(),
                mode: "deactivated",
                message: "WAREHOUSE_DEACTIVATED_HISTORY",
            });
        }
        check_purge_reason(reason)?;
        self.warehouses.hard_delete_warehouse(id, &audit)?;
        Ok(DeleteEntityOutcome {
            id: id.to_string(),
            mode: "hard_deleted",
            message: "WAREHOUSE_HARD_DELETED",
        })
    }
}

/// Rename and/or (de)activate a warehouse (ADMIN only, DEP-01 guarded).
pub struct UpdateWarehouseUseCase<'a, W>
where
    W: ?Sized,
{
    warehouses: &'a W,
}

impl<'a, W> UpdateWarehouseUseCase<'a, W>
where
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(warehouses: &'a W) -> Self {
        Self { warehouses }
    }

    /// Apply the sparse update after the DEP-01 deactivation guard.
    pub fn execute(
        &self,
        session: &SessionContext,
        id: WarehouseId,
        name: Option<String>,
        is_active: Option<bool>,
    ) -> Result<(), ApplicationError> {
        require(session, Permission::ManageWarehouses)?;
        let warehouse = self
            .warehouses
            .get_by_id(id)?
            .ok_or_else(|| ApplicationError::NotFound("warehouse".to_string()))?;
        if warehouse.code.as_str() == PRINCIPAL_DEPOT_CODE && is_active == Some(false) {
            return Err(coded("DEPOT_PRINCIPAL_IMMUTABLE"));
        }
        let clean_name = name
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());
        if let Some(ref text) = clean_name {
            baraka_domain::inventory::Warehouse::validate_name(text)
                .map_err(ApplicationError::Validation)?;
        }
        self.warehouses
            .update_warehouse(id, clean_name.as_deref(), is_active)?;
        Ok(())
    }
}
