//! Variant-matrix engine (TASK-BRK-114, Wave 4).
//!
//! A variant model groups generated product rows: each cell IS a product
//! (SKU/barcode/stock/tier prices keep living on `products` /
//! `product_units`), so single products work untouched and the matrix is
//! a grouping + ordering layer. Display order rides explicit sort keys —
//! apparel sizes never sort alphabetically wrong (`S < M < XL < XXL`),
//! waist numbers rank numerically (`38 < 44`), diameters rank by bore
//! (`Ø6 < Ø32`), anything else trails alphabetically and deterministically.
//!
//! This module is pure planning: validation + ordering + SKU naming.
//! Persistence (one `BEGIN IMMEDIATE` transaction) lives behind
//! `VariantMatrixRepository`; unit-ladder and price-break inheritance
//! is orchestrated by the application use case.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::catalog::CategoryId;
use crate::errors::DomainError;

/// Hard ceiling on generated cells per model (accident guard: a 100x100
/// typo must reject LOUDLY instead of minting 10,000 products).
pub const MAX_MATRIX_CELLS: usize = 500;

/// Stable variant-model identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantModelId(pub Uuid);

impl VariantModelId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for VariantModelId {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable variant-cell identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantCellId(pub Uuid);

impl VariantCellId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for VariantCellId {
    fn default() -> Self {
        Self::new()
    }
}

/// Variant model draft (ids are assigned at persist time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantModelDraft {
    /// Arabic model name.
    pub name_ar: String,
    /// French model name.
    pub name_fr: String,
    /// First axis display name (`Taille`, `Dimension` …).
    pub axis1_name: String,
    /// Second axis display name (`Couleur`, `Finition` …).
    pub axis2_name: String,
    /// Owning category, inherited by every generated product.
    pub category_id: Option<CategoryId>,
}

/// One planned cell: validated values + sort keys + naming + overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedCell {
    /// First-axis value (`XL`, `44`, `Ø18` …).
    pub axis1_value: String,
    /// Second-axis value (`NOIR`, `ZINGUE` …).
    pub axis2_value: String,
    /// Display rank of the first-axis value (lower shows first).
    pub axis1_sort: i64,
    /// Display rank of the second-axis value.
    pub axis2_sort: i64,
    /// Suggested SKU (`{base}-{A1}-{A2}`, uppercased, dash-joined).
    pub sku: String,
    /// Per-cell barcode override (`None` = unset — inherited unit
    /// barcodes are stripped, never duplicated).
    pub barcode: Option<String>,
    /// Per-cell sale-price override in centimes (`None` = inherit).
    pub sale_price_centimes: Option<i64>,
}

/// Full generation plan: the model draft plus every cell in
/// display order (axis1 major, axis2 minor — the 2D grid reads naturally).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixPlan {
    /// The model to persist.
    pub model: VariantModelDraft,
    /// Cells in display order.
    pub cells: Vec<PlannedCell>,
}

/// Per-cell override supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSpecOverride<'a> {
    /// First-axis value the override targets.
    pub axis1: &'a str,
    /// Second-axis value the override targets.
    pub axis2: &'a str,
    /// Replacement sale price in centimes (default-selling unit).
    pub sale_price_centimes: Option<i64>,
    /// Replacement barcode (must stay unique across the plan).
    pub barcode: Option<&'a str>,
}

/// Matrix generation spec (borrowed; the plan owns its strings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantMatrixSpec<'a> {
    /// Arabic model name.
    pub name_ar: &'a str,
    /// French model name.
    pub name_fr: &'a str,
    /// First axis display name.
    pub axis1_name: &'a str,
    /// Second axis display name.
    pub axis2_name: &'a str,
    /// First-axis values in caller order (rank comes from the value itself).
    pub axis1_values: &'a [&'a str],
    /// Second-axis values in caller order.
    pub axis2_values: &'a [&'a str],
    /// Base SKU every cell SKU derives from.
    pub base_sku: &'a str,
    /// Owning category, inherited by every generated product.
    pub category_id: Option<CategoryId>,
    /// Per-cell overrides.
    pub overrides: &'a [CellSpecOverride<'a>],
}

