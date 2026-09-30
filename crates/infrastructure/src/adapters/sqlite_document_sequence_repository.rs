//! SQLite gapless sequence repository.
//!
//! Standalone allocation (own transaction) for admin/repair use. The atomic
//! checkout allocates inside its own transaction for gapless safety; that
//! path lives in the checkout engine, not here.

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::DocumentSequenceRepository;
use baraka_domain::errors::DomainError;
use rusqlite::Connection;

use crate::adapters::sqlite_stock_repository::sentinel;
use crate::adapters::sqlite_stock_repository::{map_engine_error, tx_failure};
use crate::persistence::transaction::run_transaction;

/// Gapless sequence allocator over a shared SQLite connection.
pub struct SqliteDocumentSequenceRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteDocumentSequenceRepository {
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

/// Allocate inside an explicit transaction handle (shared with checkout).
pub fn allocate_in_tx(
    tx: &rusqlite::Transaction,
    doc_type_code: &str,
    fiscal_year: i32,
) -> Result<i64, crate::persistence::InfrastructureError> {
    let current: Option<i64> = tx
        .query_row(
            "SELECT last_sequence FROM document_sequences WHERE type = ?1 AND fiscal_year = ?2",
            rusqlite::params![doc_type_code, fiscal_year],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    match current {
        None => {
            tx.execute(
                "INSERT INTO document_sequences (type, fiscal_year, last_sequence)
                 VALUES (?1, ?2, 1)",
                rusqlite::params![doc_type_code, fiscal_year],
            )?;
            Ok(1)
        }
        Some(last) => {
            let next = last + 1;
            if next > 999_999 {
                return Err(tx_failure(sentinel::SEQUENCE_OVERFLOW.to_string()));
            }
            tx.execute(
                "UPDATE document_sequences SET last_sequence = ?1
                 WHERE type = ?2 AND fiscal_year = ?3",
                rusqlite::params![next, doc_type_code, fiscal_year],
            )?;
            Ok(next)
        }
    }
}

impl DocumentSequenceRepository for SqliteDocumentSequenceRepository {
    fn allocate_next(
        &self,
        doc_type_code: &str,
        fiscal_year: i32,
    ) -> Result<i64, ApplicationError> {
        let mut guard = self.lock()?;
        run_transaction(&mut guard, |tx| {
            allocate_in_tx(tx, doc_type_code, fiscal_year)
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Migration(message)
                if message == sentinel::SEQUENCE_OVERFLOW =>
            {
                ApplicationError::Validation(DomainError::GaplessSequenceOverflow)
            }
            other => map_engine_error(other),
        })
    }
}
