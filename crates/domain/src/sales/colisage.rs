//! Multi-level packaging breakdown — *Colisage* (TASK-BRK-109, ADR-110).
//!
//! A document line stores its quantity in base micros; the merchant thinks
//! in selling rungs (`Pièce`, `Boîte ×3`, `Demi-Dz ×6`, `Douzaine ×12`,
//! `Balle/Carton`). This module translates the former into the latter:
//! given the product's unit ladder (base micros per rung), it expresses any
//! non-negative base quantity as whole rungs, largest first.
//!
//! ```text
//! 150 pieces on [Pièce ×1, Demi-Dz ×6, Douzaine ×12]
//!   → 12 × Douzaine + 1 × Demi-Dz + 0 × Pièce (omitted)
//! ```
//!
//! Pure: no I/O, no floats. All division runs in `i128` with an explicit
//! overflow error, per the constitutional money law. Only rungs with a
//! non-zero count are emitted (a line of exactly 3 dozen prints as
//! `3 × Douzaine`, never `3 × Douzaine + 0 × …`).

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;

/// One rung of a product's packaging ladder (input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColisageRung {
    /// Base micros contained in ONE unit of this rung (10_000 = base unit).
    pub factor_micro: i64,
}

/// One whole-rung share of a broken-down quantity (output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColisageShare {
    /// Base micros per rung unit (echoes the winning rung).
    pub factor_micro: i64,
    /// Whole units of this rung (`>= 1` by construction).
    pub count: i64,
}

/// Express `quantity_base_micro` as whole rungs, largest first.
///
/// `ladder` is the product's unit ladder in ANY order (it is sorted
/// descending internally, so callers never need to pre-sort); every rung
/// must carry a strictly positive factor. A zero quantity yields zero
/// shares. The smallest rung MUST divide the quantity exactly — in practice
/// the ladder always carries the base unit (factor 10_000), so any base
/// micro quantity closes with zero remainder; a nonzero remainder is a
/// corrupt ladder and is refused, never silently rounded.
///
/// # Errors
/// [`DomainError::MoneyOverflow`] on unrepresentable intermediates;
/// [`DomainError::InvalidQuantity`] on a negative quantity, an empty ladder,
/// a non-positive rung factor, or a nonzero remainder.
pub fn breakdown(
    quantity_base_micro: i64,
    ladder: &[ColisageRung],
) -> Result<Vec<ColisageShare>, DomainError> {
    if quantity_base_micro < 0 {
        return Err(DomainError::InvalidQuantity);
    }
    if ladder.is_empty() {
        return Err(DomainError::InvalidQuantity);
    }
    if ladder.iter().any(|rung| rung.factor_micro <= 0) {
        return Err(DomainError::InvalidQuantity);
    }
    if quantity_base_micro == 0 {
        return Ok(Vec::new());
    }
    let mut rungs: Vec<i64> = ladder.iter().map(|rung| rung.factor_micro).collect();
    rungs.sort_unstable_by(|a, b| b.cmp(a));
    rungs.dedup();

    let mut remaining = i128::from(quantity_base_micro);
    let mut shares = Vec::new();
    for factor in rungs {
        let width = i128::from(factor);
        let count = remaining
            .checked_div(width)
            .ok_or(DomainError::MoneyOverflow)?;
        remaining = remaining
            .checked_rem(width)
            .ok_or(DomainError::MoneyOverflow)?;
        if count > 0 {
            let count = i64::try_from(count).map_err(|_| DomainError::MoneyOverflow)?;
            shares.push(ColisageShare {
                factor_micro: factor,
                count,
            });
        }
    }
    if remaining != 0 {
        return Err(DomainError::InvalidQuantity);
    }
    Ok(shares)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Underwear ladder: piece, half-dozen, dozen (base micros per rung).
    fn textile_ladder() -> Vec<ColisageRung> {
        vec![
            ColisageRung {
                factor_micro: 10_000,
            },
            ColisageRung {
                factor_micro: 60_000,
            },
            ColisageRung {
                factor_micro: 120_000,
            },
        ]
    }

    #[test]
    fn dozen_breakdown_prefers_largest_rungs() {
        // 150 pieces = 12 dozen + 1 half-dozen, piece rung omitted.
        let shares = breakdown(1_500_000, &textile_ladder()).expect("breakdown");
        assert_eq!(
            shares,
            vec![
                ColisageShare {
                    factor_micro: 120_000,
                    count: 12
                },
                ColisageShare {
                    factor_micro: 60_000,
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn exact_rung_yields_single_share() {
        let shares = breakdown(360_000, &textile_ladder()).expect("breakdown");
        assert_eq!(
            shares,
            vec![ColisageShare {
                factor_micro: 120_000,
                count: 3
            }]
        );
    }

    #[test]
    fn whole_metres_break_down_rolls_then_metres() {
        // 250 m of cable on a [metre, 100 m roll] ladder.
        let ladder = vec![
            ColisageRung {
                factor_micro: 10_000,
            },
            ColisageRung {
                factor_micro: 1_000_000,
            },
        ];
        let shares = breakdown(2_500_000, &ladder).expect("breakdown");
        assert_eq!(
            shares,
            vec![
                ColisageShare {
                    factor_micro: 1_000_000,
                    count: 2
                },
                ColisageShare {
                    factor_micro: 10_000,
                    count: 50
                },
            ]
        );
    }

    #[test]
    fn sub_rung_fraction_is_refused_never_rounded() {
        // 12.5 m leaves half a metre below the smallest rung: no whole-rung
        // expression exists, so surfaces must fall back to the base quantity.
        let ladder = vec![
            ColisageRung {
                factor_micro: 10_000,
            },
            ColisageRung {
                factor_micro: 1_000_000,
            },
        ];
        assert_eq!(
            breakdown(125_000, &ladder).unwrap_err(),
            DomainError::InvalidQuantity
        );
    }

    #[test]
    fn zero_quantity_yields_no_shares() {
        assert_eq!(breakdown(0, &textile_ladder()).expect("zero"), Vec::new());
    }

    #[test]
    fn corrupt_ladder_is_refused_never_rounded() {
        // Ladder without the base rung cannot express 5 pieces exactly.
        let ladder = vec![ColisageRung {
            factor_micro: 120_000,
        }];
        assert_eq!(
            breakdown(50_000, &ladder).unwrap_err(),
            DomainError::InvalidQuantity
        );
        assert_eq!(
            breakdown(-10_000, &textile_ladder()).unwrap_err(),
            DomainError::InvalidQuantity
        );
        assert_eq!(
            breakdown(10_000, &[]).unwrap_err(),
            DomainError::InvalidQuantity
        );
        assert_eq!(
            breakdown(10_000, &[ColisageRung { factor_micro: 0 }]).unwrap_err(),
            DomainError::InvalidQuantity
        );
    }
}
