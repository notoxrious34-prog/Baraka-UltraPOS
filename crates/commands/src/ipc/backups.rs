//! Backup and recovery commands: snapshots, restore, factory reset, hardening.
use super::{error_code, require_session, AppState};
use baraka_application::errors::ApplicationError;
use baraka_application::ports::SessionContext;
use baraka_application::use_cases::backup::{
    CreateBackupUseCase, ListBackupsUseCase, PrecheckRestoreUseCase, RestoreBackupUseCase,
    VerifyBackupIntegrityUseCase,
};
use baraka_application::use_cases::backup_hardening::{
    LastBackupStatusUseCase, ScheduledBackupUseCase,
};
use baraka_application::use_cases::factory_reset::{
    ExecuteFactoryResetUseCase, PrecheckFactoryResetUseCase, ResetExecution,
};
use baraka_domain::errors::DomainError;
use baraka_domain::role::Permission;

use crate::dto::{
    BackupContentSummaryDto, BackupHardeningConfigureRequestDto, BackupHardeningStatusDto,
    BackupMetadataDto, BackupScheduleDto, CreateBackupRequestDto, FactoryResetExecuteRequestDto,
    FactoryResetOutcomeDto, FactoryResetPrecheckRequestDto, LastBackupStatusDto,
    PreRestoreChecksDto, PreRestoreReportDto, PreRestoreRequestDto, ResetBackupRefDto,
    ResetPrecheckDto, RestoreBackupRequestDto, RestoreIneligibleReasonDto, RestoreOutcomeDto,
    RestoreWarningDto, ScheduledBackupReportDto, ScheduledRunOutcomeDto, UsbCopyOutcomeDto,
    VerifyBackupRequestDto, VerifyBackupResponseDto,
};

/// Require the system-settings grant (backup commands are Admin-only).
fn require_backup_admin(session: &SessionContext) -> Result<(), String> {
    if session
        .role
        .has_permission(Permission::SystemSettingsManage)
    {
        Ok(())
    } else {
        Err(error_code(&ApplicationError::Unauthorized))
    }
}

/// Map domain backup metadata to its wire view.
fn to_backup_dto(meta: &baraka_domain::backup::BackupMetadata) -> BackupMetadataDto {
    BackupMetadataDto {
        id: meta.id.to_string(),
        file_name: meta.file_name.clone(),
        file_size_bytes: meta.file_size_bytes,
        sha256_checksum: meta.sha256_checksum.clone(),
        trigger_type: match meta.trigger_type {
            baraka_domain::backup::BackupTriggerType::Manual => "MANUAL",
            baraka_domain::backup::BackupTriggerType::ShiftClose => "SHIFT_CLOSE",
            baraka_domain::backup::BackupTriggerType::Scheduled => "SCHEDULED",
            baraka_domain::backup::BackupTriggerType::Reset => "RESET",
        }
        .to_string(),
        status: match meta.status {
            baraka_domain::backup::BackupStatus::Success => "SUCCESS",
            baraka_domain::backup::BackupStatus::Failed => "FAILED",
            baraka_domain::backup::BackupStatus::Corrupted => "CORRUPTED",
        }
        .to_string(),
        schema_version: meta.schema_version,
        error_message: meta.error_message.clone(),
        created_at: meta
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }
}

/// `backups_create` command: capture a verified online snapshot
/// (TASK-BRK-052, ADR-010). Admin-only; retention keeps the newest 30.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_create(
    state: tauri::State<'_, AppState>,
    request: CreateBackupRequestDto,
) -> Result<BackupMetadataDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let trigger = match request.trigger_type.as_deref().map(str::trim) {
        None | Some("") | Some("MANUAL") => baraka_domain::backup::BackupTriggerType::Manual,
        Some("SHIFT_CLOSE") => baraka_domain::backup::BackupTriggerType::ShiftClose,
        Some("SCHEDULED") => baraka_domain::backup::BackupTriggerType::Scheduled,
        Some("RESET") => baraka_domain::backup::BackupTriggerType::Reset,
        Some(_) => {
            return Err(error_code(&ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_TRIGGER_TYPE".to_string()),
            )));
        }
    };
    let meta = CreateBackupUseCase::new(state.backups.as_ref())
        .execute(
            &session,
            trigger,
            &state.backup_dir,
            30,
            &state.terminal_id,
            env!("CARGO_PKG_VERSION"),
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_backup_dto(&meta))
}

