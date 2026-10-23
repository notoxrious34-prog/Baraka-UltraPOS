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

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CatalogMetrics, CategoryRepository, KitComponentRow, ProductRepository, SessionContext,
};
use baraka_application::use_cases::catalog::{
    ImportBatchReport, ImportDuplicatePolicy, ImportRowError, ProductImportRepository,
    ValidatedImportRow,
};
use baraka_domain::catalog::{
    search_normalization::{normalize_for_search, tokenize_query},
    Barcode, Category, CategoryId, KitItem, Product, ProductId, ProductUnit, Sku, UnitId,
};
use baraka_domain::errors::DomainError;
use baraka_domain::primitives::{FactorMicro, Money, QtyMicro};
use chrono::{SecondsFormat, Utc};
use rusqlite::types::ValueRef;
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

/// FTS5 mirror columns per product (migration 0016): French/Arabic norms,
/// SKU norm, unit-aggregated barcode norm, alias norms.
const FTS_TEXT_COLUMNS: [&str; 5] = [
    "norm_fr",
    "norm_ar",
    "norm_sku",
    "norm_barcode",
    "norm_aliases",
];

/// Split a normalized token the way the `unicode61` tokenizer does: maximal
/// runs of alphanumeric characters (punctuation such as `.`, `-`, `°` breaks
/// tokens on both sides, so `2.5mm` matches as `2` + `5mm`).
fn fts_token_parts(token: &str) -> impl Iterator<Item = &str> {
    token
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
}

/// Build the FTS5 MATCH expression for one tokenized query (ADR-101): token
/// groups ANDed, dictionary variants within a group ORed, every variant part
/// matched as a prefix query across all five mirror columns. Returns `None`
/// when no usable part exists (empty token, lone punctuation such as `"`).
/// Safety: every emitted part is alphanumeric-only, so no `"`, `*` or `:`
/// can ever reach the FTS parser — the expression travels as a bound
/// parameter and cannot fail to parse or escape its MATCH.
fn build_fts_match_query(groups: &[Vec<String>]) -> Option<String> {
    let mut and_clauses: Vec<String> = Vec::new();
    for group in groups {
        let mut or_clauses: Vec<String> = Vec::new();
        for variant in group {
            let mut part_clauses: Vec<String> = Vec::new();
            for part in fts_token_parts(variant) {
                let columned: Vec<String> = FTS_TEXT_COLUMNS
                    .iter()
                    .map(|col| format!("{col}:\"{part}\"*"))
                    .collect();
                part_clauses.push(format!("({})", columned.join(" OR ")));
            }
            if !part_clauses.is_empty() {
                or_clauses.push(format!("({})", part_clauses.join(" AND ")));
            }
        }
        if or_clauses.is_empty() {
            return None;
        }
        and_clauses.push(format!("({})", or_clauses.join(" OR ")));
    }
    if and_clauses.is_empty() {
        return None;
    }
    Some(and_clauses.join(" AND "))
}

