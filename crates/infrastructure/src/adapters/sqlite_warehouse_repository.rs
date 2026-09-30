//! SQLite warehouse repository.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::WarehouseRepository;
use baraka_domain::inventory::{Warehouse, WarehouseCode, WarehouseId};
use rusqlite::Connection;
use uuid::Uuid;

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
}
