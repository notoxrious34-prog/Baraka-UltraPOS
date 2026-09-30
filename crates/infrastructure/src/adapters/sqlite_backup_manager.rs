//! SQLite backup manager: WAL-safe online snapshots with
//! self-verification, streaming SHA-256 tamper-evidence, and `backups_log`
//! audit rows (TASK-BRK-051, ADR-010).

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{BackupLogEntry, BackupRepository, BackupVerification};
use baraka_domain::backup::{
    BackupContentSummary, LedgerScope, PreRestoreChecks, PreRestoreReport, RestoreError,
    RestoreIneligibleReason, RestoreOutcome, RestoreWarning,
};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags};

use crate::adapters::sqlite_audit_log::insert_cash_audit_row;
use crate::persistence::connection::open_connection;
use crate::persistence::migration::{checksum_of, run_migrations, MIGRATIONS};
use crate::persistence::sha256::Sha256;
use crate::persistence::transaction::run_transaction;
use crate::persistence::InfrastructureError;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("backup store failure".to_string())
}

/// 64 KiB streaming chunks bound hashing RAM regardless of database size.
const HASH_CHUNK_BYTES: usize = 64 * 1024;

/// Consistent online backup engine over a shared SQLite connection.
///
/// Restore (TASK-BRK-054, ADR-062) additionally owns the active database
/// path (quarantine/swap target) and a process-wide restore guard. Every
/// other repository shares the same `Arc<Mutex<Connection>>`, so holding
/// the lock across the swap window blocks in-flight commands instead of
/// interleaving with them.
pub struct SqliteBackupManager {
    db: Arc<Mutex<Connection>>,
    active_db_path: PathBuf,
    restore_guard: Mutex<()>,
}

