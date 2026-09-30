//! Inventory domain: warehouses, stock movements, levels, kit expansion.

pub mod kit_expansion;
pub mod movement;
pub mod stock_level;
pub mod warehouse;

pub use kit_expansion::{
    expand_kit, validate_expansion_inputs, ComponentRequirement, KitComponentSpec,
};
pub use movement::{ReferenceType, StockMovement, StockMovementType};
pub use stock_level::{apply_delta, StockLevel};
pub use warehouse::{Warehouse, WarehouseCode, WarehouseId};
