//! Label Studio print + preview use-cases (TASK-DSN-107, Wave 3).
//!
//! Two button-triggered commands emitting the studio template bytes to
//! the LABEL role (BRK-113) — no dialog, no auto-print, same spooler-only
//! gate as the classic label path. Transcription rules mirror the classic
//! label: kernel centimes formatted via `format_centimes_plain`, `DZD`
//! currency, single-copy preview, stored media, PRT-02 bitmap path shared
//! through `rasterize_label_plan` (the preview measures the job the
//! printer eats).

use baraka_domain::errors::DomainError;
use baraka_domain::printing::{
    format_centimes_plain, hardware_barcode_geometry, hardware_label_text_plan,
    render_hardware_label_40x20, render_hardware_label_40x20_mixed, render_textile_label_40x20,
    render_textile_label_40x20_mixed, studio_text_needs_bitmap, textile_barcode_geometry,
    textile_label_text_plan, LabelBarcodeGeometry, PrintRole, StudioHardwareLabel,
    StudioTextileLabel,
};
use baraka_domain::role::Permission;

use crate::errors::ApplicationError;
use crate::ports::{
    NullLabelRaster, PrinterSettingsPort, PrinterTransportPort, SessionContext, TsplLabelRasterPort,
};
use crate::use_cases::tspl_printing::{
    media_from_settings, rasterize_label_plan, require_role_spooler, PreviewLabelLine,
    TsplLabelPreview, TsplPrintOutcome,
};

/// Label Studio template selector (wire: `"textile"` | `"hardware"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioTemplate {
    /// Apparel: model + giant size + color/pack + optional retail price.
    Textile,
    /// Quincaillerie: hero bars + designation + unit/dimension + price.
    Hardware,
}

impl StudioTemplate {
    /// Parse the wire spelling; anything else is rejected LOUDLY.
    pub fn from_wire(value: &str) -> Result<Self, ApplicationError> {
        match value.trim() {
            "textile" => Ok(StudioTemplate::Textile),
            "hardware" => Ok(StudioTemplate::Hardware),
            _ => Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("STUDIO_UNKNOWN_TEMPLATE".to_string()),
            )),
        }
    }
}

/// Studio label fields (template-indexed lines; the modal fills them).
#[derive(Debug, Clone, Copy)]
pub struct StudioLabelFields<'a> {
    /// Model (textile) | designation (hardware).
    pub line1: &'a str,
    /// Size (textile) | sale unit (hardware).
    pub line2: &'a str,
    /// Color (textile) | dimension, `""` = absent (hardware).
    pub line3: &'a str,
    /// Pack badge (textile; ignored by hardware).
    pub line4: &'a str,
    /// Wholesale price-hide (textile only; hardware rejects it).
    pub hide_price: bool,
}

/// Resolve the kernel price into display text (or wholesale-hide).
/// Contradictions reject LOUDLY, never silently dropped.
fn resolve_studio_price(
    template: StudioTemplate,
    fields: &StudioLabelFields<'_>,
    price_centimes: Option<i64>,
) -> Result<Option<String>, ApplicationError> {
    let invalid = |name: &str| {
        ApplicationError::Validation(DomainError::InvalidCanonicalString(name.to_string()))
    };
    match template {
        StudioTemplate::Textile if fields.hide_price => {
            if price_centimes.is_some() {
                return Err(invalid("STUDIO_PRICE_HIDE_CONTRADICTION"));
            }
            Ok(None)
        }
        _ => {
            if fields.hide_price {
                // Hardware has no hide toggle — a set flag is a wire bug.
                return Err(invalid("STUDIO_PRICE_HIDE_CONTRADICTION"));
            }
            let centimes = price_centimes.ok_or_else(|| invalid("STUDIO_PRICE_REQUIRED"))?;
            if centimes < 0 {
                return Err(invalid("TSPL_PRICE_MUST_BE_NON_NEGATIVE"));
            }
            Ok(Some(format_centimes_plain(centimes)))
        }
    }
}

/// Silent studio-label print (Label role gate + spooler send).
pub struct PrintStudioLabelUseCase<'a, S, T, R = NullLabelRaster>
where
    S: ?Sized,
    T: ?Sized,
    R: ?Sized,
{
    settings: &'a S,
    transport: &'a T,
    label_raster: &'a R,
}

