//! SQLite catalog repository (products, units, kits, categories).
//!
//! Every write runs inside `run_transaction`. UNIQUE violations on
//! `products.sku` and `product_units.barcode` map to named domain errors;
//! every other storage failure maps to a static, non-leaking message.
//!
//! Row mappers return `rusqlite::Error` natively (corrupt stored values
//! surface as `InvalidColumnName`); the repository boundary collapses all
//! of them into the opaque store failure, except `QueryReturnedNoRows`
//! (absence) and UNIQUE violations (named duplicates).

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{CategoryRepository, KitComponentRow, ProductRepository};
use baraka_domain::catalog::{
    Barcode, Category, CategoryId, KitItem, Product, ProductId, ProductUnit, Sku, UnitId,
};
use baraka_domain::errors::DomainError;
use baraka_domain::primitives::{FactorMicro, Money, QtyMicro};
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::persistence::transaction::run_transaction;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("catalog store failure".to_string())
}

/// Corrupt-value marker for stored rows that violate domain invariants.
fn corrupt(what: &'static str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnName(what.to_string())
}

/// Map UNIQUE violations to named errors; everything else is opaque.
fn constraint_error(error: &rusqlite::Error) -> ApplicationError {
    let text = format!("{error}");
    if text.contains("products.sku") {
        ApplicationError::Validation(DomainError::DuplicateSku)
    } else if text.contains("product_units.barcode") {
        ApplicationError::Validation(DomainError::DuplicateBarcode)
    } else {
        store_failure()
    }
}

fn now_stamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

/// Catalog repository over a shared SQLite connection.
pub struct SqliteCatalogRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteCatalogRepository {
    /// Wrap the shared connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApplicationError> {
        self.db
            .lock()
            .map_err(|_| ApplicationError::Port("database lock poisoned".to_string()))
    }
}

fn parse_product_id(text: &str) -> Result<ProductId, rusqlite::Error> {
    Uuid::parse_str(text)
        .map(ProductId::from)
        .map_err(|_| corrupt("product id"))
}

fn map_product(row: &rusqlite::Row) -> Result<Product, rusqlite::Error> {
    let id_text: String = row.get(0)?;
    let sku_text: String = row.get(1)?;
    let barcode_text: Option<String> = row.get(2)?;
    let name_ar: String = row.get(3)?;
    let name_fr: String = row.get(4)?;
    let category_text: Option<String> = row.get(5)?;
    let is_kit: bool = row.get::<_, i64>(6)? != 0;
    let is_active: bool = row.get::<_, i64>(7)? != 0;
    Ok(Product {
        id: parse_product_id(&id_text)?,
        sku: Sku::new(&sku_text).map_err(|_| corrupt("sku"))?,
        barcode: barcode_text
            .map(|code| Barcode::new(&code).map_err(|_| corrupt("barcode")))
            .transpose()?,
        name_ar,
        name_fr,
        category_id: category_text
            .map(|code| {
                Uuid::parse_str(&code)
                    .map(CategoryId::from)
                    .map_err(|_| corrupt("category id"))
            })
            .transpose()?,
        is_kit,
        is_active,
    })
}

fn map_unit(row: &rusqlite::Row) -> Result<ProductUnit, rusqlite::Error> {
    let id_text: String = row.get(0)?;
    let product_text: String = row.get(1)?;
    let unit_name_ar: String = row.get(2)?;
    let unit_name_fr: String = row.get(3)?;
    let factor: i64 = row.get(4)?;
    let barcode_text: Option<String> = row.get(5)?;
    let sale: i64 = row.get(6)?;
    let purchase: i64 = row.get(7)?;
    let is_base: bool = row.get::<_, i64>(8)? != 0;
    let is_default: bool = row.get::<_, i64>(9)? != 0;
    Ok(ProductUnit {
        id: Uuid::parse_str(&id_text)
            .map(UnitId::from)
            .map_err(|_| corrupt("unit id"))?,
        product_id: parse_product_id(&product_text)?,
        unit_name_ar,
        unit_name_fr,
        factor_micro: FactorMicro::from_micros(factor),
        barcode: barcode_text
            .map(|code| Barcode::new(&code).map_err(|_| corrupt("barcode")))
            .transpose()?,
        sale_price: Money::from_centimes(sale),
        purchase_price: Money::from_centimes(purchase),
        is_base_unit: is_base,
        is_default_selling: is_default,
    })
}

