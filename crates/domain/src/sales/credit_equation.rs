//! Wholesale credit-equation snapshot (TASK-BRK-108, ADR-110 Pillar 6.3).
//!
//! A finalized credit document must be its own printed witness:
//!
//! ```text
//! charge (sale / BL / facture):  Ancien + Montant - Versement = Nouveau
//! avoir  (return / credit note): Ancien - Montant + Versement = Nouveau
//! ```
//!
//! The ledger stays the single source of truth (append-only, cached-balance
//! invariant untouched). This module freezes the exact balance movement at
//! finalization so a reprint years later cannot drift with later payments.
//!
//! Pure: no I/O, no floats. Every intermediate runs in `i128` with an
//! explicit overflow error, per the constitutional money law.

use crate::errors::DomainError;

/// Frozen balance movement of ONE finalized document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreditSnapshot {
    /// Customer ledger balance immediately before this document.
    ancien_solde_centimes: i64,
    /// Signed movement this document applies to the balance.
    /// Positive on a credit sale, negative on an AVOIR.
    credit_delta_centimes: i64,
    /// Balance immediately after this document.
    nouveau_solde_centimes: i64,
}

impl CreditSnapshot {
    /// Freeze `(before, delta)` into a snapshot.
    ///
    /// # Errors
    /// [`DomainError::MoneyOverflow`] when `before + delta` leaves `i64`.
    pub fn freeze(
        ancien_solde_centimes: i64,
        credit_delta_centimes: i64,
    ) -> Result<Self, DomainError> {
        let nouveau = i128::from(ancien_solde_centimes)
            .checked_add(i128::from(credit_delta_centimes))
            .ok_or(DomainError::MoneyOverflow)?;
        let nouveau_solde_centimes =
            i64::try_from(nouveau).map_err(|_| DomainError::MoneyOverflow)?;
        Ok(Self {
            ancien_solde_centimes,
            credit_delta_centimes,
            nouveau_solde_centimes,
        })
    }

    /// Customer balance before this document.
    pub fn ancien_solde_centimes(self) -> i64 {
        self.ancien_solde_centimes
    }

    /// Signed balance movement applied by this document.
    pub fn credit_delta_centimes(self) -> i64 {
        self.credit_delta_centimes
    }

    /// Customer balance after this document.
    pub fn nouveau_solde_centimes(self) -> i64 {
        self.nouveau_solde_centimes
    }

    /// The `Versement` line that closes the printed equation for a document
    /// whose grand total is `montant_document_centimes`.
    ///
    /// Charge: `delta = montant - versement` → `versement = montant - delta`.
    /// Avoir:  `delta = -(montant - versement)` → `versement = montant + delta`.
    ///
    /// Because a charge delta is positive and an AVOIR delta negative, BOTH
    /// collapse to `versement = montant - |delta|` — "the non-credit
    /// portion of the document". The wrapper `paid_amount_centimes` counts
    /// every tender including the CREDIT leg, so it is NOT the printed
    /// `Versement`; this function is.
    ///
    /// # Errors
    /// [`DomainError::MoneyOverflow`] when the signed arithmetic leaves `i64`.
    ///
    /// # Errors
    /// [`DomainError::MoneyOverflow`] when the sum leaves `i64`.
    pub fn versement_for(self, montant_document_centimes: i64) -> Result<i64, DomainError> {
        let delta = i128::from(self.credit_delta_centimes);
        let montant = i128::from(montant_document_centimes);
        // One expression covers both directions: montant - |delta|.
        let magnitude = delta.checked_abs().ok_or(DomainError::MoneyOverflow)?;
        let versement = montant
            .checked_sub(magnitude)
            .ok_or(DomainError::MoneyOverflow)?;
        i64::try_from(versement).map_err(|_| DomainError::MoneyOverflow)
    }

    /// Prove the printed equation holds for these exact amounts.
    ///
    /// Charge (delta >= 0): `Ancien + Montant - Versement == Nouveau`.
    /// Avoir  (delta <  0): `Ancien - Montant + Versement == Nouveau`.
    ///
    /// # Errors
    /// [`DomainError::MoneyOverflow`] on an unrepresentable intermediate;
    /// otherwise `Ok(false)` when the equation does not hold, so callers can
    /// refuse to print rather than print a lie.
    pub fn verifies(
        self,
        montant_document_centimes: i64,
        versement_centimes: i64,
    ) -> Result<bool, DomainError> {
        let ancien = i128::from(self.ancien_solde_centimes);
        let montant = i128::from(montant_document_centimes);
        let versement = i128::from(versement_centimes);
        let lhs = if self.credit_delta_centimes >= 0 {
            // charge: Ancien + Montant - Versement
            ancien
                .checked_add(montant)
                .ok_or(DomainError::MoneyOverflow)?
                .checked_sub(versement)
                .ok_or(DomainError::MoneyOverflow)?
        } else {
            // avoir: Ancien - Montant + Versement
            ancien
                .checked_sub(montant)
                .ok_or(DomainError::MoneyOverflow)?
                .checked_add(versement)
                .ok_or(DomainError::MoneyOverflow)?
        };
        Ok(lhs == i128::from(self.nouveau_solde_centimes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_sale_freezes_equation() {
        // Ancien 12,000.00 DZD, sale 4,500.00 of which 1,500.00 cash → +3,000.00.
        let snapshot = CreditSnapshot::freeze(1_200_000, 300_000).expect("freeze");
        assert_eq!(snapshot.ancien_solde_centimes(), 1_200_000);
        assert_eq!(snapshot.nouveau_solde_centimes(), 1_500_000);
        let versement = snapshot.versement_for(450_000).expect("versement");
        assert_eq!(versement, 150_000);
        assert!(snapshot.verifies(450_000, versement).expect("verify"));
    }

    #[test]
    fn avoir_freezes_negative_direction() {
        // Ancien 1,500.00; return 450.00 with 150.00 cash back → -300.00.
        let snapshot = CreditSnapshot::freeze(150_000, -30_000).expect("freeze");
        assert_eq!(snapshot.nouveau_solde_centimes(), 120_000);
        let versement = snapshot.versement_for(45_000).expect("versement");
        assert_eq!(versement, 15_000);
        assert!(snapshot.verifies(45_000, versement).expect("verify"));
    }

    #[test]
    fn zero_delta_is_identity() {
        let snapshot = CreditSnapshot::freeze(0, 0).expect("freeze");
        assert_eq!(snapshot.versement_for(99_900).expect("versement"), 99_900);
        assert!(snapshot.verifies(99_900, 99_900).expect("verify"));
    }

    #[test]
    fn overflow_is_an_error_never_a_wrap() {
        assert_eq!(
            CreditSnapshot::freeze(i64::MAX, 1).unwrap_err(),
            DomainError::MoneyOverflow
        );
        // Montant i64::MIN minus |delta| = -1.8e19: outside i64.
        let snapshot = CreditSnapshot::freeze(0, i64::MIN).expect("freeze");
        assert_eq!(
            snapshot.versement_for(i64::MIN).unwrap_err(),
            DomainError::MoneyOverflow
        );
    }
}
