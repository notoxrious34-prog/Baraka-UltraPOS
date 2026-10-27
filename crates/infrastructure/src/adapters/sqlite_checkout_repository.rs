//! Atomic sale checkout engine (single `run_transaction`).
//!
//! Order inside the transaction: idempotency claim-or-replay, explicit
//! negative-stock gate (role + confirmation, TASK-BRK-033), gapless
//! sequence allocation, document + items, leaf stock deductions (SALE_OUT,
//! each override leaf emitting a detailed audit row), payments
//! payments (+ optional cash movement), credit ledger + balance (with an
//! authoritative in-transaction limit re-check), print outbox job, command
//! log completion. Any failure rolls back everything: no document, no
//! movement, no payment, no consumed sequence number.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{CheckoutCommand, CheckoutReceipt, CheckoutRepository};
use baraka_domain::sales::{format_number, DocumentType};
use chrono::{Datelike, SecondsFormat, Utc};
use rusqlite::{Connection, Transaction};
use uuid::Uuid;

use crate::adapters::sqlite_audit_log::insert_cash_audit_row;
use crate::adapters::sqlite_document_sequence_repository::allocate_in_tx;
use crate::adapters::sqlite_print_job_repository::enqueue_in_tx;
use crate::adapters::sqlite_stock_repository::{
    apply_one, map_engine_error, next_sequence_no, sentinel, tx_failure,
};
use crate::persistence::migration::checksum_of;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;

/// Atomic checkout over a shared SQLite connection.
pub struct SqliteCheckoutRepository {
    db: Arc<Mutex<Connection>>,
}

