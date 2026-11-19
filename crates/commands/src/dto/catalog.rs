//! Catalog DTOs: products, kits, categories, discovery.
use serde::{Deserialize, Serialize};

// Catalog wire types ----------------------------------------------------
// Money travels as integer-centime strings, quantities/factors as integer
// micro strings. No float ever crosses this boundary.

/// One unit supplied at product creation (ids allocated server-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewProductUnitDto {
    /// Arabic unit name.
    pub unit_name_ar: String,
    /// French unit name.
    pub unit_name_fr: String,
    /// Base micros per selling unit (`"10000"` for the base unit).
    pub factor_micro: String,
    /// Optional barcode resolving to this unit.
    pub barcode: Option<String>,
    /// Sell price in centimes, e.g. `"15000"` for 150.00 DZD.
    pub sale_price_centimes: String,
    /// Purchase price in centimes.
    pub purchase_price_centimes: String,
    /// Artisan/wholesale price in centimes (`None` = fall back to sale price).
    pub wholesale_price_centimes: Option<String>,
    /// Bulk wholesale price in centimes (`None` = fall back to wholesale, BRK-117).
    pub super_gros_price_centimes: Option<String>,
    /// True for the single base unit.
    pub is_base_unit: bool,
    /// Default POS unit.
    pub is_default_selling: bool,
}

/// Product creation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProductDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Unique business reference.
    pub sku: String,
    /// Optional product-level barcode.
    pub barcode: Option<String>,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category id (UUID string).
    pub category_id: Option<String>,
    /// True when the product is a kit header.
    pub is_kit: bool,
    /// Full unit set (exactly one base unit).
    pub units: Vec<NewProductUnitDto>,
}

/// Stored unit view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductUnitDto {
    /// Unit id (UUID string).
    pub unit_id: String,
    /// Owning product id (UUID string).
    pub product_id: String,
    /// Arabic unit name.
    pub unit_name_ar: String,
    /// French unit name.
    pub unit_name_fr: String,
    /// Base micros per selling unit.
    pub factor_micro: String,
    /// Barcode resolving to this unit, if any.
    pub barcode: Option<String>,
    /// Sell price in centimes.
    pub sale_price_centimes: String,
    /// Purchase price in centimes.
    pub purchase_price_centimes: String,
    /// Artisan/wholesale price in centimes (`None` = fall back to sale price).
    pub wholesale_price_centimes: Option<String>,
    /// Bulk wholesale price in centimes (`None` = fall back to wholesale, BRK-117).
    pub super_gros_price_centimes: Option<String>,
    /// True for the single base unit.
    pub is_base_unit: bool,
    /// Default POS unit.
    pub is_default_selling: bool,
}

/// Stored BOM line view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitItemDto {
    /// Component product id (UUID string).
    pub component_id: String,
    /// Component unit id (UUID string).
    pub unit_id: String,
    /// Quantity per kit, in unit micros.
    pub quantity_micro: String,
}

/// Full product detail view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductResponseDto {
    /// Product id (UUID string).
    pub product_id: String,
    /// Unique business reference.
    pub sku: String,
    /// Product-level barcode, if any.
    pub barcode: Option<String>,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category id, if any.
    pub category_id: Option<String>,
    /// Kit header flag.
    pub is_kit: bool,
    /// Active flag.
    pub is_active: bool,
    /// All associated units.
    pub units: Vec<ProductUnitDto>,
    /// BOM lines (empty for non-kits and list views).
    pub kit_items: Vec<KitItemDto>,
}

/// Single-product fetch request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetProductDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
}

/// One BOM line supplied at kit definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefineKitItemDto {
    /// Component product id (UUID string).
    pub component_id: String,
    /// Component unit id (UUID string).
    pub unit_id: String,
    /// Quantity per kit, in unit micros.
    pub quantity_micro: String,
}

/// Kit definition request (replaces the kit's full BOM).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefineKitDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Kit header id (UUID string).
    pub kit_id: String,
    /// Replacement BOM lines (must be non-empty).
    pub items: Vec<DefineKitItemDto>,
}

