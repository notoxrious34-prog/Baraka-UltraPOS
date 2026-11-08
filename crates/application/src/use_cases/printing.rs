//! Printing use cases: reprint enqueue, status reads, store identity,
//! and test-print diagnostics (TASK-BRK-056, ADR-065).

use baraka_domain::printing::{
    identity_caps, DecodeSummary, DecodedLine, PreviewLine, ReceiptDocument, ReceiptLanguage,
};
use baraka_domain::printing::{ReceiptLine, ReceiptTotals, StoreIdentity};
use baraka_domain::role::Permission;
use baraka_domain::sales::DocumentId;

use crate::errors::ApplicationError;
use crate::ports::{
    CaptureSink, DocumentRepository, PrintJobRepository, PrinterTransportPort, ReceiptRendererPort,
    SessionContext, StoreIdentityPort,
};

/// Enqueue a DUPLICATA reprint job (original row untouched).
pub struct ReprintDocumentUseCase<'a, J, D>
where
    J: ?Sized,
    D: ?Sized,
{
    jobs: &'a J,
    documents: &'a D,
}

impl<'a, J, D> ReprintDocumentUseCase<'a, J, D>
where
    J: PrintJobRepository + ?Sized,
    D: DocumentRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(jobs: &'a J, documents: &'a D) -> Self {
        Self { jobs, documents }
    }

    /// Validate and enqueue the reprint.
    pub fn execute(
        &self,
        session: &SessionContext,
        document_id: DocumentId,
    ) -> Result<String, ApplicationError> {
        if !session.role.has_permission(Permission::ReprintDocument) {
            return Err(ApplicationError::Unauthorized);
        }
        self.documents
            .get_by_id(document_id)?
            .ok_or_else(|| ApplicationError::NotFound("document".to_string()))?;
        self.jobs.enqueue_reprint(document_id)
    }
}

/// Read the latest print-job status for a document.
pub struct GetPrintStatusUseCase<'a, J>
where
    J: ?Sized,
{
    jobs: &'a J,
}

impl<'a, J> GetPrintStatusUseCase<'a, J>
where
    J: PrintJobRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(jobs: &'a J) -> Self {
        Self { jobs }
    }

    /// Latest status code, or `NONE` when no job exists.
    pub fn execute(
        &self,
        _session: &SessionContext,
        document_id: DocumentId,
    ) -> Result<String, ApplicationError> {
        Ok(self
            .jobs
            .get_job_status(document_id)?
            .map(|row| row.status_code)
            .unwrap_or_else(|| "NONE".to_string()))
    }
}

/// Soft validation error codes (inline UI messages, i18n-mapped).
pub const IDENTITY_NAME_REQUIRED: &str = "STORE_NAME_REQUIRED";
/// Length-cap breach code (per-field caps in [`identity_caps`]).
pub const IDENTITY_TOO_LONG: &str = "STORE_FIELD_TOO_LONG";

fn check_identity(identity: &StoreIdentity) -> Result<StoreIdentity, ApplicationError> {
    let mut clean = StoreIdentity {
        store_name: identity.store_name.trim().to_string(),
        store_address: identity.store_address.trim().to_string(),
        store_phone: identity.store_phone.trim().to_string(),
        nif: identity.nif.trim().to_string(),
        nis: identity.nis.trim().to_string(),
        rc: identity.rc.trim().to_string(),
        ai: identity.ai.trim().to_string(),
        ticket_footer_message: identity.ticket_footer_message.trim().to_string(),
    };
    if clean.store_name.is_empty() {
        return Err(ApplicationError::Validation(
            baraka_domain::errors::DomainError::InvalidCanonicalString(
                IDENTITY_NAME_REQUIRED.to_string(),
            ),
        ));
    }
    // Length caps (chars, not bytes — Arabic values must not be
    // penalized for multi-byte encoding).
    let overlong = clean.store_name.chars().count() > identity_caps::STORE_NAME
        || clean.store_address.chars().count() > identity_caps::STORE_ADDRESS
        || clean.store_phone.chars().count() > identity_caps::STORE_PHONE
        || clean.nif.chars().count() > identity_caps::FISCAL_ID
        || clean.nis.chars().count() > identity_caps::FISCAL_ID
        || clean.rc.chars().count() > identity_caps::FISCAL_ID
        || clean.ai.chars().count() > identity_caps::FISCAL_ID
        || clean.ticket_footer_message.chars().count() > identity_caps::FOOTER_MESSAGE;
    if overlong {
        return Err(ApplicationError::Validation(
            baraka_domain::errors::DomainError::InvalidCanonicalString(
                IDENTITY_TOO_LONG.to_string(),
            ),
        ));
    }
    // Footer keeps at most FOOTER_LINES non-empty lines.
    let kept: Vec<&str> = clean
        .ticket_footer_message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(identity_caps::FOOTER_LINES)
        .collect();
    clean.ticket_footer_message = kept.join("\n");
    Ok(clean)
}

