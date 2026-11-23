//! Dual-USB mirror + 48h staleness alert (TASK-BRK-128).
//!
//! Tempdir-only. RED-first pins: a scheduled backup mirrors to BOTH
//! configured USB destinations; `LastBackupStatusUseCase::read` flags
//! `stale_48h` when the newest SUCCESS is older than 48h (or absent);
//! the post-close hook mirrors to both destinations too.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{BackupHardeningPort, Clock, SessionContext, SessionToken};
use baraka_application::use_cases::backup::CreateBackupUseCase;
use baraka_application::use_cases::backup_hardening::{
    CashCloseBackupUseCase, LastBackupStatusUseCase, ScheduledBackupUseCase,
};
use baraka_domain::backup::{BackupCadence, BackupSchedule, BackupTriggerType};
use baraka_domain::role::Role;
use baraka_domain::user::UserId;
use baraka_infrastructure::adapters::{SqliteBackupHardening, SqliteBackupManager};
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

const ACTOR_UUID: &str = "77777777-7777-7777-7777-777777777777";
const TEST_TERMINAL: &str = "TEST-TERMINAL-BR128";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    backups: SqliteBackupManager,
    hardening: SqliteBackupHardening,
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
        backups: SqliteBackupManager::new(Arc::clone(&shared), live_path),
        hardening: SqliteBackupHardening::new(Arc::clone(&shared)),
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

fn clock_at(now: DateTime<Utc>) -> FixedClock {
    FixedClock { now }
}

fn enable_daily(h: &Harness) {
    h.hardening
        .save_schedule(&BackupSchedule {
            cadence: BackupCadence::Daily,
            time_minutes: Some(2 * 60),
        })
        .expect("schedule");
}

fn db_names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.ends_with(".db").then_some(name)
            })
        })
        .collect();
    names.sort();
    names
}

#[test]
fn scheduled_backup_mirrors_to_both_usb_destinations() {
    let h = setup();
    enable_daily(&h);
    let usb1 = h._dir.path().join("usb1");
    let usb2 = h._dir.path().join("usb2");
    h.hardening.set_usb_destination(Some(&usb1)).expect("usb1");
    h.hardening
        .set_usb_second_destination(Some(&usb2))
        .expect("usb2");
    assert_eq!(h.hardening.usb_destinations().expect("list").len(), 2);

    let clock = clock_at(
        Utc.with_ymd_and_hms(2026, 10, 25, 14, 30, 0)
            .single()
            .expect("instant"),
    );
    let outcome = ScheduledBackupUseCase::new(&h.backups, &h.hardening, &clock)
        .run(&session(), &h.backup_dir, 30, TEST_TERMINAL, APP_VERSION)
        .expect("scheduled run");
    // Both mirrors land, verified, with the same file name.
    assert_eq!(outcome.usb.len(), 2);
    for copy in &outcome.usb {
        assert!(copy.verified);
    }
    assert_eq!(db_names(&usb1).len(), 1);
    assert_eq!(db_names(&usb2), db_names(&usb1));
}

#[test]
fn stale_flag_tracks_the_48h_threshold() {
    let h = setup();

    // Never backed up → stale (the alert must fire, never stay silent).
    let never = LastBackupStatusUseCase::new(&h.backups)
        .read(&clock_at(Utc::now()))
        .expect("read");
    assert_eq!(never.status, "");
    assert!(never.stale_48h);

    let before = Utc::now();
    CreateBackupUseCase::new(&h.backups)
        .execute(
            &session(),
            BackupTriggerType::Manual,
            &h.backup_dir,
            30,
            TEST_TERMINAL,
            APP_VERSION,
        )
        .expect("manual backup");

    // 47h after the SUCCESS → fresh.
    let fresh = LastBackupStatusUseCase::new(&h.backups)
        .read(&clock_at(before + chrono::Duration::hours(47)))
        .expect("read");
    assert_eq!(fresh.status, "SUCCESS");
    assert!(!fresh.stale_48h);

    // 49h after the SUCCESS → stale.
    let stale = LastBackupStatusUseCase::new(&h.backups)
        .read(&clock_at(before + chrono::Duration::hours(49)))
        .expect("read");
    assert!(stale.stale_48h);
}

#[test]
fn close_hook_mirrors_to_both_usb_destinations() {
    let h = setup();
    let usb1 = h._dir.path().join("usb1");
    let usb2 = h._dir.path().join("usb2");
    h.hardening.set_usb_destination(Some(&usb1)).expect("usb1");
    h.hardening
        .set_usb_second_destination(Some(&usb2))
        .expect("usb2");

    let (meta, copies) = CashCloseBackupUseCase::new(&h.backups, &h.hardening)
        .run_after_close(&session(), &h.backup_dir, 30, TEST_TERMINAL, APP_VERSION)
        .expect("close backup");
    assert_eq!(meta.trigger_type, BackupTriggerType::ShiftClose);
    assert_eq!(copies.len(), 2);
    assert_eq!(db_names(&usb1), db_names(&usb2));
    assert_eq!(db_names(&usb1).len(), 1);
}
