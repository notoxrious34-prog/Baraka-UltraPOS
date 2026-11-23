//! Automatic-backup hardening use cases (TASK-BRK-092).
//!
//! Scheduler + cash-close hook + USB mirror + durable last-backup read,
//! all driving the ONE `CreateBackupUseCase` pipeline (no fork). The
//! due-check is idempotent by construction: it skips when a SUCCESS backup
//! already landed inside the current cadence window. The close hook runs
//! AFTER the close transaction commits (non-blocking — a failed post-close
//! backup warns via a code, it never rolls back a committed close).

use std::path::Path;

use baraka_domain::backup::{
    BackupCadence, BackupSchedule, BackupTriggerType, LastBackupStatus, ScheduledBackupReport,
    UsbCopyOutcome,
};
use baraka_domain::errors::DomainError;
use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Utc};

use crate::errors::ApplicationError;
use crate::ports::{BackupHardeningPort, BackupRepository, Clock, SessionContext};
use crate::use_cases::backup::CreateBackupUseCase;

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// The newest SUCCESS row, if any (also used for the idempotency window).
fn newest_success<R: BackupRepository + ?Sized>(
    backups: &R,
) -> Result<Option<(String, DateTime<Utc>)>, ApplicationError> {
    let rows = backups.list_backups(1_000)?;
    for row in rows {
        if row.status == "SUCCESS" {
            let parsed = DateTime::parse_from_rfc3339(&row.created_at)
                .ok()
                .map(|dt| dt.with_timezone(&Utc));
            if let Some(instant) = parsed {
                return Ok(Some((row.file_name, instant)));
            }
        }
    }
    Ok(None)
}

/// Start of the current cadence window (absolute UTC).
///
/// Daily: today at the configured local wall-clock time (or today 00:00 when
/// the time is absent). Weekly: Monday at the configured time (or Monday
/// 00:00). Anything before the window start is "an earlier window".
fn window_start(schedule: &BackupSchedule, now: DateTime<Utc>) -> DateTime<Utc> {
    let local_now = Local.from_utc_datetime(&now.naive_utc());
    let date = local_now.date_naive();
    let date = if schedule.cadence == BackupCadence::Weekly {
        let days_from_monday = date.weekday().num_days_from_monday();
        date - Duration::days(i64::from(days_from_monday))
    } else {
        date
    };
    let (h, m) = match schedule.time_minutes {
        Some(minutes) => (minutes / 60, minutes % 60),
        None => (0, 0),
    };
    let wall = date
        .and_hms_opt(h, m, 0)
        .expect("schedule time is validated");
    Local
        .from_local_datetime(&wall)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now)
}

/// Next run instant: the window start, or tomorrow/Monday when already past.
fn next_run(schedule: &BackupSchedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if schedule.cadence == BackupCadence::Disabled {
        return None;
    }
    let start = window_start(schedule, now);
    if start <= now {
        let step = if schedule.cadence == BackupCadence::Weekly {
            Duration::days(7)
        } else {
            Duration::days(1)
        };
        Some(start + step)
    } else {
        Some(start)
    }
}

/// Result of one automatic-backup attempt: the verified snapshot plus the
/// dual-USB mirror outcomes (TASK-BRK-128, best-effort, never-blocking).
#[derive(Debug)]
pub struct ScheduledRunOutcome {
    /// Verified local snapshot metadata.
    pub backup: baraka_domain::backup::BackupMetadata,
    /// USB mirror outcomes, one per configured destination (empty when none).
    pub usb: Vec<UsbCopyOutcome>,
}

/// Scheduled-backup use case (cadence due/skip + capture + USB mirror).
pub struct ScheduledBackupUseCase<'a, R, H, C>
where
    R: ?Sized,
    H: ?Sized,
    C: ?Sized,
{
    backups: &'a R,
    hardening: &'a H,
    clock: &'a C,
}