/// Canonical apparel size order (index = rank). Matching is
/// case-insensitive on the trimmed value with inner spaces removed
/// (`"3 xl"` ranks as `"3XL"`).
const APPAREL_ORDER: &[&str] = &[
    "XXXS", "XXS", "XS", "S", "M", "L", "XL", "XXL", "2XL", "3XL", "XXXL", "4XL", "XXXXL", "5XL",
    "6XL",
];

/// Normalized comparison token (uppercase, spaceless).
fn normalize_token(raw: &str) -> String {
    raw.trim().to_uppercase().replace(' ', "")
}

/// Display rank of one axis value: `(class, rank, tail)`.
///
/// - class 0: canonical apparel sizes (index = rank);
/// - class 1: pure numbers, ranked numerically (`38 < 44`);
/// - class 2: `Ø`-prefixed diameters, ranked by bore (`Ø6 < Ø32`);
/// - class 3: everything else, alphabetical deterministic tail.
fn axis_sort_key(raw: &str) -> (u8, i64, String) {
    let token = normalize_token(raw);
    if let Some(rank) = APPAREL_ORDER.iter().position(|size| *size == token) {
        return (0, rank as i64, String::new());
    }
    if let Ok(number) = token.parse::<i64>() {
        return (1, number, String::new());
    }
    if let Some(bore) = token.strip_prefix('Ø') {
        if let Ok(diameter) = bore.parse::<i64>() {
            return (2, diameter, String::new());
        }
    }
    (3, 0, token.to_lowercase())
}

/// SKU-safe spelling of one token: uppercase, whitespace to dashes,
/// only `[A-Z0-9-_] survives (existing SKUs are Latin by convention —
/// an Arabic-only value yields an empty core and rejects LOUDLY below).
fn sku_token(raw: &str) -> String {
    raw.trim()
        .to_uppercase()
        .chars()
        .map(|ch| if ch.is_whitespace() { '-' } else { ch })
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect()
}

/// Suggested cell SKU: `{base}-{A1}-{A2}` in SKU-safe spelling.
fn suggest_cell_sku(base_sku: &str, axis1: &str, axis2: &str) -> Result<String, DomainError> {
    let core = format!(
        "{}-{}-{}",
        sku_token(base_sku),
        sku_token(axis1),
        sku_token(axis2)
    );
    let sku = core.trim_matches('-').to_string();
    if sku.is_empty() || !sku.contains('-') {
        return Err(DomainError::InvalidCanonicalString(
            "VARIANT_SKU_NOT_SUGGESTIBLE".to_string(),
        ));
    }
    Ok(sku)
}