fn map_kit_item(row: &rusqlite::Row) -> Result<KitItem, rusqlite::Error> {
    let kit_text: String = row.get(0)?;
    let component_text: String = row.get(1)?;
    let unit_text: String = row.get(2)?;
    let quantity: i64 = row.get(3)?;
    Ok(KitItem {
        kit_product_id: parse_product_id(&kit_text)?,
        component_product_id: parse_product_id(&component_text)?,
        unit_id: Uuid::parse_str(&unit_text)
            .map(UnitId::from)
            .map_err(|_| corrupt("unit id"))?,
        quantity_micro: QtyMicro::from_micros(quantity),
    })
}

fn map_category(row: &rusqlite::Row) -> Result<Category, rusqlite::Error> {
    let id_text: String = row.get(0)?;
    let code: String = row.get(1)?;
    let name_ar: String = row.get(2)?;
    let name_fr: String = row.get(3)?;
    let parent_text: Option<String> = row.get(4)?;
    let is_active: bool = row.get::<_, i64>(5)? != 0;
    Ok(Category {
        id: Uuid::parse_str(&id_text)
            .map(CategoryId::from)
            .map_err(|_| corrupt("category id"))?,
        code,
        name_ar,
        name_fr,
        parent_id: parent_text
            .map(|text| {
                Uuid::parse_str(&text)
                    .map(CategoryId::from)
                    .map_err(|_| corrupt("category id"))
            })
            .transpose()?,
        is_active,
    })
}

const PRODUCT_COLUMNS: &str = "id, sku, barcode, name_ar, name_fr, category_id, is_kit, is_active";
const UNIT_COLUMNS: &str = "id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode, sale_price_centimes, purchase_price_centimes, is_base_unit, is_default_selling";

fn fetch_product(
    conn: &Connection,
    id: &str,
) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError> {
    let product = match conn
        .query_row(
            &format!("SELECT {PRODUCT_COLUMNS} FROM products WHERE id = ?1"),
            [id],
            map_product,
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .map_err(|_| store_failure())?
    {
        Some(product) => product,
        None => return Ok(None),
    };
    let mut statement = conn
        .prepare(&format!(
            "SELECT {UNIT_COLUMNS} FROM product_units WHERE product_id = ?1 ORDER BY is_base_unit DESC, id"
        ))
        .map_err(|_| store_failure())?;
    let units = statement
        .query_map([id], map_unit)
        .map_err(|_| store_failure())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| store_failure())?;
    Ok(Some((product, units)))
}

fn infra_to_app(error: crate::persistence::InfrastructureError) -> ApplicationError {
    match error {
        crate::persistence::InfrastructureError::Sqlite(inner) => constraint_error(&inner),
        crate::persistence::InfrastructureError::Migration(message)
            if message == "product missing" =>
        {
            ApplicationError::NotFound("product".to_string())
        }
        other => ApplicationError::Port(format!("catalog store failure: {other}")),
    }
}

