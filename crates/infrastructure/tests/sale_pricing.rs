//! Pricing kernel property and vector tests (pure domain, no DB).

use baraka_domain::sales::document::DocumentType;
use baraka_domain::sales::payment::PaymentMethod;
use baraka_domain::sales::pricing::{
    allocate_global_discount, compute_timbre, price_document, settle_payments, DiscountShare,
    PaymentShare, PricedLineInput,
};

/// Deterministic xorshift64* generator (no new dependencies).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }
}

#[test]
fn global_discount_zero_drift_10k_iterations() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for round in 0..10_000 {
        let lines = 1 + rng.below(8) as usize;
        let mut shares = Vec::with_capacity(lines);
        let mut gross: i64 = 0;
        for order in 0..lines {
            let base = rng.below(100_001) as i64;
            gross += base;
            shares.push(DiscountShare {
                base,
                sort_order: order as i64,
            });
        }
        let discount = if gross == 0 {
            0
        } else {
            rng.below(gross as u64 + 1) as i64
        };
        let allocated = allocate_global_discount(discount, &shares)
            .unwrap_or_else(|error| panic!("round {round}: {error:?}"));
        assert_eq!(allocated.len(), shares.len(), "round {round}");
        assert_eq!(
            allocated.iter().sum::<i64>(),
            discount,
            "drift in round {round}"
        );
        assert!(allocated.iter().all(|share| *share >= 0), "round {round}");
    }
}

#[test]
fn allocation_tiebreak_prefers_lower_sort_order() {
    // Equal bases: remainders tie, so the lowest sort_order wins leftovers.
    let shares = [
        DiscountShare {
            base: 100,
            sort_order: 5,
        },
        DiscountShare {
            base: 100,
            sort_order: 2,
        },
        DiscountShare {
            base: 100,
            sort_order: 9,
        },
    ];
    // 100 over 300: floors [33,33,33], leftover 1 -> sort_order 2 first.
    let out = allocate_global_discount(100, &shares).expect("allocate");
    assert_eq!(out, vec![33, 34, 33]);
}

#[test]
fn timbre_boundary_vectors() {
    // Exact 100-centime step stays put: 1% of 50,000 = 500 -> minimum 500.
    assert_eq!(compute_timbre(DocumentType::Ticket, 60_000, 50_000), 500);
    // Just above a step rounds up: 1% of 50,001 = 500.01 -> 600.
    assert_eq!(compute_timbre(DocumentType::Ticket, 60_000, 50_001), 600);
    // Zero base still clamps to the minimum when cash is tendered.
    assert_eq!(compute_timbre(DocumentType::Ticket, 1_000, 0), 500);
    // FACTURE behaves like TICKET; DEVIS/AVOIR never apply.
    assert_eq!(compute_timbre(DocumentType::Facture, 1_000, 10_000), 500);
    assert_eq!(compute_timbre(DocumentType::Devis, 1_000, 10_000), 0);
    assert_eq!(compute_timbre(DocumentType::Avoir, 1_000, 10_000), 0);
}

#[test]
fn split_tender_shapes() {
    let cash = |amount| PaymentShare {
        method: PaymentMethod::Cash,
        amount_centimes: amount,
    };
    let credit = |amount| PaymentShare {
        method: PaymentMethod::Credit,
        amount_centimes: amount,
    };
    // Cash 60% + card 40% exact: paid, no change.
    let split = settle_payments(
        10_000,
        &[
            cash(6_000),
            PaymentShare {
                method: PaymentMethod::Card,
                amount_centimes: 4_000,
            },
        ],
    )
    .expect("split");
    assert_eq!(
        (split.paid_total_centimes, split.change_centimes),
        (10_000, 0)
    );
    // Cash shortfall with a credit line present: remainder, no error.
    let partial = settle_payments(10_000, &[cash(6_000), credit(3_000)]).expect("partial");
    assert_eq!(partial.remaining_due_centimes, 1_000);
}

#[test]
fn priced_document_invariants_hold() {
    let lines = [
        PricedLineInput {
            quantity_micro: 30_000,
            unit_price_centimes: 12_000,
            line_discount_centimes: 1_000,
            tva_rate_basis_points: 1_900,
            sort_order: 0,
        },
        PricedLineInput {
            quantity_micro: 15_000,
            unit_price_centimes: 8_000,
            line_discount_centimes: 0,
            tva_rate_basis_points: 900,
            sort_order: 1,
        },
    ];
    let priced = price_document(
        &lines,
        2_500,
        DocumentType::Facture,
        &[PaymentShare {
            method: PaymentMethod::Cash,
            amount_centimes: 60_000,
        }],
        true,
    )
    .expect("price");
    // Per-unit line discount (BRK-096): 3.0 x (12,000 - 1,000) = 33,000;
    // 1.5 x 8,000 = 12,000. Global 2,500 over bases [33,000, 12,000]
    // -> allocated [1,833, 667] by largest remainder.
    assert_eq!(
        priced
            .lines
            .iter()
            .map(|line| line.discounted_unit_price_centimes)
            .collect::<Vec<_>>(),
        vec![11_000, 8_000]
    );
    assert_eq!(
        priced
            .lines
            .iter()
            .map(|line| line.line_after_discount)
            .collect::<Vec<_>>(),
        vec![33_000, 12_000]
    );
    assert_eq!(
        priced
            .lines
            .iter()
            .map(|line| line.allocated_global_discount)
            .collect::<Vec<_>>(),
        vec![1_833, 667]
    );
    assert_eq!(
        priced
            .lines
            .iter()
            .map(|line| line.allocated_global_discount)
            .sum::<i64>(),
        2_500
    );
    assert_eq!(
        priced.subtotal_ht,
        priced
            .lines
            .iter()
            .map(|line| line.line_taxable_ht)
            .sum::<i64>()
    );
    assert_eq!(
        priced.total_tva,
        priced.lines.iter().map(|line| line.line_tva).sum::<i64>()
    );
    assert_eq!(
        priced.total_ttc,
        priced.subtotal_ht + priced.total_tva + priced.timbre
    );
    assert_eq!(priced.paid_total, 60_000);
    assert_eq!(priced.change, 60_000 - priced.total_ttc);
}