/// Register the deterministic `baraka_search_norm(text)` scalar function
/// (unified French/Arabic search normalization). Idempotent:
/// re-registration on the same connection replaces the previous binding.
fn ensure_functions_registered(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.create_scalar_function(
        "baraka_search_norm",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8
            | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let value = ctx.get_raw(0);
            if value == ValueRef::Null {
                return Ok(String::new());
            }
            let text = value.as_str()?;
            Ok(normalize_for_search(text))
        },
    )
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

    /// Verify the FTS5 mirror against relational truth (TASK-BRK-076, ADR-101).
    ///
    /// O(1)-ish by design: one row-count comparison plus a fixed-stride
    /// content sample (every 64th product row). Each sampled product's FTS
    /// row is compared against relational truth re-derived with the SAME
    /// domain normalization (raw names, SKU, product + unit barcodes, alias
    /// rows); any divergence (missing row, stale norms, lost aliases) is a
    /// `Port` error naming the product. Expected values are recomputed from
    /// raw columns — never from the stored norm columns — so the check stays
    /// valid for rows written before the BRK-077 repository norm writes.
    /// The query path is untouched — this is a structural health check for
    /// tests and (later) the backup/restore verification path. Token sets
    /// compare sorted because `group_concat` order is not contractual.
    pub fn verify_fts_consistency(&self) -> Result<(), ApplicationError> {
        use std::hash::{Hash, Hasher};
        let guard = self.lock()?;
        let product_count: i64 = guard
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .map_err(|_| store_failure())?;
        let fts_count: i64 = guard
            .query_row("SELECT COUNT(*) FROM products_fts", [], |row| row.get(0))
            .map_err(|_| store_failure())?;
        if product_count != fts_count {
            return Err(ApplicationError::Port(format!(
                "fts row-count mismatch: products={product_count} fts={fts_count}"
            )));
        }
        let mut sample = guard
            .prepare(
                "SELECT id, name_fr, name_ar, sku, barcode
                 FROM products WHERE rowid % 64 = 0 ORDER BY rowid",
            )
            .map_err(|_| store_failure())?;
        let rows = sample
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        for (id, name_fr, name_ar, sku, barcode) in &rows {
            let expected_barcode: String = guard
                .query_row(
                    "SELECT COALESCE(group_concat(baraka_search_norm(b), ' '), '')
                     FROM (SELECT barcode AS b FROM products WHERE id = ?1
                           UNION SELECT barcode FROM product_units WHERE product_id = ?1)
                     WHERE b IS NOT NULL",
                    rusqlite::params![id, id],
                    |row| row.get(0),
                )
                .map_err(|_| store_failure())?;
            let expected_aliases: String = guard
                .query_row(
                    "SELECT COALESCE(group_concat(alias_norm, ' '), '')
                     FROM product_aliases WHERE product_id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .map_err(|_| store_failure())?;
            let expected_sku = normalize_for_search(sku);
            let expected_barcode_norm = {
                let mut parts = vec![normalize_for_search(barcode.as_deref().unwrap_or(""))];
                let mut units = guard
                    .prepare("SELECT barcode FROM product_units WHERE product_id = ?1")
                    .map_err(|_| store_failure())?;
                let unit_codes = units
                    .query_map([id], |row| row.get::<_, Option<String>>(0))
                    .map_err(|_| store_failure())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| store_failure())?;
                for code in &unit_codes {
                    parts.push(normalize_for_search(code.as_deref().unwrap_or("")));
                }
                parts.retain(|part| !part.is_empty());
                parts.join(" ")
            };
            let (fts_fr, fts_ar, fts_sku, fts_barcode, fts_aliases): (
                String,
                String,
                String,
                String,
                String,
            ) = guard
                .query_row(
                    "SELECT norm_fr, norm_ar, norm_sku, norm_barcode, norm_aliases
                     FROM products_fts WHERE product_id = ?1",
                    [id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .map_err(|_| ApplicationError::Port(format!("fts row missing for product {id}")))?;
            // group_concat order is not contractual: compare as sorted token sets.
            let mut expected = (
                normalize_for_search(name_fr),
                normalize_for_search(name_ar),
                expected_sku,
                expected_barcode_norm,
                expected_aliases,
            );
            let mut actual = (fts_fr, fts_ar, fts_sku, fts_barcode, fts_aliases);
            let mut hash_expected = std::collections::hash_map::DefaultHasher::new();
            let mut hash_actual = std::collections::hash_map::DefaultHasher::new();
            for field in [
                &mut expected.0,
                &mut expected.1,
                &mut expected.2,
                &mut expected.3,
                &mut expected.4,
            ] {
                let mut tokens: Vec<&str> = field.split_whitespace().collect();
                tokens.sort_unstable();
                tokens.join(" ").hash(&mut hash_expected);
            }
            for field in [
                &mut actual.0,
                &mut actual.1,
                &mut actual.2,
                &mut actual.3,
                &mut actual.4,
            ] {
                let mut tokens: Vec<&str> = field.split_whitespace().collect();
                tokens.sort_unstable();
                tokens.join(" ").hash(&mut hash_actual);
            }
            // Cross-check the barcode aggregate against SQL's own computation too.
            let mut sql_tokens: Vec<&str> = expected_barcode.split_whitespace().collect();
            sql_tokens.sort_unstable();
            let mut sql_hash = std::collections::hash_map::DefaultHasher::new();
            sql_tokens.join(" ").hash(&mut sql_hash);
            let mut rust_tokens: Vec<&str> = expected.3.split_whitespace().collect();
            rust_tokens.sort_unstable();
            let mut rust_hash = std::collections::hash_map::DefaultHasher::new();
            rust_tokens.join(" ").hash(&mut rust_hash);
            if hash_expected.finish() != hash_actual.finish()
                || sql_hash.finish() != rust_hash.finish()
            {
                return Err(ApplicationError::Port(format!(
                    "fts content mismatch for product {id}"
                )));
            }
            let _ = barcode;
        }
        Ok(())
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
    let wholesale: Option<i64> = row.get(8)?;
    let is_base: bool = row.get::<_, i64>(9)? != 0;
    let is_default: bool = row.get::<_, i64>(10)? != 0;
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
        wholesale_price: wholesale.map(Money::from_centimes),
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
const UNIT_COLUMNS: &str = "id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode, sale_price_centimes, purchase_price_centimes, wholesale_price_centimes, is_base_unit, is_default_selling";

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

/// Hydrate matched ids in rank order with two round-trips (BRK-077-C1):
/// one `IN` query for products, one for their units — replacing the N+1
/// per-id `fetch_product` loop that dominated 80k search latency.
/// Chunked at 250 ids, far below the SQLite variable limit, so the 500-row
/// discovery branch stays safe. Input order is preserved; ids with no
/// product row are skipped, exactly like the retired loop.
fn fetch_products_ordered(
    conn: &Connection,
    ids: &[String],
) -> Result<Vec<(Product, Vec<ProductUnit>)>, ApplicationError> {
    use std::collections::HashMap;
    let mut products: HashMap<String, Product> = HashMap::new();
    let mut units_by_product: HashMap<String, Vec<ProductUnit>> = HashMap::new();
    for chunk in ids.chunks(250) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let product_sql =
            format!("SELECT {PRODUCT_COLUMNS} FROM products WHERE id IN ({placeholders})");
        let params: Vec<rusqlite::types::Value> = chunk
            .iter()
            .map(|id| rusqlite::types::Value::Text(id.clone()))
            .collect();
        let mut statement = conn.prepare(&product_sql).map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params), map_product)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        for product in rows {
            products.insert(product.id.to_string(), product);
        }
        let unit_sql = format!(
            "SELECT {UNIT_COLUMNS} FROM product_units WHERE product_id IN ({placeholders}) \
             ORDER BY product_id, is_base_unit DESC, id"
        );
        let params: Vec<rusqlite::types::Value> = chunk
            .iter()
            .map(|id| rusqlite::types::Value::Text(id.clone()))
            .collect();
        let mut statement = conn.prepare(&unit_sql).map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params), map_unit)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        for unit in rows {
            units_by_product
                .entry(unit.product_id.to_string())
                .or_default()
                .push(unit);
        }
    }
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(product) = products.remove(id) {
            let units = units_by_product.remove(id).unwrap_or_default();
            out.push((product, units));
        }
    }
    Ok(out)
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
                      sale_price_centimes, purchase_price_centimes, wholesale_price_centimes,
                      is_base_unit, is_default_selling)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    rusqlite::params![
                        unit.id.to_string(),
                        unit.product_id.to_string(),
                        unit.unit_name_ar,
                        unit.unit_name_fr,
                        unit.factor_micro.micros(),
                        unit.barcode.as_ref().map(Barcode::as_str),
                        unit.sale_price.centimes(),
                        unit.purchase_price.centimes(),
                        unit.wholesale_price.map(Money::centimes),
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
                         purchase_price_centimes = ?6, wholesale_price_centimes = ?7,
                         is_base_unit = ?8, is_default_selling = ?9
                     WHERE id = ?10",
                    rusqlite::params![
                        unit.unit_name_ar,
                        unit.unit_name_fr,
                        unit.factor_micro.micros(),
                        unit.barcode.as_ref().map(Barcode::as_str),
                        unit.sale_price.centimes(),
                        unit.purchase_price.centimes(),
                        unit.wholesale_price.map(Money::centimes),
                        bool_to_int(unit.is_base_unit),
                        bool_to_int(unit.is_default_selling),
                        unit.id.to_string(),
                    ],
                )?;
                if changed == 0 {
                    tx.execute(
                        "INSERT INTO product_units
                         (id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode,
                          sale_price_centimes, purchase_price_centimes, wholesale_price_centimes,
                          is_base_unit, is_default_selling)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        rusqlite::params![
                            unit.id.to_string(),
                            unit.product_id.to_string(),
                            unit.unit_name_ar,
                            unit.unit_name_fr,
                            unit.factor_micro.micros(),
                            unit.barcode.as_ref().map(Barcode::as_str),
                            unit.sale_price.centimes(),
                            unit.purchase_price.centimes(),
                            unit.wholesale_price.map(Money::centimes),
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
        include_inactive: bool,
    ) -> Result<Vec<(Product, Vec<ProductUnit>)>, ApplicationError> {
        let guard = self.lock()?;
        let capped = (limit.clamp(1, 500)) as i64;
        if query.trim().is_empty() {
            let sql = if include_inactive {
                "SELECT DISTINCT p.id FROM products p ORDER BY p.sku LIMIT ?1"
            } else {
                "SELECT DISTINCT p.id FROM products p WHERE p.is_active = 1 ORDER BY p.sku LIMIT ?1"
            };
            let ids = guard
                .prepare(sql)
                .map_err(|_| store_failure())?
                .query_map([capped], |row| row.get::<_, String>(0))
                .map_err(|_| store_failure())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| store_failure())?;
            return fetch_products_ordered(&guard, &ids);
        }
        let trimmed = query.trim();
        // Exact-barcode fast path for single-token queries (scanners):
        // UNION of two indexed point-seeks (idx_products_barcode,
        // idx_product_units_barcode). The retired OR-across-LEFT-JOIN shape
        // degraded to a multi-second scan at 80k rows (BRK-077 probe).
        if !trimmed.chars().any(char::is_whitespace) && trimmed.chars().count() >= 3 {
            let sql = if include_inactive {
                "SELECT p.id FROM products p WHERE p.barcode = ?1 \
                 UNION \
                 SELECT u.product_id FROM product_units u WHERE u.barcode = ?1 LIMIT ?2"
            } else {
                "SELECT p.id FROM products p WHERE p.barcode = ?1 AND p.is_active = 1 \
                 UNION \
                 SELECT u.product_id FROM product_units u WHERE u.barcode = ?1 \
                 AND EXISTS (SELECT 1 FROM products p2 WHERE p2.id = u.product_id AND p2.is_active = 1) LIMIT ?2"
            };
            let mut statement = guard.prepare(sql).map_err(|_| store_failure())?;
            let ids = statement
                .query_map(rusqlite::params![trimmed, capped], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|_| store_failure())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| store_failure())?;
            if !ids.is_empty() {
                return fetch_products_ordered(&guard, &ids);
            }
        }
        // ADR-101 query path (TASK-BRK-077): a single FTS5 prefix MATCH over
        // the pre-normalized mirror. Token groups ANDed, dictionary variants
        // within a group ORed — the same recall shape as the LIKE body it
        // replaces, minus leading-infix fragments (accepted delta, pinned
        // in catalog_search_fts_parity). Barcode-exact above now UNIONs two
        // indexed seeks (same recall, no join scan).
        ensure_functions_registered(&guard).map_err(|_| store_failure())?;
        let groups = tokenize_query(trimmed);
        if groups.is_empty() {
            return Ok(Vec::new());
        }
        let Some(match_query) = build_fts_match_query(&groups) else {
            return Ok(Vec::new());
        };
        // Rank-then-hydrate (TASK-BRK-084): MATCH + ranking runs first over
        // NARROW rows only (fts product_id + bm25 rank + products sku), LIMIT
        // applies before any full-row visit, and the outer query merely
        // re-applies the subquery order via the materialized sequence — it
        // never re-sorts. The retired C1 shape sorted the joined row set and
        // let DISTINCT materialize thousands of rows the LIMIT discarded.
        // Order contract (identical by construction): bm25 rank, then raw
        // `products.sku` tie-break. sku lives in `products`, not in the FTS
        // mirror, so the subquery joins it by PK point-lookup only. The UNION
        // barcode fast path above is untouched. The mirror holds exactly one
        // row per product (0016 DELETE+INSERT triggers), so no DISTINCT is
        // needed; recall/order drift fails the parity + engine suites loudly.
        let mut sql = String::from(
            "SELECT ranked.product_id FROM ( \
             SELECT inner_ranked.product_id AS product_id, \
             ROW_NUMBER() OVER (ORDER BY inner_ranked.rank, inner_ranked.sku) AS seq \
             FROM ( \
             SELECT products_fts.product_id AS product_id, \
             bm25(products_fts) AS rank, \
             p0.sku AS sku \
             FROM products_fts \
             JOIN products p0 ON p0.id = products_fts.product_id \
             WHERE products_fts MATCH ?1",
        );
        sql.push_str(if include_inactive {
            ""
        } else {
            " AND p0.is_active = 1"
        });
        sql.push_str(
            " ORDER BY rank, sku LIMIT ?2 \
             ) AS inner_ranked \
             ) AS ranked ORDER BY ranked.seq",
        );
        let patterns: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Text(match_query),
            rusqlite::types::Value::Integer(capped),
        ];
        let mut statement = guard.prepare(&sql).map_err(|_| store_failure())?;
        let ids = statement
            .query_map(rusqlite::params_from_iter(patterns), |row| {
                row.get::<_, String>(0)
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        fetch_products_ordered(&guard, &ids)
    }

    fn get_metrics(&self) -> Result<CatalogMetrics, ApplicationError> {
        let guard = self.lock()?;
        let (total_products, inactive, in_stock, out_of_stock): (i64, i64, i64, i64) = guard
            .query_row(
                "SELECT
                    COUNT(p.id),
                    COALESCE(SUM(CASE WHEN p.is_active = 0 THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN p.is_active = 1 AND COALESCE(s.qty, 0) > 0 THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN p.is_active = 1 AND COALESCE(s.qty, 0) <= 0 THEN 1 ELSE 0 END), 0)
                 FROM products p
                 LEFT JOIN (
                     SELECT product_id, SUM(quantity_base_micro) AS qty
                     FROM stock_levels
                     GROUP BY product_id
                 ) s ON s.product_id = p.id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|_| store_failure())?;
        Ok(CatalogMetrics {
            total_products,
            in_stock,
            out_of_stock,
            inactive,
        })
    }

    fn count_product_references(&self, id: ProductId) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let key = id.to_string();
        let documents: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM document_items WHERE product_id = ?1",
                [&key],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        let movements: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM stock_movements WHERE product_id = ?1",
                [&key],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        let kits: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM product_kit_items WHERE kit_product_id = ?1 OR component_product_id = ?1",
                [&key],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        Ok(documents.max(0) as u64 + movements.max(0) as u64 + kits.max(0) as u64)
    }

    fn hard_delete_product(&self, id: ProductId) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let key = id.to_string();
        run_transaction(&mut guard, |tx| {
            tx.execute("DELETE FROM stock_levels WHERE product_id = ?1", [&key])?;
            tx.execute("DELETE FROM product_units WHERE product_id = ?1", [&key])?;
            let removed = tx.execute("DELETE FROM products WHERE id = ?1", [&key])?;
            if removed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows.into());
            }
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn set_product_active(&self, id: ProductId, active: bool) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let removed = guard
            .execute(
                "UPDATE products SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![i64::from(active), stamp, id.to_string()],
            )
            .map_err(|_| store_failure())?;
        if removed == 0 {
            return Err(ApplicationError::NotFound("product".to_string()));
        }
        Ok(())
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

    fn update_category(&self, category: &Category) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        run_transaction(&mut guard, |tx| {
            let changed = tx.execute(
                "UPDATE categories
                 SET name_ar = ?1, name_fr = ?2, parent_id = ?3, is_active = ?4
                 WHERE id = ?5",
                rusqlite::params![
                    category.name_ar,
                    category.name_fr,
                    category.parent_id.map(|id| id.to_string()),
                    bool_to_int(category.is_active),
                    category.id.to_string(),
                ],
            )?;
            if changed == 0 {
                return Err(crate::persistence::InfrastructureError::Sqlite(
                    rusqlite::Error::QueryReturnedNoRows,
                ));
            }
            Ok(())
        })
        .map_err(|_| store_failure())
    }
}

