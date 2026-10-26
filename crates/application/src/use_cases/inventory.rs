//! Inventory use cases: warehouses, adjustments, transfers, reads.
//!
//! Every mutating command verifies the session permission BEFORE any
//! database transaction opens. Plain list reads require an authenticated
//! session only (enforced at the IPC boundary); the stock-presence read is
//! the exception — it shares the deletion path's `ManageWarehouses` gate
//! because it exists solely to drive the delete pre-check.

use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{
    StockMovementType, Warehouse, WarehouseCode, WarehouseId, WarehouseStockPresence,
};
use baraka_domain::role::Permission;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::ports::{
    SessionContext, StockMovementCommand, StockRepository, TransferCommand, TransferResult,
    WarehouseRepository,
};

/// Adjustment direction (explicit; sign is never inferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StockDirection {
    /// Grow stock (`ADJUSTMENT_POSITIVE`).
    Increase,
    /// Shrink stock (`ADJUSTMENT_NEGATIVE`).
    Decrease,
}

/// Stock adjustment payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustStockInput {
    /// Physical product.
    pub product_id: baraka_domain::catalog::ProductId,
    /// Target warehouse.
    pub warehouse_id: WarehouseId,
    /// Handled selling unit.
    pub unit_id: baraka_domain::catalog::UnitId,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: i64,
    /// Explicit direction.
    pub direction: StockDirection,
    /// Mandatory human reason (min 3 chars after trim).
    pub reason: String,
    /// Request to allow a negative resulting level.
    pub authorize_negative_stock: bool,
}

/// Stock adjustment outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustStockOutcome {
    /// Stored movement id.
    pub movement_id: String,
    /// Level after the movement, in base micros.
    pub new_quantity_base_micro: i64,
}

/// Warehouse-to-warehouse transfer payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferStockInput {
    /// Source warehouse (must hold the stock).
    pub source_warehouse_id: WarehouseId,
    /// Destination warehouse.
    pub destination_warehouse_id: WarehouseId,
    /// Physical product.
    pub product_id: baraka_domain::catalog::ProductId,
    /// Handled selling unit.
    pub unit_id: baraka_domain::catalog::UnitId,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: i64,
    /// Optional human reason.
    pub reason: Option<String>,
}

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// Create a warehouse (ADMIN only).
pub struct CreateWarehouseUseCase<'a, W>
where
    W: ?Sized,
{
    warehouses: &'a W,
}

impl<'a, W> CreateWarehouseUseCase<'a, W>
where
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(warehouses: &'a W) -> Self {
        Self { warehouses }
    }

    /// Validate and persist a warehouse.
    /// `terminal_id` is TerminalContext-resolved (never client-supplied).
    pub fn execute(
        &self,
        session: &SessionContext,
        code: &str,
        name: &str,
        terminal_id: &str,
    ) -> Result<WarehouseId, ApplicationError> {
        if !session.role.has_permission(Permission::ManageWarehouses) {
            return Err(ApplicationError::Unauthorized);
        }
        if terminal_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_TERMINAL_ID".to_string()),
            ));
        }
        let validated_code = WarehouseCode::new(code).map_err(ApplicationError::Validation)?;
        Warehouse::validate_name(name).map_err(ApplicationError::Validation)?;
        if self.warehouses.get_by_code(&validated_code)?.is_some() {
            return Err(ApplicationError::Conflict(
                "WAREHOUSE_CODE_EXISTS".to_string(),
            ));
        }
        let warehouse = Warehouse {
            id: WarehouseId::new(),
            code: validated_code,
            name: name.trim().to_string(),
            is_active: true,
        };
        self.warehouses
            .create_warehouse(&warehouse, terminal_id.trim())?;
        Ok(warehouse.id)
    }
}

/// List active warehouses (any authenticated session).
pub struct ListWarehousesUseCase<'a, W>
where
    W: ?Sized,
{
    warehouses: &'a W,
}

impl<'a, W> ListWarehousesUseCase<'a, W>
where
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(warehouses: &'a W) -> Self {
        Self { warehouses }
    }

    /// Return active warehouses ordered by code.
    pub fn execute(&self, _session: &SessionContext) -> Result<Vec<Warehouse>, ApplicationError> {
        self.warehouses.list_active()
    }
}

/// Depot stock-presence read (BRK-095): non-zero level counts for every
/// active depot in one GROUP BY query. Same gate as the depot deletion
/// read path (`ManageWarehouses`, mirroring `DeleteWarehouseUseCase`);
/// performs zero writes. DEP-01 is reported truthfully — the lock badge
/// stays a UI decision, never this use case's.
pub struct WarehouseStockPresenceUseCase<'a, W>
where
    W: ?Sized,
{
    warehouses: &'a W,
}

impl<'a, W> WarehouseStockPresenceUseCase<'a, W>
where
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(warehouses: &'a W) -> Self {
        Self { warehouses }
    }

    /// Non-zero level counts, code-ordered, zero-count depots included.
    pub fn execute(
        &self,
        session: &SessionContext,
    ) -> Result<Vec<WarehouseStockPresence>, ApplicationError> {
        if !session.role.has_permission(Permission::ManageWarehouses) {
            return Err(ApplicationError::Unauthorized);
        }
        self.warehouses.active_nonzero_level_counts()
    }
}

