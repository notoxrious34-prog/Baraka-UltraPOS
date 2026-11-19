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

pub mod bitmap;
pub mod common;
pub mod label;
pub mod media;
pub mod probe;
pub mod studio;
pub mod ticket_model;
pub mod ticket_plan;
pub mod ticket_render;

#[cfg(test)]
use crate::errors::DomainError;

pub use bitmap::*;
// Test-only pins: the golden tests address shared geometry through the
// hub; normal builds carry no dead re-export.
#[cfg(test)]
pub(crate) use common::{
    BARCODE_NARROW_DOTS, FONT_2_H, FONT_3_H, FONT_4_H, LABEL_BARCODE_MAX_CHARS, QUIET_ZONE_DOTS,
    TICKET_BARCODE_MAX_CHARS,
};
pub use label::*;
pub use media::*;
pub use probe::*;
pub use studio::*;
pub use ticket_model::*;
pub use ticket_plan::*;
pub use ticket_render::*;

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
             TEXT 112,16,\"2\",0,1,1,\"Vis 8x40\"\r\n\
             TEXT 100,44,\"4\",0,1,1,\"10.00\"\r\n\
             TEXT 280,54,\"1\",0,1,1,\"DZD\"\r\n\
             TEXT 132,132,\"1\",0,1,1,\"VIS-001\"\r\n\
             BARCODE 48,86,\"128\",38,0,0,2,2,\"VIS-001\"\r\n\
             PRINT 1,3\r\n"
        );
    }

    fn offset_media(x: i32, y: i32) -> TsplMediaSetup {
        TsplMediaSetup {
            offset_x_mm: x,
            offset_y_mm: y,
            ..TsplMediaSetup::default()
        }
    }

    /// DOC-06 RED: zero calibration is byte-identical to the golden above.
    #[test]
    fn calibration_zero_keeps_golden_bytes() {
        let mut calibrated = owner_label(3);
        calibrated.media = offset_media(0, 0);
        let bytes = render_label_40x20(&calibrated).expect("zero calib renders");
        let golden = render_label_40x20(&owner_label(3)).expect("golden renders");
        assert_eq!(bytes, golden);
    }

    /// DOC-06 RED: (-1 mm, 0) shifts every emitted x coordinate -8 dots.
    #[test]
    fn calibration_shifts_every_label_coordinate() {
        let mut calibrated = owner_label(1);
        calibrated.media = offset_media(-1, 0);
        let bytes = render_label_40x20(&calibrated).expect("shifted renders");
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
             TEXT 104,16,\"2\",0,1,1,\"Vis 8x40\"\r\n\
             TEXT 92,44,\"4\",0,1,1,\"10.00\"\r\n\
             TEXT 272,54,\"1\",0,1,1,\"DZD\"\r\n\
             TEXT 124,132,\"1\",0,1,1,\"VIS-001\"\r\n\
             BARCODE 40,86,\"128\",38,0,0,2,2,\"VIS-001\"\r\n\
             PRINT 1,1\r\n"
        );
    }

    /// DOC-06 RED: a shift pushing content off the 320x160 area fails LOUDLY.
    #[test]
    fn calibration_past_the_edge_rejects_loudly() {
        let mut calibrated = TsplLabel {
            name: "0123456789ABCDEFGHIJ0123",
            ..owner_label(1)
        };
        calibrated.media = offset_media(-5, 0);
        let err = render_label_40x20(&calibrated).expect_err("must reject");
        assert_eq!(
            err,
            DomainError::InvalidCanonicalString("LABEL_CALIBRATION_OUT_OF_AREA".to_string())
        );
        // Feed axis: the human line bottom sits at 132 + 12 = 144, so a
        // +3 mm feed shift (144 + 24 = 168 > 160) fails loudly.
        let mut calibrated = owner_label(1);
        calibrated.media = offset_media(0, 3);
        let err = render_label_40x20(&calibrated).expect_err("must reject");
        assert_eq!(
            err,
            DomainError::InvalidCanonicalString("LABEL_CALIBRATION_OUT_OF_AREA".to_string())
        );
    }

    /// DOC-06 RED: out-of-range calibration values never reach the renderer.
    #[test]
    fn calibration_range_rejects_loudly() {
        assert!(validate_tspl_calib_mm(5).is_ok());
        assert!(validate_tspl_calib_mm(-5).is_ok());
        assert!(validate_tspl_calib_mm(6).is_err());
        assert!(validate_tspl_calib_mm(-6).is_err());
    }

    /// DOC-06 RED: the calibration pattern pins preamble, border, ruler and tail.
    #[test]
    fn calibration_pattern_pins_preamble_border_and_tail() {
        let bytes =
            render_calibration_pattern_40x20(&TsplMediaSetup::default()).expect("pattern renders");
        let text = String::from_utf8(bytes).expect("tspl is ascii-clean");
        assert!(text.starts_with("SIZE 40 mm, 20 mm\r\nGAP 3 mm, 0 mm\r\nSET RIBBON OFF\r\n"));
        // Full-bleed border at the 320x160 media bounds (ruler reads the inset).
        assert!(text.contains("BOX 0,0,319,159,2\r\n"));
        // 1 mm ruler ticks along the top edge (8 dots = 1 mm at 203 dpi).
        assert!(text.contains("BAR 8,0,1,4\r\n"));
        assert!(text.contains("BAR 40,0,1,8\r\n"));
        // The printout documents its own correction.
        assert!(text.contains("\"X+0MM Y+0MM\""));
        assert!(text.ends_with("PRINT 1,1\r\n"));
    }

    /// DOC-06 RED: the pattern shifts WITH the calibration under test.
    #[test]
    fn calibration_pattern_shifts_with_offsets() {
        let media = offset_media(1, -1);
        let bytes = render_calibration_pattern_40x20(&media).expect("pattern renders");
        let text = String::from_utf8(bytes).expect("tspl is ascii-clean");
        assert!(text.contains("BOX 8,-8,327,151,2\r\n"));
        assert!(text.contains("\"X+1MM Y-1MM\""));
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
    fn label_brand_never_prints_even_when_set() {
        // Redesign agreement L1: the brand line is gone (no vertical room).
        // A set brand is dropped, never rejected, never printed.
        let text =
            String::from_utf8(render_label_40x20(&owner_label(1)).expect("ok")).expect("ascii");
        assert!(!text.contains("BARAKA ULTRAPOS"), "brand never prints");
        assert!(text.contains("TEXT 112,16,\"2\",0,1,1,\"Vis 8x40\""));
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
        assert!(render_label_40x20(&fitting).is_ok(), "9 chars fits");
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
    fn label_designation_single_line_truncates_at_24() {
        // Redesign agreement L4: one centered row, no second line.
        let label = TsplLabel {
            name: "0123456789ABCDEFGHIJ0123456789ABCDEFGHIJ0123456789AB",
            ..owner_label(1)
        };
        let text = String::from_utf8(render_label_40x20(&label).expect("ok")).expect("ascii");
        assert!(text.contains("TEXT 16,16,\"2\",0,1,1,\"0123456789ABCDEFGHIJ0...\""));
        assert!(!text.contains("TEXT 8,38,"), "no second name row");
        assert!(
            !text.contains("0123456789ABCDEFGHIJ0123456789AB"),
            "overflow never leaks"
        );
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
            "full name must not leak past the single-line field"
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
                credit: None,
            },
            footer: "Merci",
            is_reprint: false,
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
            name: "café 8x40",
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
            offset_x_mm: 0,
            offset_y_mm: 0,
            probe_mode: TsplProbeMode::Normal,
        }
        .validate()
        .expect("edges validate");
        TsplMediaSetup {
            gap_mm: 10,
            density: 0,
            speed: 1,
            offset_x_mm: 0,
            offset_y_mm: 0,
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

    // DOC-07 final (owner verdict 2026-11-09): ONE layout for every
    // document — v1 bytes were never correct (clipped money figures fail
    // the receipt's legal purpose), so both pinned versions render the
    // final v2 geometry; only unknown versions reject LOUDLY.
    #[test]
    fn ticket_template_version_dispatch_pins_v1_and_rejects_unknown() {
        // v1 and v2 both render the final geometry — the same bytes.
        let pinned_v1 =
            render_ticket_80mm_versioned(&owner_ticket(), TSPL_TICKET_TEMPLATE_VERSION_V1)
                .expect("v1 renders");
        let pinned_v2 = render_ticket_80mm_versioned(&owner_ticket(), TSPL_TICKET_TEMPLATE_VERSION)
            .expect("v2 renders");
        assert_eq!(pinned_v1, pinned_v2, "one layout for every document");
        // ...while the frozen v1 reference stays byte-identical (history).
        let frozen = render_ticket_80mm(&owner_ticket()).expect("renders");
        assert_ne!(pinned_v2, frozen, "final layout supersedes the frozen v1");
        let v2_text = String::from_utf8(pinned_v2).expect("ascii");
        assert!(
            v2_text.contains("REVERSE "),
            "final carries a REVERSE band, got:\n{v2_text}"
        );
        assert_eq!(
            render_ticket_80mm_versioned(&owner_ticket(), 99).expect_err("unknown rejects"),
            DomainError::InvalidCanonicalString("TSPL_TEMPLATE_VERSION_UNKNOWN".to_string())
        );
    }

    // DOC-07: wall-clock instant, not ISO.
    #[test]
    fn ticket_instant_reformats_rfc3339_to_wall_clock() {
        assert_eq!(
            format_ticket_instant("2026-11-07T19:20:04.123+00:00"),
            "07/11/2026 19:20"
        );
        assert_eq!(
            format_ticket_instant("2026-11-07T19:20:04Z"),
            "07/11/2026 19:20"
        );
        assert_eq!(
            format_ticket_instant("2026-11-07T19:20:04"),
            "07/11/2026 19:20"
        );
        // Unparseable input returns verbatim — cosmetics never kill a ticket.
        assert_eq!(format_ticket_instant("not-a-date"), "not-a-date");
        assert_eq!(format_ticket_instant(""), "");
    }

    // DOC-07: zero-hide predicate — only canonical `0.00` hides a row.
    #[test]
    fn zero_money_text_detects_only_canonical_zero() {
        assert!(money_text_is_zero("0.00"));
        assert!(!money_text_is_zero("0.01"));
        assert!(!money_text_is_zero("1310.00"));
        assert!(!money_text_is_zero("-0.00"));
        assert!(!money_text_is_zero("0.0"));
        assert!(!money_text_is_zero("abc"));
        assert!(!money_text_is_zero(""));
    }

    static MOCK_LINES_V2: [TsplTicketLine<'static>; 3] = [
        TsplTicketLine {
            sku: "MONCH-001",
            designation: "MONCH ISPANA GRIS0958",
            quantity_text: "1",
            unit_text: "PCS",
            unit_price_text: "530.00",
            line_total_text: "530.00",
            discount_text: Some("0.00"),
        },
        TsplTicketLine {
            sku: "BOXR-002",
            designation: "BOX ISPANA",
            quantity_text: "1",
            unit_text: "PCS",
            unit_price_text: "330.00",
            line_total_text: "330.00",
            discount_text: Some("0.00"),
        },
        TsplTicketLine {
            sku: "DOMI-003",
            designation: "DOMI MONCH",
            quantity_text: "1",
            unit_text: "PCS",
            unit_price_text: "450.00",
            line_total_text: "450.00",
            discount_text: Some("0.00"),
        },
    ];
    static MOCK_TAX_V2: [(&str, &str); 0] = [];

    fn mock_ticket_v2() -> TsplTicket<'static> {
        TsplTicket {
            store_name: "BarakaDEV",
            store_address: "Bordj Bou Arreridj",
            store_phone: "Tel: 0770 80 70 05",
            tax_rows: &MOCK_TAX_V2,
            number: "TICK-2026-000028",
            created_at: "2026-11-07T19:20:04.000+00:00",
            cashier_text: "Mohamed",
            warehouse_text: "Principal",
            captions: TsplCaptionLang::French,
            lines: &MOCK_LINES_V2,
            totals: TsplTicketTotals {
                subtotal_ht: "1310.00",
                global_discount: "0.00",
                tva: "0.00",
                total_ttc: "1310.00",
                paid: "1310.00",
                change: "0.00",
                payment_method: "CASH",
                credit: None,
            },
            footer: "",
            is_reprint: true,
            media: TsplMediaSetup::default(),
        }
    }

    // DOC-07: the full owner-mock scenario — names, wall-clock instant,
    // zero rows collapsed, `*** DUPLICATA ***` on a REVERSE band, F3 total
    // on a REVERSE band between double rules, 12 mm centered barcode.
    #[test]
    fn v2_plan_matches_owner_mock_and_stays_inside_printable() {
        let ticket = mock_ticket_v2();
        let (ops, _) = ticket_text_plan_v2(&ticket);

        // Every planned TEXT box sits inside the printable box.
        for op in &ops {
            if let TicketOp::Text(line) = op {
                assert!(
                    line.x_dots >= TICKET_MARGIN_LEFT_DOTS,
                    "left of printable: {:?}",
                    line.slot
                );
                assert!(
                    line.x_dots + line.box_w_dots <= TICKET_WIDTH_DOTS - TICKET_MARGIN_RIGHT_DOTS,
                    "right of printable: {:?}",
                    line.slot
                );
            }
        }

        let text =
            String::from_utf8(render_ticket_80mm_v2(&ticket).expect("renders")).expect("text");

        // Wall-clock instant, never ISO.
        assert!(text.contains("07/11/2026 19:20"), "instant, got:\n{text}");
        assert!(!text.contains("T19:20"), "no ISO T, got:\n{text}");

        // Resolved names, never ids.
        assert!(text.contains("Caissier: Mohamed"), "cashier, got:\n{text}");
        assert!(text.contains("Depot: Principal"), "depot, got:\n{text}");

        // Number + instant share one row.
        let number_y = ops
            .iter()
            .find_map(|op| match op {
                TicketOp::Text(line) if line.slot == TicketTextSlot::DocNumber => Some(line.y_dots),
                _ => None,
            })
            .expect("number planned");
        let instant_y = ops
            .iter()
            .find_map(|op| match op {
                TicketOp::Text(line) if line.slot == TicketTextSlot::Instant => Some(line.y_dots),
                _ => None,
            })
            .expect("instant planned");
        assert_eq!(
            number_y,
            instant_y - (FONT_3_H - FONT_2_H),
            "instant bottom-aligns to the F3 number"
        );

        // One-line rows: name, detail and total share a single F1 row.
        let mut item_rows = std::collections::HashMap::new();
        for op in &ops {
            if let TicketOp::Text(line) = op {
                if matches!(
                    line.slot,
                    TicketTextSlot::LineName
                        | TicketTextSlot::LineDetail
                        | TicketTextSlot::LineTotal
                ) {
                    item_rows
                        .entry(line.y_dots)
                        .or_insert_with(Vec::new)
                        .push(line.slot);
                }
            }
        }
        assert!(
            item_rows.values().any(|slots| slots.len() == 3),
            "name+detail+total share one row, got: {item_rows:?}"
        );
        let detail_face = ops
            .iter()
            .find_map(|op| match op {
                TicketOp::Text(line) if line.slot == TicketTextSlot::LineDetail => Some(line.face),
                _ => None,
            })
            .expect("detail planned");
        assert_eq!(detail_face, TicketFace::F2, "TRIAL F2: detail rides F2");
        let mid_align = ops
            .iter()
            .find_map(|op| match op {
                TicketOp::Text(line) if line.slot == TicketTextSlot::ColumnHeaderMid => {
                    Some(line.align)
                }
                _ => None,
            })
            .expect("mid header planned");
        assert_eq!(mid_align, TicketAlign::Left, "mid header opens left");

        // Three-column header on one row.
        let mut header_rows = std::collections::HashSet::new();
        for op in &ops {
            if let TicketOp::Text(line) = op {
                if matches!(
                    line.slot,
                    TicketTextSlot::ColumnHeader
                        | TicketTextSlot::ColumnHeaderMid
                        | TicketTextSlot::ColumnHeaderRight
                ) {
                    header_rows.insert(line.y_dots);
                }
            }
        }
        assert_eq!(header_rows.len(), 1, "header is one row");
        assert!(text.contains("Article"), "article col, got:\n{text}");
        assert!(text.contains("Total"), "total col, got:\n{text}");

        // Zero badge / zero discount / zero TVA collapse; subtotal stays.
        assert!(!text.contains("remise"), "no zero badge, got:\n{text}");
        assert!(
            !text.contains("Remise totale"),
            "no zero discount, got:\n{text}"
        );
        assert!(
            !text.contains(",\"1\",0,1,1,\"TVA"),
            "no zero TVA, got:\n{text}"
        );
        assert!(text.contains("Sous-total"), "subtotal stays, got:\n{text}");

        // Grand total: F4 (never the F5 slab) on a REVERSE band, framed.
        assert!(
            text.contains(",\"4\",0,1,1,\"TOTAL TTC :"),
            "F4 total caption, got:\n{text}"
        );
        assert!(
            text.contains(",\"4\",0,1,1,\"1310.00"),
            "F4 total figure, got:\n{text}"
        );
        assert!(
            !text.contains(",\"5\","),
            "no F5 slab anywhere, got:\n{text}"
        );
        let total_y = ops
            .iter()
            .find_map(|op| match op {
                TicketOp::Text(line)
                    if line.slot == TicketTextSlot::TotalTtc && line.text.starts_with("1310") =>
                {
                    Some(line.y_dots)
                }
                _ => None,
            })
            .expect("total planned");
        let band = ops
            .iter()
            .find_map(|op| match op {
                TicketOp::Reverse { x, y, w, h } if *y == total_y - TICKET_BAND_PAD_V2 => {
                    Some((*x, *w, *h))
                }
                _ => None,
            })
            .expect("REVERSE band on the total row");
        assert_eq!(
            band,
            (32, 536, FONT_4_H + 2 * TICKET_BAND_PAD_V2),
            "band spans the printable box with +8 optical padding"
        );

        // `*** DUPLICATA ***` carries its own band.
        assert!(
            text.contains("*** DUPLICATA ***"),
            "framed mark, got:\n{text}"
        );

        // Barcode: 12 mm, narrow bars, concentric inside the wide frame.
        assert!(
            text.contains("BARCODE 89,"),
            "box-centered barcode x, got:\n{text}"
        );
        assert!(
            text.contains(",\"128\",96,0,0,1,2,\"TICK-2026-000028\""),
            "12 mm narrow barcode, got:\n{text}"
        );
        // Right clipping is impossible: the rightmost figure ends at 568.
        assert!(
            text.contains("TEXT 400,"),
            "total figure ends at the 568 edge, got:\n{text}"
        );
        let frame = ops
            .iter()
            .find_map(|op| match op {
                TicketOp::Box { x1, x2, .. } => Some((*x1, *x2)),
                _ => None,
            })
            .expect("wide frame planned");
        assert_eq!(frame, (32, 568), "frame spans the printable box");
        assert!(
            text.contains("BOX 32,"),
            "wide frame spans the printable box, got:\n{text}"
        );
    }

    // Agreement item 1: long designations wrap on word boundaries, short
    // ones never split, single monsters hard-split (documented, never lost).
    #[test]
    fn wrap_two_lines_keeps_short_names_whole_and_splits_long_ones() {
        let (first, second) = wrap_two_lines("JILI BERRAK", SEG_NAME_CHARS_V2);
        assert_eq!(first, "JILI BERRAK");
        assert_eq!(second, None);
        let long = "MONCH ISPANA GRIS0958 EXTRA LONG DESIGNATION FOR TEST";
        let (first, second) = wrap_two_lines(long, SEG_NAME_CHARS_V2);
        assert!(
            first.chars().count() <= SEG_NAME_CHARS_V2,
            "first fits: {first}"
        );
        let second = second.expect("long name wraps");
        assert!(
            second.chars().count() <= SEG_NAME_CHARS_V2,
            "second fits: {second}"
        );
        assert!(
            format!("{first} {second}").starts_with("MONCH ISPANA"),
            "no word lost: {first} / {second}"
        );
    }

    // DOC-07: a real discount still prints its badge; a zero one never does.
    #[test]
    fn v2_badge_prints_only_for_nonzero_discount() {
        let ticket = mock_ticket_v2();
        let text =
            String::from_utf8(render_ticket_80mm_v2(&ticket).expect("renders")).expect("text");
        assert!(!text.contains("remise"), "zero badge collapses");

        let mut with_discount = mock_ticket_v2();
        let discounted = [TsplTicketLine {
            discount_text: Some("50.00"),
            ..MOCK_LINES_V2[0]
        }];
        with_discount.lines = &discounted;
        let shown = String::from_utf8(render_ticket_80mm_v2(&with_discount).expect("renders"))
            .expect("text");
        assert!(
            shown.contains("remise: 50.00"),
            "real badge prints, got:\n{shown}"
        );
    }

    // DSN-103: frozen credit-equation witness rows print on credit
    // documents and collapse entirely on cash-only / legacy documents.
    #[test]
    fn v2_credit_equation_prints_only_with_snapshot() {
        let plain = mock_ticket_v2();
        let text =
            String::from_utf8(render_ticket_80mm_v2(&plain).expect("renders")).expect("text");
        assert!(
            !text.contains("Ancien solde"),
            "no witness rows without snapshot, got:\n{text}"
        );

        let mut credit = mock_ticket_v2();
        credit.totals.credit = Some(TsplTicketCredit {
            ancien: "4500.00",
            montant: "200.00",
            versement: "50.00",
            nouveau: "4650.00",
        });
        let shown =
            String::from_utf8(render_ticket_80mm_v2(&credit).expect("renders")).expect("text");
        for witness in [
            "Ancien solde",
            "Montant document",
            "Versement",
            "Nouveau solde",
            "4500.00",
            "4650.00",
        ] {
            assert!(
                shown.contains(witness),
                "witness prints {witness}, got:\n{shown}"
            );
        }
    }

    // DOC-07 final: narrower payload budget — 19 chars fit, 20 reject loudly.
    #[test]
    fn v2_barcode_budget_matches_printable_width() {
        let mut ticket = mock_ticket_v2();
        ticket.number = "1234567890123456789";
        render_ticket_80mm_v2(&ticket).expect("19 chars fit");
        ticket.number = "12345678901234567890";
        assert_eq!(
            render_ticket_80mm_v2(&ticket).expect_err("20 chars reject"),
            DomainError::TsplBarcodeTooWide
        );
    }

    // DOC-07 final: 80 mm ticket ruler — full-width 1 mm ticks for
    // measuring the printer's true right dead zone on paper.
    #[test]
    fn ticket_ruler_spans_full_width_with_numbered_decades() {
        let media = TsplMediaSetup {
            gap_mm: 0,
            ..TsplMediaSetup::default()
        };
        let text = String::from_utf8(render_ticket_ruler_80mm(&media).expect("renders"))
            .expect("ascii-clean");
        assert!(text.starts_with("SIZE 80 mm, 40 mm\r\n"), "ticket stock");
        assert!(text.contains("GAP 0,0\r\n"), "continuous stock");
        assert!(text.ends_with("PRINT 1,1\r\n"), "single-copy tail");
        let bars: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with("BAR "))
            .collect();
        assert_eq!(bars.len(), 80, "1 mm ticks 0..=632 step 8");
        assert_eq!(bars[0], "BAR 0,56,2,48", "first tick tall");
        assert_eq!(bars[10], "BAR 80,56,2,48", "10 mm decade tall");
        assert_eq!(bars[1], "BAR 8,56,2,24", "minor ticks short");
        assert_eq!(bars[79], "BAR 632,56,2,24", "last tick at the far edge");
        for mark in ["\"0\"", "\"10\"", "\"20\"", "\"70\""] {
            assert!(text.contains(mark), "numbered {mark}");
        }
    }

    // DOC-05: a reprint is visibly a reprint — the engine stamps a
    // DUPLICATA band (FR TEXT / AR BITMAP via the rasterer). Originals
    // stay byte-identical to the pre-mark goldens (no band, no shift).
    #[test]
    fn ticket_reprint_mark_band_prints_duplicata_for_french() {
        let mut ticket = owner_ticket();
        ticket.is_reprint = true;
        let text = String::from_utf8(render_ticket_80mm(&ticket).expect("renders")).expect("ascii");
        assert!(
            text.contains(",\"3\",0,1,1,\"DUPLICATA\""),
            "F3 DUPLICATA band, got:\n{text}"
        );
        let depot_at = text.find("Depot:").expect("depot present");
        let mark_at = text.find("DUPLICATA").expect("mark present");
        let header_at = text.find("Qté x PU = Total").expect("header present");
        assert!(
            depot_at < mark_at && mark_at < header_at,
            "band sits between depot and column header"
        );
        let plain = String::from_utf8(render_ticket_80mm(&owner_ticket()).expect("renders"))
            .expect("ascii");
        assert!(!plain.contains("DUPLICATA"), "originals carry no mark");
    }

    #[test]
    fn ticket_reprint_mark_plan_carries_arabic_word_for_rasterer() {
        let mut ticket = owner_ticket();
        ticket.captions = TsplCaptionLang::Arabic;
        ticket.is_reprint = true;
        let (ops, _) = ticket_text_plan(&ticket);
        let mark = ops
            .iter()
            .filter_map(|op| match op {
                TicketOp::Text(line) if line.slot == TicketTextSlot::ReprintMark => Some(line),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(mark.len(), 1, "exactly one mark band");
        assert_eq!(mark[0].text, "نسخة");
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
        // The brand line is gone: an Arabic brand is dropped, not rejected.
        let brand = TsplLabel {
            brand_text: "بركة",
            ..owner_label(1)
        };
        let brand_text =
            String::from_utf8(render_label_40x20(&brand).expect("brand dropped")).expect("ascii");
        assert!(!brand_text.contains("بركة"), "brand never prints");
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
        assert_eq!(l1.x_dots, 64);
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
        let bitmap_head = b"BITMAP 64,35,2,1,0,";
        assert!(
            bytes.windows(bitmap_head.len()).any(|w| w == bitmap_head),
            "bitmap at bottom-aligned origin"
        );
        assert!(
            !bytes
                .windows(b"TEXT 64,16,".len())
                .any(|w| w == b"TEXT 64,16,"),
            "replaced TEXT line gone"
        );
        assert!(bytes.ends_with(b"\r\nPRINT 1,1\r\n"), "tail intact");
    }
}
