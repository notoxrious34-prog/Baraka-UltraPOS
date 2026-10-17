//! Pure financial pricing kernel (no I/O, no floats).
//!
//! All amounts are integer centimes; all quantities integer micros.
//! Rounding is canonical half-away-from-zero throughout.

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use crate::primitives::{round_half_away_from_zero, Money, QtyMicro};
use crate::sales::document::DocumentType;
use crate::sales::payment::PaymentMethod;

// ---------------------------------------------------------------------------
// Line pricing
// ---------------------------------------------------------------------------

/// One priced cart line input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricedLineInput {
    /// Quantity in selling-unit micros (must be positive).
    pub quantity_micro: i64,
    /// Agreed unit price in centimes.
    pub unit_price_centimes: i64,
    /// Line discount in centimes (`<=` gross).
    pub line_discount_centimes: i64,
    /// TVA rate in basis points (19% = 1900).
    pub tva_rate_basis_points: i64,
    /// Display order (deterministic discount tiebreak).
    pub sort_order: i64,
}

/// One priced line output (all centimes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricedLine {
    /// `round(qty * price)`.
    pub line_gross: i64,
    /// Gross minus line discount.
    pub line_after_discount: i64,
    /// Hare-Niemeyer share of the global discount.
    pub allocated_global_discount: i64,
    /// Taxable HT after all discounts.
    pub line_taxable_ht: i64,
    /// Line TVA.
    pub line_tva: i64,
    /// Line TTC.
    pub line_ttc: i64,
}

/// Gross for one line: `round(qty_micro * price / 10_000)`.
pub fn price_line_gross(quantity_micro: i64, unit_price_centimes: i64) -> Result<i64, DomainError> {
    if quantity_micro <= 0 {
        return Err(DomainError::InvalidQuantity);
    }
    Money::from_centimes(unit_price_centimes)
        .checked_mul_qty(QtyMicro::from_micros(quantity_micro))
        .map(|money| money.centimes())
}

/// Margin Guard (Pillar 4): integer-exact margin in basis points
/// (`100 bps = 1.00%`, `10_000 bps = 100.00%`) plus a below-cost flag.
///
/// Formula: `((selling - cost) * 10_000) / selling`, computed in `i128` —
/// no floats, no overflow. A non-positive selling price with a positive cost
/// is a total loss (`-10_000 bps`, below cost); a zero price with zero cost
/// is neutral.
pub fn calculate_margin_basis_points(
    selling_price_centimes: i64,
    cost_price_centimes: i64,
) -> (i64, bool) {
    if selling_price_centimes <= 0 {
        if cost_price_centimes > 0 {
            (-10_000, true)
        } else {
            (0, false)
        }
    } else {
        let diff = (selling_price_centimes as i128) - (cost_price_centimes as i128);
        let bps = (diff * 10_000) / (selling_price_centimes as i128);
        let bps = bps.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        (bps, selling_price_centimes < cost_price_centimes)
    }
}

/// Line TVA: `round(taxable_ht * rate_bp / 10_000)`.
pub fn compute_line_tva(
    taxable_ht_centimes: i64,
    tva_rate_basis_points: i64,
) -> Result<i64, DomainError> {
    if tva_rate_basis_points < 0 {
        return Err(DomainError::InvalidCanonicalString(
            "tva rate must be >= 0".to_string(),
        ));
    }
    let numerator = (i128::from(taxable_ht_centimes))
        .checked_mul(i128::from(tva_rate_basis_points))
        .ok_or(DomainError::MoneyOverflow)?;
    let rounded = round_half_away_from_zero(numerator, 10_000);
    i64::try_from(rounded).map_err(|_| DomainError::MoneyOverflow)
}

// ---------------------------------------------------------------------------
// Global discount allocation (Largest Remainder / Hare-Niemeyer)
// ---------------------------------------------------------------------------

/// One allocation base with its deterministic tiebreak order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscountShare {
    /// Line amount after line discounts.
    pub base: i64,
    /// Line display order (ascending tiebreak).
    pub sort_order: i64,
}

