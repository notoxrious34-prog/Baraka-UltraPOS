//! Silent TSPL renderer (TASK-BRK-097 bytes, TASK-BRK-098 professional
//! layout, ADR-105).
//!
//! Pure: structured input in, TSPL bytes out. No I/O, no settings reads,
//! no floats. The use-cases transcribe stored values into the input
//! structs; this module only lays out bytes.
//!
//! Geometry math (203 dpi ≈ 8 dots/mm EXACT per task spec; 203/25.4 =
//! 7.992, the 0.1% drift is absorbed by the printer's own calibration):
//! - label 40x20 mm -> 320 x 160 dots (`SIZE 40 mm,20 mm`).
//! - ticket 80 mm wide -> 640 dots (`SIZE 80 mm,H mm`, H computed).
//!
//! Ticket anatomy (top to bottom, BRK-098): store block (name centered,
//! address/phone/tax rows centered, blank fields collapse — never empty
//! lines), rule, doc meta (number, capture instant, cashier, depot), rule,
//! column header, item rows (designation truncated to width, right-aligned
//! `qty unit x PU = total` detail, right-aligned per-unit discount badge
//! where discounted), rule, totals hierarchy (Sous-total, Remise totale,
//! HT, TVA, then TOTAL TTC emphasized double-height), payment block
//! (method + tendered + change), doc-number Code128 with human-readable
//! number beneath, footer (owner message + constant thanks).
//!
//! Label anatomy (40x20): brand micro-line (collapses when blank),
//! designation two lines max truncated, double-height price with right-
//! aligned currency, Code128 with human-readable payload beneath.
//!
//! Barcode discipline (BRK-098, quoted and pinned):
//! - Narrow bar = 2 dots = 2/8 mm = 0.25 mm ≈ 9.8 mil at 203 dpi (1 dot
//!   ≈ 4.9 mil falls below ISO 15416-friendly practice).
//! - Quiet zones: >= 10 narrow modules each side = 20 dots reserved; the
//!   payload budget is media width minus both zones, and over-wide
//!   payloads REJECT with [`DomainError::TsplBarcodeTooWide`] — never
//!   shrunk.
//! - Ticket Code128: 64 dots (8.0 mm) tall; widest in-budget payload
//!   (24 chars = 598 dots) keeps a 9.3:1 width/height ratio, the typical
//!   13-char number 356 dots keeps 5.6:1 — ISO 15416-friendly.
//! - Label Code128: 40 dots (5.0 mm) tall; widest payload 9 chars =
//!   268 dots keeps 6.7:1, narrowest 2.3:1 (stated small-label floor).
//! - Human-readable line is mandatory beneath every barcode.
//!
//! TSPL dialect assumptions (XPrinter XP-410B emulation, owner to verify
//! on hardware; STATED, not probed):
//! - Built-in bitmap fonts: "1" = 8x12, "2" = 12x20, "3" = 16x24 dots
//!   (TSC TSPL reference metrics); scale 2 doubles the glyph box.
//! - `TEXT x,y,"F",0,1,1,"..."`, `BARCODE x,y,"128",h,1,0,2,2,"..."`,
//!   `PRINT 1,n` (1 set, n copies), `PRINT 1,1` (single ticket).
//! - `DIRECTION 1` (forward feed). Every job carries a self-describing
//!   media preamble (BRK-099, AUD-045): SIZE → GAP → DENSITY → SPEED →
//!   CODEPAGE → DIRECTION → CLS. Label gap defaults to 3 mm STARTING
//!   VALUE (OWNER-VERIFY with a ruler); ticket jobs declare `GAP 0,0`
//!   (continuous). No NVRAM luck.
//! - TSPL core has NO printer-agnostic cut command: no cut bytes are
//!   emitted by design. Physical cut relies on the printer's auto-cut
//!   configuration (deferred to owner hardware verification).
//! - TEXT payloads pass through as UTF-8 bytes. Built-in TSPL fonts do
//!   NOT shape Arabic and have no BiDi: Arabic names/captions print as
//!   unshaped logical-order glyphs unless a soft font is loaded
//!   (deferred; bytes are never destroyed — no transliteration).
//! - `"` (0x22) is excluded from barcode payloads: TSPL has no string
//!   escape and a quote would break command framing. TEXT fields replace
//!   `"` with `'` and CR/LF with a space (command-injection safety).
//! - Truncation marker is ASCII "..." (always renderable); barcode
//!   payloads are NEVER truncated (scan integrity) — over-wide payloads
//!   reject with [`DomainError::TsplBarcodeTooWide`].

use crate::errors::DomainError;

/// Dots per millimeter at 203 dpi (task-mandated exact integer).
pub const DOTS_PER_MM: i64 = 8;
/// Label media width in dots (40 mm).
pub const LABEL_WIDTH_DOTS: i64 = 320;
/// Label media height in dots (20 mm).
pub const LABEL_HEIGHT_DOTS: i64 = 160;
/// Ticket media width in dots (80 mm).
pub const TICKET_WIDTH_DOTS: i64 = 640;
/// Copies floor (a label run prints at least one label).
pub const TSPL_MIN_COPIES: u32 = 1;
/// Copies ceiling (PRINT m,n single-byte field discipline, stated).
pub const TSPL_MAX_COPIES: u32 = 99;

/// Label inter-label gap default in mm — STARTING VALUE, OWNER-VERIFY with
/// a ruler against the 40x20 liner (BRK-099). The measured value wins once
/// collected; this default never silently becomes truth.
pub const TSPL_GAP_MM_DEFAULT: u32 = 3;
/// Label gap ceiling in mm (die-cut desktop stock never exceeds this).
pub const TSPL_GAP_MM_MAX: u32 = 10;
/// Print density default (TSC 0..=15 scale, mid-dark for direct-thermal).
pub const TSPL_DENSITY_DEFAULT: u8 = 8;
/// Print density ceiling (TSC scale maximum).
pub const TSPL_DENSITY_MAX: u8 = 15;
/// Print speed default, inch/sec (conservative for 203 dpi text+barcode).
pub const TSPL_SPEED_DEFAULT: u8 = 4;
/// Print speed floor, inch/sec.
pub const TSPL_SPEED_MIN: u8 = 1;
/// Print speed ceiling, inch/sec (XP-410B class maximum).
pub const TSPL_SPEED_MAX: u8 = 6;
/// Fixed charset declaration. UTF-8 — NOT 437: our TEXT bytes ARE UTF-8
/// (French accents are multibyte; CP437 would render them as mojibake),
/// and UTF-8 keeps Arabic codepoint-accurate (unshaped per AUD-045 —
/// trial 4 decides the BITMAP fallback). Printer acceptance is
/// OWNER-VERIFY: if the XP-410B rejects the line, drop to no-CODEPAGE.
pub const TSPL_CODEPAGE_LINE: &str = "CODEPAGE UTF-8";

/// TEMPORARY-DIAGNOSTIC probe switch (BRK-100 — REMOVE in the lock-in task).
/// One owner trial discriminates the TEXT-rejection cause (AUD-047 P1–P3):
/// `Normal` = today's bytes; `NoCodepage` = drop the CODEPAGE line (P1);
/// `AsciiOnly` = LOSSY fold of non-ASCII payload bytes to `?` (P2 — never a
/// default, diagnostic only); `Font1Only` = force font "1" everywhere (P3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TsplProbeMode {
    /// Production bytes (byte-identical to pre-probe output).
    #[default]
    Normal,
    /// P1: omit `CODEPAGE UTF-8` from the preamble.
    NoCodepage,
    /// P2: LOSSY — every non-ASCII scalar becomes `?`.
    AsciiOnly,
    /// P3: every TEXT line uses built-in font "1".
    Font1Only,
}

/// Canonical wire spelling of the default probe mode.
pub const TSPL_PROBE_MODE_DEFAULT: &str = "normal";

/// Parse a stored probe-mode string; unknown values reject LOUDLY with
/// `TSPL_PROBE_MODE_UNKNOWN` — never a silent fallback to `Normal`.
pub fn parse_tspl_probe_mode(raw: &str) -> Result<TsplProbeMode, DomainError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "normal" => Ok(TsplProbeMode::Normal),
        "no_codepage" => Ok(TsplProbeMode::NoCodepage),
        "ascii_only" => Ok(TsplProbeMode::AsciiOnly),
        "font1_only" => Ok(TsplProbeMode::Font1Only),
        _ => Err(DomainError::InvalidCanonicalString(
            "TSPL_PROBE_MODE_UNKNOWN".to_string(),
        )),
    }
}

impl TsplProbeMode {
    /// Canonical wire spelling (`normal|no_codepage|ascii_only|font1_only`).
    pub fn as_str(self) -> &'static str {
        match self {
            TsplProbeMode::Normal => "normal",
            TsplProbeMode::NoCodepage => "no_codepage",
            TsplProbeMode::AsciiOnly => "ascii_only",
            TsplProbeMode::Font1Only => "font1_only",
        }
    }
}

/// Self-describing media setup carried by every silent job — no NVRAM luck
/// (BRK-099, AUD-045). Label jobs use the stored gap; ticket jobs force
/// gap 0 (`GAP 0,0` = declared continuous).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TsplMediaSetup {
    /// Inter-label gap in mm, 0..=10 (`0` = continuous, ticket only).
    pub gap_mm: u32,
    /// Print density, 0..=15.
    pub density: u8,
    /// Print speed in inch/sec, 1..=6.
    pub speed: u8,
    /// TEMPORARY-DIAGNOSTIC probe mode (BRK-100 — remove in lock-in).
    pub probe_mode: TsplProbeMode,
}

impl Default for TsplMediaSetup {
    fn default() -> Self {
        Self {
            gap_mm: TSPL_GAP_MM_DEFAULT,
            density: TSPL_DENSITY_DEFAULT,
            speed: TSPL_SPEED_DEFAULT,
            probe_mode: TsplProbeMode::Normal,
        }
    }
}

/// Reject a label gap outside 0..=10 mm.
pub fn validate_tspl_gap_mm(gap_mm: u32) -> Result<(), DomainError> {
    if gap_mm > TSPL_GAP_MM_MAX {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_GAP_MM_OUT_OF_RANGE".to_string(),
        ));
    }
    Ok(())
}

/// Reject a density outside the 0..=15 TSC scale.
pub fn validate_tspl_density(density: u8) -> Result<(), DomainError> {
    if density > TSPL_DENSITY_MAX {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_DENSITY_OUT_OF_RANGE".to_string(),
        ));
    }
    Ok(())
}

/// Reject a speed outside 1..=6 inch/sec.
pub fn validate_tspl_speed(speed: u8) -> Result<(), DomainError> {
    if !(TSPL_SPEED_MIN..=TSPL_SPEED_MAX).contains(&speed) {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_SPEED_OUT_OF_RANGE".to_string(),
        ));
    }
    Ok(())
}

impl TsplMediaSetup {
    /// Reject any out-of-range field (codes quoted, never silent clamps).
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_tspl_gap_mm(self.gap_mm)?;
        validate_tspl_density(self.density)?;
        validate_tspl_speed(self.speed)?;
        Ok(())
    }
}

/// Emit the vendor-mirrored preamble in pinned order (Seagull ground truth,
/// `Vendor-Test.prn` 594 B, 2026-11-01): SIZE → GAP → SET RIBBON OFF →
/// DIRECTION 0,0 → REFERENCE 0,0 → OFFSET 0 mm → SET PEEL OFF →
/// SET CUTTER OFF → SET TEAR ON → CLS. The vendor sends NO
/// DENSITY/SPEED/CODEPAGE — the printer runs its calibrated NVRAM values;
/// our overrides are no longer emitted (stored keys kept, unemitted).
/// P1 NoCodepage is a preamble no-op now (nothing to drop); P2/P3 still
/// fold TEXT payloads only.
fn push_media_preamble(
    out: &mut Vec<String>,
    size_line: &str,
    media: &TsplMediaSetup,
) -> Result<(), DomainError> {
    media.validate()?;
    out.push(size_line.to_string());
    if media.gap_mm == 0 {
        // Continuous stock: the manual's no-gap form (no units).
        out.push("GAP 0,0".to_string());
    } else {
        // Metric gap REQUIRES the mm suffix on both values (TSC manual:
        // `GAP 3 mm,0 mm`). Bare `GAP 3,0` reads as 3 INCHES.
        out.push(format!("GAP {} mm, 0 mm", media.gap_mm));
    }
    out.push("SET RIBBON OFF".to_string());
    out.push("DIRECTION 0,0".to_string());
    out.push("REFERENCE 0,0".to_string());
    out.push("OFFSET 0 mm".to_string());
    out.push("SET PEEL OFF".to_string());
    out.push("SET CUTTER OFF".to_string());
    out.push("SET TEAR ON".to_string());
    out.push("CLS".to_string());
    Ok(())
}

