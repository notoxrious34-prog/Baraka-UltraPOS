//! Parent-side verification battery V1–V9 (ADR-063 §5).
//!
//! Every check runs against a freshly opened connection over the
//! post-crash files — a plain reopen, no checkpoint, no WAL cleanup, no
//! manual recovery code. SQLite auto-recovery over a hot WAL is the only
//! recovery mechanism exercised.

use std::path::Path;

use baraka_infrastructure::persistence::connection::open_connection;
use rusqlite::Connection;

use super::scenario::{
    SaleKind, CASH_TTC_CENTIMES, CREDIT_TTC_CENTIMES, QTY_BASE_MICRO, SEED_LEVEL_MICRO,
};

/// Fiscal year used for the sequence scope (matches engine: Utc::now).
fn fiscal_year() -> i32 {
    chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse()
        .expect("year")
}

/// Snapshot of every effect the fixed payload may produce.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectSnapshot {
    pub documents: i64,
    pub items: i64,
    pub payments: i64,
    pub stock_movements: i64,
    pub level_micro: i64,
    pub ledger_rows: i64,
    pub ledger_sum: i64,
    pub cash_sum: i64,
    pub print_jobs: i64,
    pub command_rows: i64,
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0))
        .expect("count query")
}

fn sum_or_zero(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get::<_, Option<i64>>(0))
        .expect("sum query")
        .unwrap_or(0)
}

/// Capture the full effect snapshot for the scenario's document.
pub fn snapshot(conn: &Connection, command_id: &str) -> EffectSnapshot {
    // Effects are attributed globally: the scenario database holds only
    // seeded rows plus at most one sale, so global counts are exact.
    let level_micro: i64 = conn
        .query_row(
            "SELECT quantity_base_micro FROM stock_levels LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("level row");
    EffectSnapshot {
        documents: count(conn, "SELECT COUNT(*) FROM documents"),
        items: count(conn, "SELECT COUNT(*) FROM document_items"),
        payments: count(conn, "SELECT COUNT(*) FROM payments"),
        stock_movements: count(conn, "SELECT COUNT(*) FROM stock_movements"),
        level_micro,
        ledger_rows: count(conn, "SELECT COUNT(*) FROM customer_ledger"),
        ledger_sum: sum_or_zero(
            conn,
            "SELECT SUM(CASE WHEN entry_type = 'DEBIT' THEN amount_centimes \
             ELSE -amount_centimes END) FROM customer_ledger",
        ),
        cash_sum: sum_or_zero(
            conn,
            "SELECT SUM(cm.amount_centimes) FROM cash_movements cm \
             WHERE cm.movement_type = 'SALE_CASH_IN'",
        ),
        print_jobs: count(conn, "SELECT COUNT(*) FROM print_jobs"),
        command_rows: conn
            .query_row(
                "SELECT COUNT(*) FROM command_log WHERE command_id = ?1",
                [command_id],
                |row| row.get(0),
            )
            .expect("command count"),
    }
}

/// V1: clean open, integrity_check ok, zero foreign-key violations.
pub fn v1_integrity(conn: &Connection) -> Result<(), String> {
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("integrity_check failed: {e}"))?;
    if check.to_lowercase() != "ok" {
        return Err(format!("integrity_check = {check}"));
    }
    let violations: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("foreign_key_check failed: {e}"))?;
    if violations != 0 {
        return Err(format!("foreign_key_check rows = {violations}"));
    }
    Ok(())
}

