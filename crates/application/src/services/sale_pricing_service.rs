//! Sale pricing service: catalog-verified line preparation.
//!
//! Verifies each cart line against the catalog (existence, activity, unit
//! ownership, price agreement) and converts quantities to base micros, so
//! the pure pricing kernel receives only validated inputs. Price-override
//! *permission* is checked by the calling use case, not here.

use baraka_domain::catalog::{convert_to_base_micros, ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::sales::pricing::calculate_margin_basis_points;
use baraka_domain::sales::CustomerTier;

use crate::errors::ApplicationError;
use crate::ports::ProductRepository;

/// Catalog-verified cart line ready for the pricing kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedLine {
    /// Product sold (leaf or kit header as displayed).
    pub product_id: ProductId,
    /// Selling unit handled.
    pub unit_id: UnitId,
    /// Quantity in selling-unit micros.
    pub quantity_micro: i64,
    /// Quantity converted to base micros.
    pub quantity_base_micro: i64,
    /// Agreed unit price in centimes (catalog or accepted override).
    pub unit_price_centimes: i64,
    /// Purchase price snapshot in centimes (margin base).
    pub cost_price_centimes: i64,
    /// Margin Guard: integer-exact margin in basis points.
    pub margin_basis_points: i64,
    /// Margin Guard: true when the agreed price is strictly below cost.
    pub is_below_cost: bool,
    /// True for kit headers (expanded downstream, never moved directly).
    pub is_kit: bool,
    /// Active flag snapshot.
    pub is_active: bool,
}

/// Verify one cart line against the catalog.
///
/// `override_price`: explicit operator price, accepted verbatim here (the
/// caller gates it behind `PriceOverride`); `None` uses the tier-resolved
/// catalog price (`ProductUnit::price_for_tier`).
pub fn verify_cart_line<P>(
    products: &P,
    product_id: ProductId,
    unit_id: UnitId,
    quantity_micro: i64,
    override_price: Option<i64>,
    tier: CustomerTier,
) -> Result<VerifiedLine, ApplicationError>
where
    P: ProductRepository + ?Sized,
{
    if quantity_micro <= 0 {
        return Err(ApplicationError::InvalidQuantity);
    }
    let (product, units) = products
        .find_by_id(product_id)?
        .ok_or_else(|| ApplicationError::NotFound("product".to_string()))?;
    if !product.is_active {
        return Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString("PRODUCT_INACTIVE".to_string()),
        ));
    }
    let unit = units
        .iter()
        .find(|unit| unit.id == unit_id)
        .ok_or_else(|| {
            ApplicationError::Validation(DomainError::InvalidCanonicalString(
                "INVALID_UNIT".to_string(),
            ))
        })?;
    let unit_price = override_price.unwrap_or_else(|| unit.price_for_tier(tier).centimes());
    let cost_price = unit.purchase_price.centimes();
    let quantity_base_micro = convert_to_base_micros(quantity_micro, unit.factor_micro)
        .map_err(ApplicationError::Validation)?;
    let (margin_basis_points, is_below_cost) =
        calculate_margin_basis_points(unit_price, cost_price);
    Ok(VerifiedLine {
        product_id,
        unit_id,
        quantity_micro,
        quantity_base_micro,
        unit_price_centimes: unit_price,
        cost_price_centimes: cost_price,
        margin_basis_points,
        is_below_cost,
        is_kit: product.is_kit,
        is_active: product.is_active,
    })
}
