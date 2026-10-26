//! SQLite warehouse repository.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{DeleteAudit, WarehouseRepository};
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId, WarehouseStockPresence};
use rusqlite::Connection;
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_delete_audit_row;
use crate::persistence::transaction::run_transaction;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("warehouse store failure".to_string())
}

/// Warehouse repository over a shared SQLite connection.
pub struct SqliteWarehouseRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteWarehouseRepository {
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

fn map_warehouse(row: &rusqlite::Row) -> Result<Warehouse, rusqlite::Error> {
    let id_text: String = row.get(0)?;
    let code_text: String = row.get(1)?;
    let name: String = row.get(2)?;
    let is_active: bool = row.get::<_, i64>(3)? != 0;
    Ok(Warehouse {
        id: Uuid::parse_str(&id_text)
            .map(WarehouseId::from)
            .map_err(|_| corrupt())?,
        code: WarehouseCode::new(&code_text).map_err(|_| corrupt())?,
        name,
        is_active,
    })
}

fn corrupt() -> rusqlite::Error {
    rusqlite::Error::InvalidColumnName("warehouse row".to_string())
}

impl WarehouseRepository for SqliteWarehouseRepository {
    fn create_warehouse(
        &self,
        warehouse: &Warehouse,
        origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "INSERT INTO warehouses
                 (id, code, name, is_active, revision, origin_device_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?6)",
                rusqlite::params![
                    warehouse.id.to_string(),
                    warehouse.code.as_str(),
                    warehouse.name,
                    i64::from(warehouse.is_active),
                    origin_device_id,
                    stamp,
                ],
            )?;
            Ok(())
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Sqlite(inner) => {
                let text = format!("{inner}");
                if text.contains("warehouses.code") {
                    ApplicationError::Conflict("WAREHOUSE_CODE_EXISTS".to_string())
                } else {
                    store_failure()
                }
            }
            _ => store_failure(),
        })
    }

    fn get_by_id(&self, id: WarehouseId) -> Result<Option<Warehouse>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT id, code, name, is_active FROM warehouses WHERE id = ?1",
                [id.to_string()],
                map_warehouse,
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn get_by_code(&self, code: &WarehouseCode) -> Result<Option<Warehouse>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT id, code, name, is_active FROM warehouses WHERE code = ?1",
                [code.as_str()],
                map_warehouse,
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn list_active(&self) -> Result<Vec<Warehouse>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare("SELECT id, code, name, is_active FROM warehouses WHERE is_active = 1 ORDER BY code")
            .map_err(|_| store_failure())?;
        let warehouses = statement
            .query_map([], map_warehouse)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(warehouses)
    }

    fn update_warehouse(
        &self,
        id: WarehouseId,
        name: Option<&str>,
        is_active: Option<bool>,
    ) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        let exists: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM warehouses WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        if exists == 0 {
            return Err(ApplicationError::NotFound("warehouse".to_string()));
        }
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        if let Some(text) = name {
            guard
                .execute(
                    "UPDATE warehouses SET name = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![text, stamp, id.to_string()],
                )
                .map_err(|_| store_failure())?;
        }
        if let Some(active) = is_active {
            guard
                .execute(
                    "UPDATE warehouses SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![i64::from(active), stamp, id.to_string()],
                )
                .map_err(|_| store_failure())?;
        }
        Ok(())
    }

    fn count_warehouse_references(&self, id: WarehouseId) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let key = id.to_string();
        let movements: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM stock_movements WHERE warehouse_id = ?1",
                [&key],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        let documents: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM documents WHERE warehouse_id = ?1",
                [&key],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        Ok(movements.max(0) as u64 + documents.max(0) as u64)
    }

    fn count_warehouse_nonzero_levels(&self, id: WarehouseId) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let count: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM stock_levels WHERE warehouse_id = ?1 AND quantity_base_micro != 0",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        Ok(count.max(0) as u64)
    }

    fn active_nonzero_level_counts(&self) -> Result<Vec<WarehouseStockPresence>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT w.id, COUNT(s.warehouse_id) \
                 FROM warehouses w \
                 LEFT JOIN stock_levels s \
                   ON s.warehouse_id = w.id AND s.quantity_base_micro != 0 \
                 WHERE w.is_active = 1 \
                 GROUP BY w.id \
                 ORDER BY w.code",
            )
            .map_err(|_| store_failure())?;
        let presence = statement
            .query_map([], |row| {
                let id_text: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((id_text, count))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        presence
            .into_iter()
            .map(|(id_text, count)| {
                let warehouse_id = Uuid::parse_str(&id_text)
                    .map(WarehouseId::from)
                    .map_err(|_| corrupt())
                    .map_err(|_| store_failure())?;
                Ok(WarehouseStockPresence {
                    warehouse_id,
                    nonzero_levels: count.max(0) as u64,
                })
            })
            .collect()
    }

    fn hard_delete_warehouse(
        &self,
        id: WarehouseId,
        audit: &DeleteAudit,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let key = id.to_string();
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            tx.execute("DELETE FROM stock_levels WHERE warehouse_id = ?1", [&key])?;
            let removed = tx.execute("DELETE FROM warehouses WHERE id = ?1", [&key])?;
            if removed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows.into());
            }
            insert_delete_audit_row(tx, "PURGE", "warehouses", &key, audit, &stamp)
        })
        .map_err(|_| store_failure())
    }

    fn set_warehouse_active(
        &self,
        id: WarehouseId,
        active: bool,
        audit: &DeleteAudit,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            let removed = tx.execute(
                "UPDATE warehouses SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![i64::from(active), stamp, id.to_string()],
            )?;
            if removed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows.into());
            }
            insert_delete_audit_row(tx, "DISABLE", "warehouses", &id.to_string(), audit, &stamp)
        })
        .map_err(|_| store_failure())
    }
}