/// V2: audit hash chain validates end-to-end (exact mirror of the backup
/// verifier's rule: `record_hash = SHA256(prev|timestamp|actor|action|
/// entity|entity_id|new_json)`, chained from `"GENESIS"`, ordered by
/// `sequence_no`; NULL new value hashes as empty; empty log is valid).
pub fn v2_audit_chain(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT prev_hash, record_hash, timestamp, user_id, action, \
             entity_type, entity_id, new_value_json \
             FROM audit_log ORDER BY sequence_no ASC",
        )
        .map_err(|e| format!("audit prepare: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|e| format!("audit read: {e}"))?;
    let mut expected_prev = "GENESIS".to_string();
    for (index, row) in rows.enumerate() {
        let (prev, hash, stamp, actor, action, entity, entity_id, new_json) =
            row.map_err(|e| format!("audit row: {e}"))?;
        if prev != expected_prev {
            return Err(format!("audit prev mismatch at row {index}"));
        }
        let canonical = format!(
            "{prev}|{stamp}|{actor}|{action}|{entity}|{entity_id}|{}",
            new_json.unwrap_or_default()
        );
        if baraka_infrastructure::persistence::migration::checksum_of(&canonical) != hash {
            return Err(format!("audit hash mismatch at row {index}"));
        }
        expected_prev = hash;
    }
    Ok(())
}

/// V3: ledger + stock reconciliation (same invariants as restore P9–P10:
/// cached balances equal signed ledger sums; levels equal signed movement
/// sums over the canonical type lists; unknown entry types forbidden).
pub fn v3_reconciliation(conn: &Connection) -> Result<(), String> {
    let bad_balances: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM customers c
             WHERE c.current_balance_centimes != (
                 SELECT COALESCE(SUM(CASE WHEN l.entry_type = 'DEBIT' THEN l.amount_centimes
                                         WHEN l.entry_type = 'CREDIT' THEN -l.amount_centimes
                                         ELSE 0 END), 0)
                 FROM customer_ledger l WHERE l.customer_id = c.id)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("customer reconcile: {e}"))?;
    if bad_balances != 0 {
        return Err(format!("{bad_balances} customer balance mismatches"));
    }
    let bad_types: i64 = count(
        conn,
        "SELECT COUNT(DISTINCT customer_id) FROM customer_ledger \
         WHERE entry_type NOT IN ('DEBIT', 'CREDIT')",
    );
    if bad_types != 0 {
        return Err(format!("{bad_types} customers with unknown entry types"));
    }
    // Seeded opening levels carry their ADJUSTMENT_POSITIVE movement, so
    // the strict level == signed-sum invariant holds from the seed onward.
    // Type lists are the canonical restore lists, verbatim.
    let bad_levels: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stock_levels l
             WHERE l.quantity_base_micro != (
                 SELECT COALESCE(SUM(CASE
                     WHEN m.movement_type IN ('RETURN_IN', 'PURCHASE_IN',
                         'ADJUSTMENT_POSITIVE', 'TRANSFER_IN')
                         THEN m.quantity_base_micro
                     WHEN m.movement_type IN ('SALE_OUT', 'ADJUSTMENT_NEGATIVE',
                         'TRANSFER_OUT')
                         THEN -m.quantity_base_micro
                     ELSE 0 END), 0)
                 FROM stock_movements m
                 WHERE m.product_id = l.product_id AND m.warehouse_id = l.warehouse_id)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("stock reconcile: {e}"))?;
    if bad_levels != 0 {
        return Err(format!("{bad_levels} stock level mismatches"));
    }
    let unknown_types = count(
        conn,
        "SELECT COUNT(*) FROM (
             SELECT DISTINCT warehouse_id, product_id FROM stock_movements
             WHERE movement_type NOT IN ('RETURN_IN', 'PURCHASE_IN',
                 'ADJUSTMENT_POSITIVE', 'TRANSFER_IN',
                 'SALE_OUT', 'ADJUSTMENT_NEGATIVE', 'TRANSFER_OUT'))",
    );
    if unknown_types != 0 {
        return Err(format!("{unknown_types} unknown movement type groups"));
    }
    let orphans = count(
        conn,
        "SELECT COUNT(*) FROM (
             SELECT DISTINCT m.warehouse_id, m.product_id FROM stock_movements m
             WHERE NOT EXISTS (SELECT 1 FROM stock_levels l
                               WHERE l.warehouse_id = m.warehouse_id
                                 AND l.product_id = m.product_id))",
    );
    if orphans != 0 {
        return Err(format!("{orphans} orphan movement groups"));
    }
    Ok(())
}

/// V4: gapless numbering — last_sequence equals the committed count.
pub fn v4_gapless(conn: &Connection) -> Result<(), String> {
    let year = fiscal_year();
    let last: Option<i64> = match conn.query_row(
        "SELECT last_sequence FROM document_sequences \
         WHERE type = 'TICKET' AND fiscal_year = ?1",
        [year],
        |row| row.get(0),
    ) {
        Ok(sequence) => Some(sequence),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(format!("sequence read: {e}")),
    };
    let committed = count(conn, "SELECT COUNT(*) FROM documents WHERE type = 'TICKET'");
    match last {
        // No sequence row yet: no document may exist either.
        None if committed == 0 => Ok(()),
        None => Err(format!("no sequence row but {committed} documents")),
        Some(sequence) if sequence == committed => Ok(()),
        Some(sequence) => Err(format!(
            "number hole: last_sequence {sequence} != committed {committed}"
        )),
    }
}

/// V5: binary committed/not-committed determination. `command_log` is the
/// last write before COMMIT, so its row is the exact commit witness.
pub fn v5_committed(command_rows: i64, documents: i64) -> Result<bool, String> {
    match (command_rows, documents) {
        (1, 1) => Ok(true),
        (0, 0) => Ok(false),
        other => Err(format!("ambiguous commit state: {other:?}")),
    }
}