/// Load the store identity (session-gated upstream).
pub struct GetStoreIdentityUseCase<'a, S>
where
    S: ?Sized,
{
    settings: &'a S,
}

impl<'a, S> GetStoreIdentityUseCase<'a, S>
where
    S: StoreIdentityPort + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(settings: &'a S) -> Self {
        Self { settings }
    }

    /// Current identity (absent keys read empty).
    pub fn execute(&self, _session: &SessionContext) -> Result<StoreIdentity, ApplicationError> {
        self.settings.load_identity()
    }
}

/// Save the store identity (SystemSettingsManage-gated + audited upstream).
pub struct SaveStoreIdentityUseCase<'a, S>
where
    S: ?Sized,
{
    settings: &'a S,
}

impl<'a, S> SaveStoreIdentityUseCase<'a, S>
where
    S: StoreIdentityPort + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(settings: &'a S) -> Self {
        Self { settings }
    }

    /// Trim + soft-validate + persist + audit. Device routing stays on
    /// the worker's serial settings (single source of truth); only the
    /// diagnostics kind selector lives here.
    pub fn execute(
        &self,
        session: &SessionContext,
        identity: &StoreIdentity,
        device_kind: &str,
    ) -> Result<StoreIdentity, ApplicationError> {
        if !session
            .role
            .has_permission(Permission::SystemSettingsManage)
        {
            return Err(ApplicationError::Unauthorized);
        }
        let clean = check_identity(identity)?;
        let kind = match device_kind.trim() {
            "capture" => "capture",
            _ => "physical",
        };
        let actor = session.user_id.to_string();
        self.settings.save_identity(&clean, &actor)?;
        self.settings.save_device(kind, &actor)?;
        Ok(clean)
    }
}

/// Physical-device outcome of a diagnostics run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestPrintDeviceOutcome {
    /// Bytes transmitted to the physical endpoint.
    Sent,
    /// Capture-only run (device kind = capture).
    CaptureOnly,
    /// Physical send failed with a typed device error code.
    DeviceError(String),
}

/// Diagnostics result: layout preview + artifact + device outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct TestPrintReport {
    /// Layout blueprint from the shared blocks (pre-raster, NEVER
    /// derived from decoded pixels — Amendment 1).
    pub preview: Vec<PreviewLine>,
    /// Structural verdict over the captured artifact (decoder scope).
    pub decode_summary: DecodeSummary,
    /// Immutable artifact file name.
    pub capture_file_name: String,
    /// Physical-device outcome.
    pub device: TestPrintDeviceOutcome,
}

/// Compose the diagnostics sample receipt through the SHARED document
/// path (same `ReceiptDocument` → same template blocks as real docs).
/// The diagnostic language is fixed at invocation (Amendment 1 §2.4).
pub fn sample_test_receipt(
    identity: &StoreIdentity,
    footer: &str,
    stamp: &str,
    language: ReceiptLanguage,
) -> ReceiptDocument {
    ReceiptDocument {
        language,
        shop_name: identity.store_name.clone(),
        number: "TEST-000000".to_string(),
        created_at: stamp.to_string(),
        lines: vec![ReceiptLine {
            designation: match language {
                ReceiptLanguage::Arabic => "صفحة اختبار".to_string(),
                ReceiptLanguage::French => "Page de test".to_string(),
            },
            quantity_text: "1".to_string(),
            unit_price_text: "0.00".to_string(),
            total_text: "0.00".to_string(),
        }],
        totals: ReceiptTotals {
            subtotal_ht: "0.00".to_string(),
            tva: "0.00".to_string(),
            discount: "0.00".to_string(),
            timbre: "0.00".to_string(),
            total_ttc: "0.00".to_string(),
            paid: "0.00".to_string(),
            change: "0.00".to_string(),
        },
        payments: Vec::new(),
        is_duplicata: false,
        store_identity: identity.clone(),
        footer_message: footer.to_string(),
    }
}

