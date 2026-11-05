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
