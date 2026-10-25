//! Backup hardening engine tests (TASK-BRK-092).
//!
//! Tempdir-only: schedule due/skip rules, post-close ordering + failure
//! rule, USB copy hash-check + failure surfacing, no double backup inside
//! one window. Dead codes SHIFT_CLOSE/SCHEDULED now have live callers
//! (scheduler run + close hook).

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    BackupHardeningPort, BackupRepository, Clock, SessionContext, SessionToken,
};
use baraka_application::use_cases::backup::CreateBackupUseCase;
use baraka_application::use_cases::backup_hardening::{
    CashCloseBackupUseCase, LastBackupStatusUseCase, ScheduledBackupUseCase,
};
use baraka_application::use_cases::cash::{
    BeginClosingUseCase, ConfirmClosingUseCase, OpenCashSessionUseCase,
};
use baraka_domain::backup::{BackupCadence, BackupSchedule, BackupTriggerType};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{
    SqliteBackupHardening, SqliteBackupManager, SqliteCashSessionRepository,
    SqliteWarehouseRepository,
};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const DEPOT_UUID: &str = "01950000-0000-7000-8000-000000000001";
const TEST_TERMINAL: &str = "TEST-TERMINAL-BRP";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Fixed clock for deterministic window tests.
#[derive(Clone, Copy)]
struct FixedClock {
    now: DateTime<Utc>,
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }

    fn now_rfc3339(&self) -> String {
        self.now
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    backup_dir: std::path::PathBuf,
    db: Arc<Mutex<rusqlite::Connection>>,
    backups: SqliteBackupManager,
    hardening: SqliteBackupHardening,
    cash: SqliteCashSessionRepository,
    warehouses: SqliteWarehouseRepository,
}

fn setup() -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let live_path = dir.path().join("live.db");
    let mut conn = open_connection(&live_path).expect("open");
    run_migrations(&mut conn).expect("migrate");
    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, is_active, created_at, updated_at)
         VALUES ('77777777-7777-7777-7777-777777777777', 'admin1', 'hash', 'ADMIN',
                 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("seed user");
    let backup_dir = dir.path().join("backups");
    std::fs::create_dir_all(&backup_dir).expect("backup dir");
    let shared = Arc::new(Mutex::new(conn));
    Harness {
        _dir: dir,
        backup_dir,
        db: Arc::clone(&shared),
        backups: SqliteBackupManager::new(Arc::clone(&shared), live_path),
        hardening: SqliteBackupHardening::new(Arc::clone(&shared)),
        cash: SqliteCashSessionRepository::new(Arc::clone(&shared)),
        warehouses: SqliteWarehouseRepository::new(Arc::clone(&shared)),
    }
}

fn session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::from(Uuid::parse_str(ACTOR_UUID).expect("uuid")),
        role: Role::Admin,
        expires_at: Utc::now() + chrono::Duration::hours(1),
    }
}

fn fixed_clock() -> FixedClock {
    FixedClock {
        now: Utc
            .with_ymd_and_hms(2026, 10, 25, 14, 30, 0)
            .single()
            .expect("fixed instant"),
    }
}

fn scheduler<'a>(
    h: &'a Harness,
    clock: &'a FixedClock,
) -> ScheduledBackupUseCase<'a, SqliteBackupManager, SqliteBackupHardening, FixedClock> {
    ScheduledBackupUseCase::new(&h.backups, &h.hardening, clock)
}

fn validation_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::Validation(DomainError::InvalidCanonicalString(code)) => code.clone(),
        other => panic!("expected coded validation failure, got {other:?}"),
    }
}

#[test]
fn schedule_defaults_to_disabled_and_never_due() {
    let h = setup();
    let clock = fixed_clock();
    let loaded = h.hardening.load_schedule().expect("load");
    assert_eq!(loaded, BackupSchedule::default());
    let report = scheduler(&h, &clock).check().expect("check");
    assert_eq!(report.cadence, BackupCadence::Disabled);
    assert!(!report.due);
}

#[test]
fn unknown_cadence_code_reads_back_as_disabled() {
    let h = setup();
    h.db
        .lock()
        .expect("lock")
        .execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('backup.cadence', 'hourly', '2026-01-01T00:00:00Z')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )
        .expect("poison cadence");
    let loaded = h.hardening.load_schedule().expect("load");
    assert_eq!(loaded.cadence, BackupCadence::Disabled);
}

