//! Product units: one immutable base unit plus selling units.
//!
//! Conversion contract: `base_qty_micros = selling_qty_micros * factor_micro
//! / 10_000`, rounded with the canonical half-away-from-zero helper.

use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::catalog::{Barcode, ProductId};
use crate::errors::DomainError;
use crate::primitives::{round_half_away_from_zero, FactorMicro, Money, QtyMicro};

/// Micros per whole unit (scale 10^4): `1.0000 == 10_000`.
pub const MICROS_PER_UNIT: i64 = 10_000;

/// Stable product-unit identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitId(pub Uuid);

impl UnitId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UnitId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for UnitId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UnitId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// One purchasable/sellable packaging of a product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductUnit {
    /// Stable identifier.
    pub id: UnitId,
    /// Owning product.
    pub product_id: ProductId,
    /// Arabic unit name, e.g. `علبة`.
    pub unit_name_ar: String,
    /// French unit name, e.g. `Boîte`.
    pub unit_name_fr: String,
    /// Base micros per one selling unit (`10_000` for the base unit itself).
    pub factor_micro: FactorMicro,
    /// Optional barcode resolving to exactly this unit.
    pub barcode: Option<Barcode>,
    /// Sell price per selling unit.
    pub sale_price: Money,
    /// Purchase price per selling unit.
    pub purchase_price: Money,
    /// True for the single immutable base unit.
    pub is_base_unit: bool,
    /// Default unit offered on the POS screen.
    pub is_default_selling: bool,
}

/// Enforce the base-unit invariant over a product's full unit set:
///
/// - non-empty, all units belong to `product_id`;
/// - every factor strictly positive;
/// - exactly one base unit, pinned at factor `10_000` (1.0000).
pub fn validate_unit_set(product_id: ProductId, units: &[ProductUnit]) -> Result<(), DomainError> {
    if units.is_empty() {
        return Err(DomainError::BaseUnitMismatch);
    }
    let mut base_count = 0;
    for unit in units {
        if unit.product_id != product_id {
            return Err(DomainError::BaseUnitMismatch);
        }
        if unit.factor_micro.micros() <= 0 {
            return Err(DomainError::BaseUnitMismatch);
        }
        if unit.unit_name_ar.trim().is_empty() || unit.unit_name_fr.trim().is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "unit names must not be empty".to_string(),
            ));
        }
        if unit.is_base_unit {
            base_count += 1;
            if unit.factor_micro != FactorMicro::from_micros(MICROS_PER_UNIT) {
                return Err(DomainError::BaseUnitMismatch);
            }
        }
    }
    if base_count != 1 {
        return Err(DomainError::BaseUnitMismatch);
    }
    Ok(())
}

/// Convert a selling-unit quantity to base-unit micros.
///
/// `base = (selling_qty_micros * factor_micro) / 10_000`, canonical
/// half-away-from-zero rounding, `i128` intermediate with `i64` range check.
pub fn convert_to_base_micros(
    selling_qty_micros: i64,
    factor_micro: FactorMicro,
) -> Result<i64, DomainError> {
    let numerator = (selling_qty_micros as i128)
        .checked_mul(factor_micro.micros() as i128)
        .ok_or(DomainError::MoneyOverflow)?;
    let rounded = round_half_away_from_zero(numerator, i128::from(MICROS_PER_UNIT));
    i64::try_from(rounded).map_err(|_| DomainError::MoneyOverflow)
}

/// Quantity-aware convenience: convert a `QtyMicro` selling quantity.
pub fn convert_qty_to_base(
    quantity: QtyMicro,
    factor: FactorMicro,
) -> Result<QtyMicro, DomainError> {
    convert_to_base_micros(quantity.micros(), factor).map(QtyMicro::from_micros)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product_id() -> ProductId {
        ProductId::new()
    }

    fn base_unit(product: ProductId) -> ProductUnit {
        ProductUnit {
            id: UnitId::new(),
            product_id: product,
            unit_name_ar: "قطعة".to_string(),
            unit_name_fr: "Pièce".to_string(),
            factor_micro: FactorMicro::from_micros(10_000),
            barcode: None,
            sale_price: Money::from_centimes(15_000),
            purchase_price: Money::from_centimes(12_000),
            is_base_unit: true,
            is_default_selling: true,
        }
    }

    fn selling_unit(product: ProductId, factor: i64) -> ProductUnit {
        ProductUnit {
            id: UnitId::new(),
            product_id: product,
            unit_name_ar: "علبة".to_string(),
            unit_name_fr: "Boîte".to_string(),
            factor_micro: FactorMicro::from_micros(factor),
            barcode: None,
            sale_price: Money::from_centimes(1_450_000),
            purchase_price: Money::from_centimes(1_150_000),
            is_base_unit: false,
            is_default_selling: false,
        }
    }

    #[test]
    fn box_to_base_conversion_is_exact() {
        // 2.5000 boxes x factor 100.0000 = 250.0000 base units.
        let base =
            convert_to_base_micros(25_000, FactorMicro::from_micros(1_000_000)).expect("convert");
        assert_eq!(base, 2_500_000);
    }

    #[test]
    fn valid_set_passes_and_broken_sets_fail() {
        let product = product_id();
        let mut good = vec![base_unit(product), selling_unit(product, 1_000_000)];
        validate_unit_set(product, &good).expect("valid set");

        // No base unit.
        good[0].is_base_unit = false;
        assert_eq!(
            validate_unit_set(product, &good),
            Err(DomainError::BaseUnitMismatch)
        );
        good[0].is_base_unit = true;

        // Base factor drifted.
        good[0].factor_micro = FactorMicro::from_micros(10_001);
        assert_eq!(
            validate_unit_set(product, &good),
            Err(DomainError::BaseUnitMismatch)
        );
        good[0].factor_micro = FactorMicro::from_micros(10_000);

        // Two base units.
        let mut twin = good.clone();
        twin[1].is_base_unit = true;
        twin[1].factor_micro = FactorMicro::from_micros(10_000);
        assert_eq!(
            validate_unit_set(product, &twin),
            Err(DomainError::BaseUnitMismatch)
        );

        // Non-positive selling factor.
        let mut zero = good.clone();
        zero[1].factor_micro = FactorMicro::from_micros(0);
        assert_eq!(
            validate_unit_set(product, &zero),
            Err(DomainError::BaseUnitMismatch)
        );

        // Empty set.
        assert_eq!(
            validate_unit_set(product, &[]),
            Err(DomainError::BaseUnitMismatch)
        );
    }
}
