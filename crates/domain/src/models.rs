//! Core domain models (identifiers and value cards).
//!
//! Persistence mapping lives in `baraka-infrastructure` (TASK-BRK-002).
//! This module only defines serializable shapes and validation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;
use crate::primitives::{Money, QtyMicro};

/// Sale unit of a product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SaleUnit {
    Piece,
    Kg,
    Meter,
    Box,
}

/// Supported payment modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMode {
    Especes,
    Cheque,
    Virement,
    Credit,
}

/// Lightweight product card used by the POS search path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductCard {
    /// Stable product identifier.
    pub id: Uuid,
    /// Business reference, e.g. `VIS-8X40`.
    pub reference: String,
    /// Optional EAN / internal barcode.
    pub barcode: Option<String>,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Sale unit.
    pub unit: SaleUnit,
    /// Current sell price (DZD centimes).
    pub sell_price: Money,
    /// Quantity on hand in micros (scale 10^4).
    pub stock_qty: i64,
}

impl ProductCard {
    /// Validate invariants shared by every construction site.
    pub fn validate(
        reference: &str,
        name_fr: &str,
        name_ar: &str,
        stock_qty: i64,
    ) -> Result<(), DomainError> {
        if reference.trim().is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "reference must not be empty".to_string(),
            ));
        }
        if name_fr.trim().is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "name_fr must not be empty".to_string(),
            ));
        }
        if name_ar.trim().is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "name_ar must not be empty".to_string(),
            ));
        }
        if stock_qty < 0 {
            return Err(DomainError::NegativeMoneyForbidden);
        }
        Ok(())
    }

    /// Whether the product can cover the requested sale quantity.
    #[must_use]
    pub fn can_cover(&self, quantity: QtyMicro) -> bool {
        quantity.micros() > 0 && self.stock_qty >= quantity.micros()
    }
}

/// One validated sale line (quantity x unit price, before global discount).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaleLine {
    /// Product identifier.
    pub product_id: Uuid,
    /// Requested quantity in micros.
    pub quantity: QtyMicro,
    /// Agreed unit price (DZD centimes).
    pub unit_price: Money,
}

impl SaleLine {
    /// Build a validated sale line.
    pub fn new(product_id: Uuid, quantity: QtyMicro, unit_price: Money) -> Self {
        Self {
            product_id,
            quantity,
            unit_price,
        }
    }

    /// Deterministic line total in centimes.
    pub fn line_total(&self) -> Result<Money, DomainError> {
        self.unit_price.checked_mul_qty(self.quantity)
    }
}

/// Health snapshot exposed to the frontend cockpit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthSnapshot {
    /// Machine / app clock at snapshot time.
    pub captured_at: DateTime<Utc>,
    /// Workspace version.
    pub version: String,
    /// Offline-first flag: always true for the local core.
    pub offline_ready: bool,
}