/// Assumed bitmap font widths in dots (TSC TSPL reference) — see module
/// docs. Heights are captured in the PITCH_* line pitches below.
const FONT_1_W: i64 = 8;
const FONT_2_W: i64 = 12;
const FONT_3_W: i64 = 16;

/// Code128 auto-subset density: 11 modules per character plus 35 modules
/// of start/checksum/stop overhead (stated approximation).
const CODE128_MODULES_PER_CHAR: usize = 11;
const CODE128_OVERHEAD_MODULES: usize = 35;
/// Module width in dots (narrow = wide = 2 = 0.25 mm ≈ 9.8 mil @203 dpi).
const BARCODE_NARROW_DOTS: usize = 2;
/// Quiet zone reserved per side: 10 narrow modules × 2 dots (ISO minimum).
const QUIET_ZONE_DOTS: i64 = 10 * BARCODE_NARROW_DOTS as i64;
/// Ticket barcode height in dots (8.0 mm — see module math).
const TICKET_BARCODE_HEIGHT: i64 = 64;
/// Label Code128 bar height: 38 dots leaves a 2-dot gap to the human
/// line at y=146 (bars 106..144, text 146..158, 2-dot bottom margin).
const LABEL_BARCODE_HEIGHT: i64 = 38;
/// Maximum ticket barcode payload: 11·n + 35 modules must fit the media
/// minus both quiet zones (640 − 2×20 = 600 dots → 300 modules → n ≤ 24).
const TICKET_BARCODE_MAX_CHARS: usize =
    ((TICKET_WIDTH_DOTS as usize - 2 * QUIET_ZONE_DOTS as usize) / BARCODE_NARROW_DOTS
        - CODE128_OVERHEAD_MODULES)
        / CODE128_MODULES_PER_CHAR;
/// Maximum label barcode payload: 320 − 2×20 = 280 dots → 140 modules
/// → 11·n + 35 ≤ 140 → n ≤ 9.
const LABEL_BARCODE_MAX_CHARS: usize = ((LABEL_WIDTH_DOTS as usize - 2 * QUIET_ZONE_DOTS as usize)
    / BARCODE_NARROW_DOTS
    - CODE128_OVERHEAD_MODULES)
    / CODE128_MODULES_PER_CHAR;

/// A silent 40x20 mm shelf label, pre-formatted transcription.
pub struct TsplLabel<'a> {
    /// Brand micro-line (collapses when blank).
    pub brand_text: &'a str,
    /// Display name (single locale, chosen upstream), 2 lines max.
    pub name: &'a str,
    /// Plain price text, e.g. `"1234.50"`.
    pub price_text: &'a str,
    /// Currency code printed right of the price (collapses when blank).
    pub currency_text: &'a str,
    /// Code128 payload (SKU/barcode).
    pub barcode: &'a str,
    /// Copies, 1..=99 (enforced in `PRINT 1,n`, never by resending).
    pub copies: u32,
    /// Media identity for the self-describing preamble (validated).
    pub media: TsplMediaSetup,
}

/// Caption language for the ticket static labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsplCaptionLang {
    French,
    Arabic,
}

/// One ticket line, pre-formatted transcription of stored values.
pub struct TsplTicketLine<'a> {
    /// SKU as stored (`"—"` orphan fallback, graphique parity).
    pub sku: &'a str,
    /// Designation (locale resolved upstream, orphan fallback applied).
    pub designation: &'a str,
    /// Plain quantity, e.g. `"3"`.
    pub quantity_text: &'a str,
    /// Selling-unit label, e.g. `"Pièce"`.
    pub unit_text: &'a str,
    /// Net unit price text (discounted PU when stored, raw legacy PU).
    pub unit_price_text: &'a str,
    /// Stored line total text.
    pub line_total_text: &'a str,
    /// Per-unit discount badge text, or `None` for legacy NULL rows.
    pub discount_text: Option<&'a str>,
}

/// Stored ticket totals, pre-formatted (graphique-wire parity plus the
/// stored global discount — no recompute).
pub struct TsplTicketTotals<'a> {
    pub subtotal_ht: &'a str,
    /// Stored total global discount (`global_discount_centimes`).
    pub global_discount: &'a str,
    pub tva: &'a str,
    pub total_ttc: &'a str,
    pub paid: &'a str,
    pub change: &'a str,
    /// Payment method code as stored (e.g. `CASH`, `MIXED`).
    pub payment_method: &'a str,
}

/// A silent 80 mm ticket with the full BRK-098 receipt anatomy.
pub struct TsplTicket<'a> {
    pub store_name: &'a str,
    pub store_address: &'a str,
    pub store_phone: &'a str,
    /// Tax-id rows `(prefix, value)`; blank values collapse the row.
    pub tax_rows: &'a [(&'a str, &'a str)],
    /// Ticket number (also the barcode payload, charset-guarded).
    pub number: &'a str,
    /// Creation instant, pre-formatted (RFC3339 millis, worker parity).
    pub created_at: &'a str,
    /// Acting operator identifier (stored value, verbatim).
    pub cashier_text: &'a str,
    /// Selling warehouse identifier (stored value, verbatim).
    pub warehouse_text: &'a str,
    pub captions: TsplCaptionLang,
    pub lines: &'a [TsplTicketLine<'a>],
    pub totals: TsplTicketTotals<'a>,
    /// Footer message (empty lines skipped).
    pub footer: &'a str,
    /// Media identity for the self-describing preamble (validated;
    /// gap is forced to 0 = continuous for tickets).
    pub media: TsplMediaSetup,
}

/// Static captions per language (Arabic shaping caveat: see module docs).
struct Captions {
    subtotal: &'static str,
    discount_total: &'static str,
    tva: &'static str,
    total: &'static str,
    paid: &'static str,
    change: &'static str,
    thanks: &'static str,
    column_header: &'static str,
    discount_word: &'static str,
    depot_word: &'static str,
    cashier_word: &'static str,
}

fn captions(lang: TsplCaptionLang) -> Captions {
    match lang {
        TsplCaptionLang::French => Captions {
            subtotal: "Sous-total",
            discount_total: "Remise totale",
            tva: "TVA",
            total: "TOTAL TTC",
            paid: "Payé",
            change: "Rendu",
            thanks: "Merci de votre visite",
            column_header: "Qté x PU = Total",
            discount_word: "remise",
            depot_word: "Depot",
            cashier_word: "Caissier",
        },
        TsplCaptionLang::Arabic => Captions {
            subtotal: "المجموع الفرعي",
            discount_total: "الخصم الإجمالي",
            tva: "الضريبة",
            total: "الإجمالي",
            paid: "المدفوع",
            change: "الباقي",
            thanks: "شكراً لزيارتكم",
            column_header: "الكمية × سعر = المجموع",
            discount_word: "خصم",
            depot_word: "المخزن",
            cashier_word: "الكاشير",
        },
    }
}

/// Make free text safe for a quoted TSPL field: `"` would break command
/// framing (no escape exists) and CR/LF would inject commands.
/// Ticket printer face (DOC-02): F5 is the native 32x48 face — same
/// geometry the old scale-2 price occupied, but crisp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketFace {
    F1,
    F2,
    F3,
    F5,
}

impl TicketFace {
    fn face_str(self) -> &'static str {
        match self {
            TicketFace::F1 => "1",
            TicketFace::F2 => "2",
            TicketFace::F3 => "3",
            TicketFace::F5 => "5",
        }
    }
    fn scale(self) -> i64 {
        1
    }
    #[allow(dead_code)]
    fn cell_h(self) -> i64 {
        match self {
            TicketFace::F1 => FONT_1_H,
            TicketFace::F2 => FONT_2_H,
            TicketFace::F3 => FONT_3_H,
            TicketFace::F5 => 48,
        }
    }
}

/// Logical ticket text slot (DOC-01): readability key — bitmap lookup
/// is by LINE INDEX (dynamic rows), not by slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketTextSlot {
    StoreName,
    Contact,
    TaxRow,
    DocNumber,
    Instant,
    Cashier,
    Depot,
    ColumnHeader,
    LineName,
    LineDetail,
    LineBadge,
    TotalsRow,
    TotalTtc,
    Payment,
    Change,
    Human,
    Footer,
    Thanks,
}

/// Horizontal anchoring for bitmap re-layout (DOC-01): a BITMAP is
/// narrower than its TEXT cell, so centered/right lines re-anchor from
/// the bitmap width instead of reusing the TEXT origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketAlign {
    Left,
    Center,
    Right,
}

/// One ticket text line (DOC-01): exact text the renderer would emit,
/// TEXT origin, printer face, raster box, and alignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketTextLine {
    pub slot: TicketTextSlot,
    pub text: String,
    pub x_dots: i64,
    pub y_dots: i64,
    pub face: TicketFace,
    pub cell_h_dots: i64,
    pub box_w_dots: i64,
    pub box_h_dots: i64,
    pub align: TicketAlign,
}

/// Ticket layout op (DOC-01): text lines carry bitmaps, rules and the
/// barcode pass through untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketOp {
    Text(TicketTextLine),
    Rule(i64),
    Barcode {
        x_dots: i64,
        y_dots: i64,
        number: String,
    },
}

