//! Priceless warehouse prep-sheet builder (TASK-BRK-126).
//!
//! `Bon de Préparation`: cart lines exploded into whole-rung Colisage
//! shares for the picker. The output carries NO price field at all —
//! zero prices by construction, never by filtering.

use baraka_domain::catalog::convert_to_base_micros;
use baraka_domain::role::Permission;
use baraka_domain::sales::{colisage_breakdown, ColisageRung};

use super::catalog::require;
use crate::errors::ApplicationError;
use crate::ports::{ColisageShareView, ProductRepository, SessionContext};

/// One cart line to explode (quantities in the line unit's micros).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrepSheetLineInput {
    /// Product to pick.
    pub product_id: baraka_domain::catalog::ProductId,
    /// Selling unit the quantity is expressed in.
    pub unit_id: baraka_domain::catalog::UnitId,
    /// Quantity in selling-unit micros (`>= 0`).
    pub quantity_micro: i64,
}

/// One exploded checklist line: NO price field exists by design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepSheetLine {
    /// Product to pick.
    pub product_id: baraka_domain::catalog::ProductId,
    /// Product SKU.
    pub sku: String,
    /// Arabic product name.
    pub name_ar: String,
    /// French product name.
    pub name_fr: String,
    /// Total quantity in base micros.
    pub quantity_base_micro: i64,
    /// Whole-rung shares, largest rung first (`[]` = zero quantity).
    pub shares: Vec<ColisageShareView>,
}

/// Build a priceless prep sheet from cart lines (BRK-126).
pub struct BuildPrepSheetUseCase<'a, P>
where
    P: ?Sized,
{
    products: &'a P,
}

impl<'a, P> BuildPrepSheetUseCase<'a, P>
where
    P: ProductRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(products: &'a P) -> Self {
        Self { products }
    }

    /// Explode every line into whole-rung shares, in input order.
    pub fn execute(
        &self,
        session: &SessionContext,
        lines: &[PrepSheetLineInput],
    ) -> Result<Vec<PrepSheetLine>, ApplicationError> {
        require(session, Permission::PosSell)?;
        let mut sheet = Vec::with_capacity(lines.len());
        for line in lines {
            if line.quantity_micro < 0 {
                return Err(ApplicationError::Validation(
                    baraka_domain::errors::DomainError::InvalidQuantity,
                ));
            }
            let (product, units) = self
                .products
                .find_by_id(line.product_id)?
                .ok_or_else(|| ApplicationError::NotFound("product".to_string()))?;
            let unit = units
                .iter()
                .find(|unit| unit.id == line.unit_id)
                .ok_or_else(|| ApplicationError::NotFound("unit".to_string()))?;
            let quantity_base_micro =
                convert_to_base_micros(line.quantity_micro, unit.factor_micro)
                    .map_err(ApplicationError::Validation)?;
            let ladder: Vec<ColisageRung> = units
                .iter()
                .map(|ladder_unit| ColisageRung {
                    factor_micro: ladder_unit.factor_micro.micros(),
                })
                .collect();
            let mut shares = Vec::new();
            if quantity_base_micro > 0 {
                // A corrupt ladder refuses LOUDLY: the picker must never
                // silently under-pick a checklist line.
                let broken = colisage_breakdown(quantity_base_micro, &ladder)
                    .map_err(ApplicationError::Validation)?;
                for share in broken {
                    let rung = units
                        .iter()
                        .find(|ladder_unit| ladder_unit.factor_micro.micros() == share.factor_micro)
                        .ok_or_else(|| ApplicationError::NotFound("unit".to_string()))?;
                    shares.push(ColisageShareView {
                        unit_id: rung.id.to_string(),
                        unit_name_fr: rung.unit_name_fr.clone(),
                        unit_name_ar: rung.unit_name_ar.clone(),
                        factor_micro: share.factor_micro,
                        count: share.count,
                    });
                }
            }
            sheet.push(PrepSheetLine {
                product_id: product.id,
                sku: product.sku.as_str().to_string(),
                name_ar: product.name_ar.clone(),
                name_fr: product.name_fr.clone(),
                quantity_base_micro,
                shares,
            });
        }
        Ok(sheet)
    }
}