/// Read a stock level (any authenticated session; 0 when absent).
pub struct GetStockLevelUseCase<'a, S>
where
    S: ?Sized,
{
    stock: &'a S,
}

impl<'a, S> GetStockLevelUseCase<'a, S>
where
    S: StockRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(stock: &'a S) -> Self {
        Self { stock }
    }

    /// Current level in base micros.
    pub fn execute(
        &self,
        _session: &SessionContext,
        product_id: baraka_domain::catalog::ProductId,
        warehouse_id: WarehouseId,
    ) -> Result<i64, ApplicationError> {
        self.stock.get_stock_level(product_id, warehouse_id)
    }
}

/// Manual stock correction (GERANT/ADMIN; negative path needs override).
pub struct AdjustStockUseCase<'a, S>
where
    S: ?Sized,
{
    stock: &'a S,
}

impl<'a, S> AdjustStockUseCase<'a, S>
where
    S: StockRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(stock: &'a S) -> Self {
        Self { stock }
    }

    /// Validate, authorize, and apply one adjustment atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: AdjustStockInput,
    ) -> Result<AdjustStockOutcome, ApplicationError> {
        if !session.role.has_permission(Permission::AdjustStock) {
            return Err(ApplicationError::Unauthorized);
        }
        if input.quantity_micro <= 0 {
            return Err(ApplicationError::InvalidQuantity);
        }
        if input.reason.trim().chars().count() < 3 {
            return Err(coded("ADJUSTMENT_REASON_REQUIRED"));
        }
        let movement_type = match input.direction {
            StockDirection::Increase => StockMovementType::AdjustmentPositive,
            StockDirection::Decrease => StockMovementType::AdjustmentNegative,
        };
        // The flag is honored verbatim: requesting it requires the override
        // permission (checked exactly here); the engine itself rejects any
        // negative outcome that arrives without authorization.
        let authorize = crate::services::stock_movement_engine::resolve_adjust_authorization(
            session.role.has_permission(Permission::AdjustStock),
            session
                .role
                .has_permission(Permission::NegativeStockOverride),
            input.authorize_negative_stock,
        )?;
        let record = self.stock.apply_stock_movement(&StockMovementCommand {
            product_id: input.product_id,
            warehouse_id: input.warehouse_id,
            unit_id: input.unit_id,
            quantity_micro: input.quantity_micro,
            movement_type,
            reference_type: None,
            reference_id: None,
            transfer_batch_id: None,
            authorize_negative_stock: authorize,
            reason: Some(input.reason.trim().to_string()),
            actor: session.user_id,
        })?;
        Ok(AdjustStockOutcome {
            movement_id: record.movement_id,
            new_quantity_base_micro: record.new_quantity_base_micro,
        })
    }
}

/// Atomic warehouse transfer (GERANT/ADMIN; never negative at source).
pub struct TransferStockUseCase<'a, S, W>
where
    S: ?Sized,
    W: ?Sized,
{
    stock: &'a S,
    warehouses: &'a W,
}

impl<'a, S, W> TransferStockUseCase<'a, S, W>
where
    S: StockRepository + ?Sized,
    W: WarehouseRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(stock: &'a S, warehouses: &'a W) -> Self {
        Self { stock, warehouses }
    }

    /// Validate endpoints and move stock as one OUT/IN pair.
    pub fn execute(
        &self,
        session: &SessionContext,
        input: TransferStockInput,
    ) -> Result<TransferResult, ApplicationError> {
        if !session.role.has_permission(Permission::TransferStock) {
            return Err(ApplicationError::Unauthorized);
        }
        if input.source_warehouse_id == input.destination_warehouse_id {
            return Err(coded("TRANSFER_SAME_WAREHOUSE"));
        }
        if input.quantity_micro <= 0 {
            return Err(ApplicationError::InvalidQuantity);
        }
        for warehouse_id in [input.source_warehouse_id, input.destination_warehouse_id] {
            match self.warehouses.get_by_id(warehouse_id)? {
                Some(warehouse) if warehouse.is_active => {}
                Some(_) => return Err(coded("WAREHOUSE_INACTIVE")),
                None => {
                    return Err(ApplicationError::NotFound("warehouse".to_string()));
                }
            }
        }
        // Product/unit existence and kit rejection surface from the engine;
        // endpoint checks above all happen before any transaction begins.
        let command = TransferCommand {
            product_id: input.product_id,
            source_warehouse_id: input.source_warehouse_id,
            destination_warehouse_id: input.destination_warehouse_id,
            unit_id: input.unit_id,
            quantity_micro: input.quantity_micro,
            transfer_batch_id: Uuid::new_v4().to_string(),
            reason: input.reason.map(|reason| reason.trim().to_string()),
            actor: session.user_id,
        };
        self.stock.transfer_stock(&command)
    }
}
