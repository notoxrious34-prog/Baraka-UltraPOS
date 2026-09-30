//! SQLite read-only reporting repository (TASK-BRK-012).
//!
//! Pure SELECTs over transactional tables: no writes, no idempotency keys,
//! no materialized aggregates. Operational truth stays in the transactional
//! tables; every row returned here is derivable by re-running the query.
//! All money is `i64` centimes with `i128` intermediates; no float exists.
//! Date bounds are ISO `YYYY-MM-DD` days; `created_at`/`timestamp` columns
//! hold fixed-width RFC3339, so `>= from-day` and `< day-after(to)` bound
//! the range exactly (lexicographic compare is chronological here).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CashSessionHistoryRow, CreditOutstandingRow, DailySalesRow, GrossMarginRow, LowStockRow,
    ReportingRepository, SalesByCategoryRow, SalesByProductRow, StockMovementHistoryRow,
    UserActivityRow,
};
use baraka_domain::inventory::StockMovementType;
use baraka_domain::primitives::{Money, QtyMicro};
use rusqlite::Connection;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("reporting store failure".to_string())
}

/// Committed sale-document predicate shared by every sales aggregate.
/// Drafts (parked carts) and purchase documents (BC/BR/FF) never count.
const COMMITTED_SALES: &str = "d.type IN ('TICKET','FACTURE','BL') AND d.status = 'FINALIZED'";