/// One unit upsert supplied at product update (`None` id allocates new).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateProductUnitDto {
    /// Existing unit id (UUID string), or `None` for a brand-new unit.
    pub unit_id: Option<String>,
    /// Arabic unit name.
    pub unit_name_ar: String,
    /// French unit name.
    pub unit_name_fr: String,
    /// Base micros per selling unit (`"10000"` for the base unit).
    pub factor_micro: String,
    /// Optional barcode resolving to this unit.
    pub barcode: Option<String>,
    /// Sell price in centimes, e.g. `"15000"` for 150.00 DZD.
    pub sale_price_centimes: String,
    /// Purchase price in centimes.
    pub purchase_price_centimes: String,
    /// Artisan/wholesale price in centimes (`None` = fall back to sale price).
    pub wholesale_price_centimes: Option<String>,
    /// Bulk wholesale price in centimes (`None` = fall back to wholesale, BRK-117).
    pub super_gros_price_centimes: Option<String>,
    /// True for the single base unit.
    pub is_base_unit: bool,
    /// Default POS unit.
    pub is_default_selling: bool,
}

/// Product update request (full-state payload; SKU stays immutable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateProductDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
    /// Arabic display name.
    pub name_ar: String,
    /// French display name.
    pub name_fr: String,
    /// Owning category id (UUID string), or `None` to clear it.
    pub category_id: Option<String>,
    /// Active flag.
    pub is_active: bool,
    /// Merged unit set after applying upserts.
    pub units: Vec<UpdateProductUnitDto>,
}

/// Catalog search request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogSearchRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Prefix/multilingual query (blank returns discovery rows).
    pub query: String,
    /// Max products (clamped to 1..=500 server-side).
    pub limit: Option<u32>,
    /// Admit `is_active = 0` rows (management views). Omitted or
    /// `false` excludes inactive products (POS and sale paths).
    pub include_inactive: Option<bool>,
}

/// Database-level catalog aggregates for the management hub KPIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogMetricsDto {
    /// Every stored product row, active or not.
    pub total_products: i64,
    /// Active products with a strictly positive summed stock level.
    pub in_stock: i64,
    /// Active products with zero/negative summed level (or no level row).
    pub out_of_stock: i64,
    /// Products with `is_active = 0`.
    pub inactive: i64,
}

/// Stored category view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryResponseDto {
    /// Category id (UUID string).
    pub id: String,
    /// Unique business code, e.g. `CAT-PLOMB`.
    pub code: String,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category id (UUID string), if nested.
    pub parent_id: Option<String>,
    /// Inactive categories accept no new products.
    pub is_active: bool,
}

/// Category creation request (active by default; code unique).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateCategoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Unique business code, e.g. `CAT-PLOMB`.
    pub code: String,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category id (UUID string), if nested.
    pub parent_id: Option<String>,
}

/// Category update request (code stays immutable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCategoryRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Category id (UUID string).
    pub category_id: String,
    /// French display name.
    pub name_fr: String,
    /// Arabic display name.
    pub name_ar: String,
    /// Parent category id (UUID string), or `None` to detach it.
    pub parent_id: Option<String>,
    /// Active flag.
    pub is_active: bool,
}

/// One quantity-break rung (BRK-121): minimum quantity in the ladder
/// unit's micros and its unit price in centimes, both canonical strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceBreakDto {
    /// Minimum line quantity in ladder-unit micros.
    pub min_qty_micro: String,
    /// Rung unit price in centimes.
    pub price_centimes: String,
}

/// Quantity-break ladder read request (ascending by rung).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPriceBreaksDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
    /// Ladder unit id (UUID string).
    pub unit_id: String,
}

/// Quantity-break ladder replacement (empty clears; BRK-121b).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplacePriceBreaksDto {
    /// Authorizing session token (catalog managers only).
    pub session_token: String,
    /// Product id (UUID string).
    pub product_id: String,
    /// Ladder unit id (UUID string).
    pub unit_id: String,
    /// Replacement rungs (unsorted accepted; validated server-side).
    pub rungs: Vec<PriceBreakDto>,
}

