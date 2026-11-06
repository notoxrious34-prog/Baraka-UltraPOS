//! Printing commands: jobs, TSPL, printer and store-identity settings.
use super::{error_code, parse_wire_id, parse_wire_int, require_session, AppState};
use baraka_application::errors::ApplicationError;
use baraka_application::use_cases::printing::{
    GetPrintStatusUseCase, GetPrinterSettingsUseCase, ReprintDocumentUseCase,
    SaveStoreIdentityUseCase, TestPrintDeviceOutcome, TestPrintUseCase,
    UpdatePrinterSettingsUseCase, UpdateTsplMediaSettingsUseCase,
};
use baraka_application::use_cases::sales::{GetFiscalSettingsUseCase, UpdateFiscalSettingsUseCase};
use baraka_application::use_cases::tspl_printing::{
    PrintRawBitmapUseCase, PrintTsplLabelUseCase, PrintTsplTestLabelUseCase, PrintTsplTicketUseCase,
};
use baraka_domain::role::Permission;

use crate::dto::{
    DecodeSummaryDto, FiscalSettingsDto, PreviewLineDto, PrintStatusResponse, PrinterSettingsDto,
    RawBitmapRequest, ReprintRequest, ReprintResponse, SaveStoreIdentityRequest, StoreIdentityDto,
    StoreIdentityResponse, TestPrintRequest, TestPrintResponse, TsplLabelRequest,
    TsplPrintResponse, TsplTestLabelRequest, TsplTicketRequest, UpdateFiscalSettingsRequestDto,
    UpdatePrinterSettingsRequest,
};

/// `documents_get_print_status` command: latest outbox state for a document.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_get_print_status(
    state: tauri::State<'_, AppState>,
    session_token: String,
    document_id: String,
) -> Result<PrintStatusResponse, String> {
    let session = require_session(&state, &session_token)?;
    let id = parse_wire_id(&document_id, "document_id")
        .map(baraka_domain::sales::DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let status = GetPrintStatusUseCase::new(state.print_jobs.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    let row = state
        .print_jobs
        .get_job_status(id)
        .map_err(|error| error_code(&error))?;
    Ok(PrintStatusResponse {
        job_id: row
            .as_ref()
            .map(|item| item.job_id.clone())
            .unwrap_or_default(),
        status,
        attempt_count: row
            .as_ref()
            .map(|item| item.attempt_count.to_string())
            .unwrap_or_default(),
    })
}

/// `documents_reprint` command: enqueue a watermarked DUPLICATA job.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_reprint(
    state: tauri::State<'_, AppState>,
    request: ReprintRequest,
) -> Result<ReprintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.document_id, "document_id")
        .map(baraka_domain::sales::DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let job_id = ReprintDocumentUseCase::new(state.print_jobs.as_ref(), state.documents.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    Ok(ReprintResponse {
        job_id,
        document_id: id.to_string(),
    })
}

/// `print_tspl_label` command: silent 40x20 mm shelf-label print (BRK-097).
///
/// Button-triggered only, spooler transport only — no dialog, no preview.
/// The price travels as canonical centime string; copies ride `PRINT m,n`.
#[tauri::command(rename_all = "snake_case")]
pub fn print_tspl_label(
    state: tauri::State<'_, AppState>,
    request: TsplLabelRequest,
) -> Result<TsplPrintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let price_centimes = parse_wire_int(&request.price_centimes, "price_centimes")
        .map_err(|error| error_code(&error))?;
    let outcome = PrintTsplLabelUseCase::new(
        state.printer_settings.as_ref(),
        state.store_identity.as_ref(),
        state.printer_transport.as_ref(),
    )
    .execute(
        &session,
        request.name.trim(),
        price_centimes,
        request.barcode.trim(),
        request.copies,
    )
    .map_err(|error| error_code(&error))?;
    Ok(TsplPrintResponse {
        bytes_sent: outcome.bytes_sent,
    })
}

/// `print_tspl_ticket` command: silent 80 mm ticket print (BRK-097).
///
/// Button-triggered only, spooler transport only — no dialog, no preview.
/// Content mirrors the graphique ticket wire (stored values, transcription
/// only; discounted PU when stored, raw legacy PU otherwise).
#[tauri::command(rename_all = "snake_case")]
pub fn print_tspl_ticket(
    state: tauri::State<'_, AppState>,
    request: TsplTicketRequest,
) -> Result<TsplPrintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.document_id, "document_id")
        .map(baraka_domain::sales::DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let outcome = PrintTsplTicketUseCase::new(
        state.documents.as_ref(),
        state.printer_settings.as_ref(),
        state.store_identity.as_ref(),
        state.printer_transport.as_ref(),
    )
    .execute(&session, id)
    .map_err(|error| error_code(&error))?;
    Ok(TsplPrintResponse {
        bytes_sent: outcome.bytes_sent,
    })
}