#[test]
fn schedule_round_trip_and_daily_due_then_skip() {
    let h = setup();
    let clock = fixed_clock();
    h.hardening
        .save_schedule(&BackupSchedule {
            cadence: BackupCadence::Daily,
            time_minutes: Some(2 * 60),
        })
        .expect("save");
    let loaded = h.hardening.load_schedule().expect("load");
    assert_eq!(loaded.cadence, BackupCadence::Daily);
    assert_eq!(loaded.time_minutes, Some(120));

    // Past the 02:00 wall time, nothing succeeded yet → due.
    let before = scheduler(&h, &clock).check().expect("check");
    assert!(before.due);
    assert_eq!(before.last_backup_at, None);

    let outcome = scheduler(&h, &clock)
        .run(&session(), &h.backup_dir, 30, TEST_TERMINAL, APP_VERSION)
        .expect("scheduled run");
    assert_eq!(outcome.backup.trigger_type, BackupTriggerType::Scheduled);

    // One SUCCESS inside the window → skip (no double backup in window).
    let after = scheduler(&h, &clock).check().expect("re-check");
    assert!(!after.due);
    assert!(after.last_backup_at.is_some());
    let err = scheduler(&h, &clock)
        .run(&session(), &h.backup_dir, 30, TEST_TERMINAL, APP_VERSION)
        .expect_err("second run must be refused");
    assert_eq!(validation_code(&err), "BACKUP_NOT_DUE");

    // Exactly one local snapshot for the single due run (WAL sidecars
    // ride alongside the `.db` file — same as the manual pipeline).
    let dbs: Vec<_> = std::fs::read_dir(&h.backup_dir)
        .expect("read dir")
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.ends_with(".db").then_some(name)
            })
        })
        .collect();
    assert_eq!(dbs.len(), 1);
}

#[test]
fn weekly_due_then_skip() {
    let h = setup();
    let clock = fixed_clock();
    h.hardening
        .save_schedule(&BackupSchedule {
            cadence: BackupCadence::Weekly,
            time_minutes: Some(8 * 60),
        })
        .expect("save");
    assert!(scheduler(&h, &clock).check().expect("check").due);
    scheduler(&h, &clock)
        .run(&session(), &h.backup_dir, 30, TEST_TERMINAL, APP_VERSION)
        .expect("weekly run");
    assert!(!scheduler(&h, &clock).check().expect("re-check").due);
}

#[test]
fn due_check_is_read_only() {
    let h = setup();
    let clock = fixed_clock();
    h.hardening
        .save_schedule(&BackupSchedule {
            cadence: BackupCadence::Daily,
            time_minutes: Some(0),
        })
        .expect("save");
    let count_before: i64 =
        h.db.lock()
            .expect("lock")
            .query_row("SELECT COUNT(*) FROM backups_log", [], |row| row.get(0))
            .expect("count");
    scheduler(&h, &clock).check().expect("check");
    scheduler(&h, &clock).check().expect("check again");
    let count_after: i64 =
        h.db.lock()
            .expect("lock")
            .query_row("SELECT COUNT(*) FROM backups_log", [], |row| row.get(0))
            .expect("count");
    assert_eq!(count_before, count_after);
}

fn close_a_session(h: &Harness) -> baraka_application::ports::CashSessionView {
    let session = session();
    let depot = WarehouseId::from(Uuid::parse_str(DEPOT_UUID).expect("depot"));
    let opened = OpenCashSessionUseCase::new(&h.cash, &h.warehouses)
        .execute(&session, depot, 50_000, TEST_TERMINAL, "cmd-open-1")
        .expect("open");
    assert_eq!(opened.status, "OPEN");
    BeginClosingUseCase::new(&h.cash)
        .execute(&session, TEST_TERMINAL, "cmd-begin-1")
        .expect("begin");
    ConfirmClosingUseCase::new(&h.cash)
        .execute(
            &session,
            &opened.id,
            50_000,
            None,
            TEST_TERMINAL,
            "cmd-close-1",
        )
        .expect("confirm")
}

