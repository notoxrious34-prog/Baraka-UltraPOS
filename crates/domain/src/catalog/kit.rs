//! Kit bills of materials with deterministic cycle protection.
//!
//! The kit graph is a directed graph `kit -> component`. Adding edges must
//! never close a directed cycle; validation runs a depth-first search from
//! each proposed component over the union of stored and proposed edges.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::catalog::{ProductId, UnitId};
use crate::errors::DomainError;
use crate::primitives::QtyMicro;

/// One BOM line: `quantity_micro` of `component` (in `unit_id`) per kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitItem {
    /// Kit header product.
    pub kit_product_id: ProductId,
    /// Leaf or sub-kit component.
    pub component_product_id: ProductId,
    /// Component unit the quantity is expressed in.
    pub unit_id: UnitId,
    /// Quantity per kit, in unit micros.
    pub quantity_micro: QtyMicro,
}

/// True when `target` is reachable from `start` following `edges`.
fn reaches(start: ProductId, target: ProductId, edges: &[(ProductId, ProductId)]) -> bool {
    let mut visited: HashSet<ProductId> = HashSet::new();
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if node == target {
            return true;
        }
        if visited.insert(node) {
            for (from, to) in edges {
                if *from == node && !visited.contains(to) {
                    stack.push(*to);
                }
            }
        }
    }
    false
}

/// Validate proposed `components` for `kit_id` against stored `existing_edges`.
///
/// - Direct self-inclusion yields [`DomainError::SelfReferencingKit`].
/// - Any path from a proposed component back to `kit_id` (through stored
///   or simultaneously proposed edges) yields [`DomainError::KitCycleDetected`].
pub fn validate_kit_edges(
    kit_id: ProductId,
    components: &[ProductId],
    existing_edges: &[(ProductId, ProductId)],
) -> Result<(), DomainError> {
    let mut combined: Vec<(ProductId, ProductId)> =
        Vec::with_capacity(existing_edges.len() + components.len());
    combined.extend_from_slice(existing_edges);
    for component in components {
        combined.push((kit_id, *component));
    }
    for component in components {
        if *component == kit_id {
            return Err(DomainError::SelfReferencingKit);
        }
        if reaches(*component, kit_id, &combined) {
            return Err(DomainError::KitCycleDetected);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: u8) -> Vec<ProductId> {
        (0..n)
            .map(|i| ProductId::from(uuid::Uuid::from_bytes([i; 16])))
            .collect()
    }

    #[test]
    fn self_reference_is_rejected() {
        let kit = ProductId::new();
        assert_eq!(
            validate_kit_edges(kit, &[kit], &[]),
            Err(DomainError::SelfReferencingKit)
        );
    }

    #[test]
    fn direct_cycle_is_rejected() {
        let list = ids(2);
        let (a, b) = (list[0], list[1]);
        assert_eq!(
            validate_kit_edges(a, &[b], &[(b, a)]),
            Err(DomainError::KitCycleDetected)
        );
    }

    #[test]
    fn transitive_cycle_is_rejected() {
        let list = ids(3);
        let (a, b, c) = (list[0], list[1], list[2]);
        assert_eq!(
            validate_kit_edges(a, &[b], &[(b, c), (c, a)]),
            Err(DomainError::KitCycleDetected)
        );
    }

    #[test]
    fn multi_edge_proposal_closing_a_cycle_is_rejected() {
        // Stored: B -> A. Proposing A -> B and A -> C together must fail:
        // A -> B closes A -> B -> A even though A -> C alone is innocent.
        let list = ids(3);
        let (a, b, c) = (list[0], list[1], list[2]);
        assert_eq!(
            validate_kit_edges(a, &[b, c], &[(b, a)]),
            Err(DomainError::KitCycleDetected)
        );
    }

    #[test]
    fn valid_multilevel_hierarchy_is_accepted() {
        // A -> B -> C, A and B kits, C a leaf: no path returns to A.
        let list = ids(3);
        let (a, b, c) = (list[0], list[1], list[2]);
        validate_kit_edges(b, &[c], &[]).expect("B -> C");
        validate_kit_edges(a, &[b], &[(b, c)]).expect("A -> B over B -> C");
        validate_kit_edges(a, &[b, c], &[(b, c)]).expect("fan-out without cycle");
    }
}