/// `backups_list` command: recent backup audit rows, newest first.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<BackupMetadataDto>, String> {
    let session = require_session(&state, &session_token)?;
    require_backup_admin(&session)?;
    let rows = ListBackupsUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, 100)
        .map_err(|error| error_code(&error))?;
    Ok(rows.iter().map(to_backup_dto).collect())
}

/// `backups_verify` command: re-verify one logged snapshot on disk.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_verify(
    state: tauri::State<'_, AppState>,
    request: VerifyBackupRequestDto,
) -> Result<VerifyBackupResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let meta = VerifyBackupIntegrityUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, &request.backup_id)
        .map_err(|error| error_code(&error))?;
    Ok(VerifyBackupResponseDto {
        backup_id: request.backup_id,
        is_valid: true,
        sha256_checksum: meta.sha256_checksum,
        status: match meta.status {
            baraka_domain::backup::BackupStatus::Success => "SUCCESS",
            baraka_domain::backup::BackupStatus::Failed => "FAILED",
            baraka_domain::backup::BackupStatus::Corrupted => "CORRUPTED",
        }
        .to_string(),
        message: None,
    })
}

/// Map a typed restore failure to a stable, non-leaking wire code.
///
/// Payload-bearing variants collapse to their stable code: counts, versions,
/// and I/O detail never cross the IPC boundary (the precheck report already
/// carries the rich per-gate detail for eligible/ineligible display).
fn restore_error_code(error: &baraka_domain::backup::RestoreError) -> String {
    use baraka_domain::backup::RestoreError;
    match error {
        RestoreError::Unauthorized => "UNAUTHORIZED".to_string(),
        RestoreError::BackupNotFound => "BACKUP_NOT_FOUND".to_string(),
        RestoreError::BackupFileMissing => "BACKUP_MISSING".to_string(),
        RestoreError::BackupTampered => "BACKUP_CHECKSUM_MISMATCH".to_string(),
        RestoreError::BackupCorrupted => "BACKUP_CORRUPTED".to_string(),
        RestoreError::BackupFkViolation => "BACKUP_FK_VIOLATION".to_string(),
        RestoreError::BackupSchemaNewer { .. } => "BACKUP_SCHEMA_NEWER".to_string(),
        RestoreError::BackupAuditChainBroken => "BACKUP_AUDIT_CHAIN_BROKEN".to_string(),
        RestoreError::BackupLedgerInconsistent { .. } => "BACKUP_LEDGER_INCONSISTENT".to_string(),
        RestoreError::RestoreInProgress => "RESTORE_IN_PROGRESS".to_string(),
        RestoreError::SwapFailedAndRolledBack { .. } => "RESTORE_SWAP_FAILED".to_string(),
        RestoreError::Io(_) => "INTERNAL_ERROR".to_string(),
    }
}

/// Map a domain restore warning to its wire view.
fn to_restore_warning_dto(warning: &baraka_domain::backup::RestoreWarning) -> RestoreWarningDto {
    match warning {
        baraka_domain::backup::RestoreWarning::OpenCashSessionInBackup { opened_at } => {
            RestoreWarningDto::OpenCashSessionInBackup {
                opened_at: opened_at.clone(),
            }
        }
        baraka_domain::backup::RestoreWarning::OlderSchemaWillMigrate {
            from_version,
            to_version,
        } => RestoreWarningDto::OlderSchemaWillMigrate {
            from_version: *from_version,
            to_version: *to_version,
        },
        baraka_domain::backup::RestoreWarning::NewerActivityWillBeQuarantined {
            document_count,
        } => RestoreWarningDto::NewerActivityWillBeQuarantined {
            document_count: *document_count,
        },
    }
}

