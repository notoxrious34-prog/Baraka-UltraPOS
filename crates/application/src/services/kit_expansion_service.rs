//! Repository-backed kit expansion for movement callers.
//!
//! The pure [`expand_kit`](baraka_domain::inventory::expand_kit) algorithm
//! takes an infallible-looking fetch closure, so this service preloads the
//! reachable BOM subgraph first (with full error fidelity: unknown or
//! inactive components fail here as `InvalidKitComponent` /
//! `InactiveKitComponent`), then runs the pure expansion over the snapshot.

use std::collections::{HashMap, HashSet};

use baraka_domain::catalog::convert_to_base_micros;
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::{
    expand_kit, validate_expansion_inputs, ComponentRequirement, KitComponentSpec,
};

use crate::errors::ApplicationError;
use crate::ports::ProductRepository;

/// One expanded leaf line ready for movement application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedLeaf {
    /// Leaf physical product.
    pub product_id: ProductId,
    /// Leaf base unit.
    pub base_unit_id: UnitId,
    /// Total required base micros.
    pub quantity_base_micro: i64,
}

/// Expand `root_id` for `root_base_qty_micro` base micros.
///
/// Non-kit roots pass through as a single leaf. Kit roots expand through
/// the preloaded subgraph; inactive components abort with
/// `InactiveKitComponent`, unknown references with `InvalidKitComponent`.
pub fn expand_for_movement<P>(
    products: &P,
    root_id: ProductId,
    root_base_qty_micro: i64,
) -> Result<Vec<ExpandedLeaf>, ApplicationError>
where
    P: ProductRepository + ?Sized,
{
    validate_expansion_inputs(root_base_qty_micro).map_err(ApplicationError::Validation)?;
    let (root, root_units) = products
        .find_by_id(root_id)?
        .ok_or_else(|| ApplicationError::NotFound("product".to_string()))?;
    let root_base_unit = root_units
        .iter()
        .find(|unit| unit.is_base_unit)
        .map(|unit| unit.id)
        .ok_or_else(|| ApplicationError::Port("product has no base unit".to_string()))?;
    if !root.is_kit {
        return Ok(vec![ExpandedLeaf {
            product_id: root_id,
            base_unit_id: root_base_unit,
            quantity_base_micro: root_base_qty_micro,
        }]);
    }
    // Preload the reachable subgraph with full error fidelity.
    let mut specs: HashMap<ProductId, Vec<KitComponentSpec>> = HashMap::new();
    let mut base_units: HashMap<ProductId, UnitId> = HashMap::new();
    let mut frontier = vec![root_id];
    let mut visited: HashSet<ProductId> = HashSet::new();
    while let Some(kit_id) = frontier.pop() {
        if !visited.insert(kit_id) {
            continue;
        }
        let mut kit_specs = Vec::new();
        for row in products.get_direct_kit_items(kit_id)? {
            let (component, component_units) =
                products.find_by_id(row.component_product_id)?.ok_or(
                    ApplicationError::Validation(DomainError::InvalidKitComponent),
                )?;
            if !component.is_active {
                return Err(ApplicationError::Validation(
                    DomainError::InactiveKitComponent,
                ));
            }
            let factor = component_units
                .iter()
                .find(|unit| unit.id == row.unit_id)
                .map(|unit| unit.factor_micro)
                .ok_or(ApplicationError::Validation(
                    DomainError::InvalidKitComponent,
                ))?;
            let per_unit = convert_to_base_micros(row.quantity_micro, factor)
                .map_err(ApplicationError::Validation)?;
            if let Some(base) = component_units.iter().find(|unit| unit.is_base_unit) {
                base_units.insert(component.id, base.id);
            }
            if component.is_kit {
                frontier.push(component.id);
            }
            kit_specs.push(KitComponentSpec {
                component_product_id: component.id,
                component_is_kit: component.is_kit,
                quantity_base_micro_per_kit_base_unit: per_unit,
            });
        }
        specs.insert(kit_id, kit_specs);
    }
    let fetch = |kit_id: ProductId| -> Result<Vec<KitComponentSpec>, DomainError> {
        specs
            .get(&kit_id)
            .cloned()
            .ok_or(DomainError::InvalidKitComponent)
    };
    let base_of = |product_id: ProductId| -> Result<UnitId, DomainError> {
        base_units
            .get(&product_id)
            .copied()
            .ok_or(DomainError::InvalidKitComponent)
    };
    let requirements: Vec<ComponentRequirement> = expand_kit(
        root_id,
        true,
        root_base_unit,
        root_base_qty_micro,
        &fetch,
        &base_of,
    )
    .map_err(ApplicationError::Validation)?;
    Ok(requirements
        .into_iter()
        .map(|item| ExpandedLeaf {
            product_id: item.product_id,
            base_unit_id: item.base_unit_id,
            quantity_base_micro: item.quantity_base_micro,
        })
        .collect())
}