/// `print_raw_bitmap` command: silent raster-only print (BRK-101).
///
/// Button-triggered only, spooler transport only — no dialog, no preview.
/// The frontend packs WYSIWYG pixels (1-bit, MSB-first); every cap is
/// enforced pre-send by `render_bitmap_job`, never truncated.
#[tauri::command(rename_all = "snake_case")]
pub fn print_raw_bitmap(
    state: tauri::State<'_, AppState>,
    request: RawBitmapRequest,
) -> Result<TsplPrintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let outcome = PrintRawBitmapUseCase::new(
        state.printer_settings.as_ref(),
        state.printer_transport.as_ref(),
    )
    .execute(
        &session,
        &request.pixels,
        request.width_bytes as usize,
        request.height_rows as usize,
        request.copies,
    )
    .map_err(|error| error_code(&error))?;
    Ok(TsplPrintResponse {
        bytes_sent: outcome.bytes_sent,
    })
}

/// `printer_test_tspl_label` command: fixed-content 40x20 mm TSPL self-test
/// (TASK-PRT-01).
///
/// Button-triggered only, spooler transport only — no dialog, no preview.
/// Same production renderer as shelf labels with the STORED media + probe
/// mode, so a TSPL-only printer that silently eats the ESC/POS ticket test
/// gets a truthful answer here. ADMIN seam (`SystemSettingsManage`).
#[tauri::command(rename_all = "snake_case")]
pub fn printer_test_tspl_label(
    state: tauri::State<'_, AppState>,
    request: TsplTestLabelRequest,
) -> Result<TsplPrintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let outcome = PrintTsplTestLabelUseCase::new(
        state.printer_settings.as_ref(),
        state.store_identity.as_ref(),
        state.printer_transport.as_ref(),
    )
    .execute(&session)
    .map_err(|error| error_code(&error))?;
    Ok(TsplPrintResponse {
        bytes_sent: outcome.bytes_sent,
    })
}

