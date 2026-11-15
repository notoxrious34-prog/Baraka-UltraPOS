//! Quantity-break price ladders (BRK-121 / S1).
//!
//! A break lowers the selling price once the line quantity reaches a
//! rung (`min_qty_micro`, expressed in the ladder unit's own micros).
//! Resolution is pure: the greatest rung at or below the line quantity
//! wins; otherwise `None` (caller falls back to `price_for_tier`).
//! All money stays `i64` centimes — no floats anywhere near pricing.

use crate::primitives::Money;

/// One ladder rung: minimum quantity (unit micros) and its unit price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceBreak {
    /// Minimum line quantity in the ladder unit's micros (strictly positive).
    pub min_qty_micro: i64,
    /// Unit price in centimes once the rung is reached.
    pub price: Money,
}

/// Resolve the winning rung for `quantity_micro`.
///
/// Returns the price of the greatest rung with
/// `min_qty_micro <= quantity_micro`, or `None` when no rung applies
/// (unsorted input accepted; non-positive quantities never match).
#[must_use]
pub fn resolve_break_price(breaks: &[PriceBreak], quantity_micro: i64) -> Option<Money> {
    if quantity_micro <= 0 {
        return None;
    }
    breaks
        .iter()
        .filter(|rung| rung.min_qty_micro > 0 && rung.min_qty_micro <= quantity_micro)
        .max_by_key(|rung| rung.min_qty_micro)
        .map(|rung| rung.price)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rung(min_qty_micro: i64, price_centimes: i64) -> PriceBreak {
        PriceBreak {
            min_qty_micro,
            price: Money::from_centimes(price_centimes),
        }
    }

    #[test]
    fn no_rungs_means_no_break() {
        assert_eq!(resolve_break_price(&[], 1_000_000), None);
    }

    #[test]
    fn below_first_rung_means_no_break() {
        let ladder = [rung(10_000, 500), rung(100_000, 450)];
        assert_eq!(resolve_break_price(&ladder, 9_999), None);
    }

    #[test]
    fn greatest_reached_rung_wins_unsorted_input() {
        let ladder = [rung(100_000, 450), rung(10_000, 500), rung(50_000, 475)];
        assert_eq!(
            resolve_break_price(&ladder, 60_000),
            Some(Money::from_centimes(475))
        );
        assert_eq!(
            resolve_break_price(&ladder, 250_000),
            Some(Money::from_centimes(450))
        );
    }

    #[test]
    fn exact_rung_boundary_applies() {
        let ladder = [rung(10_000, 500)];
        assert_eq!(
            resolve_break_price(&ladder, 10_000),
            Some(Money::from_centimes(500))
        );
    }

    #[test]
    fn non_positive_quantities_never_match() {
        let ladder = [rung(10_000, 500)];
        assert_eq!(resolve_break_price(&ladder, 0), None);
        assert_eq!(resolve_break_price(&ladder, -5), None);
    }

    #[test]
    fn degenerate_non_positive_rungs_ignored() {
        let ladder = [rung(0, 100), rung(-10_000, 100), rung(10_000, 500)];
        assert_eq!(
            resolve_break_price(&ladder, 1_000_000),
            Some(Money::from_centimes(500))
        );
    }
}