/// Ticket layout plan (DOC-01 + DOC-02): ALL geometry lives here — the
/// renderer only emits. Returns the ops plus the content end (dots) for
/// the SIZE 80 mm computation.
///
/// DOC-02 deltas vs the v1 renderer: the duplicated HT row is gone
/// (owner decision 1.b — Sous-total stays), TOTAL TTC uses the native
/// F5 face, and the barcode drops its printer-generated human line
/// (the TEXT line below stays the single source).
pub fn ticket_text_plan(ticket: &TsplTicket<'_>) -> (Vec<TicketOp>, i64) {
    let mut ops: Vec<TicketOp> = Vec::new();
    let mut y: i64 = 8;
    let right_edge = TICKET_WIDTH_DOTS - TICKET_MARGIN_X;
    let caps = captions(ticket.captions);
    let narrow_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / FONT_1_W) as usize;
    let wide_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / FONT_2_W) as usize;
    let name_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / FONT_3_W) as usize;
    let total_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / (FONT_3_W * 2)) as usize;
    let inner_w = TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X;

    // Store block (blank fields collapse — never empty lines).
    let store = truncate_to_width(ticket.store_name.trim(), name_chars);
    if !store.is_empty() {
        let dots = text_dots(store.chars().count(), FONT_3_W, 1);
        ops.push(TicketOp::Text(TicketTextLine {
            slot: TicketTextSlot::StoreName,
            text: store,
            x_dots: centered_x(TICKET_WIDTH_DOTS, dots),
            y_dots: y,
            face: TicketFace::F3,
            cell_h_dots: FONT_3_H,
            box_w_dots: inner_w,
            box_h_dots: PITCH_F3,
            align: TicketAlign::Center,
        }));
        y += PITCH_F3;
    }
    for line in [ticket.store_address, ticket.store_phone] {
        let shown = truncate_to_width(line.trim(), narrow_chars);
        if !shown.is_empty() {
            let dots = text_dots(shown.chars().count(), FONT_1_W, 1);
            ops.push(TicketOp::Text(TicketTextLine {
                slot: TicketTextSlot::Contact,
                text: shown,
                x_dots: centered_x(TICKET_WIDTH_DOTS, dots),
                y_dots: y,
                face: TicketFace::F1,
                cell_h_dots: FONT_1_H,
                box_w_dots: inner_w,
                box_h_dots: PITCH_F1,
                align: TicketAlign::Center,
            }));
            y += PITCH_F1;
        }
    }
    for (prefix, value) in ticket.tax_rows {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let joined = format!("{prefix}: {trimmed}");
        let shown = truncate_to_width(&joined, narrow_chars);
        let dots = text_dots(shown.chars().count(), FONT_1_W, 1);
        ops.push(TicketOp::Text(TicketTextLine {
            slot: TicketTextSlot::TaxRow,
            text: shown,
            x_dots: centered_x(TICKET_WIDTH_DOTS, dots),
            y_dots: y,
            face: TicketFace::F1,
            cell_h_dots: FONT_1_H,
            box_w_dots: inner_w,
            box_h_dots: PITCH_F1,
            align: TicketAlign::Center,
        }));
        y += PITCH_F1;
    }
    ops.push(TicketOp::Rule(y));
    y += 10;
    // Doc meta: number, capture instant, cashier, depot.
    let number = sanitize_tspl_text(ticket.number);
    let number_shown = truncate_to_width(&number, wide_chars);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::DocNumber,
        text: number_shown,
        x_dots: TICKET_MARGIN_X,
        y_dots: y,
        face: TicketFace::F2,
        cell_h_dots: FONT_2_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F2,
        align: TicketAlign::Left,
    }));
    y += PITCH_F2;
    let instant = sanitize_tspl_text(ticket.created_at);
    let instant_shown = truncate_to_width(&instant[..instant.len().min(19)], narrow_chars);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::Instant,
        text: instant_shown,
        x_dots: TICKET_MARGIN_X,
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Left,
    }));
    y += PITCH_F1;
    let cashier = format!(
        "{}: {}",
        caps.cashier_word,
        sanitize_tspl_text(ticket.cashier_text)
    );
    let cashier_shown = truncate_to_width(&cashier, narrow_chars);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::Cashier,
        text: cashier_shown,
        x_dots: TICKET_MARGIN_X,
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Left,
    }));
    y += PITCH_F1;
    let depot = format!(
        "{}: {}",
        caps.depot_word,
        sanitize_tspl_text(ticket.warehouse_text)
    );
    let depot_shown = truncate_to_width(&depot, narrow_chars);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::Depot,
        text: depot_shown,
        x_dots: TICKET_MARGIN_X,
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Left,
    }));
    y += PITCH_F1;
    ops.push(TicketOp::Rule(y));
    y += 10;
    // Column header.
    let header = truncate_to_width(caps.column_header, narrow_chars);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::ColumnHeader,
        text: header,
        x_dots: TICKET_MARGIN_X,
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Left,
    }));
    y += PITCH_F1;
    // Item rows.
    for line in ticket.lines {
        let designation = truncate_to_width(line.designation.trim(), wide_chars);
        ops.push(TicketOp::Text(TicketTextLine {
            slot: TicketTextSlot::LineName,
            text: designation,
            x_dots: TICKET_MARGIN_X,
            y_dots: y,
            face: TicketFace::F2,
            cell_h_dots: FONT_2_H,
            box_w_dots: inner_w,
            box_h_dots: PITCH_F2,
            align: TicketAlign::Left,
        }));
        y += PITCH_F2;
        let detail = if line.unit_text.trim().is_empty() {
            format!(
                "{} x {} = {}",
                sanitize_tspl_text(line.quantity_text),
                sanitize_tspl_text(line.unit_price_text),
                sanitize_tspl_text(line.line_total_text),
            )
        } else {
            format!(
                "{} {} x {} = {}",
                sanitize_tspl_text(line.quantity_text),
                sanitize_tspl_text(line.unit_text),
                sanitize_tspl_text(line.unit_price_text),
                sanitize_tspl_text(line.line_total_text),
            )
        };
        let detail_shown = truncate_to_width(&detail, narrow_chars);
        let detail_dots = text_dots(detail_shown.chars().count(), FONT_1_W, 1);
        ops.push(TicketOp::Text(TicketTextLine {
            slot: TicketTextSlot::LineDetail,
            text: detail_shown,
            x_dots: right_x(right_edge, detail_dots),
            y_dots: y,
            face: TicketFace::F1,
            cell_h_dots: FONT_1_H,
            box_w_dots: inner_w,
            box_h_dots: PITCH_F1,
            align: TicketAlign::Right,
        }));
        y += PITCH_F1;
        if let Some(discount) = line.discount_text {
            let badge = format!("{} {}", caps.discount_word, sanitize_tspl_text(discount));
            let badge_shown = truncate_to_width(&badge, narrow_chars / 2);
            let badge_dots = text_dots(badge_shown.chars().count(), FONT_1_W, 1);
            ops.push(TicketOp::Text(TicketTextLine {
                slot: TicketTextSlot::LineBadge,
                text: badge_shown,
                x_dots: right_x(right_edge, badge_dots),
                y_dots: y,
                face: TicketFace::F1,
                cell_h_dots: FONT_1_H,
                box_w_dots: inner_w,
                box_h_dots: PITCH_F1,
                align: TicketAlign::Right,
            }));
            y += PITCH_F1;
        }
    }
    ops.push(TicketOp::Rule(y));
    y += 10;
    // Totals hierarchy (right-aligned); DOC-02: the duplicated HT row is
    // gone (owner 1.b) — Sous-total stays the single HT figure.
    let totals_rows = [
        (
            TicketTextSlot::TotalsRow,
            caps.subtotal,
            ticket.totals.subtotal_ht,
        ),
        (
            TicketTextSlot::TotalsRow,
            caps.discount_total,
            ticket.totals.global_discount,
        ),
        (TicketTextSlot::TotalsRow, caps.tva, ticket.totals.tva),
    ];
    for (slot, caption, value) in totals_rows {
        let row = format!("{caption}: {}", sanitize_tspl_text(value));
        let shown = truncate_to_width(&row, wide_chars);
        let dots = text_dots(shown.chars().count(), FONT_2_W, 1);
        ops.push(TicketOp::Text(TicketTextLine {
            slot,
            text: shown,
            x_dots: right_x(right_edge, dots),
            y_dots: y,
            face: TicketFace::F2,
            cell_h_dots: FONT_2_H,
            box_w_dots: inner_w,
            box_h_dots: PITCH_F2,
            align: TicketAlign::Right,
        }));
        y += PITCH_F2;
    }
    let total_row = format!(
        "{}: {}",
        caps.total,
        sanitize_tspl_text(ticket.totals.total_ttc)
    );
    let total_shown = truncate_to_width(&total_row, total_chars);
    let total_dots = text_dots(total_shown.chars().count(), FONT_3_W * 2, 1);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::TotalTtc,
        text: total_shown,
        x_dots: right_x(right_edge, total_dots),
        y_dots: y,
        face: TicketFace::F5,
        cell_h_dots: 48,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F3_X2,
        align: TicketAlign::Right,
    }));
    y += PITCH_F3_X2;
    // Payment block.
    let payment = format!(
        "{} ({}): {}",
        caps.paid,
        sanitize_tspl_text(ticket.totals.payment_method),
        sanitize_tspl_text(ticket.totals.paid),
    );
    let payment_shown = truncate_to_width(&payment, narrow_chars);
    let payment_dots = text_dots(payment_shown.chars().count(), FONT_1_W, 1);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::Payment,
        text: payment_shown,
        x_dots: right_x(right_edge, payment_dots),
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Right,
    }));
    y += PITCH_F1;
    let change_row = format!(
        "{}: {}",
        caps.change,
        sanitize_tspl_text(ticket.totals.change)
    );
    let change_shown = truncate_to_width(&change_row, narrow_chars);
    let change_dots = text_dots(change_shown.chars().count(), FONT_1_W, 1);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::Change,
        text: change_shown,
        x_dots: right_x(right_edge, change_dots),
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Right,
    }));
    y += PITCH_F1;
    // Doc-number barcode (centered, quiet zones >= 20 dots per side) with
    // the human-readable number beneath (DOC-02: printer-generated line
    // OFF — the TEXT line below stays the single source).
    y += 8;
    let barcode_dots = barcode_width_dots(number.chars().count()) as i64;
    let barcode_x = centered_x(TICKET_WIDTH_DOTS, barcode_dots);
    ops.push(TicketOp::Barcode {
        x_dots: barcode_x,
        y_dots: y,
        number: number.clone(),
    });
    y += TICKET_BARCODE_HEIGHT + 4;
    let human = truncate_to_width(&number, narrow_chars);
    let human_dots = text_dots(human.chars().count(), FONT_1_W, 1);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::Human,
        text: human,
        x_dots: centered_x(TICKET_WIDTH_DOTS, human_dots),
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Center,
    }));
    y += PITCH_F1 + 4;
    // Footer: owner message (collapses when blank) + constant thanks.
    let footer = truncate_to_width(ticket.footer.trim(), narrow_chars);
    if !footer.is_empty() {
        let dots = text_dots(footer.chars().count(), FONT_1_W, 1);
        ops.push(TicketOp::Text(TicketTextLine {
            slot: TicketTextSlot::Footer,
            text: footer,
            x_dots: centered_x(TICKET_WIDTH_DOTS, dots),
            y_dots: y,
            face: TicketFace::F1,
            cell_h_dots: FONT_1_H,
            box_w_dots: inner_w,
            box_h_dots: PITCH_F1,
            align: TicketAlign::Center,
        }));
        y += PITCH_F1;
    }
    let thanks = truncate_to_width(caps.thanks, narrow_chars);
    let thanks_dots = text_dots(thanks.chars().count(), FONT_1_W, 1);
    ops.push(TicketOp::Text(TicketTextLine {
        slot: TicketTextSlot::Thanks,
        text: thanks,
        x_dots: centered_x(TICKET_WIDTH_DOTS, thanks_dots),
        y_dots: y,
        face: TicketFace::F1,
        cell_h_dots: FONT_1_H,
        box_w_dots: inner_w,
        box_h_dots: PITCH_F1,
        align: TicketAlign::Center,
    }));
    y += PITCH_F1 + 16;
    (ops, y)
}

fn sanitize_tspl_text(value: &str) -> String {
    value.replace('"', "'").replace(['\r', '\n'], " ")
}

/// Code128 payload guard: printable ASCII only (0x20..=0x7E), excluding
/// `"` (0x22, unquotable in TSPL — see module docs). Never mojibake.
fn validate_code128_payload(payload: &str) -> Result<(), DomainError> {
    let ok = payload
        .bytes()
        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        && !payload.bytes().any(|byte| byte == b'"');
    if ok {
        Ok(())
    } else {
        Err(DomainError::TsplBarcodeUnencodable)
    }
}

/// Barcode width in dots for a payload at the fixed narrow width.
fn barcode_width_dots(payload_len_chars: usize) -> usize {
    (payload_len_chars * CODE128_MODULES_PER_CHAR + CODE128_OVERHEAD_MODULES) * BARCODE_NARROW_DOTS
}

/// Truncate to a field width in characters (dot budget / font width),
/// marking with ASCII "...". Character-based so multi-byte scripts are
/// not penalized.
fn truncate_to_width(value: &str, max_chars: usize) -> String {
    let clean = sanitize_tspl_text(value);
    if clean.chars().count() <= max_chars {
        return clean;
    }
    if max_chars <= 3 {
        return clean.chars().take(max_chars).collect();
    }
    let kept: String = clean.chars().take(max_chars - 3).collect();
    format!("{kept}...")
}

/// Dots a text run occupies (monospace approximation, byte-pinned).
fn text_dots(chars: usize, font_w: i64, scale: i64) -> i64 {
    chars as i64 * font_w * scale
}

/// Centered x inside the media width, clamped at 0.
fn centered_x(media: i64, dots: i64) -> i64 {
    ((media - dots) / 2).max(0)
}

/// x such that the run ends at the right edge, clamped at 0.
fn right_x(right_edge: i64, dots: i64) -> i64 {
    (right_edge - dots).max(0)
}

/// Emit one TEXT command. TEMPORARY-DIAGNOSTIC (BRK-100): `Font1Only` forces
/// face "1"; `AsciiOnly` folds the payload LOSSY (see `ascii_probe_lossy`).
fn push_text(
    out: &mut Vec<String>,
    x: i64,
    y: i64,
    font: &str,
    scale: i64,
    content: &str,
    probe: TsplProbeMode,
) {
    let face = if probe == TsplProbeMode::Font1Only {
        "1"
    } else {
        font
    };
    if probe == TsplProbeMode::AsciiOnly {
        let lossy = ascii_probe_lossy(content);
        out.push(format!(
            "TEXT {x},{y},\"{face}\",0,{scale},{scale},\"{lossy}\""
        ));
    } else {
        out.push(format!(
            "TEXT {x},{y},\"{face}\",0,{scale},{scale},\"{content}\""
        ));
    }
}

