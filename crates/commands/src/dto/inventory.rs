//! Inventory DTOs: warehouses, stock levels, adjustments, transfers.
use serde::{Deserialize, Serialize};

// Inventory wire types --------------------------------------------------
// Quantities travel as integer-micro strings; ids are UUID strings.

/// Warehouse creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWarehouseRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Unique business code (1..=20 chars).
    pub code: String,
    /// Display name (1..=100 chars).
    pub name: String,
}

/// Stored warehouse view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarehouseDto {
    /// Warehouse id (UUID string).
    pub id: String,
    /// Unique business code.
    pub code: String,
    /// Display name.
    pub name: String,
    /// Active flag.
    pub is_active: bool,
}

/// Warehouse list response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListWarehousesResponseDto {
    /// Active warehouses ordered by code.
    pub warehouses: Vec<WarehouseDto>,
}

/// Depot stock-presence row (BRK-095): non-zero level count for one depot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarehouseStockPresenceDto {
    /// Warehouse id (UUID string).
    pub warehouse_id: String,
    /// Level rows with non-zero quantity in this depot.
    pub nonzero_levels: u64,
}

/// Depot stock-presence response: one row per active depot, code-ordered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarehouseStockPresenceResponseDto {
    /// Presence rows (zero-count depots included; UI derives `stocked`).
    pub presence: Vec<WarehouseStockPresenceDto>,
}

/// Stock level read request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStockLevelRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
    /// Warehouse id (UUID string).
    pub warehouse_id: String,
}

/// Stock level view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetStockLevelResponseDto {
    /// Product id (UUID string).
    pub product_id: String,
    /// Warehouse id (UUID string).
    pub warehouse_id: String,
    /// Base unit id (UUID string; empty when unresolvable).
    pub base_unit_id: String,
    /// On-hand quantity in base micros.
    pub quantity_base_micro: String,
}

/// Manual stock correction request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjustStockRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Physical product id (UUID string).
    pub product_id: String,
    /// Target warehouse id (UUID string).
    pub warehouse_id: String,
    /// Handled selling unit id (UUID string).
    pub unit_id: String,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: String,
    /// `"INCREASE"` or `"DECREASE"` (explicit direction).
    pub direction: String,
    /// Mandatory human reason (min 3 chars after trim).
    pub reason: String,
    /// Request to allow a negative resulting level.
    pub authorize_negative_stock: bool,
}

/// Adjustment outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjustStockResponseDto {
    /// Stored movement id.
    pub movement_id: String,
    /// Level after the movement, in base micros.
    pub new_quantity_base_micro: String,
}

/// Warehouse transfer request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferStockRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Source warehouse id (UUID string).
    pub source_warehouse_id: String,
    /// Destination warehouse id (UUID string).
    pub destination_warehouse_id: String,
    /// Physical product id (UUID string).
    pub product_id: String,
    /// Handled selling unit id (UUID string).
    pub unit_id: String,
    /// Positive magnitude in selling-unit micros.
    pub quantity_micro: String,
    /// Optional human reason.
    pub reason: Option<String>,
}

/// Transfer outcome (one OUT + one IN sharing a batch id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferStockResponseDto {
    /// Shared batch id.
    pub transfer_batch_id: String,
    /// Stored OUT movement id.
    pub out_movement_id: String,
    /// Stored IN movement id.
    pub in_movement_id: String,
    /// Source level after the movement, in base micros.
    pub source_new_quantity_base_micro: String,
    /// Destination level after the movement, in base micros.
    pub destination_new_quantity_base_micro: String,
}