/// Map a domain ineligibility reason to its wire view.
fn to_ineligible_reason_dto(
    reason: &baraka_domain::backup::RestoreIneligibleReason,
) -> RestoreIneligibleReasonDto {
    use baraka_domain::backup::RestoreIneligibleReason;
    match reason {
        RestoreIneligibleReason::BackupFileMissing => RestoreIneligibleReasonDto::BackupFileMissing,
        RestoreIneligibleReason::HashMismatch => RestoreIneligibleReasonDto::HashMismatch,
        RestoreIneligibleReason::IntegrityCheckFailed => {
            RestoreIneligibleReasonDto::IntegrityCheckFailed
        }
        RestoreIneligibleReason::ForeignKeyViolation => {
            RestoreIneligibleReasonDto::ForeignKeyViolation
        }
        RestoreIneligibleReason::AuditChainBroken => RestoreIneligibleReasonDto::AuditChainBroken,
        RestoreIneligibleReason::CustomerLedgerMismatch { mismatch_count } => {
            RestoreIneligibleReasonDto::CustomerLedgerMismatch {
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::SupplierLedgerMismatch { mismatch_count } => {
            RestoreIneligibleReasonDto::SupplierLedgerMismatch {
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::StockLevelMismatch { mismatch_count } => {
            RestoreIneligibleReasonDto::StockLevelMismatch {
                mismatch_count: *mismatch_count,
            }
        }
        RestoreIneligibleReason::SchemaNewerThanApp {
            backup_version,
            app_version,
        } => RestoreIneligibleReasonDto::SchemaNewerThanApp {
            backup_version: *backup_version,
            app_version: *app_version,
        },
    }
}

/// Map a domain precheck report to its wire view.
///
/// Counts and sizes cross as numbers (they are not financial values);
/// timestamps cross as RFC3339 UTC strings.
fn to_pre_restore_report_dto(
    report: &baraka_domain::backup::PreRestoreReport,
) -> PreRestoreReportDto {
    PreRestoreReportDto {
        backup_id: report.backup_id.clone(),
        file_name: report.file_name.clone(),
        file_size_bytes: report.file_size_bytes,
        created_at: report.created_at.clone(),
        schema_version_backup: report.schema_version_backup,
        schema_version_app: report.schema_version_app,
        checks: PreRestoreChecksDto {
            file_exists: report.checks.file_exists,
            sha256_matches: report.checks.sha256_matches,
            integrity_ok: report.checks.integrity_ok,
            foreign_keys_ok: report.checks.foreign_keys_ok,
            audit_chain_ok: report.checks.audit_chain_ok,
            customer_ledger_ok: report.checks.customer_ledger_ok,
            supplier_ledger_ok: report.checks.supplier_ledger_ok,
            stock_levels_ok: report.checks.stock_levels_ok,
        },
        content_summary: BackupContentSummaryDto {
            products: report.content_summary.products,
            customers: report.content_summary.customers,
            documents_total: report.content_summary.documents_total,
            last_document_at: report.content_summary.last_document_at.clone(),
        },
        warnings: report.warnings.iter().map(to_restore_warning_dto).collect(),
        eligible: report.eligible,
        ineligible_reasons: report
            .ineligible_reasons
            .iter()
            .map(to_ineligible_reason_dto)
            .collect(),
    }
}

/// `backups_restore_precheck` command: full read-only verification report
/// for one logged backup (TASK-BRK-054, ADR-062). Admin-only; advisory only.
#[tauri::command(rename_all = "snake_case")]
pub fn backups_restore_precheck(
    state: tauri::State<'_, AppState>,
    request: PreRestoreRequestDto,
) -> Result<PreRestoreReportDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let report = PrecheckRestoreUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, &request.backup_id)
        .map_err(|error| restore_error_code(&error))?;
    Ok(to_pre_restore_report_dto(&report))
}

/// `backups_restore` command: verified destructive restore with quarantine
/// and rollback safety (TASK-BRK-054, ADR-062). Admin-only. The destructive
/// path re-verifies everything internally; nothing is written to
/// `command_log` (the replaced database would record it — the audit trail
/// is the `RESTORE_PERFORMED` event plus the preserved quarantine copy).
#[tauri::command(rename_all = "snake_case")]
pub fn backups_restore(
    state: tauri::State<'_, AppState>,
    request: RestoreBackupRequestDto,
) -> Result<RestoreOutcomeDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let outcome = RestoreBackupUseCase::new(state.backups.as_ref())
        .execute(&session, &state.backup_dir, &request.backup_id)
        .map_err(|error| restore_error_code(&error))?;
    Ok(RestoreOutcomeDto {
        restored_backup_id: outcome.restored_backup_id,
        quarantined_to: outcome.quarantined_to,
        restored_at: outcome.restored_at,
        restart_required: outcome.restart_required,
    })
}

/// `factory_reset_precheck` command: read-only reset gates (TASK-BRK-091).
/// Admin-only (`SystemSettingsManage`, same gate as restore). Zero writes.
#[tauri::command(rename_all = "snake_case")]
pub fn factory_reset_precheck(
    state: tauri::State<'_, AppState>,
    request: FactoryResetPrecheckRequestDto,
) -> Result<ResetPrecheckDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let report = PrecheckFactoryResetUseCase::new(state.backups.as_ref(), state.cash.as_ref())
        .execute(&session, &state.terminal_id, &state.backup_dir)
        .map_err(|error| error_code(&error))?;
    Ok(ResetPrecheckDto {
        open_session: report.open_session,
        backup_dir_writable: report.backup_dir_writable,
        newest_backup: report.newest_backup.map(|row| ResetBackupRefDto {
            file_name: row.file_name,
            created_at: row.created_at,
            status: row.status,
        }),
        eligible: report.eligible,
        reasons: report.reasons,
    })
}