/// Split `total_discount` across line bases with zero drift.
///
/// Floors every share, then deals the leftover cents one by one to the
/// largest fractional remainders (ties by `sort_order` ascending), so
/// `SUM(allocated) == total_discount` always holds.
pub fn allocate_global_discount(
    total_discount: i64,
    shares: &[DiscountShare],
) -> Result<Vec<i64>, DomainError> {
    if total_discount < 0 {
        return Err(DomainError::NegativeMoneyForbidden);
    }
    let gross: i128 = shares.iter().map(|share| i128::from(share.base)).sum();
    if gross < 0 {
        return Err(DomainError::NegativeMoneyForbidden);
    }
    if gross == 0 {
        return Ok(vec![0; shares.len()]);
    }
    if i128::from(total_discount) > gross {
        return Err(DomainError::DiscountExceedsTotal);
    }
    let total = i128::from(total_discount);
    let mut allocated = Vec::with_capacity(shares.len());
    let mut remainders: Vec<(u128, i64, usize)> = Vec::with_capacity(shares.len());
    let mut floored_sum: i128 = 0;
    for (index, share) in shares.iter().enumerate() {
        let numerator = total
            .checked_mul(i128::from(share.base))
            .ok_or(DomainError::MoneyOverflow)?;
        let base = numerator / gross;
        let remainder = (numerator % gross) as u128;
        floored_sum += base;
        allocated.push(base);
        remainders.push((remainder, share.sort_order, index));
    }
    let mut leftover = (total - floored_sum) as usize;
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut cursor = 0;
    while leftover > 0 {
        let index = remainders
            .get(cursor)
            .map(|entry| entry.2)
            .ok_or(DomainError::MoneyOverflow)?;
        allocated[index] += 1;
        leftover -= 1;
        cursor += 1;
    }
    allocated
        .into_iter()
        .map(|value| i64::try_from(value).map_err(|_| DomainError::MoneyOverflow))
        .collect()
}

// ---------------------------------------------------------------------------
// Timbre fiscal (non-circular Algerian formula)
// ---------------------------------------------------------------------------

/// Timbre bounds in centimes (5 DZD .. 2,500 DZD).
pub const TIMBRE_MIN_CENTIMES: i64 = 500;
/// Timbre bounds in centimes (5 DZD .. 2,500 DZD).
pub const TIMBRE_MAX_CENTIMES: i64 = 250_000;

/// Non-circular timbre: 1% of (HT + TVA), ceiling to 100 centimes, clamped.
///
/// Applies only to TICKET/FACTURE documents with a cash tender > 0.
/// `base_for_timbre` deliberately excludes any timbre (non-circular).
pub fn compute_timbre(
    doc_type: DocumentType,
    cash_tendered_centimes: i64,
    base_for_timbre_centimes: i64,
) -> i64 {
    let applicable = matches!(doc_type, DocumentType::Ticket | DocumentType::Facture)
        && cash_tendered_centimes > 0;
    if !applicable {
        return 0;
    }
    // ceil(base * 0.01 / 100) * 100, all-integer: ((base + 9999) / 10000) * 100.
    let stepped = (base_for_timbre_centimes.saturating_add(9_999) / 10_000).saturating_mul(100);
    stepped.clamp(TIMBRE_MIN_CENTIMES, TIMBRE_MAX_CENTIMES)
}

// ---------------------------------------------------------------------------
// Tender settlement
// ---------------------------------------------------------------------------

/// One tender line for settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentShare {
    /// Tender method.
    pub method: PaymentMethod,
    /// Tendered amount (must be positive).
    pub amount_centimes: i64,
}

/// Settlement outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenderResult {
    /// Total tendered.
    pub paid_total_centimes: i64,
    /// Change owed (cash surplus only).
    pub change_centimes: i64,
    /// Uncovered remainder (credit-gated upstream).
    pub remaining_due_centimes: i64,
}

/// Settle tender against a grand total.
///
/// - Every line must be positive, else `InvalidQuantity`.
/// - Surplus is allowed only against cash: non-cash tender can never exceed
///   the total, else `ChangeNotPermittedForNonCash`.
pub fn settle_payments(
    total_ttc_centimes: i64,
    payments: &[PaymentShare],
) -> Result<TenderResult, DomainError> {
    let mut paid: i128 = 0;
    let mut non_cash: i128 = 0;
    for payment in payments {
        if payment.amount_centimes <= 0 {
            return Err(DomainError::InvalidQuantity);
        }
        paid = paid
            .checked_add(i128::from(payment.amount_centimes))
            .ok_or(DomainError::MoneyOverflow)?;
        if !payment.method.is_cash() {
            non_cash = non_cash
                .checked_add(i128::from(payment.amount_centimes))
                .ok_or(DomainError::MoneyOverflow)?;
        }
    }
    let total = i128::from(total_ttc_centimes);
    if paid < total {
        return Ok(TenderResult {
            paid_total_centimes: i64::try_from(paid).map_err(|_| DomainError::MoneyOverflow)?,
            change_centimes: 0,
            remaining_due_centimes: i64::try_from(total - paid)
                .map_err(|_| DomainError::MoneyOverflow)?,
        });
    }
    if non_cash > total {
        return Err(DomainError::ChangeNotPermittedForNonCash);
    }
    Ok(TenderResult {
        paid_total_centimes: i64::try_from(paid).map_err(|_| DomainError::MoneyOverflow)?,
        change_centimes: i64::try_from(paid - total).map_err(|_| DomainError::MoneyOverflow)?,
        remaining_due_centimes: 0,
    })
}

