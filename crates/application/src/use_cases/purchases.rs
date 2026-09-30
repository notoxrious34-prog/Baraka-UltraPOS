//! Purchase use cases: BC orders, BR receipts, FF invoices, payments.
//!
//! RBAC is enforced before any repository contact (all five purchase
//! permissions are Gerant/Admin only). Validation is exhaustive at the
//! use-case boundary; every additive side effect (document, stock, ledger,
//! cash) happens inside ONE repository transaction with Lookup-then-Append
//! idempotency. ADR-029 (FF sourced from a BR must not touch stock) is
//! enforced structurally: the branch lives in the repository commit path.

use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::role::Permission;

use crate::errors::ApplicationError;
use crate::ports::{
    PurchaseDocumentView, PurchaseLineInput, PurchaseRepository, SessionContext,
    SupplierPaymentOutcome, SupplierRepository, WarehouseRepository,
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

/// Validate a command id / line set shared by every purchase mutation.
fn validate_command_and_lines(
    command_id: &str,
    lines: &[PurchaseLineInput],
) -> Result<(), ApplicationError> {
    if command_id.trim().is_empty() {
        return Err(coded("INVALID_COMMAND_ID"));
    }
    if lines.is_empty() {
        return Err(coded("PURCHASE_LINES_REQUIRED"));
    }
    for line in lines {
        if line.quantity_micro <= 0 {
            return Err(ApplicationError::Validation(
                DomainError::PurchaseLineQuantityInvalid,
            ));
        }
        if line.unit_cost_centimes < 0 {
            return Err(coded("PURCHASE_LINE_COST_INVALID"));
        }
    }
    Ok(())
}

/// Shared pre-touch validation of supplier + warehouse endpoints.
fn validate_endpoints<S, W>(
    suppliers: &S,
    warehouses: &W,
    supplier_id: &str,
    warehouse_id: WarehouseId,
) -> Result<(), ApplicationError>
where
    S: SupplierRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    if supplier_id.trim().is_empty() {
        return Err(ApplicationError::Validation(DomainError::SupplierNotFound));
    }
    let supplier = suppliers
        .get_supplier(supplier_id.trim())?
        .ok_or(ApplicationError::Validation(DomainError::SupplierNotFound))?;
    if !supplier.is_active {
        return Err(ApplicationError::Validation(DomainError::SupplierInactive));
    }
    match warehouses.get_by_id(warehouse_id)? {
        Some(warehouse) if warehouse.is_active => {}
        Some(_) => return Err(coded("WAREHOUSE_INACTIVE")),
        None => return Err(ApplicationError::NotFound("warehouse".to_string())),
    }
    Ok(())
}

/// Deterministic canonical request string for purchase idempotency.
fn canonical_lines(prefix: &str, command_id: &str, lines: &[PurchaseLineInput]) -> String {
    let mut parts = vec![format!("v1|{prefix}|{command_id}")];
    for line in lines {
        parts.push(format!(
            "{}|{}|{}|{}",
            line.product_id, line.unit_id, line.quantity_micro, line.unit_cost_centimes
        ));
    }
    parts.join(";")
}

/// Create a purchase order (BC): pure intent, zero stock/ledger effect.
pub struct CreatePurchaseOrderUseCase<'a, R, S, W>
where
    R: ?Sized,
    S: ?Sized,
    W: ?Sized,
{
    purchases: &'a R,
    suppliers: &'a S,
    warehouses: &'a W,
}

