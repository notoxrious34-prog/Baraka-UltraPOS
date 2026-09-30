//! Fixed-point financial and quantity primitives.
//!
//! Zero floating-point math. Money is integer DZD centimes (`i64`);
//! quantities and conversion factors are integer micros at scale 10^4.
//! Every intermediate uses `i128` and is range-checked back to `i64`.

use std::cmp::Reverse;

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};

use crate::errors::DomainError;

/// Baraka UltraPOS canonical financial rounding contract.
///
/// All monetary rounding uses ROUND HALF AWAY FROM ZERO:
/// - 0.5 rounds to 1, -0.5 rounds to -1 (NOT toward zero, NOT toward +infinity).
/// - This guarantees round(-x) == -round(x) for all x, which is mandatory
///   for symmetric handling of sales, returns (AVOIR), and reversal entries.
/// - This matches Architecture Doc Section 6.1's "ROUND_HALF_UP" terminology,
///   clarified here as symmetric half-away-from-zero, not truncation-biased rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RoundingMode {
    /// Symmetric half-away-from-zero (the only canonical mode).
    HalfAwayFromZero,
}

/// Round `numerator / denominator` half away from zero (`denominator > 0`).
///
/// Guarantees `round(-n, d) == -round(n, d)` for all `n`, `d`: the magnitude
/// is rounded with `(abs + half) / denominator` and the sign re-applied.
///
/// Crate-visible so catalog conversions share the identical contract.
pub(crate) fn round_half_away_from_zero(numerator: i128, denominator: i128) -> i128 {
    debug_assert!(denominator > 0);
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        -((-numerator + half) / denominator)
    }
}

/// Money in DZD centimes. `1 DZD = 100 centimes`. Never a float.
///
/// Wire format is the canonical string of integer centimes
/// (e.g. `"125050"` for 1,250.50 DZD).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(i64);

impl Money {
    /// Build from raw centimes (const-compatible; no validation).
    pub const fn from_centimes(centimes: i64) -> Self {
        Self(centimes)
    }

    /// Raw centimes value.
    pub const fn centimes(self) -> i64 {
        self.0
    }

    /// Zero amount.
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Checked addition.
    pub fn checked_add(self, other: Self) -> Result<Self, DomainError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(DomainError::MoneyOverflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, other: Self) -> Result<Self, DomainError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(DomainError::MoneyOverflow)
    }

    /// Line total `price x quantity`, rounded half away from zero.
    ///
    /// Computes `numerator = centimes * qty_micros` in `i128`, then
    /// [`RoundingMode::HalfAwayFromZero`] over a 10_000 denominator, so
    /// negative amounts (returns, reversals) round symmetrically with sales.
    pub fn checked_mul_qty(self, qty: QtyMicro) -> Result<Self, DomainError> {
        self.checked_mul_qty_with_rounding(qty, RoundingMode::HalfAwayFromZero)
    }

    /// Line total with an explicit rounding policy.
    pub fn checked_mul_qty_with_rounding(
        self,
        qty: QtyMicro,
        mode: RoundingMode,
    ) -> Result<Self, DomainError> {
        let product = (self.0 as i128)
            .checked_mul(qty.micros() as i128)
            .ok_or(DomainError::MoneyOverflow)?;
        let rounded = match mode {
            RoundingMode::HalfAwayFromZero => round_half_away_from_zero(product, 10_000),
        };
        if rounded < i64::MIN as i128 || rounded > i64::MAX as i128 {
            return Err(DomainError::MoneyOverflow);
        }
        Ok(Self(rounded as i64))
    }

    /// Split `total` across `shares` with zero rounding drift.
    ///
    /// Implements the Largest Remainder (Hamilton) method: every share is
    /// floored, then the leftover centimes go one by one to the shares with
    /// the largest fractional remainders (ties broken by share order), so
    /// `SUM(distributed) == total.centimes()` always holds.
    ///
    /// Negative totals are explicitly rejected with
    /// [`DomainError::NegativeMoneyForbidden`]: the Hamilton floor/remainder
    /// decomposition assumes a non-negative total, and generalizing it to
    /// negative totals is out of scope. This is an intentional restriction,
    /// not silent truncation - callers distribute magnitudes only.
    pub fn distribute_proportional(total: Self, shares: &[u64]) -> Result<Vec<Self>, DomainError> {
        if total.0 < 0 {
            return Err(DomainError::NegativeMoneyForbidden);
        }
        if shares.is_empty() {
            return Err(DomainError::DivisionByZero);
        }
        let total_shares: u128 = shares.iter().fold(0u128, |acc, s| acc + (*s as u128));
        if total_shares == 0 {
            return Err(DomainError::DivisionByZero);
        }
        let total_c = total.0 as i128;
        let divisor = total_shares as i128;
        let mut out = Vec::with_capacity(shares.len());
        let mut remainders: Vec<(u128, usize)> = Vec::with_capacity(shares.len());
        let mut floored_sum: i128 = 0;
        for (index, share) in shares.iter().enumerate() {
            let numerator = total_c
                .checked_mul(*share as i128)
                .ok_or(DomainError::MoneyOverflow)?;
            let base = numerator / divisor;
            let remainder = (numerator % divisor) as u128;
            floored_sum += base;
            // `base <= total_c <= i64::MAX`, so the cast is exact.
            out.push(Self(base as i64));
            remainders.push((remainder, index));
        }
        // `0 <= leftover < shares.len()` by construction of the floor step.
        let leftover = (total_c - floored_sum) as usize;
        // Stable sort: ties keep original share order (deterministic).
        remainders.sort_by_key(|remainder| Reverse(remainder.0));
        for (_, index) in remainders.iter().take(leftover) {
            out[*index] = Self(out[*index].0 + 1);
        }
        Ok(out)
    }

    /// Canonical display string, e.g. `1250.50 DZD`.
    pub fn format_dzd(self) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        format!("{}{}.{:02} DZD", sign, abs / 100, abs % 100)
    }
}

impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.parse::<i64>()
            .map(Self)
            .map_err(|_| D::Error::custom(DomainError::InvalidCanonicalString(text)))
    }
}

/// Quantity in micros at scale 10^4 (`1.0000 = 10_000 micros`).
///
/// Wire format is the canonical string of integer micros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QtyMicro(i64);

impl QtyMicro {
    /// Build from raw micros (const-compatible; no validation).
    pub const fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Raw micros value.
    pub const fn micros(self) -> i64 {
        self.0
    }

    /// Checked addition.
    pub fn checked_add(self, other: Self) -> Result<Self, DomainError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(DomainError::MoneyOverflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, other: Self) -> Result<Self, DomainError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(DomainError::MoneyOverflow)
    }

    /// Convert a selling-unit quantity to base-unit quantity:
    /// `base_qty = selling_qty * factor`, rounded half away from zero.
    pub fn checked_mul_factor(self, factor: FactorMicro) -> Result<Self, DomainError> {
        let product = (self.0 as i128)
            .checked_mul(factor.micros() as i128)
            .ok_or(DomainError::MoneyOverflow)?;
        let rounded = round_half_away_from_zero(product, 10_000);
        if rounded < i64::MIN as i128 || rounded > i64::MAX as i128 {
            return Err(DomainError::MoneyOverflow);
        }
        Ok(Self(rounded as i64))
    }
}

impl Serialize for QtyMicro {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for QtyMicro {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.parse::<i64>()
            .map(Self)
            .map_err(|_| D::Error::custom(DomainError::InvalidCanonicalString(text)))
    }
}

/// Conversion factor between a selling unit and the base unit.
///
/// Scale 10^4 (`1.0000 = 10_000`): `base_qty = selling_qty * factor`.
/// Wire format is the canonical string of integer micros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactorMicro(i64);

impl FactorMicro {
    /// Build from raw micros (const-compatible; no validation).
    pub const fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Raw micros value.
    pub const fn micros(self) -> i64 {
        self.0
    }
}

