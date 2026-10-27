//! Silent TSPL renderer (TASK-BRK-097, ADR-105).
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
//! TSPL dialect assumptions (XPrinter XP-410B emulation, owner to verify
//! on hardware; STATED, not probed):
//! - Built-in bitmap fonts: "1" = 8x12, "2" = 12x20, "3" = 16x24 dots
//!   (TSC TSPL reference metrics).
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
/// Module width in dots (narrow = wide = 2; stated, scan-safe at 203 dpi).
const BARCODE_NARROW_DOTS: usize = 2;

/// A silent 40x20 mm shelf label (name + price + Code128), pre-formatted.
pub struct TsplLabel<'a> {
    /// Display name (single locale, chosen upstream).
    pub name: &'a str,
    /// Plain price text, e.g. `"1234.50"` (renderer appends ` DZD`).
    pub price_text: &'a str,
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
}

/// Stored ticket totals, pre-formatted (graphique-wire parity: the five
/// wire fields, no recompute).
pub struct TsplTicketTotals<'a> {
    pub subtotal_ht: &'a str,
    pub tva: &'a str,
    pub total_ttc: &'a str,
    pub paid: &'a str,
    pub change: &'a str,
    /// Payment method code as stored (e.g. `CASH`, `MIXED`).
    pub payment_method: &'a str,
}

/// A silent 80 mm ticket mirroring the graphique content model.
pub struct TsplTicket<'a> {
    pub store_name: &'a str,
    pub store_address: &'a str,
    pub store_phone: &'a str,
    /// Ticket number (also the barcode payload, charset-guarded).
    pub number: &'a str,
    /// Creation instant, pre-formatted (RFC3339 millis, worker parity).
    pub created_at: &'a str,
    pub captions: TsplCaptionLang,
    pub lines: &'a [TsplTicketLine<'a>],
    pub totals: TsplTicketTotals<'a>,
    /// Footer message (empty lines skipped).
    pub footer: &'a str,
}