impl SqliteBackupManager {
    /// Wrap the shared live connection and record the active database path.
    pub fn new(db: Arc<Mutex<Connection>>, active_db_path: PathBuf) -> Self {
        Self {
            db,
            active_db_path,
            restore_guard: Mutex::new(()),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApplicationError> {
        self.db
            .lock()
            .map_err(|_| ApplicationError::Port("database lock poisoned".to_string()))
    }

    fn live_lock(&self) -> Result<MutexGuard<'_, Connection>, RestoreError> {
        self.db
            .lock()
            .map_err(|_| RestoreError::Io("database lock poisoned".to_string()))
    }
}

impl BackupRepository for SqliteBackupManager {
    fn snapshot_to(&self, dest: &Path) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|_| store_failure())?;
            }
        }
        let mut target = Connection::open(dest).map_err(|_| store_failure())?;
        // WAL-safe online copy: readers keep serving while pages stream.
        // The shared lock is held for the whole run so no writer can
        // interleave a checkpoint mid-copy.
        {
            let backup =
                rusqlite::backup::Backup::new(&guard, &mut target).map_err(|_| store_failure())?;
            backup
                .run_to_completion(100, Duration::from_millis(10), None)
                .map_err(|_| store_failure())?;
        }
        // Explicit close flushes the target header before verification.
        target.close().map_err(|_| store_failure())?;
        Ok(())
    }

    fn verify_snapshot(&self, path: &Path) -> Result<BackupVerification, ApplicationError> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| store_failure())?;
        let mut quick = conn
            .prepare("PRAGMA quick_check;")
            .map_err(|_| store_failure())?;
        let rows = quick
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| store_failure())?;
        let mut saw_row = false;
        for row in rows {
            saw_row = true;
            if row.map_err(|_| store_failure())? != "ok" {
                return Ok(BackupVerification {
                    quick_check_ok: false,
                    foreign_key_violations: 0,
                });
            }
        }
        if !saw_row {
            return Ok(BackupVerification {
                quick_check_ok: false,
                foreign_key_violations: 0,
            });
        }
        let violations: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_foreign_key_check;",
                [],
                |row| row.get(0),
            )
            .map_err(|_| store_failure())?;
        Ok(BackupVerification {
            quick_check_ok: true,
            foreign_key_violations: u64::try_from(violations).unwrap_or(u64::MAX),
        })
    }

    fn sha256_of(&self, path: &Path) -> Result<String, ApplicationError> {
        let file = File::open(path).map_err(|_| store_failure())?;
        let mut reader = BufReader::with_capacity(HASH_CHUNK_BYTES, file);
        let mut hasher = Sha256::new();
        let mut chunk = vec![0u8; HASH_CHUNK_BYTES];
        loop {
            let read = reader.read(&mut chunk).map_err(|_| store_failure())?;
            if read == 0 {
                break;
            }
            hasher.update(&chunk[..read]);
        }
        Ok(hasher.finish_hex())
    }

    fn file_size_of(&self, path: &Path) -> Result<u64, ApplicationError> {
        std::fs::metadata(path)
            .map_err(|_| store_failure())
            .map(|meta| meta.len())
    }

    fn record_backup(&self, entry: &BackupLogEntry) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        guard
            .execute(
                "INSERT INTO backups_log
                 (id, backup_id, app_version, schema_version, created_at,
                  source_device_id, database_hash, cipher_version, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    entry.id,
                    entry.backup_id,
                    entry.app_version,
                    entry.schema_version,
                    entry.created_at,
                    entry.source_device_id,
                    entry.database_hash,
                    entry.cipher_version,
                    entry.status,
                ],
            )
            .map_err(|_| store_failure())?;
        Ok(())
    }

    fn get_backup(&self, backup_id: &str) -> Result<Option<BackupLogEntry>, ApplicationError> {
        let guard = self.lock()?;
        guard
            .query_row(
                "SELECT id, backup_id, app_version, schema_version, created_at,
                        source_device_id, database_hash, cipher_version, status
                 FROM backups_log WHERE backup_id = ?1",
                [backup_id],
                |row| {
                    Ok(BackupLogEntry {
                        id: row.get(0)?,
                        backup_id: row.get(1)?,
                        file_name: row.get(1)?,
                        app_version: row.get(2)?,
                        schema_version: row.get(3)?,
                        created_at: row.get(4)?,
                        source_device_id: row.get(5)?,
                        database_hash: row.get(6)?,
                        cipher_version: row.get(7)?,
                        status: row.get(8)?,
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

    fn list_backups(&self, limit: usize) -> Result<Vec<BackupLogEntry>, ApplicationError> {
        let guard = self.lock()?;
        let mut statement = guard
            .prepare(
                "SELECT id, backup_id, app_version, schema_version, created_at,
                        source_device_id, database_hash, cipher_version, status
                 FROM backups_log ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(|_| store_failure())?;
        let rows = statement
            .query_map([limit as i64], |row| {
                Ok(BackupLogEntry {
                    id: row.get(0)?,
                    backup_id: row.get(1)?,
                    file_name: row.get(1)?,
                    app_version: row.get(2)?,
                    schema_version: row.get(3)?,
                    created_at: row.get(4)?,
                    source_device_id: row.get(5)?,
                    database_hash: row.get(6)?,
                    cipher_version: row.get(7)?,
                    status: row.get(8)?,
                })
            })
            .map_err(|_| store_failure())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| store_failure())
    }

    fn mark_status(&self, backup_id: &str, status: &str) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        guard
            .execute(
                "UPDATE backups_log SET status = ?1 WHERE backup_id = ?2",
                rusqlite::params![status, backup_id],
            )
            .map_err(|_| store_failure())?;
        Ok(())
    }

    fn delete_file(&self, path: &Path) -> Result<(), ApplicationError> {
        if !path.is_file() {
            return Ok(());
        }
        std::fs::remove_file(path).map_err(|_| store_failure())
    }

    fn schema_version(&self) -> i64 {
        MIGRATIONS.len() as i64
    }

    fn precheck_restore(
        &self,
        backup_id: &str,
        dest_dir: &Path,
    ) -> Result<PreRestoreReport, RestoreError> {
        let run = self.run_verification(backup_id.trim(), dest_dir)?;
        let all_pass = run.checks.file_exists
            && run.checks.sha256_matches
            && run.checks.integrity_ok
            && run.checks.foreign_keys_ok
            && run.checks.audit_chain_ok
            && run.checks.customer_ledger_ok
            && run.checks.supplier_ledger_ok
            && run.checks.stock_levels_ok;
        Ok(PreRestoreReport {
            backup_id: run.row.backup_id,
            file_name: run.row.file_name,
            file_size_bytes: run.file_size,
            created_at: run.row.created_at,
            schema_version_backup: run.backup_version,
            schema_version_app: run.app_version,
            checks: run.checks,
            content_summary: run.summary,
            warnings: run.warnings,
            eligible: all_pass && run.reasons.is_empty(),
            ineligible_reasons: run.reasons,
        })
    }

    fn restore_backup(
        &self,
        backup_id: &str,
        dest_dir: &Path,
        actor: &str,
    ) -> Result<RestoreOutcome, RestoreError> {
        // R0. Process-wide restore guard: a concurrent attempt fails fast.
        let _exclusive = self.restore_guard.try_lock().map_err(|error| match error {
            std::sync::TryLockError::WouldBlock => RestoreError::RestoreInProgress,
            std::sync::TryLockError::Poisoned(_) => {
                RestoreError::Io("restore guard poisoned".to_string())
            }
        })?;
        let backup_id = backup_id.trim();
        // Destructive path re-runs P1-P9 verbatim; the advisory precheck
        // report is never trusted here.
        let run = self.run_verification(backup_id, dest_dir)?;
        if let Some(first) = run.reasons.first() {
            return Err(reason_to_error(first));
        }
        // R1. QUIESCE: exclusive control of the shared connection, checkpoint
        // first. Checkpoint failure aborts BEFORE any file moves.
        let mut live = self.live_lock()?;
        live.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|_| RestoreError::Io("live checkpoint failure".to_string()))?;
        // Park a placeholder and drop the old connection so its file handles
        // close; the guard stays held, so no other command can interleave.
        // `live` is reassigned on every path below before the guard drops.
        let parked = std::mem::replace(
            &mut *live,
            Connection::open_in_memory()
                .map_err(|_| RestoreError::Io("restore staging failure".to_string()))?,
        );
        drop(parked);
        let active = self.active_db_path.clone();
        // R2. QUARANTINE (never delete). Nothing has moved yet, so a setup
        // failure reopens the untouched original and reports plain I/O.
        let quarantine_dir = dest_dir.join("quarantine");
        if let Err(error) = std::fs::create_dir_all(&quarantine_dir) {
            return Err(self.abort_before_swap(
                &mut live,
                &active,
                &format!("quarantine directory failure: {error}"),
            ));
        }
        let quarantine_base = match unique_quarantine_base(&quarantine_dir) {
            Ok(base) => base,
            Err(detail) => return Err(self.abort_before_swap(&mut live, &active, &detail)),
        };
        let quarantined_db_name = format!("{quarantine_base}.db");
        let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
        let move_one = |from: PathBuf, to: PathBuf, moved: &mut Vec<(PathBuf, PathBuf)>| {
            std::fs::rename(&from, &to)
                .map(|()| moved.push((to, from)))
                .map_err(|error| format!("quarantine move failure: {error}"))
        };
        if let Err(detail) = move_one(
            active.clone(),
            quarantine_dir.join(&quarantined_db_name),
            &mut moved,
        ) {
            // Nothing moved yet (the ledger is still empty), so reopen the
            // untouched original — routing through the rollback net here
            // would delete the in-place database before moving nothing back.
            return Err(self.abort_before_swap(&mut live, &active, &detail));
        }
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", active.display()));
            if sidecar.is_file() {
                let target = quarantine_dir.join(format!("{quarantine_base}{suffix}"));
                if let Err(detail) = move_one(sidecar, target, &mut moved) {
                    return Err(self.rollback_restore(&mut live, &active, &moved, detail));
                }
            }
        }
        // R3. SWAP: copy (never move) so the source backup stays byte-identical.
        if let Err(error) = std::fs::copy(&run.backup_path, &active) {
            return Err(self.rollback_restore(
                &mut live,
                &active,
                &moved,
                format!("swap copy failure: {error}"),
            ));
        }
        // R4. ACTIVATE on a fresh connection; R5 reopens the shared manager.
        let actor = actor.trim();
        let actor = if actor.is_empty() { "SYSTEM" } else { actor };
        match self.activate_copy(&active, &run, backup_id, actor, &quarantined_db_name) {
            Ok(restored_at) => match open_connection(&active) {
                Ok(fresh) => {
                    *live = fresh;
                    Ok(RestoreOutcome {
                        restored_backup_id: backup_id.to_string(),
                        quarantined_to: quarantined_db_name,
                        restored_at,
                        restart_required: true,
                    })
                }
                Err(error) => Err(self.rollback_restore(
                    &mut live,
                    &active,
                    &moved,
                    format!("restore reopen failure: {error}"),
                )),
            },
            Err(detail) => Err(self.rollback_restore(&mut live, &active, &moved, detail)),
        }
    }
}

/// Internal result of protocol phases P1-P12 (TASK-BRK-054, ADR-062).
///
/// Both the advisory precheck and the destructive execute path consume this
/// exact structure, so the execute path re-runs every verification verbatim
/// and never trusts UI state.
struct BackupVerificationRun {
    row: BackupLogEntry,
    backup_path: PathBuf,
    file_size: u64,
    sha_actual: String,
    backup_version: u32,
    app_version: u32,
    checks: PreRestoreChecks,
    summary: BackupContentSummary,
    warnings: Vec<RestoreWarning>,
    reasons: Vec<RestoreIneligibleReason>,
}

/// First failed ineligible reason mapped to its destructive-path error.
///
/// Reasons arrive in protocol order, so the first entry is the earliest
/// gate that rejected the backup.
fn reason_to_error(reason: &RestoreIneligibleReason) -> RestoreError {
    match reason {
        RestoreIneligibleReason::BackupFileMissing => RestoreError::BackupFileMissing,
        RestoreIneligibleReason::HashMismatch => RestoreError::BackupTampered,
        RestoreIneligibleReason::IntegrityCheckFailed => RestoreError::BackupCorrupted,
        RestoreIneligibleReason::ForeignKeyViolation => RestoreError::BackupFkViolation,
        RestoreIneligibleReason::AuditChainBroken => RestoreError::BackupAuditChainBroken,
        RestoreIneligibleReason::CustomerLedgerMismatch { mismatch_count } => {
            RestoreError::BackupLedgerInconsistent {
                scope: LedgerScope::Customer,
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::SupplierLedgerMismatch { mismatch_count } => {
            RestoreError::BackupLedgerInconsistent {
                scope: LedgerScope::Supplier,
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::StockLevelMismatch { mismatch_count } => {
            RestoreError::BackupLedgerInconsistent {
                scope: LedgerScope::Stock,
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::SchemaNewerThanApp {
            backup_version,
            app_version,
        } => RestoreError::BackupSchemaNewer {
            backup_version: *backup_version,
            app_version: *app_version,
        },
    }
}

/// All-false checks with zeroed summary: the safe default when the backup
/// file cannot be examined at all.
fn blank_checks() -> PreRestoreChecks {
    PreRestoreChecks {
        file_exists: false,
        sha256_matches: false,
        integrity_ok: false,
        foreign_keys_ok: false,
        audit_chain_ok: false,
        customer_ledger_ok: false,
        supplier_ledger_ok: false,
        stock_levels_ok: false,
    }
}

fn blank_summary() -> BackupContentSummary {
    BackupContentSummary {
        products: 0,
        customers: 0,
        documents_total: 0,
        last_document_at: None,
    }
}

/// `PRAGMA integrity_check` passes only on exactly one `"ok"` row.
fn backup_integrity_ok(conn: &Connection) -> bool {
    let mut statement = match conn.prepare("PRAGMA integrity_check;") {
        Ok(statement) => statement,
        Err(_) => return false,
    };
    let rows = match statement.query_map([], |row| row.get::<_, String>(0)) {
        Ok(rows) => rows,
        Err(_) => return false,
    };
    let mut collected = Vec::new();
    for row in rows {
        match row {
            Ok(text) => collected.push(text),
            Err(_) => return false,
        }
    }
    collected.len() == 1 && collected[0] == "ok"
}

/// Count rows reported by `PRAGMA foreign_key_check` (`None` on SQL failure).
fn backup_foreign_key_violations(conn: &Connection) -> Option<u64> {
    let violations: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_check;",
            [],
            |row| row.get(0),
        )
        .ok()?;
    u64::try_from(violations).ok()
}

/// Migration count recorded inside the backup file (`None` when unreadable).
///
/// This reads the `schema_migrations` bookkeeping table directly — the same
/// source of truth the migration runner maintains — without touching
/// `migration.rs` internals.
fn backup_schema_version(conn: &Connection) -> Option<u32> {
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations;", [], |row| {
            row.get(0)
        })
        .ok()?;
    u32::try_from(version).ok()
}

/// Recompute the full audit hash chain on an isolated connection.
///
/// Chain rule (identical to the append path, never modified here):
/// `record_hash = SHA256(prev_hash | timestamp | actor | action | entity |
/// entity_id | new_json)`, chained from `"GENESIS"`. A `NULL` new value
/// hashes as the empty string. An empty log is valid.
fn backup_audit_chain_ok(conn: &Connection) -> bool {
    let mut statement = match conn.prepare(
        "SELECT prev_hash, record_hash, timestamp, user_id, action,
                entity_type, entity_id, new_value_json
         FROM audit_log ORDER BY sequence_no ASC;",
    ) {
        Ok(statement) => statement,
        Err(_) => return false,
    };
    let rows = match statement.query_map([], |row| {
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
    }) {
        Ok(rows) => rows,
        Err(_) => return false,
    };
    let mut expected_prev = "GENESIS".to_string();
    for row in rows {
        let (prev, hash, stamp, actor, action, entity, entity_id, new_json) = match row {
            Ok(values) => values,
            Err(_) => return false,
        };
        if prev != expected_prev {
            return false;
        }
        let canonical = format!(
            "{prev}|{stamp}|{actor}|{action}|{entity}|{entity_id}|{}",
            new_json.unwrap_or_default()
        );
        if checksum_of(&canonical) != hash {
            return false;
        }
        expected_prev = hash;
    }
    true
}

/// Violating customer rows: cached balance differs from the signed ledger
/// sum (`DEBIT` adds, `CREDIT` subtracts), plus ledgers carrying an unknown
/// entry type. `None` on SQL failure.
fn backup_customer_mismatches(conn: &Connection) -> Option<u64> {
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
        .ok()?;
    let bad_types: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT customer_id) FROM customer_ledger
             WHERE entry_type NOT IN ('DEBIT', 'CREDIT')",
            [],
            |row| row.get(0),
        )
        .ok()?;
    u64::try_from(bad_balances.saturating_add(bad_types)).ok()
}

/// Violating supplier rows: same invariant on `supplier_ledger`.
fn backup_supplier_mismatches(conn: &Connection) -> Option<u64> {
    let bad_balances: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM suppliers s
             WHERE s.current_balance_centimes != (
                 SELECT COALESCE(SUM(CASE WHEN l.entry_type = 'DEBIT' THEN l.amount_centimes
                                         WHEN l.entry_type = 'CREDIT' THEN -l.amount_centimes
                                         ELSE 0 END), 0)
                 FROM supplier_ledger l WHERE l.supplier_id = s.id)",
            [],
            |row| row.get(0),
        )
        .ok()?;
    let bad_types: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT supplier_id) FROM supplier_ledger
             WHERE entry_type NOT IN ('DEBIT', 'CREDIT')",
            [],
            |row| row.get(0),
        )
        .ok()?;
    u64::try_from(bad_balances.saturating_add(bad_types)).ok()
}