/// `factory_reset_execute` command: verified destructive reset (TASK-BRK-091).
/// Admin-only. Validation (session, permission, typed token) runs before any
/// write; the mandatory verified backup runs before the quarantine-swap.
/// Nothing is written to `command_log` (the replaced database would record
/// it — the audit trail is the `RESET_PERFORMED` event in the fresh database
/// plus the preserved quarantine copy and the pre-reset backup).
#[tauri::command(rename_all = "snake_case")]
pub fn factory_reset_execute(
    state: tauri::State<'_, AppState>,
    request: FactoryResetExecuteRequestDto,
) -> Result<FactoryResetOutcomeDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let outcome = ExecuteFactoryResetUseCase::new(
        state.backups.as_ref(),
        state.cash.as_ref(),
        state.factory_reset.as_ref(),
    )
    .execute(
        &session,
        &ResetExecution {
            terminal_id: &state.terminal_id,
            backup_dir: &state.backup_dir,
            retention: 30,
            app_version: env!("CARGO_PKG_VERSION"),
            typed_token: &request.typed_token,
            actor: &session.user_id.to_string(),
        },
    )
    .map_err(|error| error_code(&error))?;
    Ok(FactoryResetOutcomeDto {
        reset_backup_file_name: outcome.reset_backup_file_name,
        quarantined_to: outcome.quarantined_db_name,
        reset_at: outcome.reset_at,
        restart_required: outcome.restart_required,
    })
}

fn to_backup_schedule_dto(schedule: &baraka_domain::backup::BackupSchedule) -> BackupScheduleDto {
    BackupScheduleDto {
        cadence: schedule.cadence.as_code().to_string(),
        time: schedule
            .time_minutes
            .and_then(baraka_domain::backup::BackupSchedule::format_time),
    }
}

fn to_scheduled_report_dto(
    report: &baraka_domain::backup::ScheduledBackupReport,
) -> ScheduledBackupReportDto {
    ScheduledBackupReportDto {
        cadence: report.cadence.as_code().to_string(),
        time: report
            .time_minutes
            .and_then(baraka_domain::backup::BackupSchedule::format_time),
        due: report.due,
        last_backup_at: report.last_backup_at.clone(),
        next_run_at: report.next_run_at.clone(),
    }
}

fn to_last_status_dto(status: &baraka_domain::backup::LastBackupStatus) -> LastBackupStatusDto {
    LastBackupStatusDto {
        status: status.status.clone(),
        file_name: status.file_name.clone(),
        created_at: status.created_at.clone(),
        error_message: status.error_message.clone(),
    }
}

fn to_usb_outcome_dto(outcome: &baraka_domain::backup::UsbCopyOutcome) -> UsbCopyOutcomeDto {
    UsbCopyOutcomeDto {
        destination: outcome.destination.clone(),
        file_name: outcome.file_name.clone(),
        file_size_bytes: outcome.file_size_bytes,
        verified: outcome.verified,
    }
}