/// Summarize a decoded artifact (decoder scope: bands + feed + unknowns).
pub fn summarize_decode(lines: &[DecodedLine]) -> DecodeSummary {
    let mut summary = DecodeSummary {
        bands: 0,
        feed_lines: 0,
        unknown_spans: 0,
    };
    for line in lines {
        match line {
            DecodedLine::RasterBand { .. } => summary.bands += 1,
            DecodedLine::Feed { lines } => summary.feed_lines += lines,
            DecodedLine::Unknown { .. } => summary.unknown_spans += 1,
        }
    }
    summary
}

/// Synchronous diagnostics command (ADR-065): no outbox row, no
/// accounting effect. ALWAYS shadow-captures; attempts the physical
/// send (through the SAME transport the worker uses) only when kind =
/// physical.
pub struct TestPrintUseCase<'a, S, R, C, T, K>
where
    S: ?Sized,
    R: ?Sized,
    C: ?Sized,
    T: ?Sized,
    K: ?Sized,
{
    settings: &'a S,
    renderer: &'a R,
    capture: &'a C,
    transport: &'a T,
    clock: &'a K,
}

impl<'a, S, R, C, T, K> TestPrintUseCase<'a, S, R, C, T, K>
where
    S: StoreIdentityPort + ?Sized,
    R: ReceiptRendererPort + ?Sized,
    C: CaptureSink + ?Sized,
    T: PrinterTransportPort + ?Sized,
    K: crate::ports::Clock + ?Sized,
{
    /// Wire the use case (timestamp via the Clock port — deterministic).
    pub fn new(
        settings: &'a S,
        renderer: &'a R,
        capture: &'a C,
        transport: &'a T,
        clock: &'a K,
    ) -> Self {
        Self {
            settings,
            renderer,
            capture,
            transport,
            clock,
        }
    }

    /// Run diagnostics: compose → preview → render → capture →
    /// summarize → maybe send.
    pub fn execute(
        &self,
        session: &SessionContext,
        language: ReceiptLanguage,
    ) -> Result<TestPrintReport, ApplicationError> {
        if !session
            .role
            .has_permission(Permission::SystemSettingsManage)
        {
            return Err(ApplicationError::Unauthorized);
        }
        let identity = self.settings.load_identity()?;
        let kind = self.settings.device_kind()?;
        let receipt = sample_test_receipt(
            &identity,
            &identity.ticket_footer_message,
            &self.clock.now_rfc3339(),
            language,
        );
        let preview = self.renderer.preview_lines(&receipt)?;
        let bytes = self.renderer.render(&receipt)?;
        let capture_file_name = self.capture.write_capture(&bytes)?;
        let decode_summary = summarize_decode(&baraka_domain::printing::escpos_decode(&bytes));
        let device = if kind == "capture" {
            TestPrintDeviceOutcome::CaptureOnly
        } else {
            match self.transport.send_raster(&bytes) {
                Ok(()) => TestPrintDeviceOutcome::Sent,
                Err(error) => TestPrintDeviceOutcome::DeviceError(device_error_code(&error)),
            }
        };
        self.settings.append_settings_audit(
            &session.user_id.to_string(),
            "PRINT_TEST_PERFORMED",
            &capture_file_name,
        )?;
        Ok(TestPrintReport {
            preview,
            decode_summary,
            capture_file_name,
            device,
        })
    }
}

/// Map a transport failure to its stable device-error code.
fn device_error_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::Validation(domain) => format!("{domain}"),
        _ => "DEVICE_ERROR".to_string(),
    }
}

/// Read the serial printer configuration (TASK-BRK-070).
///
/// Any authenticated session may read; the paper width arrives as
/// configured millimeters with the derived dot width alongside.
pub struct GetPrinterSettingsUseCase<'a, P>
where
    P: ?Sized,
{
    settings: &'a P,
}

impl<'a, P> GetPrinterSettingsUseCase<'a, P>
where
    P: crate::ports::PrinterSettingsPort + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(settings: &'a P) -> Self {
        Self { settings }
    }

    /// Return the stored configuration (safe defaults when absent).
    pub fn execute(
        &self,
        _session: &SessionContext,
    ) -> Result<crate::ports::PrinterSettings, ApplicationError> {
        self.settings.get_printer_settings()
    }
}

/// Persist the printer configuration (TASK-BRK-070, extended BRK-089-C1,
/// BRK-090).
///
/// `paper_width_mm` of `None` keeps the stored width; `Some` must be
/// exactly 58 or 80. `transport` of `None` keeps the stored arm; `Some`
/// must be `"serial"` or `"spooler"`. `queue_name` of `None` keeps the
/// stored queue; `Some` must be non-empty and at most 220 characters.
/// `ticket_render_mode` of `None` keeps the stored mode; `Some` must be
/// `"escpos"` or `"graphique"`. All rejected before any write.
pub struct UpdatePrinterSettingsUseCase<'a, P>
where
    P: ?Sized,
{
    settings: &'a P,
}