/// Static captions per language (Arabic shaping caveat: see module docs).
fn captions(
    lang: TsplCaptionLang,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match lang {
        TsplCaptionLang::French => ("Sous-total", "TVA", "TOTAL", "Payé", "Rendu"),
        TsplCaptionLang::Arabic => ("المجموع الفرعي", "الضريبة", "المجموع", "المدفوع", "الباقي"),
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

/// Emit one TEXT command, returning the new cursor line (y + pitch).
fn push_text(out: &mut Vec<String>, x: i64, y: i64, font: &str, content: &str) {
    out.push(format!("TEXT {x},{y},\"{font}\",0,1,1,\"{content}\""));
}

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
    let barcode_dots = barcode_width_dots(label.barcode.chars().count());
    if barcode_dots > (LABEL_WIDTH_DOTS - 16) as usize {
        return Err(DomainError::TsplBarcodeTooWide);
    }

    let name_chars = ((LABEL_WIDTH_DOTS - 16) / FONT_2_W) as usize;
    let price_chars = ((LABEL_WIDTH_DOTS - 16) / FONT_3_W) as usize;
    let name = truncate_to_width(label.name, name_chars);
    let price = truncate_to_width(
        &format!("{} DZD", sanitize_tspl_text(label.price_text)),
        price_chars,
    );
    let barcode = sanitize_tspl_text(label.barcode);

    let mut out = Vec::new();
    out.push("SIZE 40 mm,20 mm".to_string());
    out.push("DIRECTION 1".to_string());
    out.push("CLS".to_string());
    push_text(&mut out, 8, 8, "2", &name);
    push_text(&mut out, 8, 36, "3", &price);
    out.push(format!("BARCODE 8,72,\"128\",56,1,0,2,2,\"{barcode}\""));
    out.push(format!("PRINT 1,{}", label.copies));
    Ok(out.join("\r\n").into_bytes())
}

/// Ticket line pitches in dots (font height + leading).
const PITCH_F1: i64 = 20;
const PITCH_F2: i64 = 28;
const PITCH_F3: i64 = 32;
/// Separator rule width in characters.
const SEPARATOR_WIDTH: usize = 48;

/// Render the 80 mm gapless ticket. Height is computed from content:
/// `SIZE 80 mm,H mm` with H = ceil(total_dots / 8).
pub fn render_ticket_80mm(ticket: &TsplTicket<'_>) -> Result<Vec<u8>, DomainError> {
    if ticket.number.trim().is_empty() {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_NUMBER_MUST_NOT_BE_EMPTY".to_string(),
        ));
    }
    validate_code128_payload(ticket.number)?;
    let number = sanitize_tspl_text(ticket.number);
    let barcode_dots = barcode_width_dots(number.chars().count());
    if barcode_dots > (TICKET_WIDTH_DOTS - 32) as usize {
        return Err(DomainError::TsplBarcodeTooWide);
    }
    let barcode_x = (TICKET_WIDTH_DOTS - barcode_dots as i64) / 2;

    let (cap_subtotal, cap_tva, cap_total, cap_paid, cap_change) = captions(ticket.captions);
    let store_chars = ((TICKET_WIDTH_DOTS - 32) / FONT_3_W) as usize;
    let wide_chars = ((TICKET_WIDTH_DOTS - 32) / FONT_2_W) as usize;
    let narrow_chars = ((TICKET_WIDTH_DOTS - 32) / FONT_1_W) as usize;

    // First pass: collect body commands with a cursor so the total height
    // is exact before SIZE is emitted.
    let mut body: Vec<String> = Vec::new();
    let mut y: i64 = 8;
    let text = |body: &mut Vec<String>, y: &mut i64, font: &str, pitch: i64, content: String| {
        push_text(body, 16, *y, font, &content);
        *y += pitch;
    };

    text(
        &mut body,
        &mut y,
        "3",
        PITCH_F3,
        truncate_to_width(ticket.store_name, store_chars),
    );
    if !ticket.store_address.trim().is_empty() {
        text(
            &mut body,
            &mut y,
            "1",
            PITCH_F1,
            truncate_to_width(ticket.store_address, narrow_chars),
        );
    }
    if !ticket.store_phone.trim().is_empty() {
        text(
            &mut body,
            &mut y,
            "1",
            PITCH_F1,
            truncate_to_width(ticket.store_phone, narrow_chars),
        );
    }
    text(
        &mut body,
        &mut y,
        "1",
        PITCH_F1,
        "-".repeat(SEPARATOR_WIDTH),
    );
    text(
        &mut body,
        &mut y,
        "2",
        PITCH_F2,
        truncate_to_width(
            &format!("{}  {}", number, sanitize_tspl_text(ticket.created_at)),
            wide_chars,
        ),
    );
    for line in ticket.lines {
        text(
            &mut body,
            &mut y,
            "1",
            PITCH_F1,
            truncate_to_width(line.sku, narrow_chars),
        );
        text(
            &mut body,
            &mut y,
            "2",
            PITCH_F2,
            truncate_to_width(line.designation, wide_chars),
        );
        // Qty/unit share one line (graphique cells collapse whitespace, so
        // an empty unit renders without a double space — stated).
        let detail = if line.unit_text.trim().is_empty() {
            format!(
                "  {} x {} = {}",
                sanitize_tspl_text(line.quantity_text),
                sanitize_tspl_text(line.unit_price_text),
                sanitize_tspl_text(line.line_total_text),
            )
        } else {
            format!(
                "  {} {} x {} = {}",
                sanitize_tspl_text(line.quantity_text),
                sanitize_tspl_text(line.unit_text),
                sanitize_tspl_text(line.unit_price_text),
                sanitize_tspl_text(line.line_total_text),
            )
        };
        text(
            &mut body,
            &mut y,
            "1",
            PITCH_F1,
            truncate_to_width(&detail, narrow_chars),
        );
    }
    text(
        &mut body,
        &mut y,
        "1",
        PITCH_F1,
        "-".repeat(SEPARATOR_WIDTH),
    );
    let t = &ticket.totals;
    for (caption, value) in [
        (cap_subtotal, t.subtotal_ht),
        (cap_tva, t.tva),
        (cap_total, t.total_ttc),
    ] {
        text(
            &mut body,
            &mut y,
            "2",
            PITCH_F2,
            truncate_to_width(
                &format!("{caption}: {}", sanitize_tspl_text(value)),
                wide_chars,
            ),
        );
    }
    text(
        &mut body,
        &mut y,
        "1",
        PITCH_F1,
        truncate_to_width(
            &format!(
                "{} ({}): {}",
                cap_paid,
                sanitize_tspl_text(t.payment_method),
                sanitize_tspl_text(t.paid),
            ),
            narrow_chars,
        ),
    );
    text(
        &mut body,
        &mut y,
        "1",
        PITCH_F1,
        truncate_to_width(
            &format!("{}: {}", cap_change, sanitize_tspl_text(t.change)),
            narrow_chars,
        ),
    );
    y += 8;
    body.push(format!(
        "BARCODE {barcode_x},{y},\"128\",64,1,0,2,2,\"{number}\""
    ));
    y += 64 + 16 + 8;
    for footer_line in ticket.footer.lines() {
        if footer_line.trim().is_empty() {
            continue;
        }
        text(
            &mut body,
            &mut y,
            "1",
            PITCH_F1,
            truncate_to_width(footer_line, narrow_chars),
        );
    }
    y += 16;

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
    }

    fn owner_label(copies: u32) -> TsplLabel<'static> {
        TsplLabel {
            name: "Vis 8x40",
            price_text: "10.00",
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
             TEXT 8,8,\"2\",0,1,1,\"Vis 8x40\"\r\n\
             TEXT 8,36,\"3\",0,1,1,\"10.00 DZD\"\r\n\
             BARCODE 8,72,\"128\",56,1,0,2,2,\"VIS-001\"\r\n\
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
    fn label_barcode_guard_rejects_non_ascii_and_quotes() {
        for bad in ["VIS-é-001", "VIS 001 ✓", "BAD\"SKU", "TAB\there"] {
            let label = TsplLabel {
                name: "Vis",
                price_text: "1.00",
                barcode: bad,
                copies: 1,
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
        // 10 chars: (10*11+35)*2 = 290 dots, fits the 304-dot budget.
        let fitting = TsplLabel {
            name: "Vis",
            price_text: "1.00",
            barcode: "0123456789",
            copies: 1,
        };
        assert!(render_label_40x20(&fitting).is_ok());
        // 11 chars: (11*11+35)*2 = 312 dots, over budget — reject, never shrink.
        let wide = TsplLabel {
            barcode: "01234567890",
            ..fitting
        };
        assert_eq!(
            render_label_40x20(&wide).expect_err("too wide"),
            DomainError::TsplBarcodeTooWide
        );
    }

    #[test]
    fn label_text_truncates_and_sanitizes_without_silent_overflow() {
        let label = TsplLabel {
            name: "Vis autotaraudeuse inoxydable très longue désignation",
            price_text: "1234.50",
            barcode: "V-1",
            copies: 1,
        };
        let text = String::from_utf8(render_label_40x20(&label).expect("renders")).expect("ascii");
        assert!(
            !text.contains("désignation"),
            "full name must not leak past the field"
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

    fn owner_ticket() -> TsplTicket<'static> {
        TsplTicket {
            store_name: "BARAKA QUINCAILLERIE",
            store_address: "Rue des Frères",
            store_phone: "0550 00 00 00",
            number: "T-2026-000123",
            created_at: "2026-10-27T20:00:00.000Z",
            captions: TsplCaptionLang::French,
            lines: &[TsplTicketLine {
                sku: "VIS-001",
                designation: "Vis 8x40",
                quantity_text: "3",
                unit_text: "Pièce",
                unit_price_text: "9.00",
                line_total_text: "27.00",
            }],
            totals: TsplTicketTotals {
                subtotal_ht: "27.00",
                tva: "0.00",
                total_ttc: "27.00",
                paid: "27.00",
                change: "0.00",
                payment_method: "CASH",
            },
            footer: "Merci de votre visite",
        }
    }

    #[test]
    fn ticket_bytes_pin_size_gapless_barcode_and_copies() {
        let text = String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("renders"))
            .expect("ascii");
        // Height: cursor math pins SIZE 80 mm,59 mm (472 dots, 472/8 exact).
        assert!(text.starts_with("SIZE 80 mm,59 mm\r\nDIRECTION 1\r\nCLS\r\n"));
        assert!(!text.contains("GAP"), "gapless: no GAP command emitted");
        assert!(
            !text.contains("CUT"),
            "no printer-specific cut command emitted"
        );
        // Ticket-number barcode centered: (640 - (13*11+35)*2) / 2 = 142,
        // shifted +20 dots by the sku line: y = 348.
        assert!(text.contains("BARCODE 142,348,\"128\",64,1,0,2,2,\"T-2026-000123\""));
        assert!(text.ends_with("PRINT 1,1"));
        assert_eq!(text.matches("BARCODE").count(), 1);
    }

    #[test]
    fn ticket_totals_mirror_stored_values_verbatim() {
        let text = String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("renders"))
            .expect("ascii");
        assert!(
            text.contains("3 Pièce x 9.00 = 27.00"),
            "net PU line transcribed"
        );
        assert!(text.contains("VIS-001"), "stored sku transcribed");
        assert!(
            !text.contains("10.00"),
            "raw PU never leaks into a discounted line"
        );
        assert!(text.contains("Sous-total: 27.00"));
        assert!(text.contains("TVA: 0.00"));
        assert!(text.contains("TOTAL: 27.00"));
        assert!(text.contains("Payé (CASH): 27.00"));
        assert!(text.contains("Rendu: 0.00"));
    }

    #[test]
    fn ticket_arabic_captions_emit_without_destroying_bytes() {
        let mut ticket = owner_ticket();
        ticket.captions = TsplCaptionLang::Arabic;
        let text =
            String::from_utf8(render_ticket_80mm(&ticket).expect("renders")).expect("utf8 kept");
        assert!(text.contains("المجموع: 27.00"));
    }
}
