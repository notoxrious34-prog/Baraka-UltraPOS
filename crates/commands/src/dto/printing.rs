//! Printing DTOs: jobs, TSPL labels, tickets, printer settings, diagnostics.
use serde::{Deserialize, Serialize};

// Print wire types ------------------------------------------------------
// Serial-only transport settings; no USB fields in this task.

/// Latest print-job status view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintStatusResponse {
    /// Stored job id.
    pub job_id: String,
    /// Status code (`PENDING`, `PROCESSING`, `DONE`, `FAILED`).
    pub status: String,
    /// Physical send attempts so far.
    pub attempt_count: String,
}

/// Reprint enqueue request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReprintRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Document id (UUID string).
    pub document_id: String,
}

/// Reprint enqueue response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReprintResponse {
    /// New DUPLICATA job id.
    pub job_id: String,
    /// Echoed document id.
    pub document_id: String,
}

// Silent TSPL wire types (TASK-BRK-097, ADR-105) ---------------------------
// Money travels as integer-centime strings; copies as u32. No float ever
// crosses this boundary. Responses carry byte counts only; no content.

/// Silent 40x20 mm label print request (button-triggered, spooler only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsplLabelRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Display name (single locale, chosen by the caller).
    pub name: String,
    /// Selling price in integer centimes (canonical string).
    pub price_centimes: String,
    /// Code128 payload (SKU/barcode).
    pub barcode: String,
    /// Copies, 1..=99 (enforced in PRINT m,n, never by resending).
    pub copies: u32,
}

/// Silent TSPL print response (label or ticket).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsplPrintResponse {
    /// Bytes handed to the spooler transport.
    pub bytes_sent: usize,
}

/// Fixed-content 40x20 mm TSPL self-test request (TASK-PRT-01).
/// Session only — content, media and probe mode all come from storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsplTestLabelRequest {
    /// Authorizing session token.
    pub session_token: String,
}

/// Fixed-content 80 mm TSPL ticket self-test request (TASK-PRT-03).
/// Session only — content, media and probe mode all come from storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsplTestTicketRequest {
    /// Authorizing session token.
    pub session_token: String,
}

/// Silent raster-only print request (button-triggered, spooler only, BRK-101).
/// Pixels are 1-bit rows, MSB-first, left-to-right (see render_bitmap_job);
/// caps enforced pre-send, never truncated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawBitmapRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Packed 1-bit pixels, row-major (Vec<u8> rides a JSON number array).
    pub pixels: Vec<u8>,
    /// Frame width in BYTES (dots = width_bytes x 8, 80 mm max).
    pub width_bytes: u32,
    /// Frame height in DOTS (1638 max).
    pub height_rows: u32,
    /// Copies, 1..=99 (enforced in PRINT 1,n, never by resending).
    pub copies: u32,
}

/// Silent 80 mm ticket print request (button-triggered, spooler only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsplTicketRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Document id (UUID string).
    pub document_id: String,
}

/// Serial printer configuration view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrinterSettingsDto {
    /// COM port name (`""` when unconfigured).
    pub serial_port: String,
    /// Baud rate.
    pub serial_baud: u32,
    /// Paper width in dots (derived from `paper_width_mm`).
    pub paper_width_dots: u32,
    /// Configured paper width in millimeters (58 or 80).
    pub paper_width_mm: u32,
    /// Active transport arm ("erial" default, "poolerol for USB RAW).
    pub transport: String,
    /// Exact Windows spooler queue name ("" when unconfigured).
    pub queue_name: String,
    /// Ticket render mode ("escpos" default, "graphique" for OS-graphics
    /// tickets, ADR-104).
    pub ticket_render_mode: String,
    /// TSPL label inter-label gap in mm (default 3, OWNER-VERIFY ruler).
    pub tspl_gap_mm: u32,
    /// TSPL print density, 0..=15 (default 8).
    pub tspl_density: u8,
    /// TSPL print speed in inch/sec, 1..=6 (default 4).
    pub tspl_speed: u8,
    /// TEMPORARY-DIAGNOSTIC probe mode wire spelling (BRK-100).
    pub tspl_probe_mode: String,
}