// ---------------------------------------------------------------------------
// Full document pricing
// ---------------------------------------------------------------------------

/// Fully priced document (all centimes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricedDocument {
    /// Per-line pricing in input order.
    pub lines: Vec<PricedLine>,
    /// Sum of taxable HT.
    pub subtotal_ht: i64,
    /// Sum of line TVA.
    pub total_tva: i64,
    /// Echo of the requested global discount.
    pub global_discount: i64,
    /// Timbre fiscal.
    pub timbre: i64,
    /// Grand total.
    pub total_ttc: i64,
    /// Total tendered.
    pub paid_total: i64,
    /// Change owed.
    pub change: i64,
    /// Uncovered remainder.
    pub remaining_due: i64,
}

/// Price a whole document: gross, discounts, allocation, TVA, timbre, tender.
pub fn price_document(
    lines: &[PricedLineInput],
    global_discount_centimes: i64,
    doc_type: DocumentType,
    payments: &[PaymentShare],
    timbre_enabled: bool,
) -> Result<PricedDocument, DomainError> {
    if lines.is_empty() {
        return Err(DomainError::EmptyCart);
    }
    if global_discount_centimes < 0 {
        return Err(DomainError::NegativeMoneyForbidden);
    }
    // Gross + line discounts.
    let mut gross_lines: Vec<(i64, i64)> = Vec::with_capacity(lines.len());
    for line in lines {
        if line.line_discount_centimes < 0 {
            return Err(DomainError::NegativeMoneyForbidden);
        }
        let gross = price_line_gross(line.quantity_micro, line.unit_price_centimes)?;
        if line.line_discount_centimes > gross {
            return Err(DomainError::LineDiscountExceedsGross);
        }
        gross_lines.push((gross, gross - line.line_discount_centimes));
    }
    // Global allocation over after-discount bases.
    let shares: Vec<DiscountShare> = lines
        .iter()
        .zip(gross_lines.iter())
        .map(|(line, (_, after))| DiscountShare {
            base: *after,
            sort_order: line.sort_order,
        })
        .collect();
    let allocated = allocate_global_discount(global_discount_centimes, &shares)?;
    // TVA per line + totals.
    let mut priced = Vec::with_capacity(lines.len());
    let mut subtotal_ht: i128 = 0;
    let mut total_tva: i128 = 0;
    for ((line, (_, after)), share) in lines.iter().zip(gross_lines.iter()).zip(allocated.iter()) {
        let taxable = after
            .checked_sub(*share)
            .ok_or(DomainError::MoneyOverflow)?;
        let tva = compute_line_tva(taxable, line.tva_rate_basis_points)?;
        let ttc = taxable.checked_add(tva).ok_or(DomainError::MoneyOverflow)?;
        subtotal_ht = subtotal_ht
            .checked_add(i128::from(taxable))
            .ok_or(DomainError::MoneyOverflow)?;
        total_tva = total_tva
            .checked_add(i128::from(tva))
            .ok_or(DomainError::MoneyOverflow)?;
        priced.push(PricedLine {
            line_gross: gross_lines[priced.len()].0,
            line_after_discount: *after,
            allocated_global_discount: *share,
            line_taxable_ht: taxable,
            line_tva: tva,
            line_ttc: ttc,
        });
    }
    let subtotal_ht = i64::try_from(subtotal_ht).map_err(|_| DomainError::MoneyOverflow)?;
    let total_tva = i64::try_from(total_tva).map_err(|_| DomainError::MoneyOverflow)?;
    let cash_tendered: i64 = payments
        .iter()
        .filter(|payment| payment.method.is_cash())
        .map(|payment| i128::from(payment.amount_centimes))
        .sum::<i128>()
        .try_into()
        .map_err(|_| DomainError::MoneyOverflow)?;
    let timbre_base = subtotal_ht
        .checked_add(total_tva)
        .ok_or(DomainError::MoneyOverflow)?;
    // ADR-092: the fiscal stamp is configurable; when disabled the kernel
    // prices timbre as exactly zero (same gate as preview and checkout).
    let timbre = if timbre_enabled {
        compute_timbre(doc_type, cash_tendered, timbre_base)
    } else {
        0
    };
    let total_ttc = subtotal_ht
        .checked_add(total_tva)
        .and_then(|sum| sum.checked_add(timbre))
        .ok_or(DomainError::MoneyOverflow)?;
    let tender = settle_payments(total_ttc, payments)?;
    Ok(PricedDocument {
        lines: priced,
        subtotal_ht,
        total_tva,
        global_discount: global_discount_centimes,
        timbre,
        total_ttc,
        paid_total: tender.paid_total_centimes,
        change: tender.change_centimes,
        remaining_due: tender.remaining_due_centimes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(qty: i64, price: i64, discount: i64, tva: i64, order: i64) -> PricedLineInput {
        PricedLineInput {
            quantity_micro: qty,
            unit_price_centimes: price,
            line_discount_centimes: discount,
            tva_rate_basis_points: tva,
            sort_order: order,
        }
    }

    #[test]
    fn line_gross_and_tva_shapes() {
        // 2 pieces x 1,250.50 = 2,501.00.
        assert_eq!(price_line_gross(20_000, 125_050), Ok(250_100));
        // 19% of 10,000 = 1,900.
        assert_eq!(compute_line_tva(10_000, 1_900), Ok(1_900));
        // Half-away: 19% of 1 cent (0.19) rounds to 0; of 3 cents (0.57) to 1.
        assert_eq!(compute_line_tva(1, 1_900), Ok(0));
        assert_eq!(compute_line_tva(3, 1_900), Ok(1));
        assert_eq!(price_line_gross(0, 100), Err(DomainError::InvalidQuantity));
    }

    #[test]
    fn allocation_sums_exactly_with_tiebreak() {
        // 100 discount over bases [60, 30, 10]: floors [60,30,10], exact.
        let shares = [
            DiscountShare {
                base: 60,
                sort_order: 0,
            },
            DiscountShare {
                base: 30,
                sort_order: 1,
            },
            DiscountShare {
                base: 10,
                sort_order: 2,
            },
        ];
        assert_eq!(allocate_global_discount(100, &shares), Ok(vec![60, 30, 10]));
        // 10 discount over [34, 33, 33]: floors [3,3,3], leftover 1 goes to
        // the largest remainder (index 0 either way here).
        let tight = [
            DiscountShare {
                base: 34,
                sort_order: 0,
            },
            DiscountShare {
                base: 33,
                sort_order: 1,
            },
            DiscountShare {
                base: 33,
                sort_order: 2,
            },
        ];
        let out = allocate_global_discount(10, &tight).expect("allocate");
        assert_eq!(out.iter().sum::<i64>(), 10);
        // Discount above gross is rejected.
        assert_eq!(
            allocate_global_discount(101, &shares),
            Err(DomainError::DiscountExceedsTotal)
        );
        // Zero gross allocates zero everywhere.
        let zeros = [
            DiscountShare {
                base: 0,
                sort_order: 0,
            },
            DiscountShare {
                base: 0,
                sort_order: 1,
            },
        ];
        assert_eq!(allocate_global_discount(0, &zeros), Ok(vec![0, 0]));
    }

    #[test]
    fn timbre_bounds_and_gating() {
        // No cash -> zero, even on tickets.
        assert_eq!(compute_timbre(DocumentType::Ticket, 0, 1_000_000), 0);
        // BL never carries timbre, even with cash.
        assert_eq!(compute_timbre(DocumentType::Bl, 50_000, 1_000_000), 0);
        // Small base clamps to the 500 minimum.
        assert_eq!(compute_timbre(DocumentType::Ticket, 5_000, 10_000), 500);
        // 1% of 1,000,000 = 10,000, already on a 100 step.
        assert_eq!(
            compute_timbre(DocumentType::Facture, 1_000_000, 1_000_000),
            10_000
        );
        // Ceiling step: 1% of 1,000,050 = 10,000.5 -> 10,100.
        assert_eq!(
            compute_timbre(DocumentType::Ticket, 2_000_000, 1_000_050),
            10_100
        );
        // Huge base clamps to the 250,000 maximum.
        assert_eq!(
            compute_timbre(DocumentType::Ticket, 99_000_000, 99_000_000),
            250_000
        );
    }

    #[test]
    fn tender_change_rules() {
        let cash = |amount| PaymentShare {
            method: PaymentMethod::Cash,
            amount_centimes: amount,
        };
        let card = |amount| PaymentShare {
            method: PaymentMethod::Card,
            amount_centimes: amount,
        };
        // Exact cash: no change.
        let exact = settle_payments(1_000, &[cash(1_000)]).expect("exact");
        assert_eq!(
            (exact.change_centimes, exact.remaining_due_centimes),
            (0, 0)
        );
        // Cash overpay: change allowed.
        let over = settle_payments(1_000, &[cash(1_500)]).expect("over");
        assert_eq!(over.change_centimes, 500);
        // Card overpay: rejected.
        assert_eq!(
            settle_payments(1_000, &[card(1_500)]),
            Err(DomainError::ChangeNotPermittedForNonCash)
        );
        // Mixed exact (card share within total): accepted, no change.
        let mixed = settle_payments(1_000, &[cash(400), card(600)]).expect("mixed");
        assert_eq!(
            (mixed.paid_total_centimes, mixed.change_centimes),
            (1_000, 0)
        );
        // Underpay: remainder reported.
        let short = settle_payments(1_000, &[cash(600)]).expect("short");
        assert_eq!(short.remaining_due_centimes, 400);
        // Zero line rejected.
        assert_eq!(
            settle_payments(1_000, &[cash(0)]),
            Err(DomainError::InvalidQuantity)
        );
    }

    #[test]
    fn full_document_prices_end_to_end() {
        let lines = [
            input(20_000, 10_000, 0, 1_900, 0),
            input(10_000, 5_000, 500, 1_900, 1),
        ];
        let payments = [PaymentShare {
            method: PaymentMethod::Cash,
            amount_centimes: 30_000,
        }];
        let priced =
            price_document(&lines, 1_000, DocumentType::Ticket, &payments, true).expect("price");
        // Gross: 20,000 + 4,500 after line discount; global 1,000 splits.
        assert_eq!(priced.global_discount, 1_000);
        let allocated: i64 = priced
            .lines
            .iter()
            .map(|line| line.allocated_global_discount)
            .sum();
        assert_eq!(allocated, 1_000);
        assert_eq!(
            priced.subtotal_ht,
            priced
                .lines
                .iter()
                .map(|line| line.line_taxable_ht)
                .sum::<i64>()
        );
        assert_eq!(
            priced.total_ttc,
            priced.subtotal_ht + priced.total_tva + priced.timbre
        );
        assert!(priced.change >= 0);
    }

    #[test]
    fn empty_cart_and_bad_discounts_rejected() {
        assert_eq!(
            price_document(&[], 0, DocumentType::Ticket, &[], true),
            Err(DomainError::EmptyCart)
        );
        let bad = [input(10_000, 1_000, 2_000, 0, 0)];
        assert_eq!(
            price_document(&bad, 0, DocumentType::Ticket, &[], true),
            Err(DomainError::LineDiscountExceedsGross)
        );
    }

    #[test]
    fn timbre_toggle_gates_stamp_computation() {
        // Base 50,000 with 60,000 cash prices a 500 stamp when enabled.
        let lines = [input(10_000, 50_000, 0, 0, 0)];
        let payments = [PaymentShare {
            method: PaymentMethod::Cash,
            amount_centimes: 60_000,
        }];
        let enabled =
            price_document(&lines, 0, DocumentType::Ticket, &payments, true).expect("price");
        assert_eq!(enabled.timbre, 500);
        assert_eq!(enabled.total_ttc, 50_000 + 500);
        let disabled =
            price_document(&lines, 0, DocumentType::Ticket, &payments, false).expect("price");
        assert_eq!(disabled.timbre, 0);
        assert_eq!(disabled.total_ttc, 50_000);
    }

    #[test]
    fn margin_guard_vectors() {
        // Profitable: (15000-10000)*10000/15000 = 3333 bps.
        assert_eq!(calculate_margin_basis_points(15_000, 10_000), (3333, false));
        // Break-even.
        assert_eq!(calculate_margin_basis_points(10_000, 10_000), (0, false));
        // Loss: (8000-10000)*10000/8000 = -2500 bps, below cost.
        assert_eq!(calculate_margin_basis_points(8_000, 10_000), (-2500, true));
        // Zero cost: full 10000 bps margin.
        assert_eq!(calculate_margin_basis_points(10_000, 0), (10_000, false));
        // Zero price with cost: total loss.
        assert_eq!(calculate_margin_basis_points(0, 10_000), (-10_000, true));
        // Zero price, zero cost: neutral.
        assert_eq!(calculate_margin_basis_points(0, 0), (0, false));
    }
}
