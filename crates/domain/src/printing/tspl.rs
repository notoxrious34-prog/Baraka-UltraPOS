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
//! - `DIRECTION 1` (forward feed). No `GAP` emitted: the printer's
//!   calibrated media gap applies (DSN-098 owns media setup).
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
/// Label barcode height in dots (5.0 mm — see module math).
const LABEL_BARCODE_HEIGHT: i64 = 40;
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
}

/// Static captions per language (Arabic shaping caveat: see module docs).
struct Captions {
    subtotal: &'static str,
    discount_total: &'static str,
    ht: &'static str,
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
            ht: "HT",
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
            ht: "صافي",
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

/// Emit one TEXT command.
fn push_text(out: &mut Vec<String>, x: i64, y: i64, font: &str, scale: i64, content: &str) {
    out.push(format!(
        "TEXT {x},{y},\"{font}\",0,{scale},{scale},\"{content}\""
    ));
}

/// Emit one solid rule across the full width (BAR, 2 dots high).
fn push_rule(out: &mut Vec<String>, y: i64, media: i64) {
    out.push(format!("BAR 0,{y},{media},2"));
}

/// Ticket line pitches in dots (font height + leading).
const PITCH_F1: i64 = 20;
const PITCH_F2: i64 = 28;
const PITCH_F3: i64 = 32;
/// Advance after a double-height "3" row (2 × 24 + 8).
const PITCH_F3_X2: i64 = 56;
/// Ticket side margin.
const TICKET_MARGIN_X: i64 = 16;

/// Render the 40x20 mm label. Exact bytes are pinned in module tests.
pub fn render_label_40x20(label: &TsplLabel<'_>) -> Result<Vec<u8>, DomainError> {
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
    let barcode = sanitize_tspl_text(label.barcode);
    let right_edge = LABEL_WIDTH_DOTS - 8;

    let brand_chars = ((LABEL_WIDTH_DOTS - 16) / FONT_1_W) as usize;
    let name_chars = ((LABEL_WIDTH_DOTS - 16) / FONT_2_W) as usize;
    let price_chars = ((LABEL_WIDTH_DOTS - 16) / (FONT_3_W * 2)) as usize;

    let mut out = Vec::new();
    out.push("SIZE 40 mm,20 mm".to_string());
    out.push("DIRECTION 1".to_string());
    out.push("CLS".to_string());
    // Brand micro-line (collapses when blank).
    let brand = truncate_to_width(label.brand_text.trim(), brand_chars);
    if !brand.is_empty() {
        let x = centered_x(
            LABEL_WIDTH_DOTS,
            text_dots(brand.chars().count(), FONT_1_W, 1),
        );
        push_text(&mut out, x, 2, "1", 1, &brand);
    }
    // Designation: two lines max, second line marked "..." when cut.
    let clean_name = sanitize_tspl_text(label.name.trim());
    let first: String = clean_name.chars().take(name_chars).collect();
    let rest: String = clean_name.chars().skip(name_chars).collect();
    push_text(&mut out, 8, 16, "2", 1, &first);
    if !rest.trim().is_empty() {
        let second = truncate_to_width(&rest, name_chars);
        push_text(&mut out, 8, 38, "2", 1, &second);
    }
    // Price double-height, currency right-aligned inside the price band.
    let price = truncate_to_width(label.price_text.trim(), price_chars);
    push_text(&mut out, 8, 58, "3", 2, &price);
    let currency = truncate_to_width(label.currency_text.trim(), brand_chars);
    if !currency.is_empty() {
        let x = right_x(right_edge, text_dots(currency.chars().count(), FONT_1_W, 1));
        push_text(&mut out, x, 94, "1", 1, &currency);
    }
    out.push(format!(
        "BARCODE 8,106,\"128\",{LABEL_BARCODE_HEIGHT},1,0,2,2,\"{barcode}\""
    ));
    let human = truncate_to_width(&barcode, brand_chars);
    let human_x = centered_x(
        LABEL_WIDTH_DOTS,
        text_dots(human.chars().count(), FONT_1_W, 1),
    );
    push_text(&mut out, human_x, 148, "1", 1, &human);
    out.push(format!("PRINT 1,{}", label.copies));
    Ok(out.join("\r\n").into_bytes())
}

/// Render the 80 mm gapless ticket. Height is computed from content:
/// `SIZE 80 mm,H mm` with H = ceil(total_dots / 8).
pub fn render_ticket_80mm(ticket: &TsplTicket<'_>) -> Result<Vec<u8>, DomainError> {
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
    let number = sanitize_tspl_text(ticket.number);
    let barcode_dots = barcode_width_dots(number.chars().count()) as i64;
    let barcode_x = centered_x(TICKET_WIDTH_DOTS, barcode_dots);
    let right_edge = TICKET_WIDTH_DOTS - TICKET_MARGIN_X;
    let caps = captions(ticket.captions);

    let narrow_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / FONT_1_W) as usize;
    let wide_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / FONT_2_W) as usize;
    let name_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / FONT_3_W) as usize;
    let total_chars = ((TICKET_WIDTH_DOTS - 2 * TICKET_MARGIN_X) / (FONT_3_W * 2)) as usize;

    let mut body: Vec<String> = Vec::new();
    let mut y: i64 = 8;

    // Store block (blank fields collapse — never empty lines).
    let store = truncate_to_width(ticket.store_name.trim(), name_chars);
    if !store.is_empty() {
        let dots = text_dots(store.chars().count(), FONT_3_W, 1);
        push_text(
            &mut body,
            centered_x(TICKET_WIDTH_DOTS, dots),
            y,
            "3",
            1,
            &store,
        );
        y += PITCH_F3;
    }
    for line in [ticket.store_address, ticket.store_phone] {
        let shown = truncate_to_width(line.trim(), narrow_chars);
        if !shown.is_empty() {
            let dots = text_dots(shown.chars().count(), FONT_1_W, 1);
            push_text(
                &mut body,
                centered_x(TICKET_WIDTH_DOTS, dots),
                y,
                "1",
                1,
                &shown,
            );
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
        push_text(
            &mut body,
            centered_x(TICKET_WIDTH_DOTS, dots),
            y,
            "1",
            1,
            &shown,
        );
        y += PITCH_F1;
    }
    push_rule(&mut body, y, TICKET_WIDTH_DOTS);
    y += 10;
    // Doc meta: number, capture instant, cashier, depot.
    let number_shown = truncate_to_width(&number, wide_chars);
    push_text(&mut body, TICKET_MARGIN_X, y, "2", 1, &number_shown);
    y += PITCH_F2;
    let instant = sanitize_tspl_text(ticket.created_at);
    let instant_shown = truncate_to_width(&instant[..instant.len().min(19)], narrow_chars);
    push_text(&mut body, TICKET_MARGIN_X, y, "1", 1, &instant_shown);
    y += PITCH_F1;
    let cashier = format!(
        "{}: {}",
        caps.cashier_word,
        sanitize_tspl_text(ticket.cashier_text)
    );
    let cashier_shown = truncate_to_width(&cashier, narrow_chars);
    push_text(&mut body, TICKET_MARGIN_X, y, "1", 1, &cashier_shown);
    y += PITCH_F1;
    let depot = format!(
        "{}: {}",
        caps.depot_word,
        sanitize_tspl_text(ticket.warehouse_text)
    );
    let depot_shown = truncate_to_width(&depot, narrow_chars);
    push_text(&mut body, TICKET_MARGIN_X, y, "1", 1, &depot_shown);
    y += PITCH_F1;
    push_rule(&mut body, y, TICKET_WIDTH_DOTS);
    y += 10;
    // Column header.
    let header = truncate_to_width(caps.column_header, narrow_chars);
    push_text(&mut body, TICKET_MARGIN_X, y, "1", 1, &header);
    y += PITCH_F1;
    // Item rows.
    for line in ticket.lines {
        let designation = truncate_to_width(line.designation.trim(), wide_chars);
        push_text(&mut body, TICKET_MARGIN_X, y, "2", 1, &designation);
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
        push_text(
            &mut body,
            right_x(right_edge, detail_dots),
            y,
            "1",
            1,
            &detail_shown,
        );
        y += PITCH_F1;
        if let Some(discount) = line.discount_text {
            let badge = format!("{} {}", caps.discount_word, sanitize_tspl_text(discount));
            let badge_shown = truncate_to_width(&badge, narrow_chars / 2);
            let badge_dots = text_dots(badge_shown.chars().count(), FONT_1_W, 1);
            push_text(
                &mut body,
                right_x(right_edge, badge_dots),
                y,
                "1",
                1,
                &badge_shown,
            );
            y += PITCH_F1;
        }
    }
    push_rule(&mut body, y, TICKET_WIDTH_DOTS);
    y += 10;
    // Totals hierarchy (right-aligned), TOTAL TTC emphasized double-height.
    let totals_rows = [
        (caps.subtotal, ticket.totals.subtotal_ht),
        (caps.discount_total, ticket.totals.global_discount),
        (caps.ht, ticket.totals.subtotal_ht),
        (caps.tva, ticket.totals.tva),
    ];
    for (caption, value) in totals_rows {
        let row = format!("{caption}: {}", sanitize_tspl_text(value));
        let shown = truncate_to_width(&row, wide_chars);
        let dots = text_dots(shown.chars().count(), FONT_2_W, 1);
        push_text(&mut body, right_x(right_edge, dots), y, "2", 1, &shown);
        y += PITCH_F2;
    }
    let total_row = format!(
        "{}: {}",
        caps.total,
        sanitize_tspl_text(ticket.totals.total_ttc)
    );
    let total_shown = truncate_to_width(&total_row, total_chars);
    let total_dots = text_dots(total_shown.chars().count(), FONT_3_W, 2);
    push_text(
        &mut body,
        right_x(right_edge, total_dots),
        y,
        "3",
        2,
        &total_shown,
    );
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
    push_text(
        &mut body,
        right_x(right_edge, payment_dots),
        y,
        "1",
        1,
        &payment_shown,
    );
    y += PITCH_F1;
    let change_row = format!(
        "{}: {}",
        caps.change,
        sanitize_tspl_text(ticket.totals.change)
    );
    let change_shown = truncate_to_width(&change_row, narrow_chars);
    let change_dots = text_dots(change_shown.chars().count(), FONT_1_W, 1);
    push_text(
        &mut body,
        right_x(right_edge, change_dots),
        y,
        "1",
        1,
        &change_shown,
    );
    y += PITCH_F1;
    // Doc-number barcode (centered, quiet zones >= 20 dots per side) with
    // the human-readable number beneath.
    y += 8;
    body.push(format!(
        "BARCODE {barcode_x},{y},\"128\",{TICKET_BARCODE_HEIGHT},1,0,2,2,\"{number}\""
    ));
    y += TICKET_BARCODE_HEIGHT + 4;
    let human = truncate_to_width(&number, narrow_chars);
    let human_dots = text_dots(human.chars().count(), FONT_1_W, 1);
    push_text(
        &mut body,
        centered_x(TICKET_WIDTH_DOTS, human_dots),
        y,
        "1",
        1,
        &human,
    );
    y += PITCH_F1 + 4;
    // Footer: owner message (collapses when blank) + constant thanks.
    let footer = truncate_to_width(ticket.footer.trim(), narrow_chars);
    if !footer.is_empty() {
        let dots = text_dots(footer.chars().count(), FONT_1_W, 1);
        push_text(
            &mut body,
            centered_x(TICKET_WIDTH_DOTS, dots),
            y,
            "1",
            1,
            &footer,
        );
        y += PITCH_F1;
    }
    let thanks = truncate_to_width(caps.thanks, narrow_chars);
    let thanks_dots = text_dots(thanks.chars().count(), FONT_1_W, 1);
    push_text(
        &mut body,
        centered_x(TICKET_WIDTH_DOTS, thanks_dots),
        y,
        "1",
        1,
        &thanks,
    );
    y += PITCH_F1 + 16;

    let height_mm = (y + DOTS_PER_MM - 1) / DOTS_PER_MM;
    let mut out = Vec::new();
    out.push(format!("SIZE 80 mm,{height_mm} mm"));
    out.push("DIRECTION 1".to_string());
    out.push("CLS".to_string());
    out.extend(body);
    out.push("PRINT 1,1".to_string());
    Ok(out.join("\r\n").into_bytes())
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
        }
    }

    #[test]
    fn label_bytes_are_exact() {
        let bytes = render_label_40x20(&owner_label(3)).expect("label renders");
        let text = String::from_utf8(bytes).expect("tspl is ascii-clean");
        assert_eq!(
            text,
            "SIZE 40 mm,20 mm\r\n\
             DIRECTION 1\r\n\
             CLS\r\n\
             TEXT 100,2,\"1\",0,1,1,\"BARAKA ULTRAPOS\"\r\n\
             TEXT 8,16,\"2\",0,1,1,\"Vis 8x40\"\r\n\
             TEXT 8,58,\"3\",0,2,2,\"10.00\"\r\n\
             TEXT 288,94,\"1\",0,1,1,\"DZD\"\r\n\
             BARCODE 8,106,\"128\",40,1,0,2,2,\"VIS-001\"\r\n\
             TEXT 132,148,\"1\",0,1,1,\"VIS-001\"\r\n\
             PRINT 1,3"
        );
    }

    #[test]
    fn label_copies_live_in_print_not_resends() {
        let one =
            String::from_utf8(render_label_40x20(&owner_label(1)).expect("1")).expect("ascii");
        let many =
            String::from_utf8(render_label_40x20(&owner_label(99)).expect("99")).expect("ascii");
        assert!(one.ends_with("PRINT 1,1"));
        assert!(many.ends_with("PRINT 1,99"));
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
        }
    }

    #[test]
    fn ticket_bytes_pin_size_gapless_barcode_and_copies() {
        let text = String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("renders"))
            .expect("ascii");
        // Cursor math: 8 + 32 + 20 + 20 + 20 + 10 + 28 + 20×3 + 10 + 20
        //   + 28 + 20 + 20 + 10 + 28×4 + 56 + 20×2 + 8 + 64 + 4 + 20 + 4
        //   + 20 + 20 + 16 = 670 dots → ceil(670/8) = 84 mm.
        assert!(text.starts_with("SIZE 80 mm,84 mm\r\nDIRECTION 1\r\nCLS\r\n"));
        assert!(!text.contains("GAP"), "gapless: no GAP command emitted");
        assert!(
            !text.contains("CUT"),
            "no printer-specific cut command emitted"
        );
        // Barcode: (13*11+35)*2 = 356 dots, centered x = (640-356)/2 = 142.
        assert!(text.contains("BARCODE 142,522,\"128\",64,1,0,2,2,\"T-2026-000123\""));
        assert!(text.ends_with("PRINT 1,1"));
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
            find("HT: 27.00"),
            find("TVA: 0.00"),
            find("TEXT 112,418,\"3\",0,2,2,\"TOTAL TTC: 27.00\""),
            find("Payé (CASH): 27.00"),
            find("Rendu: 0.00"),
            find("BARCODE 142,522"),
            find("TEXT 268,590,\"1\",0,1,1,\"T-2026-000123\""),
            find("TEXT 300,614,\"1\",0,1,1,\"Merci\""),
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
        assert!(text.contains("HT: 27.00"));
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
        assert!(text.starts_with("SIZE 80 mm,74 mm\r\n"));
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
        assert!(text.contains("صافي: 27.00"));
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
        assert!(text.ends_with("PRINT 1,1"));
    }
}
