//! SQLite supplier master-data and statement repository (TASK-BRK-011).
//!
//! Mirrors the established customer-repository pattern exactly, including
//! the escaped-LIKE multilingual search and the static non-leaking failure
//! mapping. `origin_device_id` is always supplied by the caller (Terminal
//! context per ADR-028); no placeholder literal exists in this module.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    DeleteAudit, NewSupplier, SupplierLedgerRow, SupplierRepository, SupplierStatement,
    SupplierUpdate, SupplierView,
};
use baraka_domain::errors::DomainError;
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_delete_audit_row;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("supplier store failure".to_string())
}

/// Supplier repository over a shared SQLite connection.
pub struct SqliteSupplierRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteSupplierRepository {
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

const SUPPLIER_COLUMNS: &str = "id, name, phone, address, nif, rc, notes, is_active,
                                current_balance_centimes, origin_device_id,
                                created_at, updated_at";

fn map_supplier(row: &rusqlite::Row<'_>) -> Result<SupplierView, rusqlite::Error> {
    Ok(SupplierView {
        id: row.get(0)?,
        name: row.get(1)?,
        phone: row.get(2)?,
        address: row.get(3)?,
        nif: row.get(4)?,
        rc: row.get(5)?,
        notes: row.get(6)?,
        is_active: row.get::<_, i64>(7)? != 0,
        current_balance_centimes: row.get(8)?,
        origin_device_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn fetch_supplier(
    conn: &Connection,
    supplier_id: &str,
) -> Result<Option<SupplierView>, ApplicationError> {
    conn.query_row(
        &format!("SELECT {SUPPLIER_COLUMNS} FROM suppliers WHERE id = ?1"),
        [supplier_id],
        map_supplier,
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
    .map_err(|_| store_failure())
}

impl SupplierRepository for SqliteSupplierRepository {
    fn create_supplier(
        &self,
        supplier: &NewSupplier,
        origin_device_id: &str,
    ) -> Result<SupplierView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let id = Uuid::new_v4().to_string();
        let origin = origin_device_id.trim().to_string();
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "INSERT INTO suppliers
                 (id, name, phone, address, nif, rc, is_active,
                  current_balance_centimes, notes, origin_device_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 0, ?7, ?8, ?9, ?9)",
                rusqlite::params![
                    id,
                    supplier.name,
                    supplier.phone,
                    supplier.address,
                    supplier.nif,
                    supplier.rc,
                    supplier.notes,
                    origin,
                    stamp,
                ],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())?;
        fetch_supplier(&guard, &id)?.ok_or_else(store_failure)
    }

    fn update_supplier(
        &self,
        supplier_id: &str,
        update: &SupplierUpdate,
    ) -> Result<SupplierView, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        // Positional parameters track the SET list in order (customer pattern).
        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut params: Vec<String> = vec![stamp];
        if let Some(name) = update.name.as_deref() {
            sets.push(format!("name = ?{}", params.len() + 1));
            params.push(name.to_string());
        }
        match update.phone.as_ref() {
            Some(Some(phone)) => {
                sets.push(format!("phone = ?{}", params.len() + 1));
                params.push(phone.clone());
            }
            Some(None) => sets.push("phone = NULL".to_string()),
            None => {}
        }
        match update.address.as_ref() {
            Some(Some(address)) => {
                sets.push(format!("address = ?{}", params.len() + 1));
                params.push(address.clone());
            }
            Some(None) => sets.push("address = NULL".to_string()),
            None => {}
        }
        match update.nif.as_ref() {
            Some(Some(nif)) => {
                sets.push(format!("nif = ?{}", params.len() + 1));
                params.push(nif.clone());
            }
            Some(None) => sets.push("nif = NULL".to_string()),
            None => {}
        }
        match update.rc.as_ref() {
            Some(Some(rc)) => {
                sets.push(format!("rc = ?{}", params.len() + 1));
                params.push(rc.clone());
            }
            Some(None) => sets.push("rc = NULL".to_string()),
            None => {}
        }
        match update.notes.as_ref() {
            Some(Some(notes)) => {
                sets.push(format!("notes = ?{}", params.len() + 1));
                params.push(notes.clone());
            }
            Some(None) => sets.push("notes = NULL".to_string()),
            None => {}
        }
        if let Some(active) = update.is_active {
            sets.push(format!("is_active = ?{}", params.len() + 1));
            params.push(i64::from(active).to_string());
        }
        let sql = format!(
            "UPDATE suppliers SET {} WHERE id = ?{}",
            sets.join(", "),
            params.len() + 1
        );
        params.push(supplier_id.to_string());
        run_transaction(&mut guard, |tx| {
            let mut statement = tx.prepare(&sql)?;
            let changed = statement.execute(rusqlite::params_from_iter(params.iter()))?;
            if changed != 1 {
                return Err(InfrastructureError::Migration(
                    "SUPPLIER_MISSING".to_string(),
                ));
            }
            Ok(())
        })
        .map_err(|error| match error {
            InfrastructureError::Migration(message) if message == "SUPPLIER_MISSING" => {
                ApplicationError::Validation(DomainError::SupplierNotFound)
            }
            _ => store_failure(),
        })?;
        fetch_supplier(&guard, supplier_id)?
            .ok_or(ApplicationError::Validation(DomainError::SupplierNotFound))
    }

    fn get_supplier(&self, supplier_id: &str) -> Result<Option<SupplierView>, ApplicationError> {
        let guard = self.lock()?;
        fetch_supplier(&guard, supplier_id)
    }

    fn search_suppliers(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SupplierView>, ApplicationError> {
        let guard = self.lock()?;
        // Single normalization point: trim once here; SQLite LIKE is
        // case-insensitive for ASCII (Cairo: no case), so one LIKE shape
        // serves both languages with no divergent helpers.
        let trimmed = query.trim();
        let pattern = format!(
            "%{}%",
            trimmed
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let capped = (limit.clamp(1, 100)) as i64;
        let mut statement = guard
            .prepare(&format!(
                "SELECT {SUPPLIER_COLUMNS} FROM suppliers
                     WHERE name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                        OR phone LIKE ?1 ESCAPE '\\'
                        OR id LIKE ?1 ESCAPE '\\'
                     ORDER BY name LIMIT ?2"
            ))
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params![pattern, capped], map_supplier)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(rows)
    }

    fn list_suppliers(&self, active_only: bool) -> Result<Vec<SupplierView>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(&format!(
                "SELECT {SUPPLIER_COLUMNS} FROM suppliers
                 WHERE (?1 = 0 OR is_active = 1) ORDER BY name"
            ))
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([i64::from(active_only)], map_supplier)
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        Ok(rows)
    }

    fn get_statement(
        &self,
        supplier_id: &str,
    ) -> Result<Option<SupplierStatement>, ApplicationError> {
        let guard = self.lock()?;
        let supplier = fetch_supplier(&guard, supplier_id)?;
        let Some(supplier) = supplier else {
            return Ok(None);
        };
        let mut statement = guard
            .prepare(
                "SELECT id, entry_type, amount_centimes, reference_type, reference_id,
                        description, created_at, created_by
                 FROM supplier_ledger
                 WHERE supplier_id = ?1
                 ORDER BY created_at ASC, rowid ASC",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([supplier_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        // Server-side running balance in query order: DEBIT adds, CREDIT subtracts.
        let mut running: i128 = 0;
        let mut entries = Vec::with_capacity(rows.len());
        for (
            id,
            entry_type,
            amount,
            reference_type,
            reference_id,
            description,
            created_at,
            created_by,
        ) in rows
        {
            let delta = match entry_type.as_str() {
                "DEBIT" => i128::from(amount),
                "CREDIT" => -i128::from(amount),
                _ => return Err(store_failure()),
            };
            running = running.checked_add(delta).ok_or_else(store_failure)?;
            let balance_after = i64::try_from(running).map_err(|_| store_failure())?;
            entries.push(SupplierLedgerRow {
                id,
                entry_type,
                amount_centimes: amount,
                reference_type,
                reference_id,
                description,
                created_at,
                created_by,
                balance_after_centimes: balance_after,
            });
        }
        Ok(Some(SupplierStatement {
            entries,
            current_balance_centimes: supplier.current_balance_centimes,
        }))
    }

    fn count_supplier_references(&self, supplier_id: &str) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let documents: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM documents WHERE party_type = 'SUPPLIER' AND party_id = ?1",
                [supplier_id],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        let ledger: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM supplier_ledger WHERE supplier_id = ?1",
                [supplier_id],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        Ok(documents.max(0) as u64 + ledger.max(0) as u64)
    }

    fn hard_delete_supplier(
        &self,
        supplier_id: &str,
        audit: &DeleteAudit,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            let removed = tx.execute("DELETE FROM suppliers WHERE id = ?1", [supplier_id])?;
            if removed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows.into());
            }
            insert_delete_audit_row(tx, "PURGE", "suppliers", supplier_id, audit, &stamp)
        })
        .map_err(|_| store_failure())
    }

    fn set_supplier_active(
        &self,
        supplier_id: &str,
        active: bool,
        audit: &DeleteAudit,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            let removed = tx.execute(
                "UPDATE suppliers SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![i64::from(active), stamp, supplier_id],
            )?;
            if removed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows.into());
            }
            insert_delete_audit_row(tx, "DISABLE", "suppliers", supplier_id, audit, &stamp)
        })
        .map_err(|_| store_failure())
    }
}
