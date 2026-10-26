//! Warehouses: storage locations holding physical stock.

use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;

/// Stable warehouse identifier (TEXT UUID, sovereign identity scheme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WarehouseId(pub Uuid);

impl WarehouseId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WarehouseId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for WarehouseId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for WarehouseId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Validated warehouse code (trimmed, 1..=20 chars).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WarehouseCode(String);

impl WarehouseCode {
    /// Build from raw input.
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 20 {
            return Err(DomainError::InvalidCanonicalString(
                "warehouse code must be 1..=20 characters".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated code.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Storage location for physical stock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warehouse {
    /// Stable identifier.
    pub id: WarehouseId,
    /// Unique business code.
    pub code: WarehouseCode,
    /// Display name (trimmed, 1..=100 chars, validated at construction).
    pub name: String,
    /// Inactive warehouses accept no movements.
    pub is_active: bool,
}

impl Warehouse {
    /// Validate a display name.
    pub fn validate_name(name: &str) -> Result<(), DomainError> {
        let count = name.trim().chars().count();
        if count == 0 || count > 100 {
            return Err(DomainError::InvalidCanonicalString(
                "warehouse name must be 1..=100 characters".to_string(),
            ));
        }
        Ok(())
    }
}

/// Read-only stock-presence snapshot for one active depot: how many
/// level rows hold a non-zero quantity. The UI derives `stocked` as
/// `nonzero_levels > 0`; DEP-01 is reported truthfully (the lock badge
/// stays a UI decision, never this wire's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarehouseStockPresence {
    /// Stable depot identifier.
    pub warehouse_id: WarehouseId,
    /// Level rows with `quantity_base_micro != 0` in this depot.
    pub nonzero_levels: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_and_name_boundaries() {
        assert!(WarehouseCode::new("MAIN").is_ok());
        assert!(WarehouseCode::new("  MAIN  ").is_ok());
        assert_eq!(
            WarehouseCode::new("  MAIN  ").expect("trim").as_str(),
            "MAIN"
        );
        assert!(WarehouseCode::new("").is_err());
        assert!(WarehouseCode::new("   ").is_err());
        assert!(WarehouseCode::new(&"X".repeat(21)).is_err());
        assert!(WarehouseCode::new(&"X".repeat(20)).is_ok());
        assert!(Warehouse::validate_name("Principal").is_ok());
        assert!(Warehouse::validate_name("").is_err());
        assert!(Warehouse::validate_name(&"Y".repeat(101)).is_err());
    }
}