impl ProductRepository for SqliteCatalogRepository {
    fn create_product(
        &self,
        product: &Product,
        units: &[ProductUnit],
        origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = now_stamp();
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "INSERT INTO products
                 (id, sku, barcode, name_ar, name_fr, category_id, is_kit, is_active,
                  created_at, updated_at, revision, origin_device_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, 1, ?10)",
                rusqlite::params![
                    product.id.to_string(),
                    product.sku.as_str(),
                    product.barcode.as_ref().map(Barcode::as_str),
                    product.name_ar,
                    product.name_fr,
                    product.category_id.map(|id| id.to_string()),
                    bool_to_int(product.is_kit),
                    bool_to_int(product.is_active),
                    stamp,
                    origin_device_id,
                ],
            )?;
            for unit in units {
                tx.execute(
                    "INSERT INTO product_units
                     (id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode,
                      sale_price_centimes, purchase_price_centimes, is_base_unit, is_default_selling)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        unit.id.to_string(),
                        unit.product_id.to_string(),
                        unit.unit_name_ar,
                        unit.unit_name_fr,
                        unit.factor_micro.micros(),
                        unit.barcode.as_ref().map(Barcode::as_str),
                        unit.sale_price.centimes(),
                        unit.purchase_price.centimes(),
                        bool_to_int(unit.is_base_unit),
                        bool_to_int(unit.is_default_selling),
                    ],
                )?;
            }
            Ok(())
        })
        .map_err(infra_to_app)
    }

    fn update_product(
        &self,
        product: &Product,
        units: &[ProductUnit],
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = now_stamp();
        run_transaction(&mut guard, |tx| {
            let changed = tx.execute(
                "UPDATE products
                 SET name_ar = ?1, name_fr = ?2, category_id = ?3, is_active = ?4,
                     updated_at = ?5, revision = revision + 1
                 WHERE id = ?6",
                rusqlite::params![
                    product.name_ar,
                    product.name_fr,
                    product.category_id.map(|id| id.to_string()),
                    bool_to_int(product.is_active),
                    stamp,
                    product.id.to_string(),
                ],
            )?;
            if changed == 0 {
                return Err(crate::persistence::InfrastructureError::Sqlite(
                    rusqlite::Error::QueryReturnedNoRows,
                ));
            }
            for unit in units {
                let changed = tx.execute(
                    "UPDATE product_units
                     SET unit_name_ar = ?1, unit_name_fr = ?2, factor_micro = ?3,
                         barcode = ?4, sale_price_centimes = ?5,
                         purchase_price_centimes = ?6, is_base_unit = ?7,
                         is_default_selling = ?8
                     WHERE id = ?9",
                    rusqlite::params![
                        unit.unit_name_ar,
                        unit.unit_name_fr,
                        unit.factor_micro.micros(),
                        unit.barcode.as_ref().map(Barcode::as_str),
                        unit.sale_price.centimes(),
                        unit.purchase_price.centimes(),
                        bool_to_int(unit.is_base_unit),
                        bool_to_int(unit.is_default_selling),
                        unit.id.to_string(),
                    ],
                )?;
                if changed == 0 {
                    tx.execute(
                        "INSERT INTO product_units
                         (id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode,
                          sale_price_centimes, purchase_price_centimes, is_base_unit, is_default_selling)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        rusqlite::params![
                            unit.id.to_string(),
                            unit.product_id.to_string(),
                            unit.unit_name_ar,
                            unit.unit_name_fr,
                            unit.factor_micro.micros(),
                            unit.barcode.as_ref().map(Barcode::as_str),
                            unit.sale_price.centimes(),
                            unit.purchase_price.centimes(),
                            bool_to_int(unit.is_base_unit),
                            bool_to_int(unit.is_default_selling),
                        ],
                    )?;
                }
            }
            Ok(())
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Sqlite(
                rusqlite::Error::QueryReturnedNoRows,
            ) => ApplicationError::NotFound("product".to_string()),
            other => infra_to_app(other),
        })
    }

    fn find_by_id(
        &self,
        id: ProductId,
    ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError> {
        let guard = self.lock()?;
        fetch_product(&guard, &id.to_string())
    }

    fn find_by_barcode(
        &self,
        barcode: &Barcode,
    ) -> Result<Option<(Product, ProductUnit)>, ApplicationError> {
        let guard = self.lock()?;
        // Unit-level barcodes take precedence over product-level ones.
        let direct: Option<String> = guard
            .query_row(
                "SELECT product_id FROM product_units WHERE barcode = ?1",
                [barcode.as_str()],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())?;
        if let Some(product_id) = direct {
            if let Some((product, units)) = fetch_product(&guard, &product_id)? {
                if let Some(unit) = units
                    .iter()
                    .find(|unit| unit.barcode.as_ref() == Some(barcode))
                {
                    return Ok(Some((product, unit.clone())));
                }
            }
            return Ok(None);
        }
        // Fall back to the product-level barcode, resolved to its default unit.
        let product = match guard
            .query_row(
                &format!("SELECT {PRODUCT_COLUMNS} FROM products WHERE barcode = ?1"),
                [barcode.as_str()],
                map_product,
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())?
        {
            Some(product) => product,
            None => return Ok(None),
        };
        let mut statement = guard
            .prepare(&format!(
                "SELECT {UNIT_COLUMNS} FROM product_units WHERE product_id = ?1 ORDER BY is_default_selling DESC, is_base_unit DESC"
            ))
            .map_err(|_| store_failure())?;
        let units = statement
            .query_map([product.id.to_string()], map_unit)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        match units.into_iter().next() {
            Some(unit) => Ok(Some((product, unit))),
            None => Ok(None),
        }
    }

    fn find_by_sku(
        &self,
        sku: &Sku,
    ) -> Result<Option<(Product, Vec<ProductUnit>)>, ApplicationError> {
        let guard = self.lock()?;
        let product = match guard
            .query_row(
                &format!("SELECT {PRODUCT_COLUMNS} FROM products WHERE sku = ?1"),
                [sku.as_str()],
                map_product,
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())?
        {
            Some(product) => product,
            None => return Ok(None),
        };
        fetch_product(&guard, &product.id.to_string())
    }

    fn define_kit(&self, kit_id: ProductId, items: &[KitItem]) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "DELETE FROM product_kit_items WHERE kit_product_id = ?1",
                [kit_id.to_string()],
            )?;
            for item in items {
                tx.execute(
                    "INSERT INTO product_kit_items
                     (id, kit_product_id, component_product_id, unit_id, quantity_micro)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        item.kit_product_id.to_string(),
                        item.component_product_id.to_string(),
                        item.unit_id.to_string(),
                        item.quantity_micro.micros(),
                    ],
                )?;
            }
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn get_kit_items(&self, kit_id: ProductId) -> Result<Vec<KitItem>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT kit_product_id, component_product_id, unit_id, quantity_micro
                 FROM product_kit_items WHERE kit_product_id = ?1 ORDER BY rowid",
            )
            .map_err(|_| store_failure())?;
        let items = statement
            .query_map([kit_id.to_string()], map_kit_item)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(items)
    }

    fn get_all_kit_edges(&self) -> Result<Vec<(ProductId, ProductId)>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare("SELECT kit_product_id, component_product_id FROM product_kit_items")
            .map_err(|_| store_failure())?;
        let pairs = statement
            .query_map([], |row| {
                let kit: String = row.get(0)?;
                let component: String = row.get(1)?;
                Ok((kit, component))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        pairs
            .into_iter()
            .map(|(kit, component)| {
                Ok((parse_product_id(&kit).map_err(|_| store_failure())?, {
                    parse_product_id(&component).map_err(|_| store_failure())?
                }))
            })
            .collect()
    }

    fn get_direct_kit_items(
        &self,
        kit_product_id: ProductId,
    ) -> Result<Vec<KitComponentRow>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT component_product_id, unit_id, quantity_micro
                 FROM product_kit_items WHERE kit_product_id = ?1 ORDER BY rowid",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([kit_product_id.to_string()], |row| {
                let component: String = row.get(0)?;
                let unit: String = row.get(1)?;
                let quantity: i64 = row.get(2)?;
                Ok((component, unit, quantity))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(|(component, unit, quantity)| {
                Ok(KitComponentRow {
                    component_product_id: parse_product_id(&component)
                        .map_err(|_| store_failure())?,
                    unit_id: Uuid::parse_str(&unit)
                        .map(UnitId::from)
                        .map_err(|_| store_failure())?,
                    quantity_micro: quantity,
                })
            })
            .collect()
    }

    fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(Product, Vec<ProductUnit>)>, ApplicationError> {
        let guard = self.lock()?;
        let escaped = query
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("{escaped}%");
        let capped = (limit.clamp(1, 100)) as i64;
        let mut statement = guard
            .prepare(
                "SELECT DISTINCT p.id FROM products p
                 LEFT JOIN product_units u ON u.product_id = p.id
                 WHERE p.sku LIKE ?1 ESCAPE '\\'
                    OR p.name_fr LIKE ?1 ESCAPE '\\'
                    OR p.name_ar LIKE ?1 ESCAPE '\\'
                    OR u.barcode = ?2
                    OR p.barcode = ?2
                 ORDER BY p.sku LIMIT ?3",
            )
            .map_err(|_| store_failure())?;
        let ids = statement
            .query_map(rusqlite::params![pattern, query, capped], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(found) = fetch_product(&guard, &id)? {
                out.push(found);
            }
        }
        Ok(out)
    }
}

impl CategoryRepository for SqliteCatalogRepository {
    fn create_category(&self, category: &Category) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "INSERT INTO categories (id, code, name_ar, name_fr, parent_id, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    category.id.to_string(),
                    category.code,
                    category.name_ar,
                    category.name_fr,
                    category.parent_id.map(|id| id.to_string()),
                    bool_to_int(category.is_active),
                ],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn find_category_by_id(&self, id: CategoryId) -> Result<Option<Category>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT id, code, name_ar, name_fr, parent_id, is_active
                 FROM categories WHERE id = ?1",
                [id.to_string()],
                map_category,
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn list_categories(&self) -> Result<Vec<Category>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT id, code, name_ar, name_fr, parent_id, is_active
                 FROM categories ORDER BY code",
            )
            .map_err(|_| store_failure())?;
        let categories = statement
            .query_map([], map_category)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(categories)
    }
}
