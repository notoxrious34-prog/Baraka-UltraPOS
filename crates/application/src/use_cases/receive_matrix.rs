//! Matrix receiving (DSN-109): a size run lands through the model's
//! default selling units.
//!
//! The use case resolves every requested cell against the matrix view,
//! books each size in its default unit, and delegates to the atomic
//! `ReceiveGoodsUseCase` commit — one transaction, idempotent on the
//! command id. Unknown cells, cells without a default unit, and empty
//! runs refuse LOUDLY before any stock moves.

use baraka_domain::catalog::VariantModelId;
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::role::Permission;

use crate::errors::ApplicationError;
use crate::ports::{
    PurchaseDocumentView, PurchaseLineInput, PurchaseRepository, SessionContext,
    SupplierRepository, VariantMatrixRepository, WarehouseRepository,
};
use crate::use_cases::purchases::ReceiveGoodsUseCase;

/// One size cell to receive: quantity in default-unit micros, unit cost
/// per that same unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixReceiveCellInput {
    /// First-axis value (e.g. `"M"`).
    pub axis1: String,
    /// Second-axis value (e.g. `"Rouge"`).
    pub axis2: String,
    /// Quantity in default-selling-unit micros (must be > 0).
    pub quantity_micro: i64,
    /// Unit cost in centimes per default unit (>= 0).
    pub unit_cost_centimes: i64,
}

/// Size-run receipt input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveMatrixGoodsInput {
    /// Supplier credited with the receipt.
    pub supplier_id: String,
    /// Warehouse receiving the stock.
    pub warehouse_id: WarehouseId,
    /// Variant model the cells belong to.
    pub model_id: VariantModelId,
    /// Non-empty cell set.
    pub cells: Vec<MatrixReceiveCellInput>,
    /// Source purchase order, if the receipt settles one.
    pub source_bc_id: Option<String>,
    /// TerminalContext-resolved terminal id (never client-supplied).
    pub terminal_id: String,
    /// Idempotency key (Lookup-then-Append downstream).
    pub command_id: String,
}

/// Receive a matrix size run atomically.
pub struct ReceiveMatrixGoodsUseCase<'a, R, S, W, V>
where
    R: ?Sized,
    S: ?Sized,
    W: ?Sized,
    V: ?Sized,
{
    purchases: &'a R,
    suppliers: &'a S,
    warehouses: &'a W,
    matrix: &'a V,
}

impl<'a, R, S, W, V> ReceiveMatrixGoodsUseCase<'a, R, S, W, V>
where
    R: PurchaseRepository + ?Sized,
    S: SupplierRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
    V: VariantMatrixRepository + ?Sized,
{
    /// Wire the four ports (purchases, suppliers, warehouses, matrix).
    #[must_use]
    pub const fn new(purchases: &'a R, suppliers: &'a S, warehouses: &'a W, matrix: &'a V) -> Self {
        Self {
            purchases,
            suppliers,
            warehouses,
            matrix,
        }
    }

    /// Resolve cells against the model and commit one atomic BR.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: ReceiveMatrixGoodsInput,
    ) -> Result<PurchaseDocumentView, ApplicationError> {
        if !session.role.has_permission(Permission::PurchaseReceive) {
            return Err(ApplicationError::Unauthorized);
        }
        if input.cells.is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("EMPTY_MATRIX_RUN".to_string()),
            ));
        }
        let rows = self.matrix.matrix_view(input.model_id)?;
        let mut lines = Vec::with_capacity(input.cells.len());
        for cell in &input.cells {
            let row = rows
                .iter()
                .find(|row| row.axis1_value == cell.axis1 && row.axis2_value == cell.axis2)
                .ok_or_else(|| {
                    ApplicationError::NotFound(format!(
                        "matrix cell {} / {}",
                        cell.axis1, cell.axis2
                    ))
                })?;
            let unit = row.default_unit.as_ref().ok_or_else(|| {
                ApplicationError::Validation(DomainError::InvalidCanonicalString(format!(
                    "matrix cell {} / {} has no default unit",
                    cell.axis1, cell.axis2
                )))
            })?;
            if cell.quantity_micro <= 0 {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidCanonicalString("INVALID_CELL_QUANTITY".to_string()),
                ));
            }
            if cell.unit_cost_centimes < 0 {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidCanonicalString("INVALID_CELL_COST".to_string()),
                ));
            }
            lines.push(PurchaseLineInput {
                product_id: row.product_id,
                unit_id: unit.unit_id,
                quantity_micro: cell.quantity_micro,
                unit_cost_centimes: cell.unit_cost_centimes,
            });
        }
        ReceiveGoodsUseCase::new(self.purchases, self.suppliers, self.warehouses).execute(
            session,
            input.source_bc_id,
            &input.supplier_id,
            input.warehouse_id,
            lines,
            &input.terminal_id,
            &input.command_id,
        )
    }
}