/// V6: full effect set present and exact for a committed sale.
pub fn v6_committed_effects(
    conn: &Connection,
    kind: SaleKind,
    snap: &EffectSnapshot,
) -> Result<(), String> {
    let expect_level = SEED_LEVEL_MICRO - QTY_BASE_MICRO;
    if snap.level_micro != expect_level {
        return Err(format!(
            "level {} != seed-effected {expect_level}",
            snap.level_micro
        ));
    }
    if snap.stock_movements != 2 || snap.items != 1 || snap.payments != 1 {
        return Err(format!(
            "effect counts wrong: movements {} items {} payments {}",
            snap.stock_movements, snap.items, snap.payments
        ));
    }
    // The two movements are the seeded ADJUSTMENT_POSITIVE plus the
    // sale's SALE_OUT; the sale row must reference this document exactly.
    // (Checked below, after the document row is read.)
    let expect_total = match kind {
        SaleKind::Cash => CASH_TTC_CENTIMES,
        SaleKind::Credit => CREDIT_TTC_CENTIMES,
    };
    let doc: (String, i64, i64, i64) = conn
        .query_row(
            "SELECT id, ttc_centimes, paid_amount_centimes, change_amount_centimes \
             FROM documents LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| format!("document read: {e}"))?;
    let sale_movement: (String, i64) = conn
        .query_row(
            "SELECT movement_type, quantity_base_micro FROM stock_movements \
             WHERE reference_id = ?1",
            [&doc.0],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("sale movement read: {e}"))?;
    if sale_movement != ("SALE_OUT".to_string(), QTY_BASE_MICRO) {
        return Err(format!("sale movement wrong: {sale_movement:?}"));
    }
    if doc.1 != expect_total || doc.2 != expect_total || doc.3 != 0 {
        return Err(format!("document totals wrong: {doc:?}"));
    }
    let payment_method: String = conn
        .query_row(
            "SELECT payment_method FROM payments WHERE document_id = ?1",
            [&doc.0],
            |row| row.get(0),
        )
        .map_err(|e| format!("payment read: {e}"))?;
    let payment_amount: i64 = conn
        .query_row(
            "SELECT amount_centimes FROM payments WHERE document_id = ?1",
            [&doc.0],
            |row| row.get(0),
        )
        .map_err(|e| format!("payment amount: {e}"))?;
    match kind {
        SaleKind::Cash => {
            if payment_method != "CASH" || payment_amount != CASH_TTC_CENTIMES {
                return Err(format!(
                    "cash payment wrong: {payment_method} {payment_amount}"
                ));
            }
            if snap.cash_sum != CASH_TTC_CENTIMES {
                return Err(format!("cash movement sum {} wrong", snap.cash_sum));
            }
            if snap.ledger_rows != 0 {
                return Err("cash sale wrote ledger rows".to_string());
            }
        }
        SaleKind::Credit => {
            if payment_method != "CREDIT" || payment_amount != CREDIT_TTC_CENTIMES {
                return Err(format!(
                    "credit payment wrong: {payment_method} {payment_amount}"
                ));
            }
            if snap.cash_sum != 0 {
                return Err("credit sale wrote cash movements".to_string());
            }
            if snap.ledger_rows != 1 || snap.ledger_sum != CREDIT_TTC_CENTIMES {
                return Err(format!(
                    "credit ledger wrong: rows {} sum {}",
                    snap.ledger_rows, snap.ledger_sum
                ));
            }
        }
    }
    let job_status: String = conn
        .query_row(
            "SELECT status FROM print_jobs WHERE document_id = ?1",
            [&doc.0],
            |row| row.get(0),
        )
        .map_err(|e| format!("print job read: {e}"))?;
    if !["PENDING", "DONE", "FAILED"].contains(&job_status.as_str()) {
        return Err(format!("print job status {job_status} invalid"));
    }
    if snap.print_jobs != 1 {
        return Err(format!("print job count {} wrong", snap.print_jobs));
    }
    Ok(())
}

/// V7: zero artifacts for an uncommitted sale (the single seeded opening
/// movement and the seed level are the baseline, not artifacts).
pub fn v7_no_artifacts(conn: &Connection, snap: &EffectSnapshot) -> Result<(), String> {
    let clean = EffectSnapshot {
        stock_movements: 1,
        level_micro: SEED_LEVEL_MICRO,
        ..EffectSnapshot::default()
    };
    if *snap != clean {
        return Err(format!("uncommitted sale left artifacts: {snap:?}"));
    }
    let seed_movement: (String, i64) = conn
        .query_row(
            "SELECT movement_type, quantity_base_micro FROM stock_movements LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("seed movement read: {e}"))?;
    if seed_movement != ("ADJUSTMENT_POSITIVE".to_string(), SEED_LEVEL_MICRO) {
        return Err(format!("seed movement disturbed: {seed_movement:?}"));
    }
    Ok(())
}

/// Open a scenario database with a plain reopen (recovery = SQLite
/// auto-recovery; no checkpoint, no WAL cleanup, no manual steps).
pub fn reopen(db: &Path) -> Connection {
    open_connection(db).expect("post-crash reopen")
}
