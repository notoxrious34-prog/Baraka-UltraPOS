//! Pure multi-level kit expansion (no I/O).
//!
//! Quantity semantic (adapted to the TASK-BRK-004 established shape, which
//! stores each BOM line in the component unit named by `unit_id`):
//!
//! - The repository/service layer converts every BOM line to
//!   `quantity_base_micro_per_kit_base_unit`: base micros of the component
//!   required to produce ONE FULL BASE UNIT (10_000 micros) of the kit.
//! - This module then works purely in base micros:
//!   `required = round_half_away(requested_kit_base_micros
//!   * per_unit / 10_000)`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::catalog::{ProductId, UnitId};
use crate::errors::DomainError;
use crate::primitives::round_half_away_from_zero;

/// Base micros per whole base unit (scale 10^4).
const BASE_MICROS_PER_UNIT: i128 = 10_000;

/// Maximum kit nesting depth (runtime safety net; write-time cycles are
/// already rejected by TASK-BRK-004 validation).
pub const MAX_EXPANSION_DEPTH: u32 = 10;

/// One direct BOM edge, pre-converted to base micros by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitComponentSpec {
    /// Component product.
    pub component_product_id: ProductId,
    /// True when the component is itself a kit header.
    pub component_is_kit: bool,
    /// Base micros of component per ONE FULL BASE UNIT of the kit.
    pub quantity_base_micro_per_kit_base_unit: i64,
}

/// One leaf requirement: physical product, base unit, total base micros.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentRequirement {
    /// Leaf physical product.
    pub product_id: ProductId,
    /// Leaf base unit.
    pub base_unit_id: UnitId,
    /// Total required base micros (summed across all paths).
    pub quantity_base_micro: i64,
}

/// Validate expansion inputs shared by every entry point.
pub fn validate_expansion_inputs(root_base_qty_micro: i64) -> Result<(), DomainError> {
    if root_base_qty_micro <= 0 {
        return Err(DomainError::InvalidCanonicalString(
            "expansion quantity must be positive".to_string(),
        ));
    }
    Ok(())
}

/// Expand a kit (or pass a leaf through) into leaf base-micros requirements.
///
/// - Non-kit roots return a single requirement for the root itself, so
///   callers treat kits and leaves uniformly.
/// - Sub-kits recurse with their computed required base quantity.
/// - Leaves accumulate into a map keyed by `(product_id, base_unit_id)`,
///   summing across every path that references the same leaf.
/// - Output is sorted deterministically by `(product_id, base_unit_id)`.
/// - Nesting deeper than [`MAX_EXPANSION_DEPTH`] yields
///   [`DomainError::KitExpansionDepthExceeded`].
pub fn expand_kit(
    root_id: ProductId,
    root_is_kit: bool,
    root_base_unit_id: UnitId,
    root_base_qty_micro: i64,
    fetch_direct_components: &dyn Fn(ProductId) -> Result<Vec<KitComponentSpec>, DomainError>,
    base_unit_of: &dyn Fn(ProductId) -> Result<UnitId, DomainError>,
) -> Result<Vec<ComponentRequirement>, DomainError> {
    validate_expansion_inputs(root_base_qty_micro)?;
    if !root_is_kit {
        return Ok(vec![ComponentRequirement {
            product_id: root_id,
            base_unit_id: root_base_unit_id,
            quantity_base_micro: root_base_qty_micro,
        }]);
    }
    let mut accumulated: HashMap<(ProductId, UnitId), i128> = HashMap::new();
    expand_node(
        root_id,
        root_base_qty_micro,
        0,
        fetch_direct_components,
        base_unit_of,
        &mut accumulated,
    )?;
    let mut out = Vec::with_capacity(accumulated.len());
    for ((product_id, base_unit_id), total) in accumulated {
        out.push(ComponentRequirement {
            product_id,
            base_unit_id,
            quantity_base_micro: i64::try_from(total).map_err(|_| DomainError::MoneyOverflow)?,
        });
    }
    out.sort_by_key(|requirement| (requirement.product_id.0, requirement.base_unit_id.0));
    Ok(out)
}

fn expand_node(
    kit_id: ProductId,
    kit_base_qty_micro: i64,
    depth: u32,
    fetch_direct_components: &dyn Fn(ProductId) -> Result<Vec<KitComponentSpec>, DomainError>,
    base_unit_of: &dyn Fn(ProductId) -> Result<UnitId, DomainError>,
    accumulated: &mut HashMap<(ProductId, UnitId), i128>,
) -> Result<(), DomainError> {
    if depth > MAX_EXPANSION_DEPTH {
        return Err(DomainError::KitExpansionDepthExceeded);
    }
    let kit_base_qty = i128::from(kit_base_qty_micro);
    for spec in fetch_direct_components(kit_id)? {
        let numerator = kit_base_qty
            .checked_mul(i128::from(spec.quantity_base_micro_per_kit_base_unit))
            .ok_or(DomainError::MoneyOverflow)?;
        let required = round_half_away_from_zero(numerator, BASE_MICROS_PER_UNIT);
        if spec.component_is_kit {
            expand_node(
                spec.component_product_id,
                i64::try_from(required).map_err(|_| DomainError::MoneyOverflow)?,
                depth + 1,
                fetch_direct_components,
                base_unit_of,
                accumulated,
            )?;
        } else {
            let base_unit_id = base_unit_of(spec.component_product_id)?;
            let entry = accumulated
                .entry((spec.component_product_id, base_unit_id))
                .or_insert(0);
            *entry = entry
                .checked_add(required)
                .ok_or(DomainError::MoneyOverflow)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product(n: u8) -> ProductId {
        ProductId::from(uuid::Uuid::from_bytes([n; 16]))
    }

    fn unit(n: u8) -> UnitId {
        UnitId::from(uuid::Uuid::from_bytes([n; 16]))
    }

    #[test]
    fn leaf_passthrough_returns_root_itself() {
        let out = expand_kit(
            product(1),
            false,
            unit(1),
            5_000,
            &|_| Ok(Vec::new()),
            &|_| Ok(unit(1)),
        )
        .expect("passthrough");
        assert_eq!(
            out,
            vec![ComponentRequirement {
                product_id: product(1),
                base_unit_id: unit(1),
                quantity_base_micro: 5_000,
            }]
        );
    }

    #[test]
    fn depth_guard_triggers_on_synthetic_nesting() {
        // Eleven nested kits: depth 11 exceeds the guard of 10.
        let fetch = |id: ProductId| {
            let byte = id.0.as_bytes()[0];
            if byte >= 11 {
                Ok(Vec::new())
            } else {
                Ok(vec![KitComponentSpec {
                    component_product_id: ProductId::from(uuid::Uuid::from_bytes([byte + 1; 16])),
                    component_is_kit: true,
                    quantity_base_micro_per_kit_base_unit: 10_000,
                }])
            }
        };
        let error = expand_kit(product(0), true, unit(0), 10_000, &fetch, &|_| Ok(unit(0)))
            .expect_err("depth must trip");
        assert_eq!(error, DomainError::KitExpansionDepthExceeded);
    }

    #[test]
    fn rejects_non_positive_quantity() {
        assert!(
            expand_kit(product(1), false, unit(1), 0, &|_| Ok(Vec::new()), &|_| Ok(
                unit(1)
            ))
            .is_err()
        );
    }
}
