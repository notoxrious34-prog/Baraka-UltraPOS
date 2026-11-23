//! SQLite signal-economy reads (TASK-BRK-116, Wave 5).
//!
//! Pure SELECTs over settled truth: committed sale lines, summed stock,
//! open ledger entries. No writes, no thresholds — the use case pairs
//! these rows with the domain detectors. Money is `i64` centimes; per-line
//! cost uses the exact engine rounding (`Money::checked_mul_qty`), mirroring
//! the reporting repository. Windows ride SQLite `date('now', ...)` so the
//! port stays clock-free; `created_at` is fixed-width RFC3339 UTC.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CellDemandRow, CreditExposureRow, DailyUnitsRow, ModelEconomicsRow, ProductRecencyRow,
    SignalRepository,
};
use baraka_domain::primitives::{Money, QtyMicro};
use rusqlite::Connection;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("signal store failure".to_string())
}

/// Committed sale-document predicate (TICKET/FACTURE/BL, FINALIZED only).
const COMMITTED_SALES: &str = "d.type IN ('TICKET','FACTURE','BL') AND d.status = 'FINALIZED'";

/// Signal reads over a shared SQLite connection.
pub struct SqliteSignalRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteSignalRepository {
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

impl SignalRepository for SqliteSignalRepository {
    fn cell_demand(&self, window_days: u32) -> Result<Vec<CellDemandRow>, ApplicationError> {
        let window_days = window_days.clamp(1, 365);
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(&format!(
                "SELECT c.model_id, c.axis1_value, c.axis2_value, c.product_id,
                        COALESCE((SELECT SUM(s.quantity_base_micro) FROM stock_levels s
                                  WHERE s.product_id = c.product_id), 0),
                        COALESCE((SELECT SUM(di.quantity_base_micro) / 10000
                                  FROM document_items di
                                  JOIN documents d ON d.id = di.document_id
                                  WHERE di.product_id = c.product_id
                                    AND d.created_at >= date('now', '-' || ?1 || ' days')
                                    AND {COMMITTED_SALES}), 0)
                 FROM variant_cells c
                 JOIN variant_models m ON m.id = c.model_id
                 ORDER BY c.model_id, c.axis1_sort, c.axis2_sort"
            ))
            .map_err(|_| store_failure())?;
        let flat = statement
            .query_map([i64::from(window_days)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        // Per-model enumeration: the cockpit index is stable inside a model.
        let mut out = Vec::with_capacity(flat.len());
        let mut last_model = String::new();
        let mut index = 0usize;
        for (model_id, axis1, axis2, product_id, qty, sold) in flat {
            if model_id != last_model {
                last_model = model_id.clone();
                index = 0;
            }
            out.push(CellDemandRow {
                model_id,
                cell_index: index,
                axis1_value: axis1,
                axis2_value: axis2,
                product_id,
                qty_on_hand_micro: qty,
                units_sold_window: u64::try_from(sold.max(0)).map_err(|_| store_failure())?,
            });
            index += 1;
        }
        Ok(out)
    }

    fn product_recency(&self) -> Result<Vec<ProductRecencyRow>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT p.id,
                        (SELECT SUM(s.quantity_base_micro) FROM stock_levels s
                         WHERE s.product_id = p.id),
                        (SELECT MAX(m.created_at) FROM stock_movements m
                         WHERE m.product_id = p.id)
                 FROM products p
                 WHERE (SELECT COALESCE(SUM(s2.quantity_base_micro), 0) FROM stock_levels s2
                        WHERE s2.product_id = p.id) > 0",
            )
            .map_err(|_| store_failure())?;
        let today = chrono::Utc::now().date_naive();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(|(product_id, qty, last)| {
                let days = match last {
                    None => None,
                    Some(stamp) => {
                        let day = stamp
                            .get(0..10)
                            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
                        day.map(|d| (today - d).num_days().max(0) as u32)
                    }
                };
                Ok(ProductRecencyRow {
                    product_id,
                    qty_on_hand_micro: qty.unwrap_or(0),
                    days_since_last_movement: days,
                })
            })
            .collect()
    }