impl<'a, R, S, W> CreatePurchaseOrderUseCase<'a, R, S, W>
where
    R: PurchaseRepository + ?Sized,
    S: SupplierRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(purchases: &'a R, suppliers: &'a S, warehouses: &'a W) -> Self {
        Self {
            purchases,
            suppliers,
            warehouses,
        }
    }

    /// Validate, authorize, and persist the BC atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: Vec<PurchaseLineInput>,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        require(session, Permission::PurchaseOrderCreate)?;
        validate_command_and_lines(command_id, &lines)?;
        validate_endpoints(self.suppliers, self.warehouses, supplier_id, warehouse_id)?;
        let canonical = canonical_lines("purchases.createOrder", command_id.trim(), &lines);
        self.purchases.create_order(
            supplier_id.trim(),
            warehouse_id,
            &lines,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Replace the line set of a DRAFT purchase order.
pub struct UpdatePurchaseOrderUseCase<'a, R, S, W>
where
    R: ?Sized,
    S: ?Sized,
    W: ?Sized,
{
    purchases: &'a R,
    suppliers: &'a S,
    warehouses: &'a W,
}

impl<'a, R, S, W> UpdatePurchaseOrderUseCase<'a, R, S, W>
where
    R: PurchaseRepository + ?Sized,
    S: SupplierRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(purchases: &'a R, suppliers: &'a S, warehouses: &'a W) -> Self {
        Self {
            purchases,
            suppliers,
            warehouses,
        }
    }

    /// Validate, authorize, and replace the DRAFT line set atomically.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        document_id: &str,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: Vec<PurchaseLineInput>,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        require(session, Permission::PurchaseOrderCreate)?;
        validate_command_and_lines(command_id, &lines)?;
        validate_endpoints(self.suppliers, self.warehouses, supplier_id, warehouse_id)?;
        if document_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::PurchaseOrderNotFound,
            ));
        }
        let canonical = canonical_lines("purchases.updateOrder", command_id.trim(), &lines);
        self.purchases.update_order(
            document_id.trim(),
            &lines,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Receive goods: immediate BR commit + PURCHASE_IN stock effect.
pub struct ReceiveGoodsUseCase<'a, R, S, W>
where
    R: ?Sized,
    S: ?Sized,
    W: ?Sized,
{
    purchases: &'a R,
    suppliers: &'a S,
    warehouses: &'a W,
}

impl<'a, R, S, W> ReceiveGoodsUseCase<'a, R, S, W>
where
    R: PurchaseRepository + ?Sized,
    S: SupplierRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(purchases: &'a R, suppliers: &'a S, warehouses: &'a W) -> Self {
        Self {
            purchases,
            suppliers,
            warehouses,
        }
    }

    /// Validate, authorize, and commit the receipt atomically.
    /// Over-receipt against a BC requires the Gerant override permission.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        source_bc_id: Option<String>,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: Vec<PurchaseLineInput>,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        require(session, Permission::PurchaseReceive)?;
        validate_command_and_lines(command_id, &lines)?;
        validate_endpoints(self.suppliers, self.warehouses, supplier_id, warehouse_id)?;
        let override_allowed = session.role.has_permission(Permission::PriceOverride);
        let canonical = format!(
            "{};O:{};X:{};T:{}",
            canonical_lines("purchases.receiveGoods", command_id.trim(), &lines),
            source_bc_id.as_deref().unwrap_or(""),
            u8::from(override_allowed),
            terminal_id.trim(),
        );
        self.purchases.receive_goods(
            source_bc_id.as_deref().map(str::trim),
            supplier_id.trim(),
            warehouse_id,
            &lines,
            override_allowed,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Record a supplier invoice (FF): the single ledger DEBIT entry point.
pub struct RecordSupplierInvoiceUseCase<'a, R, S, W>
where
    R: ?Sized,
    S: ?Sized,
    W: ?Sized,
{
    purchases: &'a R,
    suppliers: &'a S,
    warehouses: &'a W,
}

impl<'a, R, S, W> RecordSupplierInvoiceUseCase<'a, R, S, W>
where
    R: PurchaseRepository + ?Sized,
    S: SupplierRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(purchases: &'a R, suppliers: &'a S, warehouses: &'a W) -> Self {
        Self {
            purchases,
            suppliers,
            warehouses,
        }
    }

    /// Validate, authorize, and commit the invoice atomically (ADR-029).
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        source_br_id: Option<String>,
        supplier_id: &str,
        warehouse_id: WarehouseId,
        lines: Vec<PurchaseLineInput>,
        invoice_total_centimes: i64,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        require(session, Permission::PurchaseInvoiceRecord)?;
        validate_command_and_lines(command_id, &lines)?;
        validate_endpoints(self.suppliers, self.warehouses, supplier_id, warehouse_id)?;
        if invoice_total_centimes <= 0 {
            return Err(coded("INVALID_PAYMENT_AMOUNT"));
        }
        let canonical = format!(
            "{};S:{};A:{};T:{}",
            canonical_lines("purchases.recordInvoice", command_id.trim(), &lines),
            source_br_id.as_deref().unwrap_or(""),
            invoice_total_centimes,
            terminal_id.trim(),
        );
        self.purchases.record_invoice(
            source_br_id.as_deref().map(str::trim),
            supplier_id.trim(),
            warehouse_id,
            &lines,
            invoice_total_centimes,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Record a supplier payment (ledger CREDIT + optional drawer outflow).
pub struct RecordSupplierPaymentUseCase<'a, R, S>
where
    R: ?Sized,
    S: ?Sized,
{
    purchases: &'a R,
    suppliers: &'a S,
}

impl<'a, R, S> RecordSupplierPaymentUseCase<'a, R, S>
where
    R: PurchaseRepository + ?Sized,
    S: SupplierRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(purchases: &'a R, suppliers: &'a S) -> Self {
        Self {
            purchases,
            suppliers,
        }
    }

    /// Validate, authorize, and commit the payment atomically (ADR-030).
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        supplier_id: &str,
        amount_centimes: i64,
        payment_method: &str,
        description: Option<String>,
        terminal_id: &str,
        command_id: &str,
    ) -> Result<SupplierPaymentOutcome, ApplicationError> {
        require(session, Permission::SupplierPaymentRecord)?;
        if command_id.trim().is_empty() {
            return Err(coded("INVALID_COMMAND_ID"));
        }
        if supplier_id.trim().is_empty() {
            return Err(ApplicationError::Validation(DomainError::SupplierNotFound));
        }
        if amount_centimes <= 0 {
            return Err(ApplicationError::Validation(
                DomainError::SupplierPaymentInvalidAmount,
            ));
        }
        let method = payment_method.trim().to_ascii_uppercase();
        if !matches!(method.as_str(), "CASH" | "OTHER") {
            return Err(ApplicationError::Validation(
                DomainError::SupplierPaymentMethodInvalid,
            ));
        }
        let description = description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let supplier = self
            .suppliers
            .get_supplier(supplier_id.trim())?
            .ok_or(ApplicationError::Validation(DomainError::SupplierNotFound))?;
        if !supplier.is_active {
            return Err(ApplicationError::Validation(DomainError::SupplierInactive));
        }
        let canonical = format!(
            "v1|purchases.recordPayment|{}|{}|{amount_centimes}|{method}|{}|{}",
            command_id.trim(),
            supplier_id.trim(),
            description.clone().unwrap_or_default(),
            terminal_id.trim(),
        );
        self.purchases.record_payment(
            supplier_id.trim(),
            amount_centimes,
            &method,
            description,
            terminal_id.trim(),
            command_id.trim(),
            &canonical,
            session.user_id,
        )
    }
}

/// Read any purchase document (BC/BR/FF) with its lines.
pub struct GetPurchaseDocumentUseCase<'a, R>
where
    R: ?Sized,
{
    purchases: &'a R,
}

impl<'a, R> GetPurchaseDocumentUseCase<'a, R>
where
    R: PurchaseRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(purchases: &'a R) -> Self {
        Self { purchases }
    }

    /// Load the document or fail with `PurchaseOrderNotFound`.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        document_id: &str,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        require(session, Permission::SupplierManage)?;
        if document_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::PurchaseOrderNotFound,
            ));
        }
        self.purchases
            .get_document(document_id.trim())?
            .ok_or(ApplicationError::Validation(
                DomainError::PurchaseOrderNotFound,
            ))
    }
}