/// Deterministic collision-free SKU: `PRD-` plus six uppercase hex digits
/// hashed from (French name, row index, attempt). Identical input order
/// regenerates identical codes, so re-imports under `Skip` converge to
/// skips instead of creating near-duplicates.
fn generate_import_sku(name_fr: &str, row_index: usize, attempt: u32) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name_fr.hash(&mut hasher);
    row_index.hash(&mut hasher);
    attempt.hash(&mut hasher);
    format!("PRD-{:06X}", (hasher.finish() & 0xFF_FFFF) as u32)
}

/// True when a row exists; absence is `false`, any real failure aborts.
fn row_exists(
    tx: &rusqlite::Transaction,
    sql: &str,
    value: &str,
) -> Result<bool, ApplicationError> {
    match tx.query_row(sql, rusqlite::params![value], |_| Ok(())) {
        Ok(()) => Ok(true),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(_) => Err(store_failure()),
    }
}

/// Resolve the destination warehouse: exact code-or-id match first, then
/// the first active depot by code. Fails only on an empty warehouse table.
fn resolve_import_warehouse(
    tx: &rusqlite::Transaction,
    warehouse_code: Option<&str>,
) -> Result<String, ApplicationError> {
    let requested = warehouse_code
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .unwrap_or("DEP-01");
    if let Ok(id) = tx.query_row(
        "SELECT id FROM warehouses WHERE (code = ?1 OR id = ?1) AND is_active = 1",
        rusqlite::params![requested],
        |row| row.get::<_, String>(0),
    ) {
        return Ok(id);
    }
    tx.query_row(
        "SELECT id FROM warehouses WHERE is_active = 1 ORDER BY code LIMIT 1",
        rusqlite::params![],
        |row| row.get::<_, String>(0),
    )
    .map_err(|_| store_failure())
}

