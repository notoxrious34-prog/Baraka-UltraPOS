//! SQLite customer credit repository.

use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CustomerRepository, CustomerSummary, CustomerUpdate, LedgerEntryRow, PaymentRecord,
};
use baraka_domain::errors::DomainError;
use baraka_domain::printing::{
    format_centimes_plain, DebtReceiptDocument, ReceiptLanguage, StoreIdentity,
};
use baraka_domain::sales::{CreditPolicy, CreditProfile, Customer, CustomerId, CustomerTier};
use baraka_domain::user::UserId;
use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::persistence::transaction::run_transaction;

/// Parse a stored tier code; unknown or NULL values fall back to `Retail`
/// so pre-0015 rows and loose operator edits never break hydration.
fn parse_tier(raw: Option<String>) -> CustomerTier {
    raw.as_deref()
        .map(|code| CustomerTier::from_str(code).unwrap_or_default())
        .unwrap_or_default()
}

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("customer store failure".to_string())
}

/// Read one `settings` value inside a transaction; absent keys (or an
/// uninitialized store identity) read as empty strings (graceful fallback).
fn read_setting_in_tx(tx: &rusqlite::Transaction, key: &str) -> String {
    tx.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .unwrap_or_default()
}

/// Customer credit persistence over a shared SQLite connection.
pub struct SqliteCustomerRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteCustomerRepository {
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

impl CustomerRepository for SqliteCustomerRepository {
    fn get_credit_profile(
        &self,
        customer_id: CustomerId,
    ) -> Result<Option<CreditProfile>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT credit_policy, credit_limit_centimes, current_balance_centimes, is_active
                 FROM customers WHERE id = ?1",
                [customer_id.to_string()],
                |row| {
                    let policy_text: String = row.get(0)?;
                    let limit: i64 = row.get(1)?;
                    let balance: i64 = row.get(2)?;
                    let active: bool = row.get::<_, i64>(3)? != 0;
                    Ok(CreditProfile {
                        customer_id,
                        policy: CreditPolicy::from_code(&policy_text).map_err(|_| {
                            rusqlite::Error::InvalidColumnName("credit_policy".to_string())
                        })?,
                        credit_limit_centimes: limit,
                        current_balance_centimes: balance,
                        is_active: active,
                    })
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn record_credit_debit(
        &self,
        customer_id: CustomerId,
        amount_centimes: i64,
        running_balance_centimes: i64,
        document_id: baraka_domain::sales::DocumentId,
        actor: UserId,
    ) -> Result<String, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            let sequence: i64 = tx.query_row(
                "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM customer_ledger",
                [],
                |row| row.get(0),
            )?;
            let entry_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO customer_ledger
                 (id, sequence_no, customer_id, entry_type, amount_centimes,
                  running_balance_centimes, document_type, document_id,
                  created_by, created_at)
                 VALUES (?1, ?2, ?3, 'DEBIT', ?4, ?5, 'SALE', ?6, ?7, ?8)",
                rusqlite::params![
                    entry_id,
                    sequence,
                    customer_id.to_string(),
                    amount_centimes,
                    running_balance_centimes,
                    document_id.to_string(),
                    actor.to_string(),
                    stamp,
                ],
            )?;
            Ok(entry_id)
        })
        .map_err(|_| store_failure())
    }

    fn update_cached_balance(
        &self,
        customer_id: CustomerId,
        new_balance_centimes: i64,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            let changed = tx.execute(
                "UPDATE customers
                 SET current_balance_centimes = ?1, revision = revision + 1, updated_at = ?2
                 WHERE id = ?3",
                rusqlite::params![new_balance_centimes, stamp, customer_id.to_string()],
            )?;
            if changed != 1 {
                return Err(crate::persistence::InfrastructureError::Migration(
                    "CUSTOMER_MISSING".to_string(),
                ));
            }
            Ok(())
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Migration(message)
                if message == "CUSTOMER_MISSING" =>
            {
                ApplicationError::NotFound("customer".to_string())
            }
            _ => store_failure(),
        })
    }

    fn create_customer(
        &self,
        customer: &Customer,
        origin_device_id: &str,
    ) -> Result<CustomerId, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        // Code is operator-meaningful and unique; derive a stable default.
        let code = format!("C-{}", customer.id.to_string()[..8].to_uppercase());
        run_transaction(&mut guard, |tx| {
            tx.execute(
                "INSERT INTO customers
                 (id, code, name, phone, address, credit_policy,
                  credit_limit_centimes, current_balance_centimes, is_active,
                  customer_tier,
                  created_at, updated_at, revision, origin_device_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?10, 1, ?11)",
                rusqlite::params![
                    customer.id.to_string(),
                    code,
                    customer.name,
                    customer.phone,
                    customer.address,
                    customer.credit_policy.code(),
                    customer.credit_limit_centimes,
                    i64::from(customer.is_active),
                    customer.customer_tier.as_str(),
                    stamp,
                    origin_device_id,
                ],
            )?;
            Ok(customer.id)
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Sqlite(ref inner)
                if format!("{inner}").contains("customers.phone") =>
            {
                ApplicationError::DuplicatePhone
            }
            _ => store_failure(),
        })
    }

    fn update_customer(
        &self,
        customer_id: CustomerId,
        update: &CustomerUpdate,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        // Positional parameters track the SET list in order.
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
        if let Some(policy) = update.credit_policy {
            sets.push(format!("credit_policy = ?{}", params.len() + 1));
            params.push(policy.code().to_string());
        }
        if let Some(limit) = update.credit_limit_centimes {
            sets.push(format!("credit_limit_centimes = ?{}", params.len() + 1));
            params.push(limit.to_string());
        }
        if let Some(active) = update.is_active {
            sets.push(format!("is_active = ?{}", params.len() + 1));
            params.push(i64::from(active).to_string());
        }
        if let Some(tier) = update.customer_tier {
            sets.push(format!("customer_tier = ?{}", params.len() + 1));
            params.push(tier.as_str().to_string());
        }
        sets.push("revision = revision + 1".to_string());
        let sql = format!(
            "UPDATE customers SET {} WHERE id = ?{}",
            sets.join(", "),
            params.len() + 1
        );
        params.push(customer_id.to_string());
        run_transaction(&mut guard, |tx| {
            let mut statement = tx.prepare(&sql)?;
            let changed = statement.execute(rusqlite::params_from_iter(params.iter()))?;
            if changed != 1 {
                return Err(crate::persistence::InfrastructureError::Migration(
                    "CUSTOMER_MISSING".to_string(),
                ));
            }
            Ok(())
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Migration(message)
                if message == "CUSTOMER_MISSING" =>
            {
                ApplicationError::CustomerNotFound
            }
            crate::persistence::InfrastructureError::Sqlite(ref inner)
                if format!("{inner}").contains("customers.phone") =>
            {
                ApplicationError::DuplicatePhone
            }
            _ => store_failure(),
        })
    }

    fn search_customers(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CustomerSummary>, ApplicationError> {
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
            .prepare(
                "SELECT id, name, phone, credit_policy, customer_tier FROM customers
                 WHERE name LIKE ?1 ESCAPE '\\' COLLATE NOCASE
                    OR phone LIKE ?1 ESCAPE '\\'
                    OR code LIKE ?1 ESCAPE '\\'
                    OR id LIKE ?1 ESCAPE '\\'
                 ORDER BY name LIMIT ?2",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(rusqlite::params![pattern, capped], |row| {
                let id_text: String = row.get(0)?;
                let name: String = row.get(1)?;
                let phone: Option<String> = row.get(2)?;
                let policy_text: String = row.get(3)?;
                let tier_text: Option<String> = row.get(4)?;
                Ok((id_text, name, phone, policy_text, tier_text))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(|(id_text, name, phone, policy_text, tier_text)| {
                Ok(CustomerSummary {
                    id: Uuid::parse_str(&id_text)
                        .map(CustomerId::from)
                        .map_err(|_| store_failure())?,
                    name,
                    phone,
                    credit_policy: CreditPolicy::from_code(&policy_text)
                        .map_err(|_| store_failure())?,
                    customer_tier: parse_tier(tier_text),
                })
            })
            .collect()
    }

    fn get_customer(&self, customer_id: CustomerId) -> Result<Option<Customer>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT id, name, phone, address, credit_policy, credit_limit_centimes,
                        customer_tier, is_active
                 FROM customers WHERE id = ?1",
                [customer_id.to_string()],
                |row| {
                    let id_text: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let phone: Option<String> = row.get(2)?;
                    let address: Option<String> = row.get(3)?;
                    let policy_text: String = row.get(4)?;
                    let limit: i64 = row.get(5)?;
                    let tier_text: Option<String> = row.get(6)?;
                    let active: bool = row.get::<_, i64>(7)? != 0;
                    Ok(Customer {
                        id: Uuid::parse_str(&id_text)
                            .map(CustomerId::from)
                            .map_err(|_| {
                                rusqlite::Error::InvalidColumnName("customer id".to_string())
                            })?,
                        name,
                        phone,
                        address,
                        credit_policy: CreditPolicy::from_code(&policy_text).map_err(|_| {
                            rusqlite::Error::InvalidColumnName("credit_policy".to_string())
                        })?,
                        credit_limit_centimes: limit,
                        customer_tier: parse_tier(tier_text),
                        is_active: active,
                    })
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(|_| store_failure())
    }

    fn list_customers(&self, active_only: bool) -> Result<Vec<CustomerSummary>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT id, name, phone, credit_policy, customer_tier FROM customers
                 WHERE (?1 = 0 OR is_active = 1) ORDER BY name",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([i64::from(active_only)], |row| {
                let id_text: String = row.get(0)?;
                let name: String = row.get(1)?;
                let phone: Option<String> = row.get(2)?;
                let policy_text: String = row.get(3)?;
                let tier_text: Option<String> = row.get(4)?;
                Ok((id_text, name, phone, policy_text, tier_text))
            })
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        rows.into_iter()
            .map(|(id_text, name, phone, policy_text, tier_text)| {
                Ok(CustomerSummary {
                    id: Uuid::parse_str(&id_text)
                        .map(CustomerId::from)
                        .map_err(|_| store_failure())?,
                    name,
                    phone,
                    credit_policy: CreditPolicy::from_code(&policy_text)
                        .map_err(|_| store_failure())?,
                    customer_tier: parse_tier(tier_text),
                })
            })
            .collect()
    }

    fn record_credit_payment(
        &self,
        customer_id: CustomerId,
        amount_centimes: i64,
        reference: Option<String>,
        allow_overpayment: bool,
        actor: UserId,
        terminal_id: &str,
    ) -> Result<PaymentRecord, ApplicationError> {
        if amount_centimes <= 0 {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_PAYMENT_AMOUNT".to_string()),
            ));
        }
        if terminal_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_TERMINAL_ID".to_string()),
            ));
        }
        let terminal_text = terminal_id.trim().to_string();
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            // ADR-025 mandatory precondition before any ledger mutation.
            let cash_session_id: String = tx
                .query_row(
                    "SELECT id FROM cash_sessions WHERE terminal_id = ?1 AND status = 'OPEN' LIMIT 1",
                    [&terminal_text],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => {
                        crate::persistence::InfrastructureError::Migration(
                            "NO_OPEN_CASH_SESSION".to_string(),
                        )
                    }
                    other => crate::persistence::InfrastructureError::Sqlite(other),
                })?;
            let current: Option<(i64, i64, String)> = tx
                .query_row(
                    "SELECT current_balance_centimes, revision, name FROM customers WHERE id = ?1",
                    [customer_id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map(Some)
                .or_else(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    other => Err(other),
                })?;
            let (balance, revision, customer_name) = current.ok_or_else(|| {
                crate::persistence::InfrastructureError::Migration("CUSTOMER_MISSING".to_string())
            })?;
            if amount_centimes > balance && !allow_overpayment {
                return Err(crate::persistence::InfrastructureError::Migration(format!(
                    "OVERPAYMENT:{balance}:{amount_centimes}"
                )));
            }
            let new_balance = balance - amount_centimes;
            let sequence: i64 = tx.query_row(
                "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM customer_ledger",
                [],
                |row| row.get(0),
            )?;
            let entry_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO customer_ledger
                 (id, sequence_no, customer_id, entry_type, amount_centimes,
                  running_balance_centimes, document_type, document_id,
                  created_by, created_at)
                 VALUES (?1, ?2, ?3, 'CREDIT', ?4, ?5, 'PAYMENT', ?6, ?7, ?8)",
                rusqlite::params![
                    entry_id,
                    sequence,
                    customer_id.to_string(),
                    amount_centimes,
                    new_balance,
                    reference,
                    actor.to_string(),
                    stamp,
                ],
            )?;
            let changed = tx.execute(
                "UPDATE customers
                 SET current_balance_centimes = ?1, revision = revision + 1, updated_at = ?2
                 WHERE id = ?3 AND revision = ?4",
                rusqlite::params![new_balance, stamp, customer_id.to_string(), revision],
            )?;
            if changed != 1 {
                return Err(crate::persistence::InfrastructureError::Migration(
                    "CONCURRENCY".to_string(),
                ));
            }
            // ADR-027: entire payment is cash-collected into the active session.
            tx.execute(
                "INSERT INTO cash_movements
                 (id, cash_session_id, movement_type, amount_centimes,
                  reference_type, reference_id, description, created_at, created_by)
                 VALUES (?1, ?2, 'CREDIT_PAYMENT_IN', ?3, 'CUSTOMER_PAYMENT', ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    Uuid::new_v4().to_string(),
                    cash_session_id,
                    amount_centimes,
                    entry_id,
                    format!("Credit payment {}", customer_id.to_string()),
                    stamp,
                    actor.to_string(),
                ],
            )?;
            // TASK-BRK-061: enqueue the debt receipt print job in the SAME
            // atomic transaction (ledger append, balance cache, cash
            // movement, and outbox row commit or roll back together). The
            // receipt number reuses the `customer_ledger` sequence (ADR-082).
            let footer_message =
                read_setting_in_tx(tx, "store.footer_message");
            let store_identity = StoreIdentity {
                store_name: read_setting_in_tx(tx, "store.name"),
                store_address: read_setting_in_tx(tx, "store.address"),
                store_phone: read_setting_in_tx(tx, "store.phone"),
                nif: read_setting_in_tx(tx, "store.nif"),
                nis: read_setting_in_tx(tx, "store.nis"),
                rc: read_setting_in_tx(tx, "store.rc"),
                ai: read_setting_in_tx(tx, "store.ai"),
                ticket_footer_message: footer_message.clone(),
            };
            let debt_receipt = DebtReceiptDocument {
                language: ReceiptLanguage::French,
                shop_name: "BARAKA QUINCAILLERIE".to_string(),
                receipt_number: format!("REC-{sequence:06}"),
                created_at: stamp.clone(),
                customer_name,
                amount_text: format_centimes_plain(amount_centimes),
                new_balance_text: format_centimes_plain(new_balance),
                reference_text: reference.clone(),
                store_identity,
                footer_message,
            };
            let debt_payload = serde_json::to_string(&debt_receipt).map_err(|_| {
                crate::persistence::InfrastructureError::Config(
                    "DEBT_RECEIPT_SERIALIZE".to_string(),
                )
            })?;
            super::sqlite_print_job_repository::enqueue_in_tx(
                tx,
                None,
                "DEBT_RECEIPT",
                &debt_payload,
                &stamp,
            )?;
            Ok(PaymentRecord {
                ledger_entry_id: entry_id,
                new_balance_centimes: new_balance,
            })
        })
        .map_err(|error| match error {
            crate::persistence::InfrastructureError::Migration(message)
                if message == "CUSTOMER_MISSING" =>
            {
                ApplicationError::CustomerNotFound
            }
            crate::persistence::InfrastructureError::Migration(message)
                if message == "NO_OPEN_CASH_SESSION" =>
            {
                ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
            }
            crate::persistence::InfrastructureError::Migration(message)
                if message.starts_with("OVERPAYMENT:") =>
            {
                let mut parts = message["OVERPAYMENT:".len()..].splitn(2, ':');
                ApplicationError::PaymentExceedsBalance {
                    balance_centimes: parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
                    attempted_centimes: parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
                }
            }
            crate::persistence::InfrastructureError::Migration(message)
                if message == "CONCURRENCY" =>
            {
                ApplicationError::ConcurrencyInvariantViolated
            }
            _ => store_failure(),
        })
    }

    fn get_statement(
        &self,
        customer_id: CustomerId,
        from_date: Option<String>,
        to_date: Option<String>,
    ) -> Result<Vec<LedgerEntryRow>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT entry_type, amount_centimes, document_type, document_id, created_at
                 FROM customer_ledger
                 WHERE customer_id = ?1
                   AND (?2 IS NULL OR created_at >= ?2)
                   AND (?3 IS NULL OR created_at <= ?3)
                 ORDER BY created_at ASC, sequence_no ASC",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map(
                rusqlite::params![customer_id.to_string(), from_date, to_date],
                |row| {
                    let entry_type: String = row.get(0)?;
                    let amount: i64 = row.get(1)?;
                    let reference_type: Option<String> = row.get(2)?;
                    let reference_id: Option<String> = row.get(3)?;
                    let created_at: String = row.get(4)?;
                    Ok((entry_type, amount, reference_type, reference_id, created_at))
                },
            )
            .map_err(|_| store_failure())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())?;
        // Server-side running balance in query order: DEBIT adds, CREDIT subtracts.
        let mut running: i128 = 0;
        let mut out = Vec::with_capacity(rows.len());
        for (entry_type, amount, reference_type, reference_id, created_at) in rows {
            let delta = match entry_type.as_str() {
                "DEBIT" => i128::from(amount),
                "CREDIT" => -i128::from(amount),
                _ => {
                    return Err(store_failure());
                }
            };
            running = running.checked_add(delta).ok_or_else(store_failure)?;
            let balance_after = i64::try_from(running).map_err(|_| store_failure())?;
            out.push(LedgerEntryRow {
                entry_type,
                amount_centimes: amount,
                reference_type,
                reference_id,
                created_at,
                balance_after_centimes: balance_after,
            });
        }
        Ok(out)
    }

    fn count_customer_references(&self, customer_id: CustomerId) -> Result<u64, ApplicationError> {
        let guard = self.lock()?;
        let key = customer_id.to_string();
        let documents: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM documents WHERE party_type = 'CUSTOMER' AND party_id = ?1",
                [&key],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        let ledger: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM customer_ledger WHERE customer_id = ?1",
                [&key],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        Ok(documents.max(0) as u64 + ledger.max(0) as u64)
    }

    fn hard_delete_customer(&self, customer_id: CustomerId) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        let removed = guard
            .execute(
                "DELETE FROM customers WHERE id = ?1",
                [customer_id.to_string()],
            )
            .map_err(|_| store_failure())?;
        if removed == 0 {
            return Err(ApplicationError::NotFound("customer".to_string()));
        }
        Ok(())
    }

    fn set_customer_active(
        &self,
        customer_id: CustomerId,
        active: bool,
    ) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let removed = guard
            .execute(
                "UPDATE customers SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![i64::from(active), stamp, customer_id.to_string()],
            )
            .map_err(|_| store_failure())?;
        if removed == 0 {
            return Err(ApplicationError::NotFound("customer".to_string()));
        }
        Ok(())
    }
}