#[test]
fn close_hook_runs_after_close_commit_with_shift_close_backup() {
    let h = setup();
    let closed = close_a_session(&h);
    assert_eq!(closed.status, "CLOSED");
    let closed_at = closed.closed_at.clone().expect("closed_at");

    // Close hook runs AFTER the commit: the backup lands after the close.
    let meta = CashCloseBackupUseCase::new(&h.backups)
        .run_after_close(&session(), &h.backup_dir, 30, TEST_TERMINAL, APP_VERSION)
        .expect("post-close backup");
    assert_eq!(meta.trigger_type, BackupTriggerType::ShiftClose);
    assert!(meta.created_at.to_rfc3339() >= closed_at);

    let rows = h.backups.list_backups(10).expect("list");
    assert!(rows
        .iter()
        .any(|row| row.status == "SUCCESS" && row.file_name == meta.file_name));
}

#[test]
fn close_hook_failure_warns_without_rollback() {
    let h = setup();
    let closed = close_a_session(&h);
    assert_eq!(closed.status, "CLOSED");

    // Failure injection: the destination path exists as a FILE, so the
    // snapshot cannot be written — the close must still stand. The failure
    // surfaces as an error (stable warning code upstream); it never rolls
    // back the committed close.
    let blocker = h._dir.path().join("not-a-dir");
    std::fs::write(&blocker, b"block").expect("blocker");
    let _ = CashCloseBackupUseCase::new(&h.backups)
        .run_after_close(&session(), &blocker, 30, TEST_TERMINAL, APP_VERSION)
        .expect_err("backup must fail");

    // The committed close is intact: still CLOSED in the live database.
    let guard = h.db.lock().expect("lock");
    let status: String = guard
        .query_row(
            "SELECT status FROM cash_sessions WHERE id = ?1",
            [&closed.id],
            |row| row.get(0),
        )
        .expect("status");
    assert_eq!(status, "CLOSED");
    // And no SUCCESS backup was logged for the failed attempt.
    drop(guard);
    let rows = h.backups.list_backups(10).expect("list");
    assert!(!rows.iter().any(|row| row.status == "SUCCESS"));
}

#[test]
fn usb_copy_verified_and_local_snapshot_kept() {
    let h = setup();
    let meta = CreateBackupUseCase::new(&h.backups)
        .execute(
            &session(),
            BackupTriggerType::Manual,
            &h.backup_dir,
            30,
            TEST_TERMINAL,
            APP_VERSION,
        )
        .expect("manual backup");
    let source = std::path::Path::new(&meta.file_path);
    assert!(source.exists());
    let usb_dir = h._dir.path().join("usb");

    let outcome = h.hardening.copy_to_usb(source, &usb_dir).expect("usb copy");
    assert!(outcome.verified);
    assert_eq!(outcome.file_name, meta.file_name);
    assert_eq!(outcome.file_size_bytes, meta.file_size_bytes);

    // Copy, never move: the local snapshot is untouched and identical.
    assert!(source.exists());
    let copied = usb_dir.join(&meta.file_name);
    assert!(copied.exists());
    let source_bytes = std::fs::read(source).expect("read source");
    let copied_bytes = std::fs::read(&copied).expect("read copy");
    assert_eq!(source_bytes, copied_bytes);
}

#[test]
fn usb_copy_failure_surfaces_stable_codes() {
    let h = setup();
    let usb_dir = h._dir.path().join("usb");
    // Missing source file → copy fails before any write.
    let missing = h.backup_dir.join("no-such-backup.db");
    let err = h
        .hardening
        .copy_to_usb(&missing, &usb_dir)
        .expect_err("missing source");
    assert_eq!(validation_code(&err), "USB_COPY_FAILED");
    // A directory as source → copy fails before any write.
    let err = h
        .hardening
        .copy_to_usb(&h.backup_dir, &usb_dir)
        .expect_err("dir source");
    assert_eq!(validation_code(&err), "USB_COPY_FAILED");
}

#[test]
fn last_backup_status_reads_newest_row() {
    let h = setup();
    let empty = LastBackupStatusUseCase::new(&h.backups)
        .read()
        .expect("read");
    assert_eq!(empty.status, "");
    assert_eq!(empty.file_name, None);

    let meta = CreateBackupUseCase::new(&h.backups)
        .execute(
            &session(),
            BackupTriggerType::Manual,
            &h.backup_dir,
            30,
            TEST_TERMINAL,
            APP_VERSION,
        )
        .expect("manual backup");
    let status = LastBackupStatusUseCase::new(&h.backups)
        .read()
        .expect("read");
    assert_eq!(status.status, "SUCCESS");
    assert_eq!(status.file_name, Some(meta.file_name));
    assert!(status.created_at.is_some());
}