    fn model_economics(
        &self,
        window_days: u32,
    ) -> Result<Vec<ModelEconomicsRow>, ApplicationError> {
        let window_days = window_days.clamp(1, 365);
        let guard = self.lock()?;
        // Per-line fetch: cost rides the exact engine rounding per line
        // (same as the reporting gross-margin query), then sums per model.
        let mut statement = guard
            .prepare(&format!(
                "SELECT c.model_id, di.line_ht_centimes,
                        di.cost_price_centimes, di.quantity_micro
                 FROM document_items di
                 JOIN documents d ON d.id = di.document_id
                 JOIN variant_cells c ON c.product_id = di.product_id
                 WHERE d.created_at >= date('now', '-' || ?1 || ' days')
                   AND {COMMITTED_SALES}
                 ORDER BY c.model_id"
            ))
            .map_err(|_| store_failure())?;
        let lines = statement
            .query_map([i64::from(window_days)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        let mut revenue: HashMap<String, i128> = HashMap::new();
        let mut cogs: HashMap<String, i128> = HashMap::new();
        for (model_id, line_ht, cost_price, quantity_micro) in lines {
            let cost_total = Money::from_centimes(cost_price)
                .checked_mul_qty(QtyMicro::from_micros(quantity_micro))
                .map_err(|_| store_failure())?
                .centimes();
            *revenue.entry(model_id.clone()).or_insert(0) += i128::from(line_ht);
            *cogs.entry(model_id).or_insert(0) += i128::from(cost_total);
        }
        let mut out = Vec::with_capacity(revenue.len());
        for (model_id, rev) in revenue {
            out.push(ModelEconomicsRow {
                model_id: model_id.clone(),
                revenue_centimes: i64::try_from(rev).map_err(|_| store_failure())?,
                cogs_centimes: i64::try_from(cogs.remove(&model_id).unwrap_or(0))
                    .map_err(|_| store_failure())?,
            });
        }
        out.sort_by(|a, b| a.model_id.cmp(&b.model_id));
        Ok(out)
    }

    fn credit_exposure_entries(
        &self,
        limit: usize,
    ) -> Result<Vec<CreditExposureRow>, ApplicationError> {
        let limit = limit.clamp(1, 100_000) as i64;
        let guard = self.lock()?;
        // Entries of currently-indebted customers only (advances excluded by
        // the positive-balance join); signed amounts keep the total exact.
        let mut statement = guard
            .prepare(
                "SELECT l.customer_id, l.amount_centimes,
                        CAST(julianday('now') - julianday(l.created_at) AS INTEGER)
                 FROM customer_ledger l
                 JOIN customers c ON c.id = l.customer_id
                 WHERE c.current_balance_centimes > 0
                 ORDER BY l.created_at ASC
                 LIMIT ?1",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(|(customer_id, amount, age)| {
                Ok(CreditExposureRow {
                    customer_id,
                    balance_centimes: amount,
                    age_days: u32::try_from(age.max(0)).map_err(|_| store_failure())?,
                })
            })
            .collect()
    }

    fn daily_units(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<DailyUnitsRow>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(&format!(
                "SELECT substr(d.created_at, 1, 10),
                        COALESCE(SUM(di.quantity_base_micro) / 10000, 0)
                 FROM document_items di
                 JOIN documents d ON d.id = di.document_id
                 WHERE d.created_at >= ?1 AND d.created_at < ?2
                   AND {COMMITTED_SALES}
                 GROUP BY substr(d.created_at, 1, 10)
                 ORDER BY 1"
            ))
            .map_err(|_| store_failure())?;
        // Reuse the reporting day-exclusive upper bound convention.
        let end =
            super::sqlite_reporting_repository::day_after(date_to).ok_or_else(store_failure)?;
        let rows = statement
            .query_map(rusqlite::params![date_from, end], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(|(day, units)| {
                Ok(DailyUnitsRow {
                    day,
                    base_units: u64::try_from(units.max(0)).map_err(|_| store_failure())?,
                })
            })
            .collect()
    }
}