/// Reporting reads over a shared SQLite connection.
pub struct SqliteReportingRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteReportingRepository {
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

/// Day-exclusive upper bound for a `YYYY-MM-DD` range end.
///
/// Fixed-width ISO days compare lexicographically, so the day after `to`
/// is computed by calendar rollover. Returns `None` on malformed input
/// (use cases pre-validate; this is a second defensive gate).
fn day_after(to: &str) -> Option<String> {
    let year: i32 = to[0..4].parse().ok()?;
    let month: i32 = to[5..7].parse().ok()?;
    let day: i32 = to[8..10].parse().ok()?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_len = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    let (year, month, day) = if day < month_len {
        (year, month, day + 1)
    } else if month < 12 {
        (year, month + 1, 1)
    } else {
        (year + 1, 1, 1)
    };
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

impl ReportingRepository for SqliteReportingRepository {
    fn daily_sales(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<DailySalesRow>, ApplicationError> {
        let guard = self.lock()?;
        let end = day_after(date_to).ok_or_else(store_failure)?;
        let mut statement = guard
            .prepare(&format!(
                "SELECT substr(created_at, 1, 10) AS day,
                        SUM(ttc_centimes), SUM(subtotal_ht_centimes),
                        SUM(vat_total_centimes), COUNT(*)
                 FROM documents d
                 WHERE d.created_at >= ?1 AND d.created_at < ?2
                   AND {COMMITTED_SALES}
                 GROUP BY day ORDER BY day ASC"
            ))
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params![date_from, end], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(|(day, ttc, ht, tva, count)| {
                let average = if count > 0 {
                    // Half-away-from-zero over non-negative magnitudes.
                    (i128::from(ttc) + i128::from(count) / 2) / i128::from(count)
                } else {
                    0
                };
                Ok(DailySalesRow {
                    day,
                    total_ttc_centimes: ttc,
                    total_ht_centimes: ht,
                    total_tva_centimes: tva,
                    document_count: count,
                    average_basket_centimes: i64::try_from(average).map_err(|_| store_failure())?,
                })
            })
            .collect()
    }

    fn sales_by_product(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<SalesByProductRow>, ApplicationError> {
        let guard = self.lock()?;
        let end = day_after(date_to).ok_or_else(store_failure)?;
        let mut statement = guard
            .prepare(&format!(
                "SELECT di.product_id, p.sku, p.name_fr,
                        SUM(di.quantity_base_micro), SUM(di.line_ttc_centimes),
                        COUNT(*)
                 FROM document_items di
                 JOIN documents d ON d.id = di.document_id
                 JOIN products p ON p.id = di.product_id
                 WHERE d.created_at >= ?1 AND d.created_at < ?2
                   AND {COMMITTED_SALES}
                 GROUP BY di.product_id
                 ORDER BY SUM(di.line_ttc_centimes) DESC"
            ))
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params![date_from, end], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(
                |(
                    product_id,
                    sku,
                    name_fr,
                    quantity_sold_base_micro,
                    revenue_ttc_centimes,
                    line_count,
                )| {
                    Ok(SalesByProductRow {
                        product_id,
                        sku,
                        name_fr,
                        quantity_sold_base_micro,
                        revenue_ttc_centimes,
                        line_count,
                    })
                },
            )
            .collect()
    }

    fn sales_by_category(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<SalesByCategoryRow>, ApplicationError> {
        let guard = self.lock()?;
        let end = day_after(date_to).ok_or_else(store_failure)?;
        let mut statement = guard
            .prepare(&format!(
                "SELECT p.category_id, c.code, c.name_fr,
                        SUM(di.line_ttc_centimes), COUNT(*)
                 FROM document_items di
                 JOIN documents d ON d.id = di.document_id
                 JOIN products p ON p.id = di.product_id
                 LEFT JOIN categories c ON c.id = p.category_id
                 WHERE d.created_at >= ?1 AND d.created_at < ?2
                   AND {COMMITTED_SALES}
                 GROUP BY p.category_id
                 ORDER BY SUM(di.line_ttc_centimes) DESC"
            ))
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params![date_from, end], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(
                |(category_id, category_code, name_fr, revenue_ttc_centimes, line_count)| {
                    Ok(SalesByCategoryRow {
                        category_id,
                        category_code,
                        name_fr,
                        revenue_ttc_centimes,
                        line_count,
                    })
                },
            )
            .collect()
    }

    fn gross_margin(
        &self,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<GrossMarginRow>, ApplicationError> {
        let guard = self.lock()?;
        let end = day_after(date_to).ok_or_else(store_failure)?;
        // Per-line fetch: cost is applied per line in Rust with the exact
        // engine rounding (Money::checked_mul_qty, half away from zero).
        // cost_price_centimes is NOT NULL on every committed row, so every
        // line contributes — no exclusions, no zero-fill guessing.
        let mut statement = guard
            .prepare(&format!(
                "SELECT di.product_id, p.sku, di.line_ht_centimes,
                        di.cost_price_centimes, di.quantity_micro
                 FROM document_items di
                 JOIN documents d ON d.id = di.document_id
                 JOIN products p ON p.id = di.product_id
                 WHERE d.created_at >= ?1 AND d.created_at < ?2
                   AND {COMMITTED_SALES}
                 ORDER BY di.product_id"
            ))
            .map_err(|_| store_failure())?;
        let lines = statement
            .query_map(rusqlite::params![date_from, end], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        // Aggregate per product preserving first-seen order, then sort by
        // margin descending for the default presentation.
        #[allow(clippy::type_complexity)]
        let mut buckets: HashMap<String, (String, i128, i128, bool)> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for (product_id, sku, line_ht, cost_price, quantity_micro) in lines {
            let cost_total = Money::from_centimes(cost_price)
                .checked_mul_qty(QtyMicro::from_micros(quantity_micro))
                .map_err(|_| store_failure())?
                .centimes();
            let bucket = buckets.entry(product_id.clone()).or_insert_with(|| {
                order.push(product_id.clone());
                (sku, 0, 0, false)
            });
            bucket.1 += i128::from(line_ht);
            bucket.2 += i128::from(cost_total);
            // TASK-BRK-012A: a zero unit cost means untracked cost — the line
            // contributes 0 to cost_total and flags its product bucket.
            if cost_price == 0 {
                bucket.3 = true;
            }
        }
        let mut rows = Vec::with_capacity(order.len());
        for product_id in order {
            let (sku, revenue_ht, cost_total, has_untracked_cost) =
                buckets.remove(&product_id).ok_or_else(store_failure)?;
            let margin = revenue_ht - cost_total;
            rows.push(GrossMarginRow {
                product_id,
                sku,
                revenue_ht_centimes: i64::try_from(revenue_ht).map_err(|_| store_failure())?,
                cost_total_centimes: i64::try_from(cost_total).map_err(|_| store_failure())?,
                margin_centimes: i64::try_from(margin).map_err(|_| store_failure())?,
                has_untracked_cost,
            });
        }
        rows.sort_by_key(|row| std::cmp::Reverse(row.margin_centimes));
        Ok(rows)
    }

    fn credit_outstanding(&self) -> Result<Vec<CreditOutstandingRow>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT id, name, phone, credit_policy, credit_limit_centimes,
                        current_balance_centimes
                 FROM customers
                 WHERE credit_policy != 'DISABLED' AND current_balance_centimes > 0
                 ORDER BY current_balance_centimes DESC",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(
                |(
                    customer_id,
                    name,
                    phone,
                    credit_policy,
                    credit_limit,
                    current_balance_centimes,
                )| {
                    Ok(CreditOutstandingRow {
                        customer_id,
                        name,
                        phone,
                        credit_policy: credit_policy.clone(),
                        credit_limit_centimes: if credit_policy == "UNLIMITED" {
                            None
                        } else {
                            Some(credit_limit)
                        },
                        current_balance_centimes,
                    })
                },
            )
            .collect()
    }

    fn stock_movement_history(
        &self,
        warehouse_id: Option<&str>,
        product_id: Option<&str>,
        movement_type: Option<&str>,
        date_from: &str,
        date_to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StockMovementHistoryRow>, ApplicationError> {
        let guard = self.lock()?;
        let end = day_after(date_to).ok_or_else(store_failure)?;
        let mut statement = guard
            .prepare(
                "SELECT id, movement_type, warehouse_id, product_id,
                        quantity_base_micro, reference_type, reference_id,
                        created_at, created_by
                 FROM stock_movements
                 WHERE created_at >= ?1 AND created_at < ?2
                   AND (?3 IS NULL OR warehouse_id = ?3)
                   AND (?4 IS NULL OR product_id = ?4)
                   AND (?5 IS NULL OR movement_type = ?5)
                 ORDER BY created_at DESC, sequence_no DESC
                 LIMIT ?6 OFFSET ?7",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(
                rusqlite::params![
                    date_from,
                    end,
                    warehouse_id,
                    product_id,
                    movement_type,
                    limit as i64,
                    offset as i64,
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                },
            )
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(
                |(
                    id,
                    movement_code,
                    warehouse,
                    product,
                    quantity_base,
                    reference_type,
                    reference_id,
                    created_at,
                    created_by,
                )| {
                    let signed = StockMovementType::from_code(&movement_code)
                        .map(|kind| kind.signed_delta(quantity_base))
                        .map_err(|_| store_failure())?;
                    let signed = i64::try_from(signed).map_err(|_| store_failure())?;
                    Ok(StockMovementHistoryRow {
                        id,
                        movement_type: movement_code,
                        warehouse_id: warehouse,
                        product_id: product,
                        quantity_base_micro: signed,
                        reference_type,
                        reference_id,
                        created_at,
                        created_by,
                    })
                },
            )
            .collect()
    }

    fn cash_sessions_history(
        &self,
        terminal_id: Option<&str>,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<CashSessionHistoryRow>, ApplicationError> {
        let guard = self.lock()?;
        let end = day_after(date_to).ok_or_else(store_failure)?;
        let mut statement = guard
            .prepare(
                "SELECT id, terminal_id, status, opened_at, closed_at,
                        opening_float_centimes, system_expected_centimes,
                        actual_counted_centimes, variance_centimes
                 FROM cash_sessions
                 WHERE opened_at >= ?1 AND opened_at < ?2
                   AND (?3 IS NULL OR terminal_id = ?3)
                 ORDER BY opened_at DESC",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params![date_from, end, terminal_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(
                |(
                    id,
                    terminal_id,
                    status,
                    opened_at,
                    closed_at,
                    opening_float_centimes,
                    system_expected_centimes,
                    actual_counted_centimes,
                    variance_centimes,
                )| {
                    Ok(CashSessionHistoryRow {
                        id,
                        terminal_id,
                        status,
                        opened_at,
                        closed_at,
                        opening_float_centimes,
                        system_expected_centimes,
                        actual_counted_centimes,
                        variance_centimes,
                    })
                },
            )
            .collect()
    }

    fn low_stock(&self, warehouse_id: Option<&str>) -> Result<Vec<LowStockRow>, ApplicationError> {
        let guard = self.lock()?;
        // ADR-037: low stock IS stockout/depletion (quantity_base_micro <= 0).
        // stock_levels is keyed (warehouse_id, product_id); the scan stays
        // bounded by the level-table size (one row per stocked pair).
        let mut statement = guard
            .prepare(
                "SELECT sl.product_id, p.sku, p.name_fr,
                        sl.warehouse_id, w.code, sl.quantity_base_micro
                 FROM stock_levels sl
                 JOIN products p ON p.id = sl.product_id
                 JOIN warehouses w ON w.id = sl.warehouse_id
                 WHERE sl.quantity_base_micro <= 0
                   AND (?1 IS NULL OR sl.warehouse_id = ?1)
                 ORDER BY sl.quantity_base_micro ASC",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([warehouse_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(
                |(product_id, sku, name_fr, warehouse_id, warehouse_code, quantity_base_micro)| {
                    Ok(LowStockRow {
                        product_id,
                        sku,
                        name_fr,
                        warehouse_id,
                        warehouse_code,
                        quantity_base_micro,
                    })
                },
            )
            .collect()
    }

    fn user_activity(
        &self,
        user_id: Option<&str>,
        action: Option<&str>,
        date_from: &str,
        date_to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<UserActivityRow>, ApplicationError> {
        let guard = self.lock()?;
        let end = day_after(date_to).ok_or_else(store_failure)?;
        // Newest first via the rowid-ordered PRIMARY KEY; the timestamp
        // predicate is served by idx_audit_log_timestamp (migration 0010).
        let mut statement = guard
            .prepare(
                "SELECT sequence_no, timestamp, user_id, action, entity_type,
                        entity_id, old_value_json, new_value_json
                 FROM audit_log
                 WHERE timestamp >= ?1 AND timestamp < ?2
                   AND (?3 IS NULL OR user_id = ?3)
                   AND (?4 IS NULL OR action = ?4)
                 ORDER BY sequence_no DESC
                 LIMIT ?5 OFFSET ?6",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(
                rusqlite::params![date_from, end, user_id, action, limit as i64, offset as i64,],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                    ))
                },
            )
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(
                |(
                    sequence_no,
                    timestamp,
                    user_id,
                    action,
                    entity_type,
                    entity_id,
                    old_value_json,
                    new_value_json,
                )| {
                    Ok(UserActivityRow {
                        sequence_no,
                        timestamp,
                        user_id,
                        action,
                        entity_type,
                        entity_id,
                        old_value_json,
                        new_value_json,
                    })
                },
            )
            .collect()
    }
}