/// TEMPORARY-DIAGNOSTIC LOSSY fold (BRK-100 P2 — remove in lock-in): every
/// non-ASCII scalar becomes `?`. Layout widths stay computed on the original
/// text, so alignment may shift — acceptable: the probe asks print-vs-ignore,
/// never beauty.
fn ascii_probe_lossy(content: &str) -> String {
    content
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect()
}

/// Ticket line pitches in dots (font height + leading).
const PITCH_F1: i64 = 20;
const PITCH_F2: i64 = 28;
const PITCH_F3: i64 = 32;
/// Advance after a double-height "3" row (2 × 24 + 8).
const PITCH_F3_X2: i64 = 56;
/// Ticket side margin.
const TICKET_MARGIN_X: i64 = 16;

/// Which TEXT line of the 40x20 label a bitmap replaces (PRT-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelTextSlot {
    Brand,
    NameLine1,
    NameLine2,
    Price,
    Currency,
    Human,
}

impl LabelTextSlot {
    /// Stable wire spelling for previews and logs (DOC-03).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LabelTextSlot::Brand => "brand",
            LabelTextSlot::NameLine1 => "name1",
            LabelTextSlot::NameLine2 => "name2",
            LabelTextSlot::Price => "price",
            LabelTextSlot::Currency => "currency",
            LabelTextSlot::Human => "human",
        }
    }
}

/// Engine barcode placement on the 40x20 label (DOC-03): fixed origin,
/// width from the validated Code128 payload. The preview and the
/// renderer share this — one geometry, no drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelBarcodeGeometry {
    pub x_dots: i64,
    pub y_dots: i64,
    pub width_dots: i64,
    pub height_dots: i64,
}

/// Validate the payload and return the engine barcode geometry (DOC-03).
/// Rejects LOUDLY on the same rules as the renderer (payload charset,
/// 9-char module budget) — the preview never draws what won't spool.
pub fn label_barcode_geometry(barcode: &str) -> Result<LabelBarcodeGeometry, DomainError> {
    validate_code128_payload(barcode)?;
    if barcode.chars().count() > LABEL_BARCODE_MAX_CHARS {
        // 11·n + 35 modules must fit 140 (quiet zones 20 dots per side).
        return Err(DomainError::TsplBarcodeTooWide);
    }
    Ok(LabelBarcodeGeometry {
        x_dots: 8,
        y_dots: 106,
        width_dots: barcode_width_dots(barcode.chars().count()) as i64,
        height_dots: LABEL_BARCODE_HEIGHT,
    })
}

/// One printable TEXT line of the 40x20 label (PRT-02 plan; geometry is
/// single-sourced here — the renderer and the bitmap path share it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelTextLine {
    /// Slot the line occupies (bitmap lookup key).
    pub slot: LabelTextSlot,
    /// Exact text the renderer would emit (truncated, sanitized).
    pub text: String,
    /// TEXT origin (a bitmap replacing the line bottom-aligns here).
    pub x_dots: i64,
    pub y_dots: i64,
    /// Printer cell height × scale (bitmap bottom-align reference).
    pub cell_h_dots: i64,
    /// Bitmap box the raster port must fit (cell + diacritic pad).
    pub box_w_dots: i64,
    pub box_h_dots: i64,
}

/// TSC bitmap cell heights (dots): font "1" is 8x12, "2" is 12x20,
/// "3" is 16x24 — confirmed by the label geometry itself (price at 2x24
/// ends at y=106 where the barcode starts; human at 148+12 ends at the
/// 160-dot bottom edge).
const FONT_1_H: i64 = 12;
const FONT_2_H: i64 = 20;
const FONT_3_H: i64 = 24;
/// Diacritic headroom for shaped Arabic (marks above/below the cell).
const LABEL_BITMAP_PAD_DOTS: i64 = 4;

/// Pre-rasterized 1-bit line replacing one TEXT slot (PRT-02): row-major,
/// MSB-first, each row padded to whole bytes — the `BITMAP` job packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelBitmap {
    /// Row width in bytes (dots = `width_bytes` × 8).
    pub width_bytes: usize,
    /// Row count in dots.
    pub height_rows: usize,
    /// Packed rows, `width_bytes` × `height_rows` bytes.
    pub data: Vec<u8>,
}

/// Layout the label TEXT lines (pure; truncation identical to the
/// renderer — `render_label_40x20` and the mixed path share this plan).
#[must_use]
pub fn label_text_plan(label: &TsplLabel<'_>) -> Vec<LabelTextLine> {
    // 8-dot right margin: the die-cut edge clips anything past x=304
    // (photo-verified LIVE-2026-11-06: "DZD" sliced at x=312).
    // Professional inset: 3 mm safe margin for dispensing tolerance.
    let right_edge = LABEL_WIDTH_DOTS - 24;
    let brand_chars = ((LABEL_WIDTH_DOTS - 16) / FONT_1_W) as usize;
    let name_chars = ((LABEL_WIDTH_DOTS - 16) / FONT_2_W) as usize;
    let price_chars = ((LABEL_WIDTH_DOTS - 16) / (FONT_3_W * 2)) as usize;
    let mut plan = Vec::new();
    // Brand micro-line (collapses when blank).
    let brand = truncate_to_width(label.brand_text.trim(), brand_chars);
    if !brand.is_empty() {
        let x = centered_x(
            LABEL_WIDTH_DOTS,
            text_dots(brand.chars().count(), FONT_1_W, 1),
        );
        plan.push(LabelTextLine {
            slot: LabelTextSlot::Brand,
            text: brand,
            x_dots: x,
            y_dots: 2,
            cell_h_dots: FONT_1_H,
            box_w_dots: i64::from(brand_chars as u32) * FONT_1_W,
            box_h_dots: FONT_1_H + LABEL_BITMAP_PAD_DOTS,
        });
    }
    // Designation: two lines max, second line marked "..." when cut.
    let clean_name = sanitize_tspl_text(label.name.trim());
    let first: String = clean_name.chars().take(name_chars).collect();
    let rest: String = clean_name.chars().skip(name_chars).collect();
    plan.push(LabelTextLine {
        slot: LabelTextSlot::NameLine1,
        text: first,
        x_dots: 8,
        y_dots: 16,
        cell_h_dots: FONT_2_H,
        box_w_dots: i64::from(name_chars as u32) * FONT_2_W,
        box_h_dots: FONT_2_H + LABEL_BITMAP_PAD_DOTS,
    });
    if !rest.trim().is_empty() {
        plan.push(LabelTextLine {
            slot: LabelTextSlot::NameLine2,
            text: truncate_to_width(&rest, name_chars),
            x_dots: 8,
            y_dots: 38,
            cell_h_dots: FONT_2_H,
            box_w_dots: i64::from(name_chars as u32) * FONT_2_W,
            box_h_dots: FONT_2_H + LABEL_BITMAP_PAD_DOTS,
        });
    }
    // Price double-height, currency right-aligned inside the price band.
    let price = truncate_to_width(label.price_text.trim(), price_chars);
    plan.push(LabelTextLine {
        slot: LabelTextSlot::Price,
        text: price,
        x_dots: 8,
        y_dots: 58,
        cell_h_dots: FONT_3_H * 2,
        box_w_dots: i64::from(price_chars as u32) * FONT_3_W * 2,
        box_h_dots: FONT_3_H * 2 + LABEL_BITMAP_PAD_DOTS,
    });
    let currency = truncate_to_width(label.currency_text.trim(), brand_chars);
    if !currency.is_empty() {
        let x = right_x(right_edge, text_dots(currency.chars().count(), FONT_1_W, 1));
        plan.push(LabelTextLine {
            slot: LabelTextSlot::Currency,
            text: currency,
            x_dots: x,
            y_dots: 94,
            cell_h_dots: FONT_1_H,
            box_w_dots: i64::from(brand_chars as u32) * FONT_1_W,
            box_h_dots: FONT_1_H + LABEL_BITMAP_PAD_DOTS,
        });
    }
    plan.push(LabelTextLine {
        slot: LabelTextSlot::Human,
        text: truncate_to_width(&sanitize_tspl_text(label.barcode), brand_chars),
        x_dots: centered_x(
            LABEL_WIDTH_DOTS,
            text_dots(
                truncate_to_width(&sanitize_tspl_text(label.barcode), brand_chars)
                    .chars()
                    .count(),
                FONT_1_W,
                1,
            ),
        ),
        // 2-dot bottom margin: y=148 sat exactly on the die-cut edge and
        // the last row printed half-clipped (photo-verified LIVE-2026-11-06).
        y_dots: 146,
        cell_h_dots: FONT_1_H,
        box_w_dots: i64::from(brand_chars as u32) * FONT_1_W,
        box_h_dots: FONT_1_H + LABEL_BITMAP_PAD_DOTS,
    });
    plan
}

/// Whether a label TEXT payload must travel as a bitmap (PRT-02): true
/// when the printable-TEXT check would reject it. The raw-byte and
/// lossy-fold probes never need bitmaps (they never reject TEXT).
#[must_use]
pub fn label_text_needs_bitmap(content: &str, probe: TsplProbeMode) -> bool {
    match probe {
        TsplProbeMode::NoCodepage | TsplProbeMode::AsciiOnly => false,
        TsplProbeMode::Normal | TsplProbeMode::Font1Only => !content.chars().all(|c| {
            c.is_ascii() || matches!(c, '\u{0080}'..='\u{024F}' | '\u{1E00}'..='\u{1EFF}')
        }),
    }
}

/// Reject a label TEXT payload the printer would silently eat (PRT-02):
/// any scalar outside ASCII + Latin blocks, except in the raw-byte
/// diagnostic probe. French accents ride `CODEPAGE UTF-8` (built-in
/// glyphs); Arabic/CJK/emoji have no printer glyphs and must travel as
/// a bitmap — failing LOUDLY here instead of a dead label.
fn check_label_text_printable(content: &str, probe: TsplProbeMode) -> Result<(), DomainError> {
    if probe == TsplProbeMode::NoCodepage {
        return Ok(());
    }
    let emitted = if probe == TsplProbeMode::AsciiOnly {
        ascii_probe_lossy(content)
    } else {
        content.to_string()
    };
    let printable = emitted
        .chars()
        .all(|c| c.is_ascii() || matches!(c, '\u{0080}'..='\u{024F}' | '\u{1E00}'..='\u{1EFF}'));
    if printable {
        Ok(())
    } else {
        Err(DomainError::InvalidCanonicalString(
            "LABEL_TEXT_UNPRINTABLE".to_string(),
        ))
    }
}

/// Render the 40x20 mm label. Exact bytes are pinned in module tests.
pub fn render_label_40x20(label: &TsplLabel<'_>) -> Result<Vec<u8>, DomainError> {
    render_label_40x20_mixed(label, &[])
}