/// Canonical stock movement signs shared by every reconciliation query.
const STOCK_POSITIVE_TYPES: &str =
    "'RETURN_IN', 'PURCHASE_IN', 'ADJUSTMENT_POSITIVE', 'TRANSFER_IN'";
const STOCK_NEGATIVE_TYPES: &str = "'SALE_OUT', 'ADJUSTMENT_NEGATIVE', 'TRANSFER_OUT'";

/// Violating `(warehouse, product)` pairs.
///
/// Reconciles `stock_levels.quantity_base_micro` against the signed sum of
/// `stock_movements.quantity_base_micro`: the stock engine maintains the
/// base-micro column exclusively (`quantity_micro` is a legacy column pinned
/// at its default on engine-written rows, so reconciling it would reject
/// every healthy database). Unknown movement types and movement groups with
/// no level row also count as violations. `None` on SQL failure.
fn backup_stock_mismatches(conn: &Connection) -> Option<u64> {
    let bad_levels: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM stock_levels l
                 WHERE l.quantity_base_micro != (
                     SELECT COALESCE(SUM(CASE
                         WHEN m.movement_type IN ({STOCK_POSITIVE_TYPES}) THEN m.quantity_base_micro
                         WHEN m.movement_type IN ({STOCK_NEGATIVE_TYPES}) THEN -m.quantity_base_micro
                         ELSE 0 END), 0)
                     FROM stock_movements m
                     WHERE m.warehouse_id = l.warehouse_id AND m.product_id = l.product_id)"
            ),
            [],
            |row| row.get(0),
        )
        .ok()?;
    let unknown_types: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM (
                     SELECT DISTINCT warehouse_id, product_id FROM stock_movements
                     WHERE movement_type NOT IN ({STOCK_POSITIVE_TYPES}, {STOCK_NEGATIVE_TYPES}))"
            ),
            [],
            |row| row.get(0),
        )
        .ok()?;
    let orphan_movements: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (
                 SELECT DISTINCT m.warehouse_id, m.product_id FROM stock_movements m
                 WHERE NOT EXISTS (SELECT 1 FROM stock_levels l
                                   WHERE l.warehouse_id = m.warehouse_id
                                     AND l.product_id = m.product_id))",
            [],
            |row| row.get(0),
        )
        .ok()?;
    u64::try_from(
        bad_levels
            .saturating_add(unknown_types)
            .saturating_add(orphan_movements),
    )
    .ok()
}