impl<'a, P> UpdatePrinterSettingsUseCase<'a, P>
where
    P: crate::ports::PrinterSettingsPort + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(settings: &'a P) -> Self {
        Self { settings }
    }

    /// Validate permission, width, baud, transport arm, queue name, and
    /// ticket render mode, then persist atomically.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        serial_port: &str,
        serial_baud: u32,
        paper_width_mm: Option<u32>,
        transport: Option<&str>,
        queue_name: Option<&str>,
        ticket_render_mode: Option<&str>,
    ) -> Result<crate::ports::PrinterSettings, ApplicationError> {
        if !session
            .role
            .has_permission(Permission::ManagePrinterSettings)
        {
            return Err(ApplicationError::Unauthorized);
        }
        if let Some(mm) = paper_width_mm {
            baraka_domain::printing::validate_paper_width_mm(mm)?;
        }
        if serial_baud == 0 {
            return Err(ApplicationError::Validation(
                baraka_domain::errors::DomainError::InvalidCanonicalString(
                    "serial_baud must be non-zero".to_string(),
                ),
            ));
        }
        if let Some(selection) = transport {
            baraka_domain::printing::validate_printer_transport(selection)?;
        }
        if let Some(queue) = queue_name {
            baraka_domain::printing::validate_spooler_queue_name(queue)?;
        }
        if let Some(mode) = ticket_render_mode {
            baraka_domain::printing::validate_ticket_render_mode(mode)?;
        }
        self.settings.update_printer_settings(
            serial_port,
            serial_baud,
            paper_width_mm,
            transport,
            queue_name,
            ticket_render_mode,
        )?;
        self.settings.get_printer_settings()
    }
}

/// Persist the TSPL media setup (TASK-BRK-099, ADMIN-gated).
///
/// `gap_mm` of `None` keeps the stored gap; `Some` must be 0..=10 mm.
/// `density` of `None` keeps the stored density; `Some` must be 0..=15.
/// `speed` of `None` keeps the stored speed; `Some` must be 1..=6 in/s.
/// `offset_*_mm` of `None` keeps the stored shift; `Some` must be -5..=+5.
/// All rejected before any write. The gap default (3 mm) is a STARTING
/// VALUE — OWNER-VERIFY with a ruler; the measured value wins.
pub struct UpdateTsplMediaSettingsUseCase<'a, P>
where
    P: ?Sized,
{
    settings: &'a P,
}

impl<'a, P> UpdateTsplMediaSettingsUseCase<'a, P>
where
    P: crate::ports::PrinterSettingsPort + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(settings: &'a P) -> Self {
        Self { settings }
    }

    /// Validate permission + bounds, then persist atomically.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        gap_mm: Option<u32>,
        density: Option<u8>,
        speed: Option<u8>,
        // TEMPORARY-DIAGNOSTIC (BRK-100): validated spelling, never silent.
        probe_mode: Option<String>,
        offset_x_mm: Option<i32>,
        offset_y_mm: Option<i32>,
    ) -> Result<crate::ports::PrinterSettings, ApplicationError> {
        if !session
            .role
            .has_permission(Permission::ManagePrinterSettings)
        {
            return Err(ApplicationError::Unauthorized);
        }
        if let Some(gap) = gap_mm {
            baraka_domain::printing::validate_tspl_gap_mm(gap)?;
        }
        if let Some(density) = density {
            baraka_domain::printing::validate_tspl_density(density)?;
        }
        if let Some(speed) = speed {
            baraka_domain::printing::validate_tspl_speed(speed)?;
        }
        if let Some(mode) = probe_mode.as_deref() {
            baraka_domain::printing::parse_tspl_probe_mode(mode)?;
        }
        // DOC-06 calibration: validated LOUDLY before any write.
        if let Some(offset) = offset_x_mm {
            baraka_domain::printing::validate_tspl_calib_mm(offset)?;
        }
        if let Some(offset) = offset_y_mm {
            baraka_domain::printing::validate_tspl_calib_mm(offset)?;
        }
        self.settings.update_tspl_media_settings(
            gap_mm,
            density,
            speed,
            probe_mode,
            offset_x_mm,
            offset_y_mm,
        )?;
        self.settings.get_printer_settings()
    }
}