/// Render the 40x20 mm label with bitmap slots (PRT-02): every planned
/// TEXT line whose slot has a bitmap is replaced by a `BITMAP` chunk at
/// the line origin (bottom-aligned); all other lines emit TEXT unchanged,
/// so an all-ASCII job is byte-identical to `render_label_40x20`.
pub fn render_label_40x20_mixed(
    label: &TsplLabel<'_>,
    bitmaps: &[(LabelTextSlot, LabelBitmap)],
) -> Result<Vec<u8>, DomainError> {
    if label.copies < TSPL_MIN_COPIES || label.copies > TSPL_MAX_COPIES {
        return Err(DomainError::TsplCopiesOutOfRange);
    }
    if label.name.trim().is_empty() {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_NAME_MUST_NOT_BE_EMPTY".to_string(),
        ));
    }
    if label.price_text.trim().is_empty() {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_PRICE_MUST_NOT_BE_EMPTY".to_string(),
        ));
    }
    validate_code128_payload(label.barcode)?;
    if label.barcode.chars().count() > LABEL_BARCODE_MAX_CHARS {
        // 11·n + 35 modules must fit 140 (quiet zones 20 dots per side).
        return Err(DomainError::TsplBarcodeTooWide);
    }
    let code = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    let mut seen = [false; 6];
    for (slot, _) in bitmaps {
        let index = *slot as usize;
        if seen[index] {
            return Err(code("LABEL_DUPLICATE_BITMAP_SLOT"));
        }
        seen[index] = true;
    }
    let barcode = sanitize_tspl_text(label.barcode);
    // Single-sourced geometry (DOC-03): the preview reads the same origin.
    let geometry = label_barcode_geometry(label.barcode)?;
    let (bx, by, bh) = (geometry.x_dots, geometry.y_dots, geometry.height_dots);
    let probe = label.media.probe_mode;
    let mut head: Vec<String> = Vec::new();
    push_media_preamble(&mut head, "SIZE 40 mm, 20 mm", &label.media)?;
    let mut job: Vec<u8> = head.join("\r\n").into_bytes();
    job.extend_from_slice(b"\r\n");
    for line in &label_text_plan(label) {
        if let Some((_, raster)) = bitmaps.iter().find(|(slot, _)| slot == &line.slot) {
            if raster.data.len() != raster.width_bytes * raster.height_rows {
                return Err(code("LABEL_BITMAP_LENGTH_MISMATCH"));
            }
            if raster.width_bytes == 0 || raster.height_rows == 0 {
                return Err(code("BITMAP_DIMENSIONS_MUST_BE_POSITIVE"));
            }
            if raster
                .width_bytes
                .checked_mul(8)
                .filter(|dots| *dots <= BITMAP_MAX_WIDTH_DOTS)
                .is_none()
            {
                return Err(code("BITMAP_WIDTH_TOO_WIDE"));
            }
            if raster.height_rows > BITMAP_MAX_HEIGHT_DOTS {
                return Err(code("BITMAP_TOO_TALL"));
            }
            if raster.data.iter().all(|byte| *byte == 0x00)
                || raster.data.iter().all(|byte| *byte == 0xFF)
            {
                return Err(code("BITMAP_BLANK_IMAGE"));
            }
            let width_dots =
                i64::try_from(raster.width_bytes * 8).map_err(|_| code("BITMAP_TOO_LARGE"))?;
            let height_dots =
                i64::try_from(raster.height_rows).map_err(|_| code("BITMAP_TOO_LARGE"))?;
            if width_dots > line.box_w_dots || height_dots > line.box_h_dots {
                return Err(code("LABEL_BITMAP_EXCEEDS_SLOT"));
            }
            // Bottom-align to the TEXT cell (taller rasters grow upward).
            let bitmap_y = line.y_dots + line.cell_h_dots - height_dots;
            if line.x_dots < 0
                || bitmap_y < 0
                || line.x_dots + width_dots > LABEL_WIDTH_DOTS
                || bitmap_y + height_dots > LABEL_HEIGHT_DOTS
            {
                return Err(code("LABEL_BITMAP_OUT_OF_AREA"));
            }
            job.extend_from_slice(
                format!(
                    "BITMAP {},{},{},{},0,",
                    line.x_dots, bitmap_y, raster.width_bytes, raster.height_rows
                )
                .as_bytes(),
            );
            job.extend_from_slice(&raster.data);
            job.extend_from_slice(b"\r\n");
        } else {
            check_label_text_printable(&line.text, probe)?;
            let mut text_out: Vec<String> = Vec::new();
            let (font, scale) = match line.slot {
                LabelTextSlot::Brand | LabelTextSlot::Currency | LabelTextSlot::Human => ("1", 1),
                LabelTextSlot::NameLine1 | LabelTextSlot::NameLine2 => ("2", 1),
                // Native 32x48 face: same geometry as "3"x2 but crisp
                // (no scaler stair-steps on the price curves).
                LabelTextSlot::Price => ("5", 1),
            };
            push_text(
                &mut text_out,
                line.x_dots,
                line.y_dots,
                font,
                scale,
                &line.text,
                probe,
            );
            job.extend_from_slice(text_out.join("\r\n").as_bytes());
            job.extend_from_slice(b"\r\n");
        }
        if line.slot == LabelTextSlot::Currency {
            job.extend_from_slice(
                format!("BARCODE {bx},{by},\"128\",{bh},0,0,2,2,\"{barcode}\"\r\n").as_bytes(),
            );
        }
    }
    job.extend_from_slice(format!("PRINT 1,{}\r\n", label.copies).as_bytes());
    Ok(job)
}

/// Print-template version stamped on every new document (DOC-04) and
/// pinned on reprint: v1 is the renderer as of the DOC-01/DOC-02
/// redesign. Future geometry changes bump this and add a dispatch branch;
/// v1 stays frozen so old documents reprint byte-identical forever.
pub const TSPL_TICKET_TEMPLATE_VERSION: u32 = 1;

/// Render the 80 mm ticket with a pinned template version (DOC-04).
pub fn render_ticket_80mm_versioned(
    ticket: &TsplTicket<'_>,
    template_version: u32,
) -> Result<Vec<u8>, DomainError> {
    render_ticket_80mm_mixed_versioned(ticket, &[], template_version)
}

/// Render the 80 mm ticket with bitmaps and a pinned template version
/// (DOC-04): v1 is byte-identical to [`render_ticket_80mm_mixed`]; any
/// unknown version rejects LOUDLY so a reprint can never silently drift.
pub fn render_ticket_80mm_mixed_versioned(
    ticket: &TsplTicket<'_>,
    bitmaps: &[(usize, LabelBitmap)],
    template_version: u32,
) -> Result<Vec<u8>, DomainError> {
    match template_version {
        TSPL_TICKET_TEMPLATE_VERSION => render_ticket_80mm_mixed(ticket, bitmaps),
        _ => Err(DomainError::InvalidCanonicalString(
            "TSPL_TEMPLATE_VERSION_UNKNOWN".to_string(),
        )),
    }
}

/// Render the 80 mm gapless ticket. Height is computed from content:
/// `SIZE 80 mm,H mm` with H = ceil(total_dots / 8).
pub fn render_ticket_80mm(ticket: &TsplTicket<'_>) -> Result<Vec<u8>, DomainError> {
    render_ticket_80mm_mixed(ticket, &[])
}