/// Shared no-rasterer behind every default-constructed studio use case.
static NULL_STUDIO_RASTER: NullLabelRaster = NullLabelRaster;

impl<'a, S, T> PrintStudioLabelUseCase<'a, S, T, NullLabelRaster>
where
    S: PrinterSettingsPort + ?Sized,
    T: PrinterTransportPort + ?Sized,
{
    /// Wire the use case to its ports (no Arabic rasterer attached).
    pub fn new(settings: &'a S, transport: &'a T) -> Self {
        Self {
            settings,
            transport,
            label_raster: &NULL_STUDIO_RASTER,
        }
    }
}

impl<'a, S, T, R> PrintStudioLabelUseCase<'a, S, T, R>
where
    S: PrinterSettingsPort + ?Sized,
    T: PrinterTransportPort + ?Sized,
    R: TsplLabelRasterPort + ?Sized,
{
    /// Attach the Arabic-capable line rasterer (PRT-02); without it any
    /// non-Latin TEXT slot fails LOUDLY instead of a dead label.
    pub fn with_label_raster<Q>(self, raster: &'a Q) -> PrintStudioLabelUseCase<'a, S, T, Q>
    where
        Q: TsplLabelRasterPort + ?Sized,
    {
        PrintStudioLabelUseCase {
            settings: self.settings,
            transport: self.transport,
            label_raster: raster,
        }
    }

    /// Validate, render, and spool one studio-label run.
    pub fn execute(
        &self,
        session: &SessionContext,
        template: StudioTemplate,
        fields: &StudioLabelFields<'_>,
        price_centimes: Option<i64>,
        barcode: &str,
        copies: u32,
    ) -> Result<TsplPrintOutcome, ApplicationError> {
        if !session.role.has_permission(Permission::PosSell) {
            return Err(ApplicationError::Unauthorized);
        }
        require_role_spooler(self.settings, PrintRole::Label)?;
        let media = media_from_settings(self.settings)?;
        let price_text = resolve_studio_price(template, fields, price_centimes)?;
        let bytes = match template {
            StudioTemplate::Textile => {
                let label = StudioTextileLabel {
                    model: fields.line1,
                    size: fields.line2,
                    color: fields.line3,
                    pack: fields.line4,
                    price_text: price_text.as_deref(),
                    currency_text: "DZD",
                    barcode,
                    copies,
                    media,
                };
                let plan = textile_label_text_plan(&label);
                if plan
                    .iter()
                    .any(|line| studio_text_needs_bitmap(&line.text, label.media.probe_mode))
                {
                    let bitmaps =
                        rasterize_label_plan(self.label_raster, &plan, label.media.probe_mode)?;
                    render_textile_label_40x20_mixed(&label, &bitmaps)?
                } else {
                    render_textile_label_40x20(&label)?
                }
            }
            StudioTemplate::Hardware => {
                let price = price_text.as_deref().unwrap_or("");
                let label = StudioHardwareLabel {
                    designation: fields.line1,
                    unit_text: fields.line2,
                    dimension_text: if fields.line3.trim().is_empty() {
                        None
                    } else {
                        Some(fields.line3)
                    },
                    price_text: price,
                    currency_text: "DZD",
                    barcode,
                    copies,
                    media,
                };
                let plan = hardware_label_text_plan(&label);
                if plan
                    .iter()
                    .any(|line| studio_text_needs_bitmap(&line.text, label.media.probe_mode))
                {
                    let bitmaps =
                        rasterize_label_plan(self.label_raster, &plan, label.media.probe_mode)?;
                    render_hardware_label_40x20_mixed(&label, &bitmaps)?
                } else {
                    render_hardware_label_40x20(&label)?
                }
            }
        };
        self.transport
            .send_raster_for_role(PrintRole::Label, &bytes)?;
        Ok(TsplPrintOutcome {
            bytes_sent: bytes.len(),
        })
    }
}

/// Read-only studio-label preview (DOC-03 parity): the SAME structs the
/// print path spools — plan, barcode origin, exact job length.
pub struct PreviewStudioLabelUseCase<'a, S, R = NullLabelRaster>
where
    S: ?Sized,
    R: ?Sized,
{
    settings: &'a S,
    preview_raster: &'a R,
}

/// Shared no-rasterer behind every default-constructed preview use case.
static NULL_STUDIO_PREVIEW_RASTER: NullLabelRaster = NullLabelRaster;

impl<'a, S> PreviewStudioLabelUseCase<'a, S, NullLabelRaster>
where
    S: PrinterSettingsPort + ?Sized,
{
    /// Wire the use case to its ports (no preview rasterer attached).
    pub fn new(settings: &'a S) -> Self {
        Self {
            settings,
            preview_raster: &NULL_STUDIO_PREVIEW_RASTER,
        }
    }
}

impl<'a, S, R> PreviewStudioLabelUseCase<'a, S, R>
where
    S: PrinterSettingsPort + ?Sized,
    R: TsplLabelRasterPort + ?Sized,
{
    /// Attach the bitmap rasterer (DOC-03): the preview renders the same
    /// mixed job the printer eats, so the byte length predicts exactly.
    pub fn with_preview_raster<Q>(&self, raster: &'a Q) -> PreviewStudioLabelUseCase<'a, S, Q>
    where
        Q: TsplLabelRasterPort + ?Sized,
    {
        PreviewStudioLabelUseCase {
            settings: self.settings,
            preview_raster: raster,
        }
    }

    /// Build the engine plan for the studio fields and return it with the
    /// barcode origin and the exact job length.
    pub fn execute(
        &self,
        session: &SessionContext,
        template: StudioTemplate,
        fields: &StudioLabelFields<'_>,
        price_centimes: Option<i64>,
        barcode: &str,
    ) -> Result<TsplLabelPreview, ApplicationError> {
        if !session.role.has_permission(Permission::PosSell) {
            return Err(ApplicationError::Unauthorized);
        }
        let media = media_from_settings(self.settings)?;
        let price_text = resolve_studio_price(template, fields, price_centimes)?;
        // Identical construction to the print path (single copy preview).
        let (plan, geometry, job) = match template {
            StudioTemplate::Textile => {
                let label = StudioTextileLabel {
                    model: fields.line1,
                    size: fields.line2,
                    color: fields.line3,
                    pack: fields.line4,
                    price_text: price_text.as_deref(),
                    currency_text: "DZD",
                    barcode,
                    copies: 1,
                    media,
                };
                let plan = textile_label_text_plan(&label);
                let geometry = textile_barcode_geometry(label.barcode)?;
                let job = if plan
                    .iter()
                    .any(|line| studio_text_needs_bitmap(&line.text, label.media.probe_mode))
                {
                    let bitmaps =
                        rasterize_label_plan(self.preview_raster, &plan, label.media.probe_mode)?;
                    render_textile_label_40x20_mixed(&label, &bitmaps)?
                } else {
                    render_textile_label_40x20(&label)?
                };
                (plan, geometry, job)
            }
            StudioTemplate::Hardware => {
                let price = price_text.as_deref().unwrap_or("");
                let label = StudioHardwareLabel {
                    designation: fields.line1,
                    unit_text: fields.line2,
                    dimension_text: if fields.line3.trim().is_empty() {
                        None
                    } else {
                        Some(fields.line3)
                    },
                    price_text: price,
                    currency_text: "DZD",
                    barcode,
                    copies: 1,
                    media,
                };
                let plan = hardware_label_text_plan(&label);
                let geometry = hardware_barcode_geometry(label.barcode)?;
                let job = if plan
                    .iter()
                    .any(|line| studio_text_needs_bitmap(&line.text, label.media.probe_mode))
                {
                    let bitmaps =
                        rasterize_label_plan(self.preview_raster, &plan, label.media.probe_mode)?;
                    render_hardware_label_40x20_mixed(&label, &bitmaps)?
                } else {
                    render_hardware_label_40x20(&label)?
                };
                (plan, geometry, job)
            }
        };
        Ok(TsplLabelPreview {
            lines: plan
                .into_iter()
                .map(|line| PreviewLabelLine {
                    slot: line.slot,
                    text: line.text,
                    x_dots: line.x_dots,
                    y_dots: line.y_dots,
                    cell_h_dots: line.cell_h_dots,
                })
                .collect(),
            barcode: LabelBarcodeGeometry {
                x_dots: geometry.x_dots,
                y_dots: geometry.y_dots,
                width_dots: geometry.width_dots,
                height_dots: geometry.height_dots,
            },
            job_len: job.len(),
        })
    }
}
