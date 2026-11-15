//! Catalog domain: categories, products, units, kit bills of materials.

pub mod category;
pub mod kit;
pub mod price_breaks;
pub mod product;
pub mod search_normalization;
pub mod unit;

pub use category::{Category, CategoryId};
pub use kit::{validate_kit_edges, KitItem};
pub use price_breaks::{resolve_break_price, validate_ladder, PriceBreak};
pub use product::{Barcode, Product, ProductId, Sku};
pub use search_normalization::{
    expand_token_variants, fold_accents_and_diacritics, normalize_arabic, normalize_for_search,
    tokenize_query, ALGERIAN_HARDWARE_DICTIONARY,
};
pub use unit::{convert_to_base_micros, validate_unit_set, ProductUnit, UnitId};