/// Validate the spec and plan every cell in display order (axis1 major,
/// axis2 minor). Pure: no I/O, no ids — persistence assigns those.
pub fn plan_variant_matrix(spec: &VariantMatrixSpec<'_>) -> Result<MatrixPlan, DomainError> {
    let invalid = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    if spec.name_ar.trim().is_empty() || spec.name_fr.trim().is_empty() {
        return Err(invalid("VARIANT_MODEL_NAME_EMPTY"));
    }
    if spec.axis1_name.trim().is_empty() || spec.axis2_name.trim().is_empty() {
        return Err(invalid("VARIANT_AXIS_NAME_EMPTY"));
    }
    if spec.base_sku.trim().is_empty() {
        return Err(invalid("VARIANT_BASE_SKU_EMPTY"));
    }
    let axis1: Vec<String> = spec
        .axis1_values
        .iter()
        .map(|v| v.trim().to_string())
        .collect();
    let axis2: Vec<String> = spec
        .axis2_values
        .iter()
        .map(|v| v.trim().to_string())
        .collect();
    if axis1.is_empty() || axis2.is_empty() {
        return Err(invalid("VARIANT_AXIS_EMPTY"));
    }
    if axis1.iter().any(|v| v.is_empty()) || axis2.iter().any(|v| v.is_empty()) {
        return Err(invalid("VARIANT_AXIS_VALUE_EMPTY"));
    }
    let mut seen = std::collections::HashSet::new();
    for value in axis1.iter().chain(axis2.iter()) {
        if !seen.insert(normalize_token(value)) {
            return Err(invalid("VARIANT_AXIS_DUPLICATE"));
        }
    }
    // NOTE: dup detection spans both axes on purpose — a value living on
    // both axes (`"NOIR"` × `"NOIR"`) would mint unreadable cells.
    let count = axis1.len() * axis2.len();
    if count > MAX_MATRIX_CELLS {
        return Err(invalid("VARIANT_MATRIX_TOO_LARGE"));
    }
    // Overrides target normalized values; barcodes stay unique plan-wide.
    let mut override_by_cell: std::collections::HashMap<(String, String), &CellSpecOverride<'_>> =
        std::collections::HashMap::new();
    let mut override_barcodes = std::collections::HashSet::new();
    for over in spec.overrides {
        let key = (normalize_token(over.axis1), normalize_token(over.axis2));
        let on_axis =
            |values: &[String], target: &str| values.iter().any(|v| normalize_token(v) == target);
        if !on_axis(&axis1, &key.0) || !on_axis(&axis2, &key.1) {
            return Err(invalid("VARIANT_CELL_OVERRIDE_UNKNOWN"));
        }
        if over.sale_price_centimes.is_some_and(|price| price < 0) {
            return Err(invalid("VARIANT_OVERRIDE_PRICE_NEGATIVE"));
        }
        if let Some(barcode) = over.barcode {
            if barcode.trim().is_empty() {
                return Err(invalid("VARIANT_OVERRIDE_BARCODE_EMPTY"));
            }
            if !override_barcodes.insert(normalize_token(barcode)) {
                return Err(invalid("VARIANT_DUPLICATE_BARCODE"));
            }
        }
        if override_by_cell.insert(key, over).is_some() {
            return Err(invalid("VARIANT_CELL_OVERRIDE_DUPLICATE"));
        }
    }
    // Display order: sorted axis1 major, sorted axis2 minor.
    let mut sorted1 = axis1.clone();
    let mut sorted2 = axis2.clone();
    sorted1.sort_by_key(|a| axis_sort_key(a));
    sorted2.sort_by_key(|a| axis_sort_key(a));
    let mut cells = Vec::with_capacity(count);
    for a1 in &sorted1 {
        for a2 in &sorted2 {
            let key = (normalize_token(a1), normalize_token(a2));
            let over = override_by_cell.get(&key).copied();
            cells.push(PlannedCell {
                axis1_value: a1.clone(),
                axis2_value: a2.clone(),
                axis1_sort: axis_sort_key(a1).1,
                axis2_sort: axis_sort_key(a2).1,
                sku: suggest_cell_sku(spec.base_sku, a1, a2)?,
                barcode: over
                    .and_then(|o| o.barcode)
                    .map(|barcode| barcode.trim().to_string()),
                sale_price_centimes: over.and_then(|o| o.sale_price_centimes),
            });
        }
    }
    Ok(MatrixPlan {
        model: VariantModelDraft {
            name_ar: spec.name_ar.trim().to_string(),
            name_fr: spec.name_fr.trim().to_string(),
            axis1_name: spec.axis1_name.trim().to_string(),
            axis2_name: spec.axis2_name.trim().to_string(),
            category_id: spec.category_id,
        },
        cells,
    })
}

/// Persisted variant model (mirrors the `variant_models` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantModel {
    /// Stable model identifier.
    pub id: VariantModelId,
    /// Arabic model name.
    pub name_ar: String,
    /// French model name.
    pub name_fr: String,
    /// First axis display name.
    pub axis1_name: String,
    /// Second axis display name.
    pub axis2_name: String,
    /// Owning category, inherited by every generated product.
    pub category_id: Option<CategoryId>,
    /// Soft kill-switch (inactive models hide from DSN-108/109).
    pub is_active: bool,
}