/// One per-cell override at matrix generation (BRK-114).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantCellOverrideDto {
    /// First-axis value the override targets.
    pub axis1: String,
    /// Second-axis value the override targets.
    pub axis2: String,
    /// Replacement sale price in centimes (`None` = inherit template).
    pub sale_price_centimes: Option<String>,
    /// Replacement product-level barcode (unique plan-wide).
    pub barcode: Option<String>,
}

/// Variant-matrix generation request (BRK-114, Wave 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateVariantMatrixDto {
    /// Authorizing session token (catalog managers only).
    pub session_token: String,
    /// Arabic model name.
    pub name_ar: String,
    /// French model name.
    pub name_fr: String,
    /// First axis display name (`Taille`, `Dimension` …).
    pub axis1_name: String,
    /// Second axis display name (`Couleur`, `Finition` …).
    pub axis2_name: String,
    /// First-axis values.
    pub axis1_values: Vec<String>,
    /// Second-axis values.
    pub axis2_values: Vec<String>,
    /// Base SKU every cell SKU derives from.
    pub base_sku: String,
    /// Owning category id (UUID string), inherited by every cell.
    pub category_id: Option<String>,
    /// Template product id (UUID string) lending ladder, tiers, rungs.
    pub template_product_id: String,
    /// Per-cell overrides.
    pub overrides: Vec<VariantCellOverrideDto>,
}

/// Variant-matrix generation response: the model id plus every minted SKU.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateVariantMatrixResponseDto {
    /// Persisted model id (UUID string).
    pub model_id: String,
    /// Minted cells.
    pub cell_count: usize,
    /// Every minted SKU in display order.
    pub skus: Vec<String>,
}

/// Matrix 2D-grid view request (DSN-108/109 read path).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixViewRequestDto {
    /// Authorizing session token.
    pub session_token: String,
    /// Model id (UUID string).
    pub model_id: String,
}

/// One 2D-grid row: the cell, its backing product, live totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantMatrixViewRowDto {
    /// First-axis value.
    pub axis1_value: String,
    /// Second-axis value.
    pub axis2_value: String,
    /// Backing product id (UUID string).
    pub product_id: String,
    /// Backing product SKU.
    pub sku: String,
    /// Arabic product name.
    pub name_ar: String,
    /// Product-level barcode, if any.
    pub barcode: Option<String>,
    /// On-hand summed across ALL warehouses, in base-unit micros.
    pub on_hand_micro: String,
    /// Default-selling unit price in centimes, if a default unit exists.
    pub sale_price_centimes: Option<String>,
}

/// One série line: a first-axis value plus pieces per single série (BRK-115).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerieLineDto {
    /// First-axis value (`M`, `44`, `Ø18` …).
    pub axis1_value: String,
    /// Pieces of this size per single série.
    pub pieces: u32,
}

/// Série composition request: resolve a size run on one color into a kit (BRK-115).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeSerieDto {
    /// Authorizing session token (catalog managers only).
    pub session_token: String,
    /// Série header product id (UUID string, must carry `is_kit`).
    pub kit_id: String,
    /// Source variant-model id (UUID string).
    pub model_id: String,
    /// Second-axis value the série runs on (`NOIR` …).
    pub axis2_value: String,
    /// Size run (`2M + 4L + 4XL + 2XXL` …).
    pub lines: Vec<SerieLineDto>,
    /// Precheck warehouse id (UUID string, omit for structural-only compose).
    pub warehouse_id: Option<String>,
    /// How many séries the precheck covers (≥1 when a warehouse is given).
    pub serie_count: u32,
}

/// One reported shortfall: required vs on-hand in base-unit micros (BRK-115).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerieShortfallDto {
    /// Component product id (UUID string).
    pub product_id: String,
    /// Component SKU.
    pub sku: String,
    /// Required base micros, canonical string.
    pub required_micro: String,
    /// On-hand base micros in the precheck warehouse, canonical string.
    pub on_hand_micro: String,
}

/// Série composition response: persisted kit plus the availability report (BRK-115).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposeSerieResponseDto {
    /// Série header product id (UUID string).
    pub kit_id: String,
    /// Persisted components.
    pub component_count: usize,
    /// Pieces per single série.
    pub total_pieces: u32,
    /// Shortfalls in the precheck warehouse (empty when sufficient or skipped).
    pub shortfalls: Vec<SerieShortfallDto>,
}