impl SqliteCheckoutRepository {
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

/// Read-only level probe: current base micros, 0 when no row exists.
/// Never creates rows (unlike the write path's fetch-or-create), so the
/// shortage gate itself has zero storage side effects.
fn read_level_or_zero(
    tx: &Transaction,
    product_id: &str,
    warehouse_id: &str,
) -> Result<i64, InfrastructureError> {
    tx.query_row(
        "SELECT quantity_base_micro FROM stock_levels WHERE product_id = ?1 AND warehouse_id = ?2",
        rusqlite::params![product_id, warehouse_id],
        |row| row.get(0),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
    .map(|level| level.unwrap_or(0))
    .map_err(InfrastructureError::from)
}

/// Commercial designation + sku for shortage and audit messages.
fn product_name_sku(
    tx: &Transaction,
    product_id: &str,
) -> Result<(String, String), InfrastructureError> {
    tx.query_row(
        "SELECT name_fr, sku FROM products WHERE id = ?1",
        [product_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
    .map_err(InfrastructureError::from)?
    .ok_or_else(|| tx_failure(sentinel::MISSING_PRODUCT.to_string()))
}

/// Look up a command row: id, hash, payload.
fn lookup_command(
    tx: &Transaction,
    command_id: &str,
) -> Result<Option<(String, String)>, InfrastructureError> {
    tx.query_row(
        "SELECT request_hash, response_payload FROM command_log WHERE command_id = ?1",
        [command_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
    .map_err(InfrastructureError::from)
}

/// Evaluate a stored command row: replay on hash match (non-empty
/// payload), conflict on mismatch, proceed on empty (crashed claim).
fn evaluate_stored(
    stored_hash: &str,
    payload: &str,
    request_hash: &str,
) -> Result<Option<CheckoutReceipt>, InfrastructureError> {
    if stored_hash != request_hash {
        return Err(tx_failure(sentinel::IDEMPOTENCY_CONFLICT.to_string()));
    }
    if payload.is_empty() {
        return Ok(None);
    }
    let receipt: CheckoutReceipt =
        serde_json::from_str(payload).map_err(|_| tx_failure("REPLAY_CORRUPT".to_string()))?;
    Ok(Some(receipt))
}

/// Open cash session id for a terminal (mandatory precondition).
fn require_open_session_in_tx(
    tx: &Transaction,
    terminal_id: &str,
) -> Result<String, InfrastructureError> {
    tx.query_row(
        "SELECT id FROM cash_sessions WHERE terminal_id = ?1 AND status = 'OPEN' LIMIT 1",
        [terminal_id],
        |row| row.get(0),
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => tx_failure("NO_OPEN_CASH_SESSION".to_string()),
        other => InfrastructureError::from(other),
    })
}

/// Authoritative in-transaction credit re-check; returns the new balance.
fn authorize_credit_in_tx(
    tx: &Transaction,
    customer_id: &str,
    credit_amount: i64,
    override_granted: bool,
) -> Result<i64, InfrastructureError> {
    let profile: Option<(String, i64, i64, bool)> = tx
        .query_row(
            "SELECT credit_policy, credit_limit_centimes, current_balance_centimes, is_active
             FROM customers WHERE id = ?1",
            [customer_id],
            |row| {
                let policy: String = row.get(0)?;
                let limit: i64 = row.get(1)?;
                let balance: i64 = row.get(2)?;
                let active: bool = row.get::<_, i64>(3)? != 0;
                Ok((policy, limit, balance, active))
            },
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    let (policy, limit, balance, active) =
        profile.ok_or_else(|| tx_failure("CUSTOMER_MISSING".to_string()))?;
    if !active || policy == "DISABLED" {
        return Err(tx_failure("CREDIT_DISABLED".to_string()));
    }
    let proposed = (i128::from(balance) + i128::from(credit_amount))
        .try_into()
        .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
    if policy == "LIMITED" {
        let proposed_value: i64 = proposed;
        if proposed_value > limit && !override_granted {
            return Err(tx_failure(sentinel::credit_limit_exceeded(
                limit,
                proposed_value,
            )));
        }
    }
    Ok(proposed)
}

impl CheckoutRepository for SqliteCheckoutRepository {
    fn checkout_atomic(
        &self,
        command: &CheckoutCommand,
    ) -> Result<CheckoutReceipt, ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let fiscal_year = Utc::now().year();
        let request_hash = checksum_of(&command.request_canonical);
        let actor = command.actor.to_string();
        run_transaction(&mut guard, |tx| {
            // 1. Idempotency fast path (read committed state).
            if let Some((stored_hash, payload)) = lookup_command(tx, &command.command_id)? {
                if let Some(receipt) = evaluate_stored(&stored_hash, &payload, &request_hash)? {
                    return Ok(receipt);
                }
            }
            // 1b. Explicit negative-stock gate (TASK-BRK-033, authoritative).
            // Leaf requested base micros are aggregated per product and
            // compared against pre-write levels (0 when no row exists).
            // Shortage handling, all BEFORE any document/stock write:
            // - actor without the standing grant -> InsufficientStock
            //   (hard block; the confirmation flag is ignored);
            // - granted actor without explicit in-session confirmation ->
            //   NegativeStockConfirmationRequired;
            // - granted + confirmed -> deductions proceed authorized and
            //   each override leaf emits a detailed audit row (step 5b).
            // Checkout leaves always carry base-unit quantities, so the
            // summed selling micros equal the base micros the engine
            // deducts below.
            let warehouse_id = command.warehouse_id.to_string();
            let mut requested_base: HashMap<String, i64> = HashMap::new();
            for leaf in &command.leaf_deductions {
                let entry = requested_base
                    .entry(leaf.product_id.to_string())
                    .or_insert(0);
                *entry = entry.saturating_add(leaf.quantity_micro);
            }
            let mut available_before: HashMap<String, i64> = HashMap::new();
            for product_id in requested_base.keys() {
                let available = read_level_or_zero(tx, product_id, &warehouse_id)?;
                available_before.insert(product_id.clone(), available);
            }
            for (product_id, requested) in &requested_base {
                let available = available_before.get(product_id).copied().unwrap_or(0);
                if available < *requested {
                    if !command.negative_override_permitted {
                        return Err(tx_failure(sentinel::insufficient(available, *requested)));
                    }
                    if !command.allow_negative_stock {
                        let (name, _) = product_name_sku(tx, product_id)?;
                        return Err(tx_failure(sentinel::negconfirm(
                            product_id, &name, available, *requested,
                        )));
                    }
                }
            }
            // 1c. ADR-025 mandatory precondition: OPEN session on this terminal.
            // Inside the SAME transaction, before any stock/document/ledger write.
            let cash_session_id = require_open_session_in_tx(tx, &command.terminal_id)?;
            // 2. Gapless number.
            let doc_type = DocumentType::from_code(&command.doc_type_code)
                .map_err(|_| tx_failure("INVALID_DOC_TYPE".to_string()))?;
            let sequence = allocate_in_tx(tx, doc_type.code(), fiscal_year)?;
            let number = format_number(doc_type.prefix(), fiscal_year, sequence);
            let document_id = Uuid::new_v4().to_string();
            // 3. Document header.
            let method_codes: Vec<&str> = {
                let mut seen = Vec::new();
                for payment in &command.payments {
                    if !seen.contains(&payment.method_code.as_str()) {
                        seen.push(payment.method_code.as_str());
                    }
                }
                seen
            };
            let payment_method = match method_codes.as_slice() {
                [] => "NONE",
                [single] => single,
                _ => "MIXED",
            };
            tx.execute(
                "INSERT INTO documents
                 (id, number, fiscal_year, type, party_type, party_id, warehouse_id,
                  document_language, language, subtotal_ht_centimes, vat_total_centimes,
                  global_discount_centimes, timbre_centimes, rounding_centimes,
                  ttc_centimes, paid_amount_centimes, change_amount_centimes,
                  payment_method, status, created_at, created_by, revision,
                  origin_device_id, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, ?11, ?12, 0,
                         ?13, ?14, ?15, ?16, 'FINALIZED', ?17, ?18, 1, ?19, ?20)",
                rusqlite::params![
                    document_id,
                    number,
                    i64::from(fiscal_year),
                    command.doc_type_code,
                    command.customer_id.map(|_| "CUSTOMER"),
                    command.customer_id.map(|id| id.to_string()),
                    command.warehouse_id.to_string(),
                    command.document_language.to_string(),
                    command.totals.subtotal_ht,
                    command.totals.total_tva,
                    command.totals.global_discount,
                    command.totals.timbre,
                    command.totals.total_ttc,
                    command.totals.paid_total,
                    command.totals.change,
                    payment_method,
                    stamp,
                    actor,
                    command.terminal_id,
                    command.notes,
                ],
            )?;
            // 4. Item snapshots.
            for line in &command.priced_lines {
                tx.execute(
                    "INSERT INTO document_items
                     (id, document_id, product_id, unit_id, quantity_micro,
                      quantity_base_micro, unit_price_centimes, discount_centimes,
                      discount_per_unit_centimes, tva_rate_micro, line_ht_centimes,
                      line_tva_centimes, line_ttc_centimes, cost_price_centimes,
                      sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        document_id,
                        line.product_id.to_string(),
                        line.unit_id.to_string(),
                        line.quantity_micro,
                        line.quantity_base_micro,
                        line.unit_price_centimes,
                        line.discount_centimes,
                        line.discount_per_unit_centimes,
                        line.tva_rate_micro,
                        line.line_ht_centimes,
                        line.line_tva_centimes,
                        line.line_ttc_centimes,
                        line.cost_price_centimes,
                        line.sort_order,
                    ],
                )?;
            }
            // 5. Leaf stock deductions (SALE_OUT, atomic with everything).
            // Override leaves are collected for the detailed audit rows
            // in step 5b (the engine additionally appends its own minimal
            // chain row per authorized negative write).
            let mut override_leaves: Vec<(String, i64)> = Vec::new();
            for leaf in &command.leaf_deductions {
                let (_, _, override_applied) = apply_one(
                    tx,
                    &leaf.product_id.to_string(),
                    &command.warehouse_id.to_string(),
                    &leaf.unit_id.to_string(),
                    leaf.quantity_micro,
                    baraka_domain::inventory::StockMovementType::SaleOut,
                    Some("SALE"),
                    Some(document_id.as_str()),
                    None,
                    leaf.authorize_negative_stock,
                    command.notes.as_deref(),
                    &actor,
                    &stamp,
                )?;
                if override_applied {
                    override_leaves
                        .push((leaf.product_id.to_string(), leaf.quantity_micro));
                }
            }
            // 5b. Detailed override audit rows (TASK-BRK-033): one immutable
            // NEGATIVE_STOCK_OVERRIDE record per override leaf carrying the
            // sale context (document number, sku, before/requested/after).
            for (product_id, requested_leaf) in &override_leaves {
                let available = available_before.get(product_id).copied().unwrap_or(0);
                let resulting = read_level_or_zero(tx, product_id, &warehouse_id)?;
                let (_, sku) = product_name_sku(tx, product_id)?;
                let detail = format!(
                    "{{\"document_number\":\"{number}\",\"sku\":\"{sku}\",\"available_qty_micro\":{available},\"requested_qty_micro\":{requested_leaf},\"resulting_qty_micro\":{resulting}}}"
                );
                insert_cash_audit_row(
                    tx,
                    "NEGATIVE_STOCK_OVERRIDE",
                    "product",
                    product_id,
                    None,
                    &detail,
                    &actor,
                    &stamp,
                )?;
            }
            // 6. Payments (cash legs bound to the mandatory OPEN session) +
            // net cash drawer movement (tendered minus change, same txn).
            let mut cash_tendered: i128 = 0;
            for payment in &command.payments {
                if payment.method_code == "CASH" {
                    cash_tendered += i128::from(payment.amount_centimes);
                }
                let sequence_no = next_sequence_no(tx)?;
                tx.execute(
                    "INSERT INTO payments
                     (id, sequence_no, document_id, customer_id, amount_centimes,
                      payment_method, cash_session_id, created_by, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        sequence_no,
                        document_id,
                        if payment.method_code == "CREDIT" {
                            command.customer_id.map(|id| id.to_string())
                        } else {
                            None::<String>
                        },
                        payment.amount_centimes,
                        payment.method_code,
                        if payment.method_code == "CASH" {
                            Some(cash_session_id.clone())
                        } else {
                            None::<String>
                        },
                        actor,
                        stamp,
                    ],
                )?;
            }
            let net_cash_delta: i128 = cash_tendered - i128::from(command.totals.change);
            if net_cash_delta < 0 {
                return Err(tx_failure("INVALID_PAYMENT_STATE".to_string()));
            }
            if net_cash_delta > 0 {
                let net_cash = i64::try_from(net_cash_delta)
                    .map_err(|_| tx_failure(sentinel::OVERFLOW.to_string()))?;
                tx.execute(
                    "INSERT INTO cash_movements
                     (id, cash_session_id, movement_type, amount_centimes,
                      reference_type, reference_id, description, created_at, created_by)
                     VALUES (?1, ?2, 'SALE_CASH_IN', ?3, 'DOCUMENT', ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        cash_session_id,
                        net_cash,
                        document_id,
                        number,
                        stamp,
                        actor,
                    ],
                )?;
            }
            // 7. Credit ledger + cached balance (authoritative re-check).
            if command.credit_amount_centimes > 0 {
                let customer_id = command
                    .customer_id
                    .map(|id| id.to_string())
                    .ok_or_else(|| tx_failure("CREDIT_CUSTOMER_REQUIRED".to_string()))?;
                let new_balance = authorize_credit_in_tx(
                    tx,
                    &customer_id,
                    command.credit_amount_centimes,
                    command.credit_override_granted,
                )?;
                let ledger_sequence: i64 = tx.query_row(
                    "SELECT COALESCE(MAX(sequence_no), 0) + 1 FROM customer_ledger",
                    [],
                    |row| row.get(0),
                )?;
                tx.execute(
                    "INSERT INTO customer_ledger
                     (id, sequence_no, customer_id, entry_type, amount_centimes,
                      running_balance_centimes, document_type, document_id,
                      created_by, created_at)
                     VALUES (?1, ?2, ?3, 'DEBIT', ?4, ?5, 'SALE', ?6, ?7, ?8)",
                    rusqlite::params![
                        Uuid::new_v4().to_string(),
                        ledger_sequence,
                        customer_id,
                        command.credit_amount_centimes,
                        new_balance,
                        document_id,
                        actor,
                        stamp,
                    ],
                )?;
                tx.execute(
                    "UPDATE customers
                     SET current_balance_centimes = ?1, revision = revision + 1, updated_at = ?2
                     WHERE id = ?3",
                    rusqlite::params![new_balance, stamp, customer_id],
                )?;
            }
            // 8. Transactional outbox print job.
            let payload = serde_json::json!({
                "document_id": document_id,
                "number": number,
                "total_ttc_centimes": command.totals.total_ttc,
            })
            .to_string();
            let job_id = enqueue_in_tx(tx, Some(document_id.as_str()), "RECEIPT", &payload, &stamp)?;
            // 9. Record the idempotency row atomically with the sale
            // (ADR-064: the claim check in step 1 and this INSERT are both
            // inside the sale transaction — there is no separately
            // committed claim, so claim-without-result is impossible).
            let receipt = CheckoutReceipt {
                document_id: document_id.clone(),
                document_number: number.clone(),
                subtotal_ht_centimes: command.totals.subtotal_ht.to_string(),
                tva_total_centimes: command.totals.total_tva.to_string(),
                discount_total_centimes: command.totals.global_discount.to_string(),
                timbre_centimes: command.totals.timbre.to_string(),
                total_ttc_centimes: command.totals.total_ttc.to_string(),
                paid_amount_centimes: command.totals.paid_total.to_string(),
                change_amount_centimes: command.totals.change.to_string(),
                print_job_id: job_id,
            };
            let payload_json = serde_json::to_string(&receipt)
                .map_err(|_| tx_failure("RECEIPT_ENCODE".to_string()))?;
            // 9. Append the completed idempotency row. On a PK conflict
            // (lost commit race) re-read and replay instead of failing.
            match tx.execute(
                "INSERT INTO command_log
                 (command_id, request_hash, command_type, response_payload, created_at, created_by)
                 VALUES (?1, ?2, 'sales.checkout', ?3, ?4, ?5)",
                rusqlite::params![command.command_id, request_hash, payload_json, stamp, actor],
            ) {
                Ok(_) => Ok(receipt),
                Err(_) => match lookup_command(tx, &command.command_id)? {
                    Some((stored_hash, payload)) => {
                        match evaluate_stored(&stored_hash, &payload, &request_hash)? {
                            Some(receipt) => Ok(receipt),
                            None => Err(tx_failure("CLAIM_RACE".to_string())),
                        }
                    }
                    None => Err(tx_failure("CLAIM_RACE".to_string())),
                },
            }
        })
        .map_err(map_engine_error)
    }
}
