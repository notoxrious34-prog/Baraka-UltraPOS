//! Vertical profiles (BRK-117): the trade the counter speaks.
//!
//! Profiles live in configuration + `store_identity`, NEVER in core math.
//! A profile selects which customer tiers the trade offers and which tier
//! new walk-in customers default to; prices, ledger entries, and migration
//! history are untouched by switching (ADR-110/115).

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use crate::sales::customer::CustomerTier;

/// The commercial vertical the storefront operates as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VerticalProfile {
    /// Men's underwear retail + wholesale (`Détail / Gros / Super-Gros`).
    #[default]
    ApparelWholesaleRetail,
    /// General hardware / trade (`Détail / Artisan / Gros`).
    HardwareGeneral,
    /// Both trades on one counter (union of tier vocabularies).
    HybridEnterprise,
}

impl VerticalProfile {
    /// Canonical storage/wire code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ApparelWholesaleRetail => "apparel_wholesale_retail",
            Self::HardwareGeneral => "hardware_general",
            Self::HybridEnterprise => "hybrid_enterprise",
        }
    }

    /// Tiers this trade offers, cheapest-attention order (retail first).
    /// No math here: pure surface vocabulary for the POS + directory.
    #[must_use]
    pub const fn offered_tiers(self) -> &'static [CustomerTier] {
        match self {
            Self::ApparelWholesaleRetail => &[
                CustomerTier::Retail,
                CustomerTier::Wholesale,
                CustomerTier::SuperGros,
            ],
            Self::HardwareGeneral => &[
                CustomerTier::Retail,
                CustomerTier::Artisan,
                CustomerTier::Wholesale,
            ],
            Self::HybridEnterprise => &[
                CustomerTier::Retail,
                CustomerTier::Artisan,
                CustomerTier::Wholesale,
                CustomerTier::SuperGros,
            ],
        }
    }

    /// Tier assigned to a new customer when the cashier picks none.
    /// Conservative by design: walk-in detail price everywhere.
    #[must_use]
    pub const fn default_tier(self) -> CustomerTier {
        CustomerTier::Retail
    }
}

impl FromStr for VerticalProfile {
    type Err = DomainError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_lowercase().as_str() {
            "apparel_wholesale_retail" | "apparel" | "sous-vetement" | "sous-vêtement" => {
                Ok(Self::ApparelWholesaleRetail)
            }
            "hardware_general" | "hardware" | "quincaillerie" => Ok(Self::HardwareGeneral),
            "hybrid_enterprise" | "hybrid" | "hybride" => Ok(Self::HybridEnterprise),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown vertical profile: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_parse_and_offer_the_documented_tiers() {
        assert_eq!(
            "apparel_wholesale_retail"
                .parse::<VerticalProfile>()
                .unwrap(),
            VerticalProfile::ApparelWholesaleRetail
        );
        assert_eq!(
            "quincaillerie".parse::<VerticalProfile>().unwrap(),
            VerticalProfile::HardwareGeneral
        );
        assert_eq!(
            "HYBRID".parse::<VerticalProfile>().unwrap(),
            VerticalProfile::HybridEnterprise
        );
        assert!("boulangerie".parse::<VerticalProfile>().is_err());

        // Apparel speaks Gros/Super-Gros, never Artisan; hardware the reverse.
        assert_eq!(
            VerticalProfile::ApparelWholesaleRetail.offered_tiers(),
            &[
                CustomerTier::Retail,
                CustomerTier::Wholesale,
                CustomerTier::SuperGros
            ]
        );
        assert!(!VerticalProfile::ApparelWholesaleRetail
            .offered_tiers()
            .contains(&CustomerTier::Artisan));
        assert!(VerticalProfile::HardwareGeneral
            .offered_tiers()
            .contains(&CustomerTier::Artisan));
        assert!(!VerticalProfile::HardwareGeneral
            .offered_tiers()
            .contains(&CustomerTier::SuperGros));
        // Hybrid is the union; every profile defaults to walk-in retail.
        assert_eq!(VerticalProfile::HybridEnterprise.offered_tiers().len(), 4);
        for profile in [
            VerticalProfile::ApparelWholesaleRetail,
            VerticalProfile::HardwareGeneral,
            VerticalProfile::HybridEnterprise,
        ] {
            assert_eq!(profile.default_tier(), CustomerTier::Retail);
            assert_eq!(
                profile,
                profile.as_str().parse::<VerticalProfile>().unwrap()
            );
        }
    }
}