/// `printer_get_settings` command: read printer configuration (serial + spooler selection).
#[tauri::command(rename_all = "snake_case")]
pub fn printer_get_settings(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<PrinterSettingsDto, String> {
    let session = require_session(&state, &session_token)?;
    let settings = GetPrinterSettingsUseCase::new(state.printer_settings.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(PrinterSettingsDto {
        serial_port: settings.serial_port,
        serial_baud: settings.serial_baud,
        paper_width_dots: settings.paper_width_dots,
        paper_width_mm: settings.paper_width_mm,
        transport: settings.transport,
        queue_name: settings.queue_name,
        ticket_render_mode: settings.ticket_render_mode,
        tspl_gap_mm: settings.tspl_gap_mm,
        tspl_density: settings.tspl_density,
        tspl_speed: settings.tspl_speed,
        // TEMPORARY-DIAGNOSTIC (BRK-100): probe switch wire.
        tspl_probe_mode: settings.tspl_probe_mode.clone(),
    })
}

/// `printer_update_settings` command (ADMIN only): persist COM port + baud + transport selection.
#[tauri::command(rename_all = "snake_case")]
pub fn printer_update_settings(
    state: tauri::State<'_, AppState>,
    request: UpdatePrinterSettingsRequest,
) -> Result<PrinterSettingsDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut settings = UpdatePrinterSettingsUseCase::new(state.printer_settings.as_ref())
        .execute(
            &session,
            &request.serial_port,
            request.serial_baud,
            request.paper_width_mm,
            request.transport.as_deref(),
            request.queue_name.as_deref(),
            request.ticket_render_mode.as_deref(),
        )
        .map_err(|error| error_code(&error))?;
    // TSPL media rides a separate validated write (only when addressed).
    if request.tspl_gap_mm.is_some()
        || request.tspl_density.is_some()
        || request.tspl_speed.is_some()
        || request.tspl_probe_mode.is_some()
    {
        settings = UpdateTsplMediaSettingsUseCase::new(state.printer_settings.as_ref())
            .execute(
                &session,
                request.tspl_gap_mm,
                request.tspl_density,
                request.tspl_speed,
                // TEMPORARY-DIAGNOSTIC (BRK-100): validated spelling upstairs.
                request.tspl_probe_mode,
            )
            .map_err(|error| error_code(&error))?;
    }
    Ok(PrinterSettingsDto {
        serial_port: settings.serial_port,
        serial_baud: settings.serial_baud,
        paper_width_dots: settings.paper_width_dots,
        paper_width_mm: settings.paper_width_mm,
        transport: settings.transport,
        queue_name: settings.queue_name,
        ticket_render_mode: settings.ticket_render_mode,
        tspl_gap_mm: settings.tspl_gap_mm,
        tspl_density: settings.tspl_density,
        tspl_speed: settings.tspl_speed,
        // TEMPORARY-DIAGNOSTIC (BRK-100): probe switch wire.
        tspl_probe_mode: settings.tspl_probe_mode,
    })
}

/// `store_identity_get` command: read identity + device kind (session).
#[tauri::command(rename_all = "snake_case")]
pub fn store_identity_get(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<StoreIdentityResponse, String> {
    require_session(&state, &session_token)?;
    let identity = state
        .store_identity
        .load_identity()
        .map_err(|error| error_code(&error))?;
    let device_kind = state
        .store_identity
        .device_kind()
        .map_err(|error| error_code(&error))?;
    Ok(StoreIdentityResponse {
        identity: to_identity_dto(&identity),
        device_kind,
    })
}

/// `store_identity_save` command (SystemSettingsManage): trim +
/// soft-validate + persist + audit (audited inside the settings repo).
#[tauri::command(rename_all = "snake_case")]
pub fn store_identity_save(
    state: tauri::State<'_, AppState>,
    request: SaveStoreIdentityRequest,
) -> Result<StoreIdentityResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    if !session
        .role
        .has_permission(Permission::SystemSettingsManage)
    {
        return Err(error_code(&ApplicationError::Unauthorized));
    }
    let saved = SaveStoreIdentityUseCase::new(state.store_identity.as_ref())
        .execute(
            &session,
            &from_identity_dto(&request.identity),
            &request.device_kind,
        )
        .map_err(|error| error_code(&error))?;
    let device_kind = state
        .store_identity
        .device_kind()
        .map_err(|error| error_code(&error))?;
    Ok(StoreIdentityResponse {
        identity: to_identity_dto(&saved),
        device_kind,
    })
}

/// `fiscal_settings_get` command: read the timbre toggle (ADR-092,
/// any authenticated session; absent key reads as enabled).
#[tauri::command(rename_all = "snake_case")]
pub fn fiscal_settings_get(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<FiscalSettingsDto, String> {
    let session = require_session(&state, &session_token)?;
    let timbre_enabled = GetFiscalSettingsUseCase::new(state.fiscal.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(FiscalSettingsDto { timbre_enabled })
}

/// `fiscal_settings_update` command: persist the timbre toggle (ADR-092,
/// ADMIN only via `SystemSettingsManage`).
#[tauri::command(rename_all = "snake_case")]
pub fn fiscal_settings_update(
    state: tauri::State<'_, AppState>,
    request: UpdateFiscalSettingsRequestDto,
) -> Result<FiscalSettingsDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let timbre_enabled = UpdateFiscalSettingsUseCase::new(state.fiscal.as_ref())
        .execute(&session, request.timbre_enabled)
        .map_err(|error| error_code(&error))?;
    Ok(FiscalSettingsDto { timbre_enabled })
}

/// `printer_test_print` command (SystemSettingsManage): synchronous
/// diagnostics (ADR-065 as amended) — shared-path sample receipt,
/// layout preview, unconditional shadow capture, physical attempt
/// through the worker's transport only when kind = physical.
#[tauri::command(rename_all = "snake_case")]
pub fn printer_test_print(
    state: tauri::State<'_, AppState>,
    request: TestPrintRequest,
) -> Result<TestPrintResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    if !session
        .role
        .has_permission(Permission::SystemSettingsManage)
    {
        return Err(error_code(&ApplicationError::Unauthorized));
    }
    let language = match request.language.trim().to_lowercase().as_str() {
        "ar" | "arabic" | "arabe" => baraka_domain::printing::ReceiptLanguage::Arabic,
        _ => baraka_domain::printing::ReceiptLanguage::French,
    };
    let report = TestPrintUseCase::new(
        state.store_identity.as_ref(),
        state.printer_renderer.as_ref(),
        state.printer_capture.as_ref(),
        state.printer_transport.as_ref(),
        state.clock.as_ref(),
    )
    .execute(&session, language)
    .map_err(|error| error_code(&error))?;
    Ok(TestPrintResponse {
        preview: report
            .preview
            .iter()
            .map(|line| PreviewLineDto {
                kind: line.kind.clone(),
                centered: line.centered,
                size_px: line.size_px,
                text: line.text.clone(),
            })
            .collect(),
        decode_summary: DecodeSummaryDto {
            bands: report.decode_summary.bands,
            feed_lines: report.decode_summary.feed_lines,
            unknown_spans: report.decode_summary.unknown_spans,
        },
        capture_file_name: report.capture_file_name,
        device_outcome: match report.device {
            TestPrintDeviceOutcome::Sent => "sent".to_string(),
            TestPrintDeviceOutcome::CaptureOnly => "capture-only".to_string(),
            TestPrintDeviceOutcome::DeviceError(code) => code,
        },
    })
}

fn to_identity_dto(identity: &baraka_domain::printing::StoreIdentity) -> StoreIdentityDto {
    StoreIdentityDto {
        store_name: identity.store_name.clone(),
        store_address: identity.store_address.clone(),
        store_phone: identity.store_phone.clone(),
        nif: identity.nif.clone(),
        nis: identity.nis.clone(),
        rc: identity.rc.clone(),
        ai: identity.ai.clone(),
        ticket_footer_message: identity.ticket_footer_message.clone(),
    }
}

fn from_identity_dto(dto: &StoreIdentityDto) -> baraka_domain::printing::StoreIdentity {
    baraka_domain::printing::StoreIdentity {
        store_name: dto.store_name.clone(),
        store_address: dto.store_address.clone(),
        store_phone: dto.store_phone.clone(),
        nif: dto.nif.clone(),
        nis: dto.nis.clone(),
        rc: dto.rc.clone(),
        ai: dto.ai.clone(),
        ticket_footer_message: dto.ticket_footer_message.clone(),
    }
}
