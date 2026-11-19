//! SQLite variant-matrix repository (TASK-BRK-114, Wave 4).
//!
//! `generate_matrix` writes the model row plus every cell (products +
//! units + cells + inherited price-break rungs) in ONE `BEGIN IMMEDIATE`
//! transaction: no half-written matrices. UNIQUE violations on
//! `products.sku` / `product_units.barcode` map to the same named domain
//! errors as the catalog repository, so a SKU clash fails LOUDLY with the
//! whole matrix rolled back. `matrix_view` serves the DSN-108/109 2D grid
//! in ONE query (stock summed across warehouses, default-selling price
//! inline) — never N+1.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    MatrixCellWrite, MatrixViewUnit, VariantMatrixRepository, VariantMatrixViewRow,
};
use baraka_domain::catalog::{
    Barcode, ProductId, Sku, UnitId, VariantCell, VariantCellId, VariantModel, VariantModelId,
};
use baraka_domain::errors::DomainError;
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::persistence::transaction::run_transaction;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("variant-matrix store failure".to_string())
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

/// SQLite variant-matrix repository sharing the application connection.
pub struct SqliteVariantMatrixRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteVariantMatrixRepository {
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

impl VariantMatrixRepository for SqliteVariantMatrixRepository {
    fn generate_matrix(
        &self,
        model: &VariantModel,
        cells: &[MatrixCellWrite],
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = now_stamp();
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "INSERT INTO variant_models
                 (id, name_ar, name_fr, axis1_name, axis2_name, category_id,
                  is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                rusqlite::params![
                    model.id.0.to_string(),
                    model.name_ar,
                    model.name_fr,
                    model.axis1_name,
                    model.axis2_name,
                    model.category_id.map(|id| id.to_string()),
                    bool_to_int(model.is_active),
                    stamp,
                ],
            )?;
            for cell in cells {
                tx.execute(
                    "INSERT INTO products
                     (id, sku, barcode, name_ar, name_fr, category_id, is_kit, is_active,
                      created_at, updated_at, revision, origin_device_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 1, ?7, ?7, 1, 'variant-matrix')",
                    rusqlite::params![
                        cell.product.id.to_string(),
                        cell.product.sku.as_str(),
                        cell.product.barcode.as_ref().map(Barcode::as_str),
                        cell.product.name_ar,
                        cell.product.name_fr,
                        cell.product.category_id.map(|id| id.to_string()),
                        stamp,
                    ],
                )?;
                for unit in &cell.units {
                    tx.execute(
                        "INSERT INTO product_units
                         (id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode,
                          sale_price_centimes, purchase_price_centimes, wholesale_price_centimes,
                          is_base_unit, is_default_selling, super_gros_price_centimes)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        rusqlite::params![
                            unit.id.to_string(),
                            unit.product_id.to_string(),
                            unit.unit_name_ar,
                            unit.unit_name_fr,
                            unit.factor_micro.micros(),
                            unit.barcode.as_ref().map(Barcode::as_str),
                            unit.sale_price.centimes(),
                            unit.purchase_price.centimes(),
                            unit.wholesale_price
                                .map(baraka_domain::primitives::Money::centimes),
                            bool_to_int(unit.is_base_unit),
                            bool_to_int(unit.is_default_selling),
                            unit.super_gros_price
                                .map(baraka_domain::primitives::Money::centimes),
                        ],
                    )?;
                }
                for (unit_id, rungs) in &cell.breaks {
                    for rung in rungs {
                        tx.execute(
                            "INSERT INTO price_breaks
                             (id, product_id, unit_id, min_qty_micro, price_centimes)
                             VALUES (?1, ?2, ?3, ?4, ?5)",
                            rusqlite::params![
                                Uuid::new_v4().to_string(),
                                cell.product.id.to_string(),
                                unit_id.to_string(),
                                rung.min_qty_micro,
                                rung.price.centimes(),
                            ],
                        )?;
                    }
                }
                tx.execute(
                    "INSERT INTO variant_cells
                     (id, model_id, product_id, axis1_value, axis2_value, axis1_sort, axis2_sort)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        model.id.0.to_string(),
                        cell.product.id.to_string(),
                        cell.axis1_value,
                        cell.axis2_value,
                        cell.axis1_sort,
                        cell.axis2_sort,
                    ],
                )?;
            }
            Ok(())
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Sqlite(inner) => constraint_error(&inner),
            other => ApplicationError::Port(format!("variant-matrix store failure: {other}")),
        })
    }

    fn find_variant_model(
        &self,
        id: VariantModelId,
    ) -> Result<Option<VariantModel>, ApplicationError> {
        let guard = self.lock()?;
        let mut query = guard
            .prepare(
                "SELECT name_ar, name_fr, axis1_name, axis2_name, category_id, is_active
                 FROM variant_models WHERE id = ?1",
            )
            .map_err(|_| store_failure())?;
        let row: Option<(String, String, String, String, Option<String>, i64)> = query
            .query_row([id.0.to_string()], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .optional()
            .map_err(|_| store_failure())?;
        row.map(
            |(name_ar, name_fr, axis1_name, axis2_name, category_id, is_active)| {
                let category = category_id
                    .map(|raw| {
                        raw.parse::<Uuid>()
                            .map(baraka_domain::catalog::CategoryId::from)
                            .map_err(|_| {
                                rusqlite::Error::InvalidColumnName("category_id".to_string())
                            })
                    })
                    .transpose()
                    .map_err(|_| store_failure())?;
                Ok(VariantModel {
                    id,
                    name_ar,
                    name_fr,
                    axis1_name,
                    axis2_name,
                    category_id: category,
                    is_active: is_active != 0,
                })
            },
        )
        .transpose()
    }

    fn matrix_cells(&self, model_id: VariantModelId) -> Result<Vec<VariantCell>, ApplicationError> {
        let guard = self.lock()?;
        let mut query = guard
            .prepare(
                "SELECT id, model_id, product_id, axis1_value, axis2_value, axis1_sort, axis2_sort
                 FROM variant_cells WHERE model_id = ?1 ORDER BY axis1_sort, axis2_sort",
            )
            .map_err(|_| store_failure())?;
        let rows = query
            .query_map([model_id.0.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            })
            .map_err(|_| store_failure())?;
        let mut cells = Vec::new();
        for row in rows {
            let (id, model_raw, product_raw, axis1_value, axis2_value, axis1_sort, axis2_sort) =
                row.map_err(|_| store_failure())?;
            let parse_id = |raw: String, what: &'static str| {
                raw.parse::<Uuid>()
                    .map_err(|_| rusqlite::Error::InvalidColumnName(what.to_string()))
            };
            cells.push(VariantCell {
                id: VariantCellId(parse_id(id, "variant_cells.id").map_err(|_| store_failure())?),
                model_id: VariantModelId(
                    parse_id(model_raw, "variant_cells.model_id").map_err(|_| store_failure())?,
                ),
                product_id: ProductId(
                    parse_id(product_raw, "variant_cells.product_id")
                        .map_err(|_| store_failure())?,
                ),
                axis1_value,
                axis2_value,
                axis1_sort,
                axis2_sort,
            });
        }
        Ok(cells)
    }

    fn matrix_view(
        &self,
        model_id: VariantModelId,
    ) -> Result<Vec<VariantMatrixViewRow>, ApplicationError> {
        let guard = self.lock()?;
        let mut query = guard
            .prepare(
                "SELECT c.axis1_value, c.axis2_value, p.id, p.sku, p.name_ar, p.barcode,
                        COALESCE((SELECT SUM(s.quantity_micro) FROM stock_levels s
                                  WHERE s.product_id = p.id), 0),
                        u.sale_price_centimes,
                        u.id, u.unit_name_ar, u.unit_name_fr, u.factor_micro,
                        u.barcode, u.wholesale_price_centimes,
                        u.super_gros_price_centimes, u.is_base_unit
                 FROM variant_cells c
                 JOIN products p ON p.id = c.product_id
                 LEFT JOIN product_units u
                   ON u.product_id = p.id AND u.is_default_selling = 1
                 WHERE c.model_id = ?1
                 ORDER BY c.axis1_sort, c.axis2_sort",
            )
            .map_err(|_| store_failure())?;
        let rows = query
            .query_map([model_id.0.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<i64>>(13)?,
                    row.get::<_, Option<i64>>(14)?,
                    row.get::<_, Option<i64>>(15)?,
                ))
            })
            .map_err(|_| store_failure())?;
        let mut view = Vec::new();
        for row in rows {
            let (
                axis1_value,
                axis2_value,
                product_raw,
                sku_raw,
                name_ar,
                barcode_raw,
                on_hand_micro,
                sale_price_centimes,
                unit_id_raw,
                unit_name_ar,
                unit_name_fr,
                factor_micro,
                unit_barcode,
                wholesale_price_centimes,
                super_gros_price_centimes,
                is_base_unit,
            ) = row.map_err(|_| store_failure())?;
            let product_id = product_raw
                .parse::<Uuid>()
                .map(ProductId)
                .map_err(|_| store_failure())?;
            let sku = Sku::new(&sku_raw).map_err(|_| store_failure())?;
            let barcode = barcode_raw
                .map(|raw| Barcode::new(&raw))
                .transpose()
                .map_err(|_| store_failure())?;
            let default_unit = match (
                unit_id_raw,
                unit_name_ar,
                unit_name_fr,
                factor_micro,
                is_base_unit,
            ) {
                (Some(id_raw), Some(ar), Some(fr), Some(factor), Some(base)) => {
                    let unit_id = id_raw
                        .parse::<Uuid>()
                        .map(UnitId)
                        .map_err(|_| store_failure())?;
                    Some(MatrixViewUnit {
                        unit_id,
                        unit_name_ar: ar,
                        unit_name_fr: fr,
                        factor_micro: factor,
                        unit_barcode,
                        wholesale_price_centimes,
                        super_gros_price_centimes,
                        is_base_unit: base != 0,
                    })
                }
                _ => None,
            };
            view.push(VariantMatrixViewRow {
                axis1_value,
                axis2_value,
                product_id,
                sku,
                name_ar,
                barcode,
                on_hand_micro,
                sale_price_centimes,
                default_unit,
            });
        }
        Ok(view)
    }

    fn list_models(&self) -> Result<Vec<VariantModel>, ApplicationError> {
        let guard = self.lock()?;
        let mut query = guard
            .prepare(
                "SELECT id, name_ar, name_fr, axis1_name, axis2_name, category_id, is_active
                 FROM variant_models ORDER BY name_ar",
            )
            .map_err(|_| store_failure())?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            })
            .map_err(|_| store_failure())?;
        let mut models = Vec::new();
        for row in rows {
            let (id_raw, name_ar, name_fr, axis1_name, axis2_name, category_id, is_active) =
                row.map_err(|_| store_failure())?;
            let id = id_raw
                .parse::<Uuid>()
                .map(VariantModelId)
                .map_err(|_| store_failure())?;
            let category = category_id
                .map(|raw| {
                    raw.parse::<Uuid>()
                        .map(baraka_domain::catalog::CategoryId::from)
                        .map_err(|_| store_failure())
                })
                .transpose()?;
            models.push(VariantModel {
                id,
                name_ar,
                name_fr,
                axis1_name,
                axis2_name,
                category_id: category,
                is_active: is_active != 0,
            });
        }
        Ok(models)
    }
}
