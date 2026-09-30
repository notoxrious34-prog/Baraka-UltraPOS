//! Products, SKUs, and barcodes.

use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::catalog::CategoryId;
use crate::errors::DomainError;

/// Stable product identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProductId(pub Uuid);

impl ProductId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProductId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ProductId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ProductId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Unique stock-keeping unit code (trimmed, never empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sku(String);

impl Sku {
    /// Build from raw input.
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "SKU must not be empty".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated code.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Barcode value mapped to exactly one product unit (trimmed, never empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Barcode(String);

impl Barcode {
    /// Build from raw input.
    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "barcode must not be empty".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Catalog product (leaf good or kit header).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    /// Stable identifier.
    pub id: ProductId,
    /// Unique business reference.
    pub sku: Sku,
    /// Optional product-level barcode (unit barcodes take precedence).
    pub barcode: Option<Barcode>,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category, if classified.
    pub category_id: Option<CategoryId>,
    /// True when this product expands into a bill of materials.
    pub is_kit: bool,
    /// Inactive products accept no new documents.
    pub is_active: bool,
}

impl Product {
    /// Validate display names.
    pub fn validate_names(name_ar: &str, name_fr: &str) -> Result<(), DomainError> {
        if name_ar.trim().is_empty() || name_fr.trim().is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "product names must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}