/// Persisted variant cell (mirrors the `variant_cells` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantCell {
    /// Stable cell identifier.
    pub id: VariantCellId,
    /// Owning model.
    pub model_id: VariantModelId,
    /// Generated product backing this cell.
    pub product_id: crate::catalog::ProductId,
    /// First-axis value.
    pub axis1_value: String,
    /// Second-axis value.
    pub axis2_value: String,
    /// Display rank of the first-axis value.
    pub axis1_sort: i64,
    /// Display rank of the second-axis value.
    pub axis2_sort: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec<'a>(axis1: &'a [&'a str], axis2: &'a [&'a str]) -> VariantMatrixSpec<'a> {
        VariantMatrixSpec {
            name_ar: "سيرية قطن",
            name_fr: "SERIE COTON",
            axis1_name: "Taille",
            axis2_name: "Couleur",
            axis1_values: axis1,
            axis2_values: axis2,
            base_sku: "CTN-100",
            category_id: None,
            overrides: &[],
        }
    }

    #[test]
    fn apparel_sizes_rank_logically_never_alphabetically() {
        let plan = plan_variant_matrix(&spec(&["XXL", "M", "S", "XL"], &["NOIR"])).expect("plans");
        let sizes: Vec<&str> = plan
            .cells
            .iter()
            .map(|cell| cell.axis1_value.as_str())
            .collect();
        assert_eq!(sizes, vec!["S", "M", "XL", "XXL"]);
    }

    #[test]
    fn numeric_and_diameter_axes_rank_by_measure() {
        let plan =
            plan_variant_matrix(&spec(&["44", "38", "56"], &["ZINGUE"])).expect("plans numeric");
        let sizes: Vec<&str> = plan
            .cells
            .iter()
            .map(|cell| cell.axis1_value.as_str())
            .collect();
        assert_eq!(sizes, vec!["38", "44", "56"]);
        let plan =
            plan_variant_matrix(&spec(&["Ø32", "Ø6", "Ø18"], &["BRUT"])).expect("plans diameters");
        let sizes: Vec<&str> = plan
            .cells
            .iter()
            .map(|cell| cell.axis1_value.as_str())
            .collect();
        assert_eq!(sizes, vec!["Ø6", "Ø18", "Ø32"]);
    }

    #[test]
    fn skus_derive_dashed_and_unique_per_cell() {
        let plan =
            plan_variant_matrix(&spec(&["XL", "M"], &["NOIR", "BLANC"])).expect("plans skus");
        let skus: Vec<&str> = plan.cells.iter().map(|cell| cell.sku.as_str()).collect();
        assert_eq!(
            skus,
            vec![
                "CTN-100-M-BLANC",
                "CTN-100-M-NOIR",
                "CTN-100-XL-BLANC",
                "CTN-100-XL-NOIR"
            ]
        );
    }

    #[test]
    fn duplicates_reject_loudly_case_insensitively() {
        let bad = spec(&["XL", " xl "], &["NOIR"]);
        assert!(plan_variant_matrix(&bad).is_err(), "axis dup");
        let bad = VariantMatrixSpec {
            overrides: &[CellSpecOverride {
                axis1: "3XL",
                axis2: "NOIR",
                sale_price_centimes: None,
                barcode: None,
            }],
            ..spec(&["XL"], &["NOIR"])
        };
        assert!(
            plan_variant_matrix(&bad).is_err(),
            "override of unknown cell"
        );
    }

    #[test]
    fn oversize_matrices_reject_before_any_persist() {
        let many: Vec<String> = (0..30).map(|i| format!("S{i}")).collect();
        let many_ref: Vec<&str> = many.iter().map(String::as_str).collect();
        let filler: Vec<&str> = (0..30).map(|_| "C").collect();
        let bad = spec(&many_ref, &filler);
        assert!(plan_variant_matrix(&bad).is_err(), "30x30 must not plan");
    }

    #[test]
    fn empty_names_values_and_negative_override_reject() {
        let bad = VariantMatrixSpec {
            name_ar: "  ",
            ..spec(&["XL"], &["NOIR"])
        };
        assert!(plan_variant_matrix(&bad).is_err(), "empty name");
        let bad = spec(&[], &["NOIR"]);
        assert!(plan_variant_matrix(&bad).is_err(), "empty axis");
        let bad = VariantMatrixSpec {
            overrides: &[CellSpecOverride {
                axis1: "XL",
                axis2: "NOIR",
                sale_price_centimes: Some(-50),
                barcode: None,
            }],
            ..spec(&["XL"], &["NOIR"])
        };
        assert!(
            plan_variant_matrix(&bad).is_err(),
            "negative override price"
        );
    }
}
