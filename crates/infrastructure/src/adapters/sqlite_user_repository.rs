//! SQLite-backed user repository (lockout writes via `run_transaction`).

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{BootstrapFirstAdminCommand, UserRepository};
use baraka_domain::role::Role;
use baraka_domain::user::{User, UserId, Username};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_cash_audit_row;
use crate::adapters::sqlite_stock_repository::{map_engine_error, tx_failure};
use crate::persistence::transaction::run_transaction;

/// User repository over a shared SQLite connection.
pub struct SqliteUserRepository {
    db: Arc<Mutex<Connection>>,
}

/// Fixed, non-leaking mapping for every SQLite failure in this repository.
///
/// The message is deliberately static: rusqlite error text can echo SQL
/// fragments, and no storage detail may ever reach authentication callers.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("user store failure".to_string())
}

impl SqliteUserRepository {
    /// Wrap the shared connection (writes run inside transactions).
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApplicationError> {
        self.db
            .lock()
            .map_err(|_| ApplicationError::Port("database lock poisoned".to_string()))
    }

    fn map_row(row: &rusqlite::Row) -> Result<User, ApplicationError> {
        let id_text: String = row.get(0).map_err(|_| store_failure())?;
        let id = Uuid::parse_str(&id_text)
            .map(UserId::from)
            .map_err(|_| ApplicationError::Port("stored user id unreadable".to_string()))?;
        let username_text: String = row.get(1).map_err(|_| store_failure())?;
        let username = Username::new(&username_text)
            .map_err(|_| ApplicationError::Port("stored username unreadable".to_string()))?;
        let role_text: String = row.get(2).map_err(|_| store_failure())?;
        let role = Role::from_code(&role_text)
            .map_err(|_| ApplicationError::Port("stored role unreadable".to_string()))?;
        let is_active: bool = row.get::<_, i64>(3).map_err(|_| store_failure())? != 0;
        let attempts_raw: i64 = row.get(4).map_err(|_| store_failure())?;
        let failed_login_attempts = u8::try_from(attempts_raw).unwrap_or(u8::MAX);
        let locked_text: Option<String> = row.get(5).map_err(|_| store_failure())?;
        let locked_until = locked_text
            .map(|text| {
                DateTime::parse_from_rfc3339(&text)
                    .map(|fixed| fixed.with_timezone(&Utc))
                    .map_err(|_| {
                        ApplicationError::Port("stored lock timestamp unreadable".to_string())
                    })
            })
            .transpose()?;
        Ok(User {
            id,
            username,
            role,
            is_active,
            failed_login_attempts,
            locked_until,
        })
    }
}

impl UserRepository for SqliteUserRepository {
    fn find_by_username(&self, username: &str) -> Result<Option<User>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT id, username, role, is_active, failed_login_attempts, locked_until
             FROM users WHERE username = ?1",
            )
            .map_err(|_| store_failure())?;
        let mut rows = statement.query([username]).map_err(|_| store_failure())?;
        match rows.next().map_err(|_| store_failure())? {
            Some(row) => Ok(Some(Self::map_row(row)?)),
            None => Ok(None),
        }
    }

    fn record_login_failure(
        &self,
        user_id: UserId,
        locked_until: Option<DateTime<Utc>>,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let locked_text =
            locked_until.map(|instant| instant.to_rfc3339_opts(SecondsFormat::Millis, true));
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "UPDATE users
                 SET failed_login_attempts = failed_login_attempts + 1, locked_until = ?1
                 WHERE id = ?2",
                rusqlite::params![locked_text, user_id.as_uuid().to_string()],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn reset_login_failures(&self, user_id: UserId) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "UPDATE users
                 SET failed_login_attempts = 0, locked_until = NULL
                 WHERE id = ?1",
                rusqlite::params![user_id.as_uuid().to_string()],
            )?;
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn get_password_hash(&self, user_id: UserId) -> Result<String, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                [user_id.as_uuid().to_string()],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    ApplicationError::NotFound("user".to_string())
                }
                _ => store_failure(),
            })
    }

    fn get_pin_hash(&self, user_id: UserId) -> Result<Option<String>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT pin_hash FROM users WHERE id = ?1",
                [user_id.as_uuid().to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    ApplicationError::NotFound("user".to_string())
                }
                _ => store_failure(),
            })
    }

    fn count_users(&self) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let count: i64 = guard
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
            .map_err(|_| store_failure())?;
        u64::try_from(count).map_err(|_| store_failure())
    }

    fn bootstrap_first_admin(
        &self,
        command: &BootstrapFirstAdminCommand,
    ) -> Result<UserId, ApplicationError> {
        let mut guard = self.lock()?;
        run_transaction(&mut guard, |tx| {
            // In-transaction re-check: closes the check-then-act race window.
            // Two simultaneous first-run attempts serialize on BEGIN
            // IMMEDIATE; the loser observes a non-empty table and is
            // rejected without inserting anything.
            let count: i64 = tx.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
            if count != 0 {
                return Err(tx_failure("BOOTSTRAP_ALREADY_COMPLETED".to_string()));
            }
            let user_id_text = command.user_id.as_uuid().to_string();
            tx.execute(
                "INSERT INTO users
                 (id, username, password_hash, pin_hash, role,
                  is_active, created_at, updated_at,
                  failed_login_attempts, locked_until)
                 VALUES (?1, ?2, ?3, NULL, ?4, 1, ?5, ?5, 0, NULL)",
                rusqlite::params![
                    user_id_text,
                    command.username,
                    command.password_hash,
                    Role::Admin.code(),
                    command.created_at,
                ],
            )?;
            let new_json = serde_json::json!({
                "username": command.username,
                "role": Role::Admin.code(),
                "origin_device_id": command.terminal_id,
            })
            .to_string();
            insert_cash_audit_row(
                tx,
                "FIRST_ADMIN_BOOTSTRAP",
                "users",
                &user_id_text,
                None,
                &new_json,
                &user_id_text,
                &command.created_at,
            )?;
            Ok(command.user_id)
        })
        .map_err(map_engine_error)
    }
}
