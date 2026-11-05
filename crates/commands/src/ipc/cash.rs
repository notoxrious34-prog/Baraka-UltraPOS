//! Cash commands: sessions, closings, expenses, reports.
use super::{error_code, parse_wire_id, parse_wire_int, require_session, AppState};
use baraka_application::use_cases::backup_hardening::CashCloseBackupUseCase;
use baraka_application::use_cases::cash::{
    BeginClosingUseCase, CancelClosingUseCase, ConfirmClosingUseCase, GetActiveSessionUseCase,
    GetSessionReportUseCase, OpenCashSessionUseCase, RecordExpenseUseCase,
};
use baraka_domain::inventory::WarehouseId;

use crate::dto::{
    CashClosingCommandDto, CashMovementDto, CashMovementTotalDto, CashSessionDto,
    CashSessionReportDto, ConfirmClosingRequestDto, GetSessionReportRequestDto,
    OpenCashSessionRequestDto, RecordExpenseRequestDto,
};

fn to_cash_session_dto(view: &baraka_application::ports::CashSessionView) -> CashSessionDto {
    CashSessionDto {
        id: view.id.clone(),
        warehouse_id: view.warehouse_id.clone(),
        terminal_id: view.terminal_id.clone(),
        status: view.status.clone(),
        opened_by: view.opened_by.clone(),
        opened_by_username: view.opened_by_username.clone(),
        opened_at: view.opened_at.clone(),
        opening_float_centimes: view.opening_float_centimes.to_string(),
        closed_by: view.closed_by.clone(),
        closed_by_username: view.closed_by_username.clone(),
        closed_at: view.closed_at.clone(),
        actual_counted_centimes: view.actual_counted_centimes.map(|v| v.to_string()),
        system_expected_centimes: view.system_expected_centimes.map(|v| v.to_string()),
        variance_centimes: view.variance_centimes.map(|v| v.to_string()),
        closing_notes: view.closing_notes.clone(),
        revision: view.revision.to_string(),
        created_at: view.created_at.clone(),
        updated_at: view.updated_at.clone(),
        backup_warning: None,
    }
}

fn to_cash_movement_dto(view: &baraka_application::ports::CashMovementView) -> CashMovementDto {
    CashMovementDto {
        id: view.id.clone(),
        cash_session_id: view.cash_session_id.clone(),
        movement_type: view.movement_type.clone(),
        amount_centimes: view.amount_centimes.to_string(),
        reference_type: view.reference_type.clone(),
        reference_id: view.reference_id.clone(),
        description: view.description.clone(),
        created_at: view.created_at.clone(),
        created_by: view.created_by.clone(),
        created_by_username: view.created_by_username.clone(),
    }
}

/// `cash.open` command: open a session with an opening float.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_open(
    state: tauri::State<'_, AppState>,
    request: OpenCashSessionRequestDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let warehouse_id = parse_wire_id(&request.warehouse_id, "warehouse_id")
        .map(WarehouseId::from)
        .map_err(|error| error_code(&error))?;
    let view = OpenCashSessionUseCase::new(state.cash.as_ref(), state.warehouses.as_ref())
        .execute(
            &session,
            warehouse_id,
            parse_wire_int(&request.opening_float_centimes, "opening_float_centimes")
                .map_err(|error| error_code(&error))?,
            &state.terminal_id,
            &request.command_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_session_dto(&view))
}

/// `cash.beginClosing` command: freeze the expected total (OPEN -> CLOSING).
#[tauri::command(rename_all = "snake_case")]
pub fn cash_begin_closing(
    state: tauri::State<'_, AppState>,
    request: CashClosingCommandDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = BeginClosingUseCase::new(state.cash.as_ref())
        .execute(&session, &state.terminal_id, &request.command_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_session_dto(&view))
}

/// `cash.cancelClosing` command: revert CLOSING -> OPEN.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_cancel_closing(
    state: tauri::State<'_, AppState>,
    request: CashClosingCommandDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = CancelClosingUseCase::new(state.cash.as_ref())
        .execute(&session, &state.terminal_id, &request.command_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_session_dto(&view))
}

/// `cash.confirmClosing` command: count and seal (CLOSING -> CLOSED).
#[tauri::command(rename_all = "snake_case")]
pub fn cash_confirm_closing(
    state: tauri::State<'_, AppState>,
    request: ConfirmClosingRequestDto,
) -> Result<CashSessionDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = ConfirmClosingUseCase::new(state.cash.as_ref())
        .execute(
            &session,
            &request.session_id,
            parse_wire_int(&request.actual_counted_centimes, "actual_counted_centimes")
                .map_err(|error| error_code(&error))?,
            request.closing_notes,
            &state.terminal_id,
            &request.command_id,
        )
        .map_err(|error| error_code(&error))?;
    // TASK-BRK-092 close hook: AFTER the close commits (never before — a
    // stranded till is worse than a missed automatic backup). A failed
    // post-close backup surfaces as a non-blocking `backup_warning` code;
    // the committed close is never rolled back.
    let backup_warning = match CashCloseBackupUseCase::new(state.backups.as_ref()).run_after_close(
        &session,
        &state.backup_dir,
        30,
        &state.terminal_id,
        env!("CARGO_PKG_VERSION"),
    ) {
        Ok(_) => None,
        Err(error) => Some(error_code(&error)),
    };
    let mut dto = to_cash_session_dto(&view);
    dto.backup_warning = backup_warning;
    Ok(dto)
}

/// `cash.getActiveSession` command: active OPEN/CLOSING session, if any.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_get_active_session(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Option<CashSessionDto>, String> {
    let session = require_session(&state, &session_token)?;
    let view = GetActiveSessionUseCase::new(state.cash.as_ref())
        .execute(&session, &state.terminal_id)
        .map_err(|error| error_code(&error))?;
    Ok(view.as_ref().map(to_cash_session_dto))
}

/// `cash.recordExpense` command: append an EXPENSE_OUT movement.
#[tauri::command(rename_all = "snake_case")]
pub fn cash_record_expense(
    state: tauri::State<'_, AppState>,
    request: RecordExpenseRequestDto,
) -> Result<CashMovementDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = RecordExpenseUseCase::new(state.cash.as_ref())
        .execute(
            &session,
            parse_wire_int(&request.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
            &request.description,
            &state.terminal_id,
            &request.command_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_cash_movement_dto(&view))
}

/// `cash.getSessionReport` command: X-Report (interim) or Z-Report (final).
#[tauri::command(rename_all = "snake_case")]
pub fn cash_get_session_report(
    state: tauri::State<'_, AppState>,
    request: GetSessionReportRequestDto,
) -> Result<CashSessionReportDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let report = GetSessionReportUseCase::new(state.cash.as_ref())
        .execute(&session, &request.session_id)
        .map_err(|error| error_code(&error))?;
    Ok(CashSessionReportDto {
        session: to_cash_session_dto(&report.session),
        movements: report.movements.iter().map(to_cash_movement_dto).collect(),
        totals_by_type: report
            .totals_by_type
            .iter()
            .map(|total| CashMovementTotalDto {
                movement_type: total.movement_type.clone(),
                total_centimes: total.total_centimes.to_string(),
                count: total.count.to_string(),
            })
            .collect(),
        is_final: report.is_final,
    })
}
