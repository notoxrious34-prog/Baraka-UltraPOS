//! Product categories.

use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;

/// Stable category identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoryId(pub Uuid);

impl CategoryId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CategoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for CategoryId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for CategoryId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Product category (tree via optional `parent_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    /// Stable identifier.
    pub id: CategoryId,
    /// Unique business code, e.g. `VIS`.
    pub code: String,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Parent category, if nested.
    pub parent_id: Option<CategoryId>,
    /// Inactive categories accept no new products.
    pub is_active: bool,
}

impl Category {
    /// Validate code and display names.
    pub fn validate(code: &str, name_ar: &str, name_fr: &str) -> Result<(), DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "category code must not be empty".to_string(),
            ));
        }
        if name_ar.trim().is_empty() || name_fr.trim().is_empty() {
            return Err(DomainError::InvalidCanonicalString(
                "category names must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}