impl Serialize for FactorMicro {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for FactorMicro {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.parse::<i64>()
            .map(Self)
            .map_err(|_| D::Error::custom(DomainError::InvalidCanonicalString(text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_add_sub_basics() {
        let total = Money::from_centimes(10_000)
            .checked_add(Money::from_centimes(2_500))
            .expect("add");
        assert_eq!(total.centimes(), 12_500);
        let rest = total.checked_sub(Money::from_centimes(500)).expect("sub");
        assert_eq!(rest.centimes(), 12_000);
    }

    #[test]
    fn money_overflow_boundaries() {
        assert_eq!(
            Money::from_centimes(i64::MAX).checked_add(Money::from_centimes(1)),
            Err(DomainError::MoneyOverflow)
        );
        assert_eq!(
            Money::from_centimes(i64::MIN).checked_sub(Money::from_centimes(1)),
            Err(DomainError::MoneyOverflow)
        );
    }

    #[test]
    fn money_mul_qty_exact_and_half_up() {
        // 100.50 DZD x 3.0000 = 301.50 DZD.
        let line = Money::from_centimes(10_050)
            .checked_mul_qty(QtyMicro::from_micros(30_000))
            .expect("exact line");
        assert_eq!(line.centimes(), 30_150);
        // 1 centime x 0.5000 = 0.5 centime -> half-up to 1.
        let tie = Money::from_centimes(1)
            .checked_mul_qty(QtyMicro::from_micros(5_000))
            .expect("tie rounds up");
        assert_eq!(tie.centimes(), 1);
        // 1.00 DZD x 0.3333 = 33.33 centimes -> 33.
        let down = Money::from_centimes(100)
            .checked_mul_qty(QtyMicro::from_micros(3_333))
            .expect("rounds down");
        assert_eq!(down.centimes(), 33);
    }

    #[test]
    fn round_half_away_from_zero_is_symmetric() {
        // Exact halves round away from zero in both directions.
        assert_eq!(round_half_away_from_zero(5_000, 10_000), 1);
        assert_eq!(round_half_away_from_zero(-5_000, 10_000), -1);
        assert_eq!(round_half_away_from_zero(15_000, 10_000), 2);
        assert_eq!(round_half_away_from_zero(-15_000, 10_000), -2);
        // Zero numerator stays zero.
        assert_eq!(round_half_away_from_zero(0, 10_000), 0);
        // Below-half truncates, above-half rounds, symmetrically.
        assert_eq!(round_half_away_from_zero(4_999, 10_000), 0);
        assert_eq!(round_half_away_from_zero(-4_999, 10_000), 0);
        assert_eq!(round_half_away_from_zero(5_001, 10_000), 1);
        assert_eq!(round_half_away_from_zero(-5_001, 10_000), -1);
        // Symmetry property over a wide numerator band.
        for numerator in (-200_000i128..=200_000).step_by(997) {
            assert_eq!(
                round_half_away_from_zero(-numerator, 10_000),
                -round_half_away_from_zero(numerator, 10_000),
                "asymmetric at numerator={numerator}"
            );
        }
    }

    #[test]
    fn checked_mul_qty_is_sign_symmetric() {
        // Money::from_centimes is a const constructor with no sign
        // validation, so negative amounts are directly expressible and
        // must round as the mirror image of their positive counterpart.
        let prices = [1i64, 5, 33, 100, 12_345, 999_999];
        let quantities = [1i64, 5_000, 10_000, 13_333, 25_000, 77_777];
        for price in prices {
            for qty_micros in quantities {
                let qty = QtyMicro::from_micros(qty_micros);
                let positive = Money::from_centimes(price)
                    .checked_mul_qty(qty)
                    .expect("positive line");
                let negative = Money::from_centimes(-price)
                    .checked_mul_qty(qty)
                    .expect("negative line");
                assert_eq!(
                    negative.centimes(),
                    -positive.centimes(),
                    "asymmetric for price={price} qty_micros={qty_micros}"
                );
            }
        }
    }

    #[test]
    fn checked_mul_qty_handles_sign_edge_cases() {
        // Double negative yields the positive magnitude result.
        let debt_relief = Money::from_centimes(-10_050)
            .checked_mul_qty(QtyMicro::from_micros(-30_000))
            .expect("double negative");
        assert_eq!(debt_relief.centimes(), 30_150);
        // Exact negative half rounds away from zero: -1.5 centimes -> -2.
        let negative_tie = Money::from_centimes(-3)
            .checked_mul_qty(QtyMicro::from_micros(5_000))
            .expect("negative tie");
        assert_eq!(negative_tie.centimes(), -2);
        // Zero quantity zeroes any amount, including the largest.
        let zeroed = Money::from_centimes(i64::MAX)
            .checked_mul_qty(QtyMicro::from_micros(0))
            .expect("zero quantity");
        assert_eq!(zeroed.centimes(), 0);
        // Zero amount stays zero against any quantity.
        let nothing = Money::zero()
            .checked_mul_qty(QtyMicro::from_micros(77_777))
            .expect("zero amount");
        assert_eq!(nothing.centimes(), 0);
    }

    #[test]
    fn distribute_has_zero_drift() {
        let out = Money::distribute_proportional(Money::from_centimes(100), &[1, 1, 1])
            .expect("distribute");
        assert_eq!(
            out.iter().copied().map(Money::centimes).collect::<Vec<_>>(),
            vec![34, 33, 33]
        );
        // Sweep totals and share shapes; the distributed sum must equal
        // the total every single time.
        let shapes: [&[u64]; 5] = [
            &[1, 1],
            &[1, 1, 1],
            &[3, 3, 4],
            &[7, 1, 1, 1],
            &[5, 5, 5, 5, 5],
        ];
        for total in [0i64, 1, 2, 99, 100, 101, 12_345, 1_000_000] {
            for shares in shapes {
                let parts = Money::distribute_proportional(Money::from_centimes(total), shares)
                    .expect("distribute sweep");
                assert_eq!(parts.len(), shares.len());
                let sum: i64 = parts.iter().copied().map(Money::centimes).sum();
                assert_eq!(sum, total, "drift for total={total} shares={shares:?}");
            }
        }
    }

    #[test]
    fn distribute_rejects_degenerate_inputs() {
        assert_eq!(
            Money::distribute_proportional(Money::from_centimes(-1), &[1, 1]),
            Err(DomainError::NegativeMoneyForbidden)
        );
        assert_eq!(
            Money::distribute_proportional(Money::from_centimes(10), &[]),
            Err(DomainError::DivisionByZero)
        );
        assert_eq!(
            Money::distribute_proportional(Money::from_centimes(10), &[0, 0]),
            Err(DomainError::DivisionByZero)
        );
        let empty = Money::distribute_proportional(Money::zero(), &[4, 6]).expect("zero total");
        assert_eq!(
            empty
                .iter()
                .copied()
                .map(Money::centimes)
                .collect::<Vec<_>>(),
            vec![0, 0]
        );
    }

    #[test]
    fn wire_format_is_canonical_string() {
        let json = serde_json::to_string(&Money::from_centimes(125_050)).expect("serialize");
        assert_eq!(json, "\"125050\"");
        let back: Money = serde_json::from_str("\"125050\"").expect("deserialize");
        assert_eq!(back.centimes(), 125_050);
        let bad: Result<Money, _> = serde_json::from_str("\"12x50\"");
        assert!(bad.is_err());
        let qty_json = serde_json::to_string(&QtyMicro::from_micros(25_000)).expect("qty");
        assert_eq!(qty_json, "\"25000\"");
        let factor_json = serde_json::to_string(&FactorMicro::from_micros(10_000)).expect("factor");
        assert_eq!(factor_json, "\"10000\"");
    }

    #[test]
    fn format_dzd_shapes() {
        assert_eq!(Money::from_centimes(125_050).format_dzd(), "1250.50 DZD");
        assert_eq!(Money::zero().format_dzd(), "0.00 DZD");
        assert_eq!(Money::from_centimes(5).format_dzd(), "0.05 DZD");
        assert_eq!(Money::from_centimes(-12_505).format_dzd(), "-125.05 DZD");
    }

    #[test]
    fn qty_factor_conversions() {
        // 4.0000 selling units x factor 2.5000 = 10.0000 base units.
        let base = QtyMicro::from_micros(40_000)
            .checked_mul_factor(FactorMicro::from_micros(25_000))
            .expect("convert");
        assert_eq!(base.micros(), 100_000);
        // Identity factor is exact.
        let same = QtyMicro::from_micros(12_345)
            .checked_mul_factor(FactorMicro::from_micros(10_000))
            .expect("identity");
        assert_eq!(same.micros(), 12_345);
        assert_eq!(
            QtyMicro::from_micros(i64::MAX).checked_add(QtyMicro::from_micros(1)),
            Err(DomainError::MoneyOverflow)
        );
    }
}