/// Render the 80 mm ticket with bitmap slots (DOC-01): every planned
/// TEXT line carrying a bitmap (keyed by LINE INDEX — tickets have
/// dynamic rows) is replaced by a `BITMAP` chunk re-anchored from its
/// alignment (bottom-aligned to the TEXT cell, like labels); all other
/// lines emit TEXT unchanged, so an all-TEXT job is byte-identical to
/// `render_ticket_80mm`.
pub fn render_ticket_80mm_mixed(
    ticket: &TsplTicket<'_>,
    bitmaps: &[(usize, LabelBitmap)],
) -> Result<Vec<u8>, DomainError> {
    if ticket.number.trim().is_empty() {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_NUMBER_MUST_NOT_BE_EMPTY".to_string(),
        ));
    }
    validate_code128_payload(ticket.number)?;
    if ticket.number.chars().count() > TICKET_BARCODE_MAX_CHARS {
        // 11·n + 35 modules must fit 300 (quiet zones 20 dots per side).
        return Err(DomainError::TsplBarcodeTooWide);
    }
    // Bitmap lookup is positional: duplicate indices would silently shadow.
    for (i, (a, _)) in bitmaps.iter().enumerate() {
        for (b, _) in &bitmaps[..i] {
            if a == b {
                return Err(DomainError::InvalidCanonicalString(
                    "TICKET_DUPLICATE_BITMAP".to_string(),
                ));
            }
        }
    }
    let probe = ticket.media.probe_mode;
    let (ops, end_y) = ticket_text_plan(ticket);
    let code = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    let mut body: Vec<u8> = Vec::new();
    let mut text_idx: usize = 0;
    for op in &ops {
        match op {
            TicketOp::Rule(y) => {
                body.extend_from_slice(format!("BAR 0,{y},{TICKET_WIDTH_DOTS},2\r\n").as_bytes());
            }
            TicketOp::Barcode {
                x_dots,
                y_dots,
                number,
            } => {
                body.extend_from_slice(
                    format!(
                        "BARCODE {x_dots},{y_dots},\"128\",{TICKET_BARCODE_HEIGHT},0,0,2,2,\"{number}\"\r\n"
                    )
                    .as_bytes(),
                );
            }
            TicketOp::Text(line) => {
                let idx = text_idx;
                text_idx += 1;
                if let Some((_, raster)) = bitmaps.iter().find(|(id, _)| *id == idx) {
                    if raster.data.len() != raster.width_bytes * raster.height_rows {
                        return Err(code("TICKET_BITMAP_LENGTH_MISMATCH"));
                    }
                    if raster.width_bytes == 0 || raster.height_rows == 0 {
                        return Err(code("BITMAP_DIMENSIONS_MUST_BE_POSITIVE"));
                    }
                    if raster
                        .width_bytes
                        .checked_mul(8)
                        .filter(|dots| *dots <= BITMAP_MAX_WIDTH_DOTS)
                        .is_none()
                    {
                        return Err(code("BITMAP_WIDTH_TOO_WIDE"));
                    }
                    if raster.height_rows > BITMAP_MAX_HEIGHT_DOTS {
                        return Err(code("BITMAP_TOO_TALL"));
                    }
                    if raster.data.iter().all(|byte| *byte == 0x00)
                        || raster.data.iter().all(|byte| *byte == 0xFF)
                    {
                        return Err(code("BITMAP_BLANK_IMAGE"));
                    }
                    let width_dots = i64::try_from(raster.width_bytes * 8)
                        .map_err(|_| code("BITMAP_TOO_LARGE"))?;
                    let height_dots =
                        i64::try_from(raster.height_rows).map_err(|_| code("BITMAP_TOO_LARGE"))?;
                    if width_dots > line.box_w_dots || height_dots > line.box_h_dots {
                        return Err(code("TICKET_BITMAP_EXCEEDS_SLOT"));
                    }
                    // Bottom-align to the TEXT cell (taller rasters grow upward).
                    let bitmap_y = line.y_dots + line.cell_h_dots - height_dots;
                    // Re-anchor from the bitmap width (TEXT x assumed full cell).
                    let bitmap_x = match line.align {
                        TicketAlign::Left => line.x_dots,
                        TicketAlign::Center => (TICKET_WIDTH_DOTS - width_dots) / 2,
                        TicketAlign::Right => (TICKET_WIDTH_DOTS - TICKET_MARGIN_X) - width_dots,
                    };
                    if bitmap_x < 0 || bitmap_y < 0 || bitmap_x + width_dots > TICKET_WIDTH_DOTS {
                        return Err(code("TICKET_BITMAP_OUT_OF_AREA"));
                    }
                    body.extend_from_slice(
                        format!(
                            "BITMAP {bitmap_x},{bitmap_y},{},{},0,",
                            raster.width_bytes, raster.height_rows
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(&raster.data);
                    body.extend_from_slice(b"\r\n");
                } else {
                    let mut text_out: Vec<String> = Vec::new();
                    push_text(
                        &mut text_out,
                        line.x_dots,
                        line.y_dots,
                        line.face.face_str(),
                        line.face.scale(),
                        &line.text,
                        probe,
                    );
                    body.extend_from_slice(text_out.join("\r\n").as_bytes());
                    body.extend_from_slice(b"\r\n");
                }
            }
        }
    }

    let height_mm = (end_y + DOTS_PER_MM - 1) / DOTS_PER_MM;
    // Continuous stock declares zero gap (no-gap form, self-describing);
    // density/speed ride the stored setup like labels.
    let continuous = TsplMediaSetup {
        gap_mm: 0,
        ..ticket.media
    };
    let mut head: Vec<String> = Vec::new();
    push_media_preamble(
        &mut head,
        &format!("SIZE 80 mm, {height_mm} mm"),
        &continuous,
    )?;
    let mut job: Vec<u8> = head.join("\r\n").into_bytes();
    job.extend_from_slice(b"\r\n");
    job.extend_from_slice(&body);
    job.extend_from_slice(b"PRINT 1,1\r\n");
    Ok(job)
}

/// BITMAP raster bounds (BRK-101; architect-verified TSC reference
/// `BITMAP X,Y,width,height,mode,data`: width IN BYTES, height IN DOTS,
/// mode 0 OVERWRITE, raw binary data after the comma).
/// Width ceiling = 80 mm ticket (640 dots); height ceiling guards the
/// spooler transfer; total ceiling = 128 KiB (ticket ~52 KiB typical).
pub const BITMAP_MAX_WIDTH_DOTS: usize = 640;
pub const BITMAP_MAX_HEIGHT_DOTS: usize = 1638;
pub const BITMAP_MAX_BYTES: usize = 131072;

/// Render a silent raster-only job (BRK-101 — TEXT path is DEAD, owner
/// all-red on P1–P3): self-describing preamble
/// SIZE → GAP → DENSITY → SPEED → DIRECTION → CLS (**NO CODEPAGE — zero
/// text bytes anywhere**), then `BITMAP x,y,w,h,0,<raw>`, then `PRINT 1,n`.
/// SIZE is derived from the raster geometry (millimeters, integer math).
/// Pure: parameters in, bytes out. Refuses, never truncates.
#[allow(clippy::too_many_arguments)]
pub fn render_bitmap_job(
    media: &TsplMediaSetup,
    x: i64,
    y: i64,
    width_bytes: usize,
    height_rows: usize,
    data: &[u8],
    copies: u32,
) -> Result<Vec<u8>, DomainError> {
    media.validate()?;
    let code = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    if x < 0 || y < 0 {
        return Err(code("BITMAP_ORIGIN_MUST_BE_NON_NEGATIVE"));
    }
    if width_bytes == 0 || height_rows == 0 {
        return Err(code("BITMAP_DIMENSIONS_MUST_BE_POSITIVE"));
    }
    if width_bytes
        .checked_mul(8)
        .filter(|dots| *dots <= BITMAP_MAX_WIDTH_DOTS)
        .is_none()
    {
        return Err(code("BITMAP_WIDTH_TOO_WIDE"));
    }
    if height_rows > BITMAP_MAX_HEIGHT_DOTS {
        return Err(code("BITMAP_TOO_TALL"));
    }
    let expected = width_bytes
        .checked_mul(height_rows)
        .ok_or_else(|| code("BITMAP_TOO_LARGE"))?;
    if expected > BITMAP_MAX_BYTES {
        return Err(code("BITMAP_TOO_LARGE"));
    }
    if data.len() != expected {
        return Err(code("BITMAP_LENGTH_MISMATCH"));
    }
    // Anti-blank guard: an all-white or all-black frame is a rasterizer bug
    // or a dead transfer — never worth spooling.
    if data.iter().all(|byte| *byte == 0x00) || data.iter().all(|byte| *byte == 0xFF) {
        return Err(code("BITMAP_BLANK_IMAGE"));
    }
    if !(TSPL_MIN_COPIES..=TSPL_MAX_COPIES).contains(&copies) {
        return Err(code("BITMAP_COPIES_OUT_OF_RANGE"));
    }
    // Width is byte-exact so dots (bytes×8) divide evenly into whole mm.
    let width_mm = width_bytes;
    let height_mm = height_rows.div_ceil(DOTS_PER_MM as usize);
    let mut job: Vec<u8> = Vec::new();
    for line in [
        format!("SIZE {width_mm} mm, {height_mm} mm"),
        if media.gap_mm == 0 {
            // Continuous stock: the manual's no-gap form (no units).
            "GAP 0,0".to_string()
        } else {
            // Metric gap REQUIRES the mm suffix (TSC manual: `GAP 3 mm,0 mm`).
            format!("GAP {} mm, 0 mm", media.gap_mm)
        },
        // Vendor-mirrored init (Seagull ground truth): no DENSITY/SPEED/
        // CODEPAGE — the printer runs its calibrated NVRAM values.
        "SET RIBBON OFF".to_string(),
        "DIRECTION 0,0".to_string(),
        "REFERENCE 0,0".to_string(),
        "OFFSET 0 mm".to_string(),
        "SET PEEL OFF".to_string(),
        "SET CUTTER OFF".to_string(),
        "SET TEAR ON".to_string(),
        "CLS".to_string(),
        format!("BITMAP {x},{y},{width_bytes},{height_rows},0,"),
    ] {
        job.extend_from_slice(line.as_bytes());
        if !line.starts_with("BITMAP ") {
            job.extend_from_slice(b"\r\n");
        }
    }
    job.extend_from_slice(data);
    job.extend_from_slice(format!("\r\nPRINT 1,{copies}\r\n").as_bytes());
    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_math_is_pinned() {
        assert_eq!(DOTS_PER_MM, 8);
        assert_eq!(40 * DOTS_PER_MM, LABEL_WIDTH_DOTS);
        assert_eq!(20 * DOTS_PER_MM, LABEL_HEIGHT_DOTS);
        assert_eq!(80 * DOTS_PER_MM, TICKET_WIDTH_DOTS);
        assert_eq!((LABEL_WIDTH_DOTS, LABEL_HEIGHT_DOTS), (320, 160));
        // Quiet zones: 10 modules × 2 dots per side.
        assert_eq!(QUIET_ZONE_DOTS, 20);
        // Module budgets: 11·n + 35 ≤ (media − 2×20)/2.
        assert_eq!(TICKET_BARCODE_MAX_CHARS, 24);
        assert_eq!(LABEL_BARCODE_MAX_CHARS, 9);
        // Narrow bar: 2 dots = 0.25 mm ≈ 9.8 mil.
        assert_eq!(BARCODE_NARROW_DOTS, 2);
    }

    fn owner_label(copies: u32) -> TsplLabel<'static> {
        TsplLabel {
            brand_text: "BARAKA ULTRAPOS",
            name: "Vis 8x40",
            price_text: "10.00",
            currency_text: "DZD",
            barcode: "VIS-001",
            copies,
            media: TsplMediaSetup::default(),
        }
    }

    #[test]
    fn label_bytes_are_exact() {
        let bytes = render_label_40x20(&owner_label(3)).expect("label renders");
        let text = String::from_utf8(bytes).expect("tspl is ascii-clean");
        assert_eq!(
            text,
            "SIZE 40 mm, 20 mm\r\n\
             GAP 3 mm, 0 mm\r\n\
             SET RIBBON OFF\r\n\
             DIRECTION 0,0\r\n\
             REFERENCE 0,0\r\n\
             OFFSET 0 mm\r\n\
             SET PEEL OFF\r\n\
             SET CUTTER OFF\r\n\
             SET TEAR ON\r\n\
             CLS\r\n\
             TEXT 100,2,\"1\",0,1,1,\"BARAKA ULTRAPOS\"\r\n\
             TEXT 8,16,\"2\",0,1,1,\"Vis 8x40\"\r\n\
             TEXT 8,58,\"5\",0,1,1,\"10.00\"\r\n\
             TEXT 272,94,\"1\",0,1,1,\"DZD\"\r\n\
             BARCODE 8,106,\"128\",38,0,0,2,2,\"VIS-001\"\r\n\
             TEXT 132,146,\"1\",0,1,1,\"VIS-001\"\r\n\
             PRINT 1,3\r\n"
        );
    }

    #[test]
    fn label_copies_live_in_print_not_resends() {
        let one =
            String::from_utf8(render_label_40x20(&owner_label(1)).expect("1")).expect("ascii");
        let many =
            String::from_utf8(render_label_40x20(&owner_label(99)).expect("99")).expect("ascii");
        assert!(one.ends_with("PRINT 1,1\r\n"));
        assert!(many.ends_with("PRINT 1,99\r\n"));
        assert_eq!(many.matches("BARCODE").count(), 1);
        assert_eq!(
            render_label_40x20(&owner_label(0)).expect_err("0 rejected"),
            DomainError::TsplCopiesOutOfRange
        );
        assert_eq!(
            render_label_40x20(&owner_label(100)).expect_err("100 rejected"),
            DomainError::TsplCopiesOutOfRange
        );
    }

    #[test]
    fn label_brand_collapses_when_blank() {
        let label = TsplLabel {
            brand_text: "   ",
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&label).expect("ok")).expect("ascii");
        assert!(!text.contains("BARAKA ULTRAPOS"), "blank brand collapses");
        assert!(text.contains("TEXT 8,16,\"2\",0,1,1,\"Vis 8x40\""));
    }

    #[test]
    fn label_barcode_guard_rejects_non_ascii_and_quotes() {
        for bad in ["VIS-é-001", "VIS 001 ✓", "BAD\"SKU", "TAB\there"] {
            let label = TsplLabel {
                barcode: bad,
                ..owner_label(1)
            };
            assert_eq!(
                render_label_40x20(&label).expect_err("guard rejects"),
                DomainError::TsplBarcodeUnencodable,
                "payload: {bad:?}"
            );
        }
    }

    #[test]
    fn label_barcode_width_rejects_instead_of_shrinking() {
        // 9 chars: (9*11+35)*2 = 268 dots ≤ 280 quiet-zone budget — fits.
        let fitting = TsplLabel {
            barcode: "012345678",
            ..owner_label(1)
        };
        assert!(render_label_40x20(&fitting).is_ok());
        // 10 chars: 290 dots > 280 — reject loudly, never shrink.
        let wide = TsplLabel {
            barcode: "0123456789",
            ..fitting
        };
        assert_eq!(
            render_label_40x20(&wide).expect_err("too wide"),
            DomainError::TsplBarcodeTooWide
        );
    }

    #[test]
    fn label_designation_wraps_two_lines_and_truncates() {
        let label = TsplLabel {
            name: "0123456789ABCDEFGHIJ0123456789ABCDEFGHIJ0123456789AB",
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&label).expect("ok")).expect("ascii");
        assert!(text.contains("TEXT 8,16,\"2\",0,1,1,\"0123456789ABCDEFGHIJ01234\""));
        assert!(text.contains("TEXT 8,38,\"2\",0,1,1,\"56789ABCDEFGHIJ0123456...\""));
    }

    #[test]
    fn label_text_truncates_and_sanitizes_without_silent_overflow() {
        let label = TsplLabel {
            name: "Vis autotaraudeuse inoxydable très longue désignation encore",
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&label).expect("renders")).expect("ascii");
        assert!(
            !text.contains("désignation encore"),
            "full name must not leak past the two-line field"
        );
        assert!(text.contains("..."), "truncation is marked");
        let quoted = TsplLabel {
            name: "Vis 8\" acier\nzingué",
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&quoted).expect("renders")).expect("ascii");
        assert!(
            text.contains("Vis 8' acier zingué"),
            "quotes/newlines sanitized"
        );
    }

    static TAX_ROWS: [(&str, &str); 2] = [("NIF", ""), ("RC", "RC-16-12345")];

    fn owner_ticket() -> TsplTicket<'static> {
        TsplTicket {
            store_name: "BARAKA QUINCAILLERIE",
            store_address: "Rue des Frères",
            store_phone: "0550 00 00 00",
            tax_rows: &TAX_ROWS,
            number: "T-2026-000123",
            created_at: "2026-10-27T20:00:00.000Z",
            cashier_text: "00000000-0000-0000-0000-0000000000ab",
            warehouse_text: "11111111-2222-3333-4444-555555555555",
            captions: TsplCaptionLang::French,
            lines: &[TsplTicketLine {
                sku: "VIS-001",
                designation: "Vis 8x40",
                quantity_text: "3",
                unit_text: "Pièce",
                unit_price_text: "9.00",
                line_total_text: "27.00",
                discount_text: Some("1.00"),
            }],
            totals: TsplTicketTotals {
                subtotal_ht: "27.00",
                global_discount: "0.00",
                tva: "0.00",
                total_ttc: "27.00",
                paid: "27.00",
                change: "0.00",
                payment_method: "CASH",
            },
            footer: "Merci",
            media: TsplMediaSetup::default(),
        }
    }

    #[test]
    fn ticket_bytes_pin_size_media_preamble_and_copies() {
        let text = String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("renders"))
            .expect("ascii");
        // Cursor math: 8 + 32 + 20 + 20 + 20 + 10 + 28 + 20×3 + 10 + 20
        //   + 28 + 20 + 20 + 10 + 28×3 + 56 + 20×2 + 8 + 64 + 4 + 20 + 4
        //   + 20 + 20 + 16 = 642 dots (DOC-02: HT row removed) → ceil(642/8) = 81 mm.
        // Continuous stock declares the no-gap form: GAP 0,0.
        assert!(text.starts_with(
            "SIZE 80 mm, 81 mm\r\nGAP 0,0\r\nSET RIBBON OFF\r\nDIRECTION 0,0\r\nREFERENCE 0,0\r\nOFFSET 0 mm\r\nSET PEEL OFF\r\nSET CUTTER OFF\r\nSET TEAR ON\r\nCLS\r\n"
        ));
        assert!(
            !text.lines().any(|l| l == "CUT" || l.starts_with("CUT ")),
            "no cut ACTION emitted (vendor SET CUTTER OFF only disables it)"
        );
        // Barcode: (13*11+35)*2 = 356 dots, centered x = (640-356)/2 = 142.
        // DOC-02: printer-generated human line OFF (single TEXT source below).
        assert!(text.contains("BARCODE 142,494,\"128\",64,0,0,2,2,\"T-2026-000123\""));
        assert!(text.ends_with("PRINT 1,1\r\n"));
        assert_eq!(text.matches("BARCODE").count(), 1);
    }

    #[test]
    fn ticket_anatomy_order_is_pinned_block_by_block() {
        let text = String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("renders"))
            .expect("ascii");
        let rows: Vec<&str> = text.lines().collect();
        let find = |needle: &str| {
            rows.iter()
                .position(|row| row.contains(needle))
                .unwrap_or_else(|| panic!("missing {needle}"))
        };
        let chain = [
            find("TEXT 160,8,\"3\",0,1,1,\"BARAKA QUINCAILLERIE\""),
            find("TEXT 264,40,\"1\",0,1,1,\"Rue des Frères\""),
            find("TEXT 268,60,\"1\",0,1,1,\"0550 00 00 00\""),
            find("TEXT 260,80,\"1\",0,1,1,\"RC: RC-16-12345\""),
            find("BAR 0,100,640,2"),
            find("TEXT 16,110,\"2\",0,1,1,\"T-2026-000123\""),
            find("2026-10-27T20:00:00"),
            find("Caissier: 00000000-0000-0000-0000-0000000000ab"),
            find("Depot: 11111111-2222-3333-4444-555555555555"),
            find("BAR 0,198,640,2"),
            find("Qté x PU = Total"),
            find("TEXT 16,228,\"2\",0,1,1,\"Vis 8x40\""),
            find("TEXT 448,256,\"1\",0,1,1,\"3 Pièce x 9.00 = 27.00\""),
            find("TEXT 536,276,\"1\",0,1,1,\"remise 1.00\""),
            find("BAR 0,296,640,2"),
            find("Sous-total: 27.00"),
            find("Remise totale: 0.00"),
            // DOC-02 (owner 1.b): HT row removed — Sous-total stays.
            find("TVA: 0.00"),
            find("TEXT 112,390,\"5\",0,1,1,\"TOTAL TTC: 27.00\""),
            find("Payé (CASH): 27.00"),
            find("Rendu: 0.00"),
            find("BARCODE 142,494"),
            find("TEXT 268,562,\"1\",0,1,1,\"T-2026-000123\""),
            find("TEXT 300,586,\"1\",0,1,1,\"Merci\""),
            find("Merci de votre visite"),
        ];
        assert!(
            chain.windows(2).all(|pair| pair[0] < pair[1]),
            "anatomy order broken: {chain:?}"
        );
    }

    #[test]
    fn ticket_totals_mirror_stored_values_verbatim() {
        let text = String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("renders"))
            .expect("ascii");
        assert!(
            text.contains("3 Pièce x 9.00 = 27.00"),
            "net PU line transcribed"
        );
        assert!(
            !text.contains("10.00"),
            "raw PU never leaks into a discounted line"
        );
        assert!(text.contains("Sous-total: 27.00"));
        assert!(text.contains("Remise totale: 0.00"));
        // DOC-02 (owner 1.b): the duplicated HT row is gone.
        assert!(!text.contains("HT:"));
        assert!(text.contains("TVA: 0.00"));
        assert!(text.contains("TOTAL TTC: 27.00"));
        assert!(text.contains("Payé (CASH): 27.00"));
        assert!(text.contains("Rendu: 0.00"));
    }

    #[test]
    fn ticket_blank_identity_fields_collapse_without_gaps() {
        let mut ticket = owner_ticket();
        ticket.store_address = "";
        ticket.store_phone = "   ";
        static EMPTY_TAX: [(&str, &str); 1] = [("NIF", "")];
        ticket.tax_rows = &EMPTY_TAX;
        ticket.footer = "";
        let text = String::from_utf8(render_ticket_80mm(&ticket).expect("ok")).expect("ascii");
        assert!(!text.contains("Rue des Frères"));
        assert!(!text.contains("0550"));
        assert!(!text.contains("NIF"));
        assert!(!text.contains("\"Merci\""), "blank footer collapses");
        assert!(text.contains("Merci de votre visite"));
        // 4 collapsed rows × 20 dots: 670 − 80 = 590 → ceil(590/8) = 74 mm.
        assert!(text.starts_with("SIZE 80 mm, 71 mm\r\n"));
    }

    #[test]
    fn ticket_discount_badge_absent_for_legacy_null_rows() {
        let mut ticket = owner_ticket();
        static LEGACY: [TsplTicketLine<'static>; 1] = [TsplTicketLine {
            sku: "—",
            designation: "Clou 5cm",
            quantity_text: "1",
            unit_text: "",
            unit_price_text: "5.00",
            line_total_text: "5.00",
            discount_text: None,
        }];
        ticket.lines = &LEGACY;
        let text = String::from_utf8(render_ticket_80mm(&ticket).expect("ok")).expect("ascii");
        assert!(text.contains("1 x 5.00 = 5.00"));
        assert!(!text.contains("remise"), "no badge on legacy rows");
    }

    #[test]
    fn preamble_order_is_pinned_byte_for_byte() {
        let bytes = render_label_40x20(&owner_label(1)).expect("label renders");
        let text = String::from_utf8(bytes).expect("tspl is ascii-clean");
        let head: Vec<&str> = text.split("\r\n").take(10).collect();
        assert_eq!(
            head,
            [
                "SIZE 40 mm, 20 mm",
                "GAP 3 mm, 0 mm",
                "SET RIBBON OFF",
                "DIRECTION 0,0",
                "REFERENCE 0,0",
                "OFFSET 0 mm",
                "SET PEEL OFF",
                "SET CUTTER OFF",
                "SET TEAR ON",
                "CLS",
            ],
            "preamble order: SIZE → GAP → RIBBON → DIRECTION → REFERENCE → OFFSET → PEEL → CUTTER → TEAR → CLS"
        );
    }

    #[test]
    fn media_defaults_are_owner_verify_starting_values() {
        // OWNER-VERIFY: gap 3 mm is a starting value — the owner measures
        // the 40x20 liner with a ruler and the measured value wins.
        let media = TsplMediaSetup::default();
        assert_eq!(media.gap_mm, TSPL_GAP_MM_DEFAULT);
        assert_eq!(media.gap_mm, 3);
        assert_eq!(media.density, TSPL_DENSITY_DEFAULT);
        assert_eq!(media.speed, TSPL_SPEED_DEFAULT);
        assert_eq!(media.probe_mode, TsplProbeMode::Normal);
        media.validate().expect("defaults validate");
    }

    // TEMPORARY-DIAGNOSTIC pins (BRK-100 — remove with the switch in lock-in).

    #[test]
    fn probe_mode_parses_all_four_spellings_and_rejects_unknown() {
        assert_eq!(
            parse_tspl_probe_mode("normal").expect("normal"),
            TsplProbeMode::Normal
        );
        assert_eq!(
            parse_tspl_probe_mode("no_codepage").expect("p1"),
            TsplProbeMode::NoCodepage
        );
        assert_eq!(
            parse_tspl_probe_mode("ascii_only").expect("p2"),
            TsplProbeMode::AsciiOnly
        );
        assert_eq!(
            parse_tspl_probe_mode("font1_only").expect("p3"),
            TsplProbeMode::Font1Only
        );
        assert_eq!(TsplProbeMode::Normal.as_str(), "normal");
        assert_eq!(
            parse_tspl_probe_mode("bogus").expect_err("unknown rejected"),
            DomainError::InvalidCanonicalString("TSPL_PROBE_MODE_UNKNOWN".to_string())
        );
        assert_eq!(
            parse_tspl_probe_mode("").expect_err("empty rejected"),
            DomainError::InvalidCanonicalString("TSPL_PROBE_MODE_UNKNOWN".to_string())
        );
    }

    #[test]
    fn probe_no_codepage_drops_only_the_codepage_line() {
        let probed = TsplLabel {
            media: TsplMediaSetup {
                probe_mode: TsplProbeMode::NoCodepage,
                ..TsplMediaSetup::default()
            },
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&probed).expect("renders"))
            .expect("tspl is ascii-clean");
        let head: Vec<&str> = text.split("\r\n").take(10).collect();
        assert_eq!(
            head,
            [
                "SIZE 40 mm, 20 mm",
                "GAP 3 mm, 0 mm",
                "SET RIBBON OFF",
                "DIRECTION 0,0",
                "REFERENCE 0,0",
                "OFFSET 0 mm",
                "SET PEEL OFF",
                "SET CUTTER OFF",
                "SET TEAR ON",
                "CLS",
            ],
            "P1: vendor preamble, no CODEPAGE anywhere"
        );
        assert!(!text.contains("CODEPAGE"), "no CODEPAGE anywhere");
    }

    #[test]
    fn probe_ascii_only_folds_lossy_with_question_marks() {
        // LOSSY documented: accents + Arabic collapse to `?`; the probe asks
        // print-vs-ignore, never beauty.
        assert_eq!(ascii_probe_lossy("café Üمر"), "caf? ???");
        let probed = TsplLabel {
            brand_text: "café",
            media: TsplMediaSetup {
                probe_mode: TsplProbeMode::AsciiOnly,
                ..TsplMediaSetup::default()
            },
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&probed).expect("renders"))
            .expect("probe output is ascii-clean");
        assert!(
            text.contains("TEXT") && text.contains("caf?"),
            "P2: payload folded, got: {text}"
        );
        assert!(text.is_ascii(), "P2 output is pure ASCII");
    }

    #[test]
    fn probe_font1_only_unifies_every_text_face() {
        let probed = TsplLabel {
            media: TsplMediaSetup {
                probe_mode: TsplProbeMode::Font1Only,
                ..TsplMediaSetup::default()
            },
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&probed).expect("renders"))
            .expect("tspl is ascii-clean");
        for line in text.lines().filter(|l| l.starts_with("TEXT ")) {
            assert!(
                line.contains(",\"1\",0,"),
                "P3: every TEXT uses font 1, got: {line}"
            );
        }
        assert!(
            text.lines().filter(|l| l.starts_with("TEXT ")).count() > 0,
            "label carries TEXT lines"
        );
    }

    #[test]
    fn probe_normal_is_byte_identical_to_today() {
        // Regression: Normal == pre-probe output — `label_bytes_are_exact`
        // pins the bytes; here we assert the mode itself is a no-op.
        let normal = render_label_40x20(&owner_label(1)).expect("renders");
        let explicit = TsplLabel {
            media: TsplMediaSetup {
                probe_mode: TsplProbeMode::Normal,
                ..TsplMediaSetup::default()
            },
            ..owner_label(1)
        };
        let bytes = render_label_40x20(&explicit).expect("renders");
        assert_eq!(normal, bytes, "explicit Normal == default output");
    }

    #[test]
    fn media_bounds_reject_loudly() {
        assert_eq!(
            validate_tspl_gap_mm(11).expect_err("gap 11 rejected"),
            DomainError::InvalidCanonicalString("TSPL_GAP_MM_OUT_OF_RANGE".to_string())
        );
        assert_eq!(
            validate_tspl_density(16).expect_err("density 16 rejected"),
            DomainError::InvalidCanonicalString("TSPL_DENSITY_OUT_OF_RANGE".to_string())
        );
        for bad_speed in [0u8, 7u8] {
            assert_eq!(
                validate_tspl_speed(bad_speed).expect_err("speed rejected"),
                DomainError::InvalidCanonicalString("TSPL_SPEED_OUT_OF_RANGE".to_string()),
                "speed {bad_speed}"
            );
        }
        // Bounds also reject through the renderer (never silent clamps).
        let bad = TsplLabel {
            media: TsplMediaSetup {
                gap_mm: 11,
                ..TsplMediaSetup::default()
            },
            ..owner_label(1)
        };
        assert!(
            render_label_40x20(&bad).is_err(),
            "gap 11 rejected in bytes"
        );
        // Edges validate: gap 0 (continuous) and maxima.
        TsplMediaSetup {
            gap_mm: 0,
            density: 15,
            speed: 6,
            probe_mode: TsplProbeMode::Normal,
        }
        .validate()
        .expect("edges validate");
        TsplMediaSetup {
            gap_mm: 10,
            density: 0,
            speed: 1,
            probe_mode: TsplProbeMode::Normal,
        }
        .validate()
        .expect("edges validate");
    }

    #[test]
    fn ticket_barcode_rejects_over_wide_numbers() {
        // 24 chars: (24*11+35)*2 = 598 dots ≤ 600 quiet-zone budget — fits.
        let fitting = TsplTicket {
            number: "T-2026-00012345678901234",
            ..owner_ticket()
        };
        assert_eq!(fitting.number.chars().count(), 24);
        assert!(render_ticket_80mm(&fitting).is_ok(), "24 chars fits");
        // 25 chars: 620 dots > 600 — reject loudly, never shrink.
        let wide = TsplTicket {
            number: "T-2026-000123456789012345",
            ..owner_ticket()
        };
        assert_eq!(wide.number.chars().count(), 25);
        assert_eq!(
            render_ticket_80mm(&wide).expect_err("too wide"),
            DomainError::TsplBarcodeTooWide
        );
    }

    #[test]
    fn ticket_arabic_captions_emit_without_destroying_bytes() {
        let mut ticket = owner_ticket();
        ticket.captions = TsplCaptionLang::Arabic;
        let text =
            String::from_utf8(render_ticket_80mm(&ticket).expect("renders")).expect("utf8 kept");
        assert!(text.contains("المجموع الفرعي: 27.00"));
        assert!(text.contains("الخصم الإجمالي: 0.00"));
        // DOC-02 (owner 1.b): the duplicated HT row is gone — Sous-total
        // stays the single HT figure.
        assert!(!text.contains("صافي:"));
        assert!(text.contains("الضريبة: 0.00"));
        assert!(text.contains("الإجمالي: 27.00"));
        assert!(text.contains("المدفوع (CASH): 27.00"));
        assert!(text.contains("الباقي: 0.00"));
        assert!(text.contains("الكمية × سعر = المجموع"));
        assert!(text.contains("خصم 1.00"));
        assert!(text.contains("الكاشير: 00000000-0000-0000-0000-0000000000ab"));
        assert!(text.contains("المخزن: 11111111-2222-3333-4444-555555555555"));
        assert!(text.contains("شكراً لزيارتكم"));
    }

    #[test]
    fn ticket_copy_count_is_always_single_print() {
        let text =
            String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("ok")).expect("ascii");
        assert!(text.ends_with("PRINT 1,1\r\n"));
    }

    // DOC-04: the template version pins geometry — v1 is today's renderer
    // byte-for-byte, anything else rejects LOUDLY (future v2 adds a branch
    // here; v1 stays frozen so old reprints never change).
    #[test]
    fn ticket_template_version_dispatch_pins_v1_and_rejects_unknown() {
        let pinned = render_ticket_80mm_versioned(&owner_ticket(), TSPL_TICKET_TEMPLATE_VERSION)
            .expect("v1 renders");
        let current = render_ticket_80mm(&owner_ticket()).expect("renders");
        assert_eq!(pinned, current, "v1 is the current geometry");
        assert_eq!(
            render_ticket_80mm_versioned(&owner_ticket(), 99).expect_err("unknown rejects"),
            DomainError::InvalidCanonicalString("TSPL_TEMPLATE_VERSION_UNKNOWN".to_string())
        );
    }

    // TASK-PRT-01 (LIVE-2026-11-06): the XP-410B silently discards any job
    // whose final line is not CRLF-terminated — every renderer must emit
    // the trailing newline, never a bare final command.
    #[test]
    fn all_tspl_jobs_end_with_crlf() {
        let label =
            String::from_utf8(render_label_40x20(&owner_label(1)).expect("label")).expect("ascii");
        assert!(label.ends_with("\r\n"), "label job must end with CRLF");
        let ticket =
            String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("ticket")).expect("ascii");
        assert!(ticket.ends_with("\r\n"), "ticket job must end with CRLF");
        let data = [0xAA_u8, 0x55_u8];
        let job = render_bitmap_job(&TsplMediaSetup::default(), 0, 0, 2, 1, &data, 1)
            .expect("bitmap renders");
        assert!(job.ends_with(b"\r\n"), "bitmap job must end with CRLF");
    }

    // BRK-101 BITMAP pins (TEXT path is DEAD — raster only).

    #[test]
    fn bitmap_job_is_byte_exact_on_a_two_byte_fixture() {
        let data = [0xAA_u8, 0x55_u8];
        let job = render_bitmap_job(&TsplMediaSetup::default(), 0, 0, 2, 1, &data, 1)
            .expect("fixture renders");
        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(
            b"SIZE 2 mm, 1 mm\r\nGAP 3 mm, 0 mm\r\nSET RIBBON OFF\r\nDIRECTION 0,0\r\nREFERENCE 0,0\r\nOFFSET 0 mm\r\nSET PEEL OFF\r\nSET CUTTER OFF\r\nSET TEAR ON\r\nCLS\r\nBITMAP 0,0,2,1,0,",
        );
        expected.extend_from_slice(&data);
        expected.extend_from_slice(b"\r\nPRINT 1,1\r\n");
        assert_eq!(job, expected, "raster job byte-exact");
        assert!(
            !job.windows(8).any(|w| w == b"CODEPAGE"),
            "zero text bytes: no CODEPAGE anywhere"
        );
    }

    #[test]
    fn bitmap_job_ignores_the_temporary_probe_switch() {
        let data = [0xAA_u8, 0x55_u8];
        let plain =
            render_bitmap_job(&TsplMediaSetup::default(), 0, 0, 2, 1, &data, 1).expect("renders");
        for mode in [
            TsplProbeMode::NoCodepage,
            TsplProbeMode::AsciiOnly,
            TsplProbeMode::Font1Only,
        ] {
            let media = TsplMediaSetup {
                probe_mode: mode,
                ..TsplMediaSetup::default()
            };
            let probed = render_bitmap_job(&media, 0, 0, 2, 1, &data, 1).expect("renders");
            assert_eq!(probed, plain, "probe mode has no effect on raster jobs");
        }
    }

    #[test]
    fn bitmap_job_refuses_bad_geometry_copies_and_blank_frames() {
        let media = TsplMediaSetup::default();
        let bad = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
        // Length mismatch.
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 2, 1, &[0xAA], 1).expect_err("short"),
            bad("BITMAP_LENGTH_MISMATCH")
        );
        // Zero dimension.
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 0, 1, &[], 1).expect_err("zero width"),
            bad("BITMAP_DIMENSIONS_MUST_BE_POSITIVE")
        );
        // Too wide: 81 bytes = 648 dots > 640.
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 81, 1, &[0xAA; 81], 1).expect_err("wide"),
            bad("BITMAP_WIDTH_TOO_WIDE")
        );
        // Too tall.
        assert_eq!(
            render_bitmap_job(
                &media,
                0,
                0,
                1,
                BITMAP_MAX_HEIGHT_DOTS + 1,
                &[0xAA; BITMAP_MAX_HEIGHT_DOTS + 1],
                1
            )
            .expect_err("tall"),
            bad("BITMAP_TOO_TALL")
        );
        // Too large: 81 x 1638 would be 132678 > 131072 — but the width cap
        // fires first (648 dots > 640); the total cap is defense-in-depth
        // for future cap changes. Pin the implied ceiling instead: the
        // largest legal frame (80 x 1638 = 131040 B) renders.
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 81, 1638, &[0xAA; 81 * 1638], 1).expect_err("huge"),
            bad("BITMAP_WIDTH_TOO_WIDE")
        );
        let mut max = vec![0xAA_u8; 80 * BITMAP_MAX_HEIGHT_DOTS];
        max[0] = 0x55;
        // Implied ceiling pinned by the render below: the largest legal
        // frame (80 x 1638 = 131040 B) stays under the 128 KiB total cap.
        render_bitmap_job(&media, 0, 0, 80, BITMAP_MAX_HEIGHT_DOTS, &max, 1)
            .expect("max geometry renders");
        // All-white and all-black frames rejected (anti-blank guard).
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 2, 1, &[0x00, 0x00], 1).expect_err("white"),
            bad("BITMAP_BLANK_IMAGE")
        );
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 2, 1, &[0xFF, 0xFF], 1).expect_err("black"),
            bad("BITMAP_BLANK_IMAGE")
        );
        // Copies out of range.
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 2, 1, &[0xAA, 0x55], 0).expect_err("zero copies"),
            bad("BITMAP_COPIES_OUT_OF_RANGE")
        );
        assert_eq!(
            render_bitmap_job(&media, 0, 0, 2, 1, &[0xAA, 0x55], 100).expect_err("100 copies"),
            bad("BITMAP_COPIES_OUT_OF_RANGE")
        );
        // Negative origin.
        assert_eq!(
            render_bitmap_job(&media, -1, 0, 2, 1, &[0xAA, 0x55], 1).expect_err("neg x"),
            bad("BITMAP_ORIGIN_MUST_BE_NON_NEGATIVE")
        );
    }

    #[test]
    fn bitmap_job_carries_label_and_ticket_geometry() {
        // 40x20 mm label: 40 bytes x 160 rows = 6400 B.
        let mut label = vec![0xAA_u8; 40 * 160];
        label[0] = 0x55;
        let job = render_bitmap_job(&TsplMediaSetup::default(), 0, 0, 40, 160, &label, 2)
            .expect("label geometry");
        let head = String::from_utf8(job[..40].to_vec()).expect("header ascii");
        assert!(
            head.starts_with("SIZE 40 mm, 20 mm\r\n"),
            "label size, got: {head}"
        );
        assert!(job.ends_with(b"\r\nPRINT 1,2\r\n"), "copies ride PRINT");
        // 80 mm ticket, 84 mm tall: 80 bytes x 672 rows = 53760 B.
        let mut ticket = vec![0xAA_u8; 80 * 672];
        ticket[0] = 0x55;
        let job = render_bitmap_job(&TsplMediaSetup::default(), 0, 0, 80, 672, &ticket, 1)
            .expect("ticket geometry");
        let head = String::from_utf8(job[..40].to_vec()).expect("header ascii");
        assert!(
            head.starts_with("SIZE 80 mm, 84 mm\r\n"),
            "ticket size, got: {head}"
        );
    }

    // TASK-PRT-02 RED: Arabic in a TEXT line must fail LOUDLY (the printer
    // silently eats the whole job — never ship that silence upstream).
    #[test]
    fn label_arabic_text_rejects_loudly() {
        let bad = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
        let label = TsplLabel {
            name: "JILI ISPANA ابيض",
            ..owner_label(1)
        };
        assert_eq!(
            render_label_40x20(&label).expect_err("arabic name rejects"),
            bad("LABEL_TEXT_UNPRINTABLE")
        );
        let brand = TsplLabel {
            brand_text: "بركة",
            ..owner_label(1)
        };
        assert_eq!(
            render_label_40x20(&brand).expect_err("arabic brand rejects"),
            bad("LABEL_TEXT_UNPRINTABLE")
        );
        // French accents ride CODEPAGE UTF-8 — still printable TEXT.
        let french = TsplLabel {
            name: "Clé à molette",
            ..owner_label(1)
        };
        render_label_40x20(&french).expect("french accents print");
    }

    // TASK-PRT-02 RED: a bitmap slot replaces exactly one TEXT line; the
    // job stays fully CRLF-terminated and byte-pinned elsewhere.
    #[test]
    fn label_mixed_bitmap_replaces_name_line() {
        use super::{label_text_plan, render_label_40x20_mixed, LabelBitmap, LabelTextSlot};
        let label = TsplLabel {
            name: "JILI ISPANA ابيض",
            ..owner_label(1)
        };
        let plan = label_text_plan(&label);
        let l1 = plan
            .iter()
            .find(|line| line.slot == LabelTextSlot::NameLine1)
            .expect("name line 1 planned");
        assert_eq!(l1.x_dots, 8);
        assert_eq!(l1.y_dots, 16);
        let bytes = render_label_40x20_mixed(
            &label,
            &[(
                LabelTextSlot::NameLine1,
                LabelBitmap {
                    width_bytes: 2,
                    height_rows: 1,
                    data: vec![0xAA, 0x55],
                },
            )],
        )
        .expect("mixed renders");
        // BITMAP line present at the text origin (byte search — the chunk
        // carries raw pixels, never UTF-8); the replaced TEXT line gone.
        let bitmap_head = b"BITMAP 8,35,2,1,0,";
        assert!(
            bytes.windows(bitmap_head.len()).any(|w| w == bitmap_head),
            "bitmap at bottom-aligned origin"
        );
        assert!(
            !bytes
                .windows(b"TEXT 8,16,".len())
                .any(|w| w == b"TEXT 8,16,"),
            "replaced TEXT line gone"
        );
        assert!(bytes.ends_with(b"\r\nPRINT 1,1\r\n"), "tail intact");
    }
}