/// Serial printer configuration update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePrinterSettingsRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// COM port name.
    pub serial_port: String,
    /// Baud rate.
    pub serial_baud: u32,
    /// Paper width in millimeters (58 or 80; absent keeps stored width).
    #[serde(default)]
    pub paper_width_mm: Option<u32>,
    /// Transport arm ("serialol or "poolerol; absent keeps stored arm).
    #[serde(default)]
    pub transport: Option<String>,
    /// Spooler queue name (absent keeps stored queue).
    #[serde(default)]
    pub queue_name: Option<String>,
    /// Ticket render mode ("escpos" or "graphique"; absent keeps
    /// stored mode).
    #[serde(default)]
    pub ticket_render_mode: Option<String>,
    /// TSPL label gap in mm (0..=10; absent keeps stored gap).
    #[serde(default)]
    pub tspl_gap_mm: Option<u32>,
    /// TSPL print density (0..=15; absent keeps stored density).
    #[serde(default)]
    pub tspl_density: Option<u8>,
    /// TSPL print speed in inch/sec (1..=6; absent keeps stored speed).
    #[serde(default)]
    pub tspl_speed: Option<u8>,
    /// TEMPORARY-DIAGNOSTIC probe mode selector (BRK-100).
    #[serde(default)]
    pub tspl_probe_mode: Option<String>,
}

// Store identity + diagnostics wire types ---------------------------------
// Persisted as namespaced `store.*` / `printing.device.kind` keys in the
// existing key-value `settings` table — zero schema changes.

/// Store identity view (8 fields; empty = omitted from the header).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreIdentityDto {
    /// Shop display name (required).
    pub store_name: String,
    /// Street address line.
    pub store_address: String,
    /// Phone number.
    pub store_phone: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub nif: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub nis: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub rc: String,
    /// Fiscal identifiers (soft validation only — ADR-013 deferred).
    pub ai: String,
    /// Free-text footer (`\n`-separated, up to 3 lines).
    pub ticket_footer_message: String,
}

/// Identity + device load response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreIdentityResponse {
    /// Current identity values.
    pub identity: StoreIdentityDto,
    /// Diagnostics device kind (`physical` | `capture`).
    pub device_kind: String,
}

/// Identity save request (values trimmed server-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveStoreIdentityRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Identity values to persist.
    pub identity: StoreIdentityDto,
    /// Diagnostics device kind (`physical` | `capture`).
    pub device_kind: String,
}

/// One monospace preview line (shared layout blueprint, pre-raster).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewLineDto {
    /// `text` | `rule` | `gap`.
    pub kind: String,
    /// Centered line.
    pub centered: bool,
    /// Pixel em size.
    pub size_px: f32,
    /// Concatenated run texts (`""` for rules/gaps).
    pub text: String,
}

/// Structural verdict over the captured artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodeSummaryDto {
    /// Raster band count.
    pub bands: u32,
    /// Trailing feed lines.
    pub feed_lines: u32,
    /// Unknown-byte spans.
    pub unknown_spans: u32,
}

/// Diagnostics run request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestPrintRequest {
    /// Authorizing session token.
    pub session_token: String,
    /// Diagnostic language (`ar` | `fr`).
    pub language: String,
}

/// Diagnostics run response (Amendment 1 §2.3 shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestPrintResponse {
    /// Layout preview from the shared blocks.
    pub preview: Vec<PreviewLineDto>,
    /// Structural verdict over the capture.
    pub decode_summary: DecodeSummaryDto,
    /// Immutable artifact file name.
    pub capture_file_name: String,
    /// `capture-only` | `sent` | typed device-error code.
    pub device_outcome: String,
}
