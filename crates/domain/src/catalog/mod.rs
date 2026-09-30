//! Catalog domain: categories, products, units, kit bills of materials.

pub mod category;
pub mod kit;
pub mod product;
pub mod unit;

pub use category::{Category, CategoryId};
pub use kit::{validate_kit_edges, KitItem};
pub use product::{Barcode, Product, ProductId, Sku};
pub use unit::{convert_to_base_micros, validate_unit_set, ProductUnit, UnitId};