/// Human-scale content of the backup (`None` on SQL failure).
fn backup_content_summary(conn: &Connection) -> Option<BackupContentSummary> {
    let products: i64 = conn
        .query_row("SELECT COUNT(*) FROM products;", [], |row| row.get(0))
        .ok()?;
    let customers: i64 = conn
        .query_row("SELECT COUNT(*) FROM customers;", [], |row| row.get(0))
        .ok()?;
    let documents_total: i64 = conn
        .query_row("SELECT COUNT(*) FROM documents;", [], |row| row.get(0))
        .ok()?;
    let last_document_at: Option<String> = conn
        .query_row("SELECT MAX(created_at) FROM documents;", [], |row| {
            row.get(0)
        })
        .ok()?;
    Some(BackupContentSummary {
        products: u64::try_from(products).unwrap_or(0),
        customers: u64::try_from(customers).unwrap_or(0),
        documents_total: u64::try_from(documents_total).unwrap_or(0),
        last_document_at,
    })
}

/// Earliest still-open cash session inside the backup, if any.
///
/// `None` only on SQL failure; an empty table is a valid "no open session"
/// answer, not corruption.
fn backup_open_cash_session(conn: &Connection) -> Option<Option<RestoreWarning>> {
    match conn.query_row(
        "SELECT opened_at FROM cash_sessions WHERE status = 'OPEN'
         ORDER BY opened_at ASC LIMIT 1;",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(opened_at) => Some(Some(RestoreWarning::OpenCashSessionInBackup { opened_at })),
        Err(rusqlite::Error::QueryReturnedNoRows) => Some(None),
        Err(_) => None,
    }
}