impl<'a, R, H, C> ScheduledBackupUseCase<'a, R, H, C>
where
    R: BackupRepository + ?Sized,
    H: BackupHardeningPort + ?Sized,
    C: Clock + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(backups: &'a R, hardening: &'a H, clock: &'a C) -> Self {
        Self {
            backups,
            hardening,
            clock,
        }
    }

    /// Read-only due check (zero writes): is a scheduled backup due now?
    pub fn check(&self) -> Result<ScheduledBackupReport, ApplicationError> {
        let schedule = self.hardening.load_schedule()?;
        let now = self.clock.now();
        let last_success = newest_success(self.backups)?;
        let start = window_start(&schedule, now);
        // Idempotency rule: a SUCCESS backup inside the current window
        // satisfies the cadence — skip (no double backup per window).
        let due = match schedule.cadence {
            BackupCadence::Disabled => false,
            BackupCadence::Daily | BackupCadence::Weekly => {
                now >= start
                    && last_success
                        .as_ref()
                        .map(|(_, at)| *at < start)
                        .unwrap_or(true)
            }
        };
        Ok(ScheduledBackupReport {
            cadence: schedule.cadence,
            time_minutes: schedule.time_minutes,
            due,
            last_backup_at: last_success
                .as_ref()
                .map(|(_, at)| at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
            next_run_at: next_run(&schedule, now)
                .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
        })
    }

    /// Run a scheduled backup when due, then mirror to USB when configured.
    ///
    /// The capture reuses `CreateBackupUseCase` verbatim (single pipeline).
    /// USB copy is best-effort: a failed/mismatched copy is reported in the
    /// outcome but never invalidates the already-verified local snapshot.
    pub fn run(
        &self,
        session: &SessionContext,
        dest_dir: &Path,
        retention: usize,
        terminal_id: &str,
        app_version: &str,
    ) -> Result<ScheduledRunOutcome, ApplicationError> {
        let report = self.check()?;
        if !report.due {
            return Err(coded("BACKUP_NOT_DUE"));
        }
        let backup = CreateBackupUseCase::new(self.backups).execute(
            session,
            BackupTriggerType::Scheduled,
            dest_dir,
            retention,
            terminal_id,
            app_version,
        )?;
        // Dual mirror (TASK-BRK-128): every configured destination gets a
        // copy; each copy is best-effort and never invalidates the snapshot.
        let source = std::path::Path::new(&backup.file_path);
        let mut mirrors: Vec<UsbCopyOutcome> = Vec::new();
        for destination in self.hardening.usb_destinations()? {
            if let Ok(outcome) = self.hardening.copy_to_usb(source, &destination) {
                mirrors.push(outcome);
            }
        }
        Ok(ScheduledRunOutcome {
            backup,
            usb: mirrors,
        })
    }

    /// On-launch due-check entry point (no operator session exists yet).
    ///
    /// Builds a synthetic system session (the capture pipeline does not
    /// consult the session — it records only the terminal id) and runs a
    /// due backup. Not-due is a silent no-op, never a startup failure.
    pub fn run_system(
        &self,
        dest_dir: &Path,
        retention: usize,
        terminal_id: &str,
        app_version: &str,
    ) -> Result<(), ApplicationError> {
        if !self.check()?.due {
            return Ok(());
        }
        let session = SessionContext {
            token: crate::ports::SessionToken::new_random(),
            user_id: baraka_domain::user::UserId::new(),
            role: baraka_domain::role::Role::Admin,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        };
        self.run(&session, dest_dir, retention, terminal_id, app_version)
            .map(|_| ())
    }
}

/// Post-close automatic backup (TASK-BRK-092, dual mirror TASK-BRK-128).
///
/// Runs AFTER the close transaction commits: the close must never be rolled
/// back or reported failed because the automatic backup could not run. A
/// failed post-close backup returns a stable warning code and leaves a
/// FAILED `backups_log` row — it is visible, never silent.
pub struct CashCloseBackupUseCase<'a, R, H>
where
    R: ?Sized,
    H: ?Sized,
{
    backups: &'a R,
    hardening: &'a H,
}

impl<'a, R, H> CashCloseBackupUseCase<'a, R, H>
where
    R: BackupRepository + ?Sized,
    H: BackupHardeningPort + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(backups: &'a R, hardening: &'a H) -> Self {
        Self { backups, hardening }
    }

    /// Capture a verified SHIFT_CLOSE backup after the close committed,
    /// then mirror to every configured USB destination (best-effort).
    pub fn run_after_close(
        &self,
        session: &SessionContext,
        dest_dir: &Path,
        retention: usize,
        terminal_id: &str,
        app_version: &str,
    ) -> Result<(baraka_domain::backup::BackupMetadata, Vec<UsbCopyOutcome>), ApplicationError>
    {
        let backup = CreateBackupUseCase::new(self.backups).execute(
            session,
            BackupTriggerType::ShiftClose,
            dest_dir,
            retention,
            terminal_id,
            app_version,
        )?;
        let source = std::path::Path::new(&backup.file_path);
        let mut mirrors: Vec<UsbCopyOutcome> = Vec::new();
        for destination in self.hardening.usb_destinations()? {
            if let Ok(outcome) = self.hardening.copy_to_usb(source, &destination) {
                mirrors.push(outcome);
            }
        }
        Ok((backup, mirrors))
    }
}

/// Durable last-backup read (replaces volatile success notes).
pub struct LastBackupStatusUseCase<'a, R>
where
    R: ?Sized,
{
    backups: &'a R,
}

impl<'a, R> LastBackupStatusUseCase<'a, R>
where
    R: BackupRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(backups: &'a R) -> Self {
        Self { backups }
    }

    /// Newest logged row (any status) as durable UI state, plus the 48h
    /// staleness flag computed against the newest SUCCESS row
    /// (TASK-BRK-128 — never-backed-up counts as stale).
    pub fn read<C>(&self, clock: &C) -> Result<LastBackupStatus, ApplicationError>
    where
        C: Clock + ?Sized,
    {
        let stale = baraka_domain::backup::backup_is_stale(
            newest_success(self.backups)?.map(|(_, at)| at),
            clock.now(),
        );
        let rows = self.backups.list_backups(1)?;
        match rows.into_iter().next() {
            None => Ok(LastBackupStatus {
                status: String::new(),
                file_name: None,
                created_at: None,
                error_message: None,
                stale_48h: stale,
            }),
            Some(row) => Ok(LastBackupStatus {
                status: row.status,
                file_name: Some(row.file_name),
                created_at: Some(row.created_at),
                error_message: None,
                stale_48h: stale,
            }),
        }
    }
}