impl ProductImportRepository for SqliteCatalogRepository {
    fn import_batch(
        &self,
        actor: &SessionContext,
        rows: Vec<(usize, ValidatedImportRow)>,
        warehouse_code: Option<String>,
        on_duplicate: ImportDuplicatePolicy,
        origin_device_id: &str,
    ) -> Result<ImportBatchReport, ApplicationError> {
        let mut guard = self.lock()?;
        let tx = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| store_failure())?;
        let outcome = import_rows_in_transaction(
            &tx,
            &actor.user_id.to_string(),
            origin_device_id,
            &rows,
            warehouse_code.as_deref(),
            on_duplicate,
        );
        match outcome {
            Ok(report) => {
                tx.commit().map_err(|_| store_failure())?;
                Ok(report)
            }
            // Transaction drops uncommitted: full rollback on abort.
            Err(error) => Err(error),
        }
    }
}

/// Row loop of the atomic import. Products, base units, and opening stock
/// land in the caller's `BEGIN IMMEDIATE` transaction; any `Err` return
/// rolls the whole batch back.
fn import_rows_in_transaction(
    tx: &rusqlite::Transaction,
    actor_user_id: &str,
    origin_device_id: &str,
    rows: &[(usize, ValidatedImportRow)],
    warehouse_code: Option<&str>,
    on_duplicate: ImportDuplicatePolicy,
) -> Result<ImportBatchReport, ApplicationError> {
    let warehouse_id = resolve_import_warehouse(tx, warehouse_code)?;
    let stamp = now_stamp();
    let mut imported_count = 0usize;
    let mut skipped_count = 0usize;
    let mut errors = Vec::new();
    let mut seen_skus = HashSet::new();
    let mut seen_barcodes = HashSet::new();
    for (row_index, row) in rows {
        let mut sku = row.sku.as_deref().unwrap_or("").trim().to_uppercase();
        if sku.is_empty() {
            let mut attempt = 0u32;
            sku = loop {
                let candidate = generate_import_sku(&row.name_fr, *row_index, attempt);
                attempt = attempt.saturating_add(1);
                let taken = row_exists(tx, "SELECT 1 FROM products WHERE sku = ?1", &candidate)?
                    || seen_skus.contains(&candidate);
                if !taken {
                    break candidate;
                }
            };
        }
        if row_exists(tx, "SELECT 1 FROM products WHERE sku = ?1", &sku)?
            || !seen_skus.insert(sku.clone())
        {
            match on_duplicate {
                ImportDuplicatePolicy::Error => {
                    return Err(ApplicationError::Conflict(format!(
                        "SKU already exists: {sku}"
                    )));
                }
                ImportDuplicatePolicy::Skip => {
                    skipped_count += 1;
                    errors.push(ImportRowError {
                        row_index: *row_index,
                        sku: Some(sku),
                        name_fr: row.name_fr.clone(),
                        error: "SKU already exists (skipped)".to_string(),
                    });
                    continue;
                }
            }
        }
        // Barcode guard: an already-assigned barcode is cleared so the
        // product creation still succeeds (UNIQUE lives on product_units).
        let mut barcode = row.barcode.clone();
        if let Some(code) = barcode.clone() {
            let taken = row_exists(tx, "SELECT 1 FROM products WHERE barcode = ?1", &code)?
                || row_exists(tx, "SELECT 1 FROM product_units WHERE barcode = ?1", &code)?
                || !seen_barcodes.insert(code);
            if taken {
                barcode = None;
            }
        }
        // Category guard: unknown ids detach instead of failing the row.
        let category_id = match row.category_id.as_deref() {
            Some(id) if row_exists(tx, "SELECT 1 FROM categories WHERE id = ?1", id)? => {
                Some(id.to_string())
            }
            _ => None,
        };
        let product_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO products
             (id, sku, barcode, name_ar, name_fr, category_id,
              is_kit, is_active, created_at, updated_at, revision, origin_device_id)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, 0, 1, ?6, ?6, 1, ?7)",
            rusqlite::params![
                product_id,
                sku,
                row.name_ar,
                row.name_fr,
                category_id,
                stamp,
                origin_device_id,
            ],
        )
        .map_err(|_| store_failure())?;
        let unit_id = Uuid::new_v4().to_string();
        let unit_name_fr = row
            .base_unit_name_fr
            .clone()
            .unwrap_or_else(|| "Pièce".to_string());
        let unit_name_ar = row
            .base_unit_name_ar
            .clone()
            .unwrap_or_else(|| "قطعة".to_string());
        tx.execute(
            "INSERT INTO product_units
             (id, product_id, unit_name_ar, unit_name_fr, factor_micro, barcode,
              sale_price_centimes, purchase_price_centimes, wholesale_price_centimes,
              is_base_unit, is_default_selling)
             VALUES (?1, ?2, ?3, ?4, 10000, ?5, ?6, ?7, NULL, 1, 1)",
            rusqlite::params![
                unit_id,
                product_id,
                unit_name_ar,
                unit_name_fr,
                barcode,
                row.sale_price_centimes,
                row.purchase_price_centimes,
            ],
        )
        .map_err(|_| store_failure())?;
        // DEP-01 opening stock: upsert the level, then append the audit
        // movement (`ADJUSTMENT_POSITIVE`, mirroring manual INCREASE).
        if row.initial_quantity_micro > 0 {
            let quantity = row.initial_quantity_micro;
            tx.execute(
                "INSERT INTO stock_levels
                 (warehouse_id, product_id, quantity_micro, quantity_base_micro, revision, updated_at)
                 VALUES (?1, ?2, ?3, ?3, 0, ?4)
                 ON CONFLICT(warehouse_id, product_id) DO UPDATE SET
                   quantity_micro = stock_levels.quantity_micro + excluded.quantity_micro,
                   quantity_base_micro = stock_levels.quantity_base_micro + excluded.quantity_base_micro,
                   revision = stock_levels.revision + 1,
                   updated_at = excluded.updated_at",
                rusqlite::params![warehouse_id, product_id, quantity, stamp],
            )
            .map_err(|_| store_failure())?;
            let sequence_no: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM stock_movements",
                    rusqlite::params![],
                    |row| row.get(0),
                )
                .map_err(|_| store_failure())?;
            tx.execute(
                "INSERT INTO stock_movements
                 (id, sequence_no, warehouse_id, product_id, movement_type, unit_id,
                  quantity_micro, quantity_base_micro, reference_type, reference_id,
                  transfer_batch_id, authorized_negative_stock, reason, created_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'ADJUSTMENT_POSITIVE', ?5,
                         ?6, ?6, NULL, NULL, NULL, 0, 'Stock initial (import)', ?7, ?8)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    sequence_no,
                    warehouse_id,
                    product_id,
                    unit_id,
                    quantity,
                    actor_user_id,
                    stamp,
                ],
            )
            .map_err(|_| store_failure())?;
        }
        imported_count += 1;
    }
    Ok(ImportBatchReport {
        total_rows: rows.len(),
        imported_count,
        skipped_count,
        errors,
        duration_ms: 0,
    })
}