/// UTC timestamped quarantine base name (`active-YYYYMMDD-HHMMSSZ`),
/// suffixed while it collides (bounded attempts, never panics).
fn unique_quarantine_base(quarantine_dir: &Path) -> Result<String, String> {
    let stamp = Utc::now().format("%Y%m%d-%H%M%SZ").to_string();
    for attempt in 0..1000 {
        let base = if attempt == 0 {
            format!("active-{stamp}")
        } else {
            format!("active-{stamp}-{attempt}")
        };
        if !quarantine_dir.join(format!("{base}.db")).exists() {
            return Ok(base);
        }
    }
    Err("quarantine name exhaustion".to_string())
}

/// Minimal JSON string escaper for the audit payload (no new dependency).
fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ch if (ch as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out
}

impl SqliteBackupManager {
    /// Protocol phases P1-P12: strictly read-only, zero writes anywhere.
    ///
    /// P1 loads the log row (missing aborts `BackupNotFound`); P2 stats the
    /// file (missing/empty yields safe defaults + `BackupFileMissing`);
    /// P3-P9 verify hash, integrity, foreign keys, schema order, audit
    /// chain, and the three ledgers; P10-P12 add the content summary and
    /// both warnings. Live-database failures surface as `Io`; every
    /// backup-file problem lands in `reasons` with `eligible == false`.
    fn run_verification(
        &self,
        backup_id: &str,
        dest_dir: &Path,
    ) -> Result<BackupVerificationRun, RestoreError> {
        // P1. Log row from the LIVE database.
        let row = self
            .get_backup(backup_id)
            .map_err(|_| RestoreError::Io("backup store failure".to_string()))?
            .ok_or(RestoreError::BackupNotFound)?;
        let backup_path = dest_dir.join(&row.file_name);
        let app_version = u32::try_from(MIGRATIONS.len()).unwrap_or(u32::MAX);
        let checks_all_false = || BackupVerificationRun {
            row: row.clone(),
            backup_path: backup_path.clone(),
            file_size: 0,
            sha_actual: String::new(),
            backup_version: 0,
            app_version,
            checks: blank_checks(),
            summary: blank_summary(),
            warnings: Vec::new(),
            reasons: vec![RestoreIneligibleReason::BackupFileMissing],
        };
        // P2. Stat the backup file; missing/empty fills safe defaults.
        let file_size = match std::fs::metadata(&backup_path) {
            Ok(meta) if meta.is_file() && meta.len() > 0 => meta.len(),
            _ => return Ok(checks_all_false()),
        };
        // P3. Streaming SHA-256 against the recorded binding.
        let sha_actual = self
            .sha256_of(&backup_path)
            .map_err(|_| RestoreError::Io("backup hash failure".to_string()))?;
        let mut checks = PreRestoreChecks {
            file_exists: true,
            sha256_matches: true,
            integrity_ok: true,
            foreign_keys_ok: true,
            audit_chain_ok: true,
            customer_ledger_ok: true,
            supplier_ledger_ok: true,
            stock_levels_ok: true,
        };
        let mut reasons: Vec<RestoreIneligibleReason> = Vec::new();
        if sha_actual != row.database_hash {
            checks.sha256_matches = false;
            reasons.push(RestoreIneligibleReason::HashMismatch);
        }
        // P4. Isolated READ-ONLY connection to the backup file (closed on
        // every return via drop; never written).
        let backup_conn =
            match Connection::open_with_flags(&backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
                Ok(conn) => conn,
                Err(_) => {
                    checks.integrity_ok = false;
                    checks.foreign_keys_ok = false;
                    checks.audit_chain_ok = false;
                    checks.customer_ledger_ok = false;
                    checks.supplier_ledger_ok = false;
                    checks.stock_levels_ok = false;
                    reasons.push(RestoreIneligibleReason::IntegrityCheckFailed);
                    reasons.push(RestoreIneligibleReason::ForeignKeyViolation);
                    reasons.push(RestoreIneligibleReason::AuditChainBroken);
                    reasons.push(RestoreIneligibleReason::CustomerLedgerMismatch {
                        mismatch_count: 0,
                    });
                    reasons.push(RestoreIneligibleReason::SupplierLedgerMismatch {
                        mismatch_count: 0,
                    });
                    reasons.push(RestoreIneligibleReason::StockLevelMismatch { mismatch_count: 0 });
                    return Ok(BackupVerificationRun {
                        row,
                        backup_path,
                        file_size,
                        sha_actual,
                        backup_version: 0,
                        app_version,
                        checks,
                        summary: blank_summary(),
                        warnings: Vec::new(),
                        reasons,
                    });
                }
            };
        // A backup-file query failure degrades to an integrity failure
        // (the file is not a trustworthy app database); deduped below.
        let flag_unreadable =
            |checks: &mut PreRestoreChecks, reasons: &mut Vec<RestoreIneligibleReason>| {
                checks.integrity_ok = false;
                if !reasons.contains(&RestoreIneligibleReason::IntegrityCheckFailed) {
                    reasons.push(RestoreIneligibleReason::IntegrityCheckFailed);
                }
            };
        // P5. Physical integrity.
        if !backup_integrity_ok(&backup_conn) {
            checks.integrity_ok = false;
            reasons.push(RestoreIneligibleReason::IntegrityCheckFailed);
        }
        // P6. Referential integrity.
        match backup_foreign_key_violations(&backup_conn) {
            Some(0) => {}
            Some(_) => {
                checks.foreign_keys_ok = false;
                reasons.push(RestoreIneligibleReason::ForeignKeyViolation);
            }
            None => {
                checks.foreign_keys_ok = false;
                reasons.push(RestoreIneligibleReason::ForeignKeyViolation);
                flag_unreadable(&mut checks, &mut reasons);
            }
        }
        // P7. Schema order against the running binary.
        let backup_version = match backup_schema_version(&backup_conn) {
            Some(version) => version,
            None => {
                flag_unreadable(&mut checks, &mut reasons);
                0
            }
        };
        if backup_version > app_version {
            reasons.push(RestoreIneligibleReason::SchemaNewerThanApp {
                backup_version,
                app_version,
            });
        }
        // P8. Full audit hash chain.
        if !backup_audit_chain_ok(&backup_conn) {
            checks.audit_chain_ok = false;
            reasons.push(RestoreIneligibleReason::AuditChainBroken);
        }
        // P9. Ledger reconciliation (read-only).
        match backup_customer_mismatches(&backup_conn) {
            Some(0) => {}
            Some(mismatch_count) => {
                checks.customer_ledger_ok = false;
                reasons.push(RestoreIneligibleReason::CustomerLedgerMismatch { mismatch_count });
            }
            None => {
                checks.customer_ledger_ok = false;
                flag_unreadable(&mut checks, &mut reasons);
            }
        }
        match backup_supplier_mismatches(&backup_conn) {
            Some(0) => {}
            Some(mismatch_count) => {
                checks.supplier_ledger_ok = false;
                reasons.push(RestoreIneligibleReason::SupplierLedgerMismatch { mismatch_count });
            }
            None => {
                checks.supplier_ledger_ok = false;
                flag_unreadable(&mut checks, &mut reasons);
            }
        }
        match backup_stock_mismatches(&backup_conn) {
            Some(0) => {}
            Some(mismatch_count) => {
                checks.stock_levels_ok = false;
                reasons.push(RestoreIneligibleReason::StockLevelMismatch { mismatch_count });
            }
            None => {
                checks.stock_levels_ok = false;
                flag_unreadable(&mut checks, &mut reasons);
            }
        }
        // P10. Content summary.
        let summary = match backup_content_summary(&backup_conn) {
            Some(summary) => summary,
            None => {
                flag_unreadable(&mut checks, &mut reasons);
                blank_summary()
            }
        };
        let mut warnings: Vec<RestoreWarning> = Vec::new();
        // P11. Open cash session inside the BACKUP.
        match backup_open_cash_session(&backup_conn) {
            Some(maybe_warning) => {
                if let Some(warning) = maybe_warning {
                    warnings.push(warning);
                }
            }
            None => flag_unreadable(&mut checks, &mut reasons),
        }
        drop(backup_conn);
        // Schema-older warning (advisory only; activation migrates).
        if backup_version < app_version
            && !reasons.contains(&RestoreIneligibleReason::IntegrityCheckFailed)
        {
            warnings.push(RestoreWarning::OlderSchemaWillMigrate {
                from_version: backup_version,
                to_version: app_version,
            });
        }
        // P12. Live documents newer than the backup (LIVE database probe).
        let newer_live: i64 = self
            .live_lock()?
            .query_row(
                "SELECT COUNT(*) FROM documents WHERE created_at > ?1;",
                [&row.created_at],
                |query_row| query_row.get(0),
            )
            .map_err(|_| RestoreError::Io("live document probe failure".to_string()))?;
        let newer_live = u64::try_from(newer_live).unwrap_or(0);
        if newer_live > 0 {
            warnings.push(RestoreWarning::NewerActivityWillBeQuarantined {
                document_count: newer_live,
            });
        }
        Ok(BackupVerificationRun {
            row,
            backup_path,
            file_size,
            sha_actual,
            backup_version,
            app_version,
            checks,
            summary,
            warnings,
            reasons,
        })
    }