/// `backup_hardening_status` command: stored schedule + USB destination +
/// durable last-backup state + due-check report (TASK-BRK-092).
/// Admin-only, read-only (zero writes).
#[tauri::command(rename_all = "snake_case")]
pub fn backup_hardening_status(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<BackupHardeningStatusDto, String> {
    let session = require_session(&state, &session_token)?;
    require_backup_admin(&session)?;
    let schedule = state
        .backup_hardening
        .as_ref()
        .load_schedule()
        .map_err(|error| error_code(&error))?;
    let usb_destination = state
        .backup_hardening
        .as_ref()
        .usb_destination()
        .map_err(|error| error_code(&error))?
        .map(|path| path.to_string_lossy().to_string());
    let last_status = LastBackupStatusUseCase::new(state.backups.as_ref())
        .read()
        .map_err(|error| error_code(&error))?;
    let due = ScheduledBackupUseCase::new(
        state.backups.as_ref(),
        state.backup_hardening.as_ref(),
        state.clock.as_ref(),
    )
    .check()
    .map_err(|error| error_code(&error))?;
    Ok(BackupHardeningStatusDto {
        schedule: to_backup_schedule_dto(&schedule),
        usb_destination,
        last_status: to_last_status_dto(&last_status),
        due: to_scheduled_report_dto(&due),
    })
}

/// `backup_hardening_configure` command: persist cadence + time + USB
/// destination (TASK-BRK-092). Admin-only. Cadence validates to
/// `disabled`/`daily`/`weekly` (`INVALID_BACKUP_CADENCE`); non-disabled
/// cadences require a valid `HH:MM` (`INVALID_BACKUP_TIME`); a blank USB
/// destination clears the setting.
#[tauri::command(rename_all = "snake_case")]
pub fn backup_hardening_configure(
    state: tauri::State<'_, AppState>,
    request: BackupHardeningConfigureRequestDto,
) -> Result<BackupScheduleDto, String> {
    let session = require_session(&state, &request.session_token)?;
    require_backup_admin(&session)?;
    let cadence = baraka_domain::backup::BackupCadence::from_code(request.cadence.trim())
        .ok_or_else(|| {
            error_code(&ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_BACKUP_CADENCE".to_string()),
            ))
        })?;
    let time_minutes = match cadence {
        baraka_domain::backup::BackupCadence::Disabled => None,
        _ => {
            let raw = request.time.as_deref().map(str::trim).unwrap_or("");
            if raw.is_empty() {
                return Err(error_code(&ApplicationError::Validation(
                    DomainError::InvalidCanonicalString("INVALID_BACKUP_TIME".to_string()),
                )));
            }
            Some(
                baraka_domain::backup::BackupSchedule::parse_time(raw).ok_or_else(|| {
                    error_code(&ApplicationError::Validation(
                        DomainError::InvalidCanonicalString("INVALID_BACKUP_TIME".to_string()),
                    ))
                })?,
            )
        }
    };
    let schedule = baraka_domain::backup::BackupSchedule {
        cadence,
        time_minutes,
    };
    state
        .backup_hardening
        .as_ref()
        .save_schedule(&schedule)
        .map_err(|error| error_code(&error))?;
    let usb = request
        .usb_destination
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    state
        .backup_hardening
        .as_ref()
        .set_usb_destination(usb.map(std::path::Path::new))
        .map_err(|error| error_code(&error))?;
    Ok(to_backup_schedule_dto(&schedule))
}

/// `backup_schedule_run` command: on-action due-check entry point
/// (TASK-BRK-092). Admin-only. Runs one scheduled capture (same pipeline as
/// manual, `Scheduled` trigger) plus the best-effort USB mirror when the
/// cadence is due; `BACKUP_NOT_DUE` when the window already holds a SUCCESS
/// backup.
#[tauri::command(rename_all = "snake_case")]
pub fn backup_schedule_run(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<ScheduledRunOutcomeDto, String> {
    let session = require_session(&state, &session_token)?;
    require_backup_admin(&session)?;
    let outcome = ScheduledBackupUseCase::new(
        state.backups.as_ref(),
        state.backup_hardening.as_ref(),
        state.clock.as_ref(),
    )
    .run(
        &session,
        &state.backup_dir,
        30,
        &state.terminal_id,
        env!("CARGO_PKG_VERSION"),
    )
    .map_err(|error| error_code(&error))?;
    Ok(ScheduledRunOutcomeDto {
        backup: to_backup_dto(&outcome.backup),
        usb: outcome.usb.as_ref().map(to_usb_outcome_dto),
    })
}