    /// Reopen the untouched original after a pre-swap setup failure.
    ///
    /// Nothing moved yet, so the active database is byte-untouched; the
    /// manager simply resumes on it and the failure reports as plain I/O.
    fn abort_before_swap(
        &self,
        live: &mut MutexGuard<'_, Connection>,
        active: &Path,
        detail: &str,
    ) -> RestoreError {
        match open_connection(active) {
            Ok(original) => {
                **live = original;
                RestoreError::Io(detail.to_string())
            }
            Err(error) => RestoreError::SwapFailedAndRolledBack {
                detail: format!("{detail}; reopen failure: {error}"),
            },
        }
    }

    /// ROLLBACK RULE: move every quarantined file back, reopen the original
    /// database, and report the swap failure. Best-effort renames: even a
    /// partial move-back is attempted file by file before reopening.
    fn rollback_restore(
        &self,
        live: &mut MutexGuard<'_, Connection>,
        active: &Path,
        moved: &[(PathBuf, PathBuf)],
        detail: String,
    ) -> RestoreError {
        let _ = std::fs::remove_file(active);
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(PathBuf::from(format!("{}{suffix}", active.display())));
        }
        for (quarantined, original) in moved.iter().rev() {
            let _ = std::fs::rename(quarantined, original);
        }
        match open_connection(active) {
            Ok(original) => {
                **live = original;
            }
            Err(error) => {
                return RestoreError::SwapFailedAndRolledBack {
                    detail: format!("{detail}; rollback reopen failure: {error}"),
                };
            }
        }
        RestoreError::SwapFailedAndRolledBack { detail }
    }

    /// R4. ACTIVATE: open the swapped copy through the startup path, migrate
    /// when older, re-verify integrity, append the chain-linked
    /// `RESTORE_PERFORMED` audit event, checkpoint, and close.
    fn activate_copy(
        &self,
        active: &Path,
        run: &BackupVerificationRun,
        backup_id: &str,
        actor: &str,
        quarantined_db_name: &str,
    ) -> Result<String, String> {
        let mut fresh =
            open_connection(active).map_err(|error| format!("activation open failure: {error}"))?;
        // R4a. Older backups migrate through the embedded runner, untouched.
        if run.backup_version < run.app_version {
            run_migrations(&mut fresh)
                .map_err(|error| format!("activation migration failure: {error}"))?;
        }
        // R4b. Integrity of the activated copy.
        if !backup_integrity_ok(&fresh) {
            return Err("activation integrity failure".to_string());
        }
        // R4c. Chain-linked audit event continuing the restored tail.
        let restored_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let new_json = format!(
            "{{\"backup_id\":\"{}\",\"backup_sha256\":\"{}\",\"quarantined_file\":\"{}\",\"restored_at\":\"{}\"}}",
            json_escape(backup_id),
            json_escape(&run.sha_actual),
            json_escape(quarantined_db_name),
            json_escape(&restored_at),
        );
        run_transaction(&mut fresh, |tx| {
            insert_cash_audit_row(
                tx,
                "RESTORE_PERFORMED",
                "backup",
                backup_id,
                None,
                &new_json,
                actor,
                &restored_at,
            )
            .map_err(|error| InfrastructureError::Config(format!("restore audit failure: {error}")))
        })
        .map_err(|error| format!("restore audit failure: {error}"))?;
        // R4d. Checkpoint and close the activation connection.
        fresh
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|error| format!("activation checkpoint failure: {error}"))?;
        fresh
            .close()
            .map_err(|(_, error)| format!("activation close failure: {error}"))?;
        Ok(restored_at)
    }
}
