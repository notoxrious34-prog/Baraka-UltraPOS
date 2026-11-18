//! Ticket layout plans v1 + v2 (DOC-01/DOC-02/DOC-07): geometry only.
//!
//! ALL ticket geometry lives here — renderers only emit. The v1 plan is the
//! frozen historical reference; v2 is the owner receipt redesign.

use super::common::*;
use super::media::*;
use super::ticket_model::*;

/// Ticket barcode height in dots (8.0 mm — see module math).
const TICKET_BARCODE_HEIGHT: i64 = 64;

/// Ticket line pitches in dots (font height + leading).
const PITCH_F1: i64 = 20;
const PITCH_F2: i64 = 28;
const PITCH_F3: i64 = 32;
/// DOC-07 cosmetics (owner answers 2026-11-09): v2 rows breathe +4 dots.
/// Dedicated constants — the frozen v1 geometry keeps the base pitches.
const PITCH_F1_V2: i64 = 24;
const PITCH_F2_V2: i64 = 32;
const PITCH_F3_V2: i64 = 36;
const PITCH_F4_V2: i64 = 44;
/// Advance after a double-height "3" row (2 × 24 + 8).
const PITCH_F3_X2: i64 = 56;

/// DOC-07 final (owner verdict 2026-11-09: both prints clip): the printer's
/// dead zone starts well before the paper edge, so the printable box is
/// pulled further in — left margin 4 mm, right margin 9 mm. Deliberate
/// asymmetry: the left side proved crisp down to x=0, the right side eats
/// ink from ~x590. A ticket-printer ruler (`render_ticket_ruler_80mm`)
/// measures the exact edge; these bounds move again only from its reading.
pub const TICKET_MARGIN_LEFT_DOTS: i64 = 32;
pub const TICKET_MARGIN_RIGHT_DOTS: i64 = 72;
pub const TICKET_PRINTABLE_DOTS: i64 =
    TICKET_WIDTH_DOTS - TICKET_MARGIN_LEFT_DOTS - TICKET_MARGIN_RIGHT_DOTS;
/// v2 barcode: 12 mm tall for fast scanner reads, centered.
pub const TICKET_BARCODE_HEIGHT_V2: i64 = 96;
/// v2 payload budget inside the 536-dot printable area with 20-dot quiet
/// zones on both sides: floor((536 - 40) / 2 / 11) -> 19 chars.
pub const TICKET_BARCODE_MAX_CHARS_V2: usize = 19;

/// Make free text safe for a quoted TSPL field: `"` would break command
/// framing (no escape exists) and CR/LF would inject commands.
/// Ticket printer face (DOC-02): F5 is the native 32x48 face — same
/// geometry the old scale-2 price occupied, but crisp. F4 (DOC-07
/// cosmetics) is the native 24x32 face for the store name + grand total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketFace {
    F1,
    F2,
    F3,
    F4,
    F5,
}

impl TicketFace {
    pub(crate) fn face_str(self) -> &'static str {
        match self {
            TicketFace::F1 => "1",
            TicketFace::F2 => "2",
            TicketFace::F3 => "3",
            TicketFace::F4 => "4",
            TicketFace::F5 => "5",
        }
    }
    pub(crate) fn scale(self) -> i64 {
        1
    }
    fn cell_h(self) -> i64 {
        match self {
            TicketFace::F1 => FONT_1_H,
            TicketFace::F2 => FONT_2_H,
            TicketFace::F3 => FONT_3_H,
            TicketFace::F4 => FONT_4_H,
            TicketFace::F5 => 48,
        }
    }
    /// Monospace cell width of the face (DOC-07 alignment math).
    pub(crate) fn width(self) -> i64 {
        match self {
            TicketFace::F1 => FONT_1_W,
            TicketFace::F2 => FONT_2_W,
            TicketFace::F3 => FONT_3_W,
            TicketFace::F4 => FONT_4_W,
            TicketFace::F5 => FONT_3_W * 2,
        }
    }
    /// v2 row pitch: the +4 breathing room (v1 uses the PITCH_F* constants).
    fn pitch_v2(self) -> i64 {
        match self {
            TicketFace::F1 => PITCH_F1_V2,
            TicketFace::F2 => PITCH_F2_V2,
            TicketFace::F3 => PITCH_F3_V2,
            TicketFace::F4 => PITCH_F4_V2,
            TicketFace::F5 => 56,
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
    /// DUPLICATA stamp stamped on reprints only (DOC-05).
    ReprintMark,
    ColumnHeader,
    /// DOC-07 v2 three-column header: mid + right segments share the row.
    ColumnHeaderMid,
    ColumnHeaderRight,
    LineName,
    LineDetail,
    /// DOC-07 v2: the line net total rides the detail row, right side.
    LineTotal,
    LineBadge,
    TotalsRow,
    /// DOC-07 v2 split totals: caption on the left, figure on the right.
    TotalsCaption,
    TotalTtc,
    Payment,
    Change,
    /// DSN-103 wholesale credit-equation witness rows (4 captions + figures).
    CreditEquation,
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
    Rule {
        x: i64,
        y: i64,
        w: i64,
    },
    /// DOC-07 v2: double bar framing the grand total (`===` look).
    RuleDouble {
        x: i64,
        y: i64,
        w: i64,
    },
    /// DOC-07 v2: TSPL REVERSE band behind the grand total (black band,
    /// white figure) — emitted AFTER the TEXT/BITMAP it inverts.
    Reverse {
        x: i64,
        y: i64,
        w: i64,
        h: i64,
    },
    Barcode {
        x_dots: i64,
        y_dots: i64,
        number: String,
        /// Bar height in dots (v1 64 = 8 mm, v2 96 = 12 mm).
        height_dots: i64,
        /// Narrow bar width in dots (v1 2, v2 1 for a slimmer symbol).
        narrow: u8,
    },
    /// DOC-07 cosmetics: thin frame around the page-centered barcode
    /// block (coupon look) — emitted as `BOX x1,y1,x2,y2,2`.
    Box {
        x1: i64,
        y1: i64,
        x2: i64,
        y2: i64,
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
    ops.push(TicketOp::Rule {
        x: 0,
        y,
        w: TICKET_WIDTH_DOTS,
    });
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
    // DOC-05: reprint stamp — a centered F3 band between depot and the
    // first rule. French prints as TEXT; the Arabic word rides the
    // rasterer (BITMAP via the mixed renderer).
    // Originals emit nothing here: byte-identical to the pre-mark goldens.
    if ticket.is_reprint {
        let mark = truncate_to_width(caps.reprint_mark, name_chars);
        let mark_dots = text_dots(mark.chars().count(), FONT_3_W, 1);
        ops.push(TicketOp::Text(TicketTextLine {
            slot: TicketTextSlot::ReprintMark,
            text: mark,
            x_dots: centered_x(TICKET_WIDTH_DOTS, mark_dots),
            y_dots: y,
            face: TicketFace::F3,
            cell_h_dots: FONT_3_H,
            box_w_dots: inner_w,
            box_h_dots: PITCH_F3,
            align: TicketAlign::Center,
        }));
        y += PITCH_F3;
    }
    ops.push(TicketOp::Rule {
        x: 0,
        y,
        w: TICKET_WIDTH_DOTS,
    });
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
    ops.push(TicketOp::Rule {
        x: 0,
        y,
        w: TICKET_WIDTH_DOTS,
    });
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
    // the human-readable number beneath (DOC-02: printer-generated human
    // line OFF — the TEXT line below stays the single source).
    y += 8;
    let barcode_dots = barcode_width_dots(number.chars().count()) as i64;
    let barcode_x = centered_x(TICKET_WIDTH_DOTS, barcode_dots);
    ops.push(TicketOp::Barcode {
        x_dots: barcode_x,
        y_dots: y,
        number: number.clone(),
        height_dots: TICKET_BARCODE_HEIGHT,
        narrow: BARCODE_NARROW_DOTS as u8,
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

/// DOC-07 v2 ticket plan — owner-requested receipt redesign.
///
/// Every difference from [`ticket_text_plan`] is deliberate:
/// * the printable box shrinks to 568 dots (left 3 mm, right 6 mm) so the
///   printer's dead right edge can never clip a figure again — the reported
///   `530.00 -> 53` and `Sous-total: 1310.` clipping;
/// * the three-column header (`Article | Qté x P.U | Total`) plus an
///   indented detail row replace the hugging-left header;
/// * a zero discount badge, a zero `Remise totale` and a zero `TVA` row
///   collapse, so a plain sale prints no noise;
/// * the grand total drops from F5 to F3 inside a TSPL REVERSE band and is
///   framed by a double rule — emphasis without pixelation;
/// * the reprint stamp reads `*** DUPLICATA ***` on its own REVERSE band;
/// * the barcode grows to 12 mm with 1-dot bars, centered with quiet zones.
///
/// Geometry only: the caller resolves names and the instant first (v2 never
/// prints a raw identifier — see [`format_ticket_instant`]).
/// One-line item segments (dots, inside the printable box — owner agreement
/// TICKET-ONELINE items 1-2, TRIAL F2 2026-11-09: name 240 + detail 164 +
/// total 132 = 536; revert to 264/140/132 + F1 + 33 on owner word).
pub const SEG_NAME_W_V2: i64 = 240;
pub const SEG_DETAIL_W_V2: i64 = 164;
pub const SEG_TOTAL_W_V2: i64 = 132;
/// Name first-row capacity at F2 (240 / 12).
pub const SEG_NAME_CHARS_V2: usize = 20;
/// Black-band vertical padding (agreement item 3): the band grows 4 dots
/// above the glyphs and 4 below; the text drops 4 (true optical centering).
pub const TICKET_BAND_PAD_V2: i64 = 4;

/// Split a designation into at most two rows on word boundaries (agreement
/// item 1): the first row fills `width_chars`, the overflow words continue
/// on a second row inside the same segment (truncated only when two full
/// rows still do not fit — documented, never silent beyond that).
pub fn wrap_two_lines(text: &str, width_chars: usize) -> (String, Option<String>) {
    let text = text.trim();
    if width_chars == 0 {
        return (String::new(), None);
    }
    if text.chars().count() <= width_chars {
        return (text.to_string(), None);
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut first = String::new();
    let mut first_len = 0usize;
    let mut split_at = words.len();
    for (i, word) in words.iter().enumerate() {
        let word_len = word.chars().count();
        let add = if first.is_empty() {
            word_len
        } else {
            1 + word_len
        };
        if first_len + add <= width_chars {
            if !first.is_empty() {
                first.push(' ');
            }
            first.push_str(word);
            first_len += add;
        } else {
            split_at = i;
            break;
        }
    }
    if split_at >= words.len() {
        return (first, None);
    }
    // A single word longer than the row hard-splits (rare, documented).
    let mut second = if first.is_empty() {
        let head: String = text.chars().take(width_chars).collect();
        let tail: String = text.chars().skip(width_chars).collect();
        first = head;
        tail
    } else {
        words[split_at..].join(" ")
    };
    second = truncate_to_width(&second, width_chars);
    if second.trim().is_empty() {
        return (first, None);
    }
    (first, Some(second))
}

pub fn ticket_text_plan_v2(ticket: &TsplTicket<'_>) -> (Vec<TicketOp>, i64) {
    let mut ops: Vec<TicketOp> = Vec::new();
    let mut y: i64 = 8;
    let caps = captions(ticket.captions);
    let left = TICKET_MARGIN_LEFT_DOTS;
    let full = TICKET_PRINTABLE_DOTS;
    let narrow_chars = (full / FONT_1_W) as usize;
    let name_chars = (full / FONT_3_W) as usize;

    // One-line item segments (agreement items 1-2): name | detail | total.
    let seg_detail_x = left + SEG_NAME_W_V2;
    let seg_total_x = left + SEG_NAME_W_V2 + SEG_DETAIL_W_V2;
    debug_assert_eq!(
        SEG_NAME_W_V2 + SEG_DETAIL_W_V2 + SEG_TOTAL_W_V2,
        TICKET_PRINTABLE_DOTS
    );
    // Totals: caption block + value block.
    let cap_w = 330;
    let val_x = left + cap_w;
    let val_w = full - cap_w;

    let mk = |slot: TicketTextSlot,
              text: String,
              x: i64,
              w: i64,
              y: i64,
              face: TicketFace,
              align: TicketAlign| {
        TicketOp::Text(TicketTextLine {
            slot,
            text,
            x_dots: x,
            y_dots: y,
            face,
            cell_h_dots: face.cell_h(),
            box_w_dots: w,
            box_h_dots: face.pitch_v2(),
            align,
        })
    };

    // Store block (blank fields collapse — never empty lines).
    let store = truncate_to_width(ticket.store_name.trim(), (full / FONT_4_W) as usize);
    if !store.is_empty() {
        ops.push(mk(
            TicketTextSlot::StoreName,
            store,
            left,
            full,
            y,
            TicketFace::F4,
            TicketAlign::Center,
        ));
        y += PITCH_F4_V2;
    }
    for line in [ticket.store_address, ticket.store_phone] {
        let shown = truncate_to_width(line.trim(), (full / FONT_3_W) as usize);
        if !shown.is_empty() {
            ops.push(mk(
                TicketTextSlot::Contact,
                shown,
                left,
                full,
                y,
                TicketFace::F3,
                TicketAlign::Center,
            ));
            y += PITCH_F3_V2;
        }
    }
    for (prefix, value) in ticket.tax_rows {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let joined = format!("{prefix}: {trimmed}");
        ops.push(mk(
            TicketTextSlot::TaxRow,
            truncate_to_width(&joined, (full / FONT_3_W) as usize),
            left,
            full,
            y,
            TicketFace::F3,
            TicketAlign::Center,
        ));
        y += PITCH_F3_V2;
    }
    ops.push(TicketOp::Rule {
        x: left,
        y,
        w: full,
    });
    y += 10;

    // Number (left, F2) + instant (right, F1 bottom-aligned to the F2 cell).
    let number = sanitize_tspl_text(ticket.number.trim());
    let num_w = 340;
    ops.push(mk(
        TicketTextSlot::DocNumber,
        truncate_to_width(&number, (num_w / FONT_3_W) as usize),
        left,
        num_w,
        y,
        TicketFace::F3,
        TicketAlign::Left,
    ));
    let instant = format_ticket_instant(ticket.created_at);
    ops.push(mk(
        TicketTextSlot::Instant,
        truncate_to_width(&instant, ((full - num_w) / FONT_2_W) as usize),
        left + num_w,
        full - num_w,
        y + (FONT_3_H - FONT_2_H),
        TicketFace::F2,
        TicketAlign::Right,
    ));
    y += PITCH_F3_V2;

    // Cashier (left half) + depot (right half): resolved names, never ids.
    // When either side overflows its half (orphaned raw identifiers), the
    // row splits into two full-width rows so the fallback stays verbatim.
    let half = full / 2;
    let half_chars = (half / FONT_2_W) as usize;
    let cashier_full = format!("{}: {}", caps.cashier_word, ticket.cashier_text.trim());
    let depot_full = format!("{}: {}", caps.depot_word, ticket.warehouse_text.trim());
    if cashier_full.chars().count() <= half_chars && depot_full.chars().count() <= half_chars {
        ops.push(mk(
            TicketTextSlot::Cashier,
            truncate_to_width(&cashier_full, half_chars),
            left,
            half,
            y,
            TicketFace::F2,
            TicketAlign::Left,
        ));
        ops.push(mk(
            TicketTextSlot::Depot,
            truncate_to_width(&depot_full, half_chars),
            left + half,
            half,
            y,
            TicketFace::F2,
            TicketAlign::Right,
        ));
        y += PITCH_F2_V2;
    } else {
        // Orphan fallback stays F1: a raw 36-char id + caption is 46
        // chars — F2 halves would truncate it, breaking the verbatim
        // contract. Ugly ids stay whole; pretty names ride F2 above.
        for (slot, row) in [
            (TicketTextSlot::Cashier, cashier_full),
            (TicketTextSlot::Depot, depot_full),
        ] {
            ops.push(mk(
                slot,
                truncate_to_width(&row, narrow_chars),
                left,
                full,
                y,
                TicketFace::F1,
                TicketAlign::Left,
            ));
            y += PITCH_F1_V2;
        }
    }

    // Reprint stamp: `*** DUPLICATA ***` on its own REVERSE band (DOC-05
    // stamp, DOC-07 emphasis). Emitted text first, band after (TSPL inverts
    // what is already on the media).
    if ticket.is_reprint {
        let band = format!("*** {} ***", caps.reprint_mark);
        ops.push(mk(
            TicketTextSlot::ReprintMark,
            truncate_to_width(&band, name_chars),
            left,
            full,
            y + TICKET_BAND_PAD_V2,
            TicketFace::F3,
            TicketAlign::Center,
        ));
        ops.push(TicketOp::Reverse {
            x: left,
            y,
            w: full,
            h: FONT_3_H + 2 * TICKET_BAND_PAD_V2,
        });
        y += FONT_3_H + 2 * TICKET_BAND_PAD_V2 + 2 * TICKET_BAND_PAD_V2;
    }

    // Column header: Article | Qté x P.U | Total (agreement item 2 —
    // same three segments as the rows; the mid column opens left so the
    // detail indent never wobbles).
    ops.push(mk(
        TicketTextSlot::ColumnHeader,
        truncate_to_width(caps.column_article, (SEG_NAME_W_V2 / FONT_2_W) as usize),
        left,
        SEG_NAME_W_V2,
        y,
        TicketFace::F2,
        TicketAlign::Left,
    ));
    ops.push(mk(
        TicketTextSlot::ColumnHeaderMid,
        truncate_to_width(caps.column_qty_price, (SEG_DETAIL_W_V2 / FONT_2_W) as usize),
        seg_detail_x,
        SEG_DETAIL_W_V2,
        y,
        TicketFace::F2,
        TicketAlign::Left,
    ));
    ops.push(mk(
        TicketTextSlot::ColumnHeaderRight,
        truncate_to_width(caps.column_total, (SEG_TOTAL_W_V2 / FONT_2_W) as usize),
        seg_total_x,
        SEG_TOTAL_W_V2,
        y,
        TicketFace::F2,
        TicketAlign::Right,
    ));
    y += PITCH_F2_V2;
    ops.push(TicketOp::Rule {
        x: left,
        y,
        w: full,
    });
    y += 8;

    // Lines (agreement items 1-2, TRIAL F2): ONE F2 row — name |
    // `qty x price` | total — plus a wrapped overflow row for long
    // designations. The unit never prints (owner requirement); quantities
    // without a unit render `1 x 400.00` with no double space.
    for line in ticket.lines {
        let (first, second) = wrap_two_lines(line.designation.trim(), SEG_NAME_CHARS_V2);
        ops.push(mk(
            TicketTextSlot::LineName,
            first,
            left,
            SEG_NAME_W_V2,
            y,
            TicketFace::F2,
            TicketAlign::Left,
        ));
        let detail = format!(
            "{} x {}",
            line.quantity_text.trim(),
            line.unit_price_text.trim()
        );
        ops.push(mk(
            TicketTextSlot::LineDetail,
            truncate_to_width(&detail, (SEG_DETAIL_W_V2 / FONT_2_W) as usize),
            seg_detail_x,
            SEG_DETAIL_W_V2,
            y,
            TicketFace::F2,
            TicketAlign::Left,
        ));
        ops.push(mk(
            TicketTextSlot::LineTotal,
            truncate_to_width(
                line.line_total_text.trim(),
                (SEG_TOTAL_W_V2 / FONT_2_W) as usize,
            ),
            seg_total_x,
            SEG_TOTAL_W_V2,
            y,
            TicketFace::F2,
            TicketAlign::Right,
        ));
        y += PITCH_F2_V2;
        if let Some(overflow) = second {
            ops.push(mk(
                TicketTextSlot::LineName,
                overflow,
                left,
                SEG_NAME_W_V2,
                y,
                TicketFace::F2,
                TicketAlign::Left,
            ));
            y += PITCH_F2_V2;
        }

        // Zero discount collapses (owner: `remise 0.00` under every product
        // wasted paper). Unparseable text stays visible on purpose.
        if let Some(discount) = line.discount_text {
            let shown = discount.trim();
            if !money_text_is_zero(shown) {
                let badge = format!(
                    "{}: {}",
                    caps.discount_word,
                    truncate_to_width(shown, narrow_chars / 2)
                );
                ops.push(mk(
                    TicketTextSlot::LineBadge,
                    badge,
                    left + 24,
                    full - 24,
                    y,
                    TicketFace::F1,
                    TicketAlign::Left,
                ));
                y += PITCH_F1_V2;
            }
        }
    }
    ops.push(TicketOp::Rule {
        x: left,
        y,
        w: full,
    });
    y += 8;

    // Totals: caption left, figure right; zero rows collapse.
    for (slot, caption, value, hide_when_zero) in [
        (
            TicketTextSlot::TotalsCaption,
            caps.subtotal,
            ticket.totals.subtotal_ht,
            false,
        ),
        (
            TicketTextSlot::TotalsCaption,
            caps.discount_total,
            ticket.totals.global_discount,
            true,
        ),
        (
            TicketTextSlot::TotalsCaption,
            caps.tva,
            ticket.totals.tva,
            true,
        ),
    ] {
        let shown = value.trim();
        if hide_when_zero && money_text_is_zero(shown) {
            continue;
        }
        let caption_text = format!("{caption} :");
        ops.push(mk(
            slot,
            truncate_to_width(&caption_text, (cap_w / FONT_2_W) as usize),
            left,
            cap_w,
            y,
            TicketFace::F2,
            TicketAlign::Left,
        ));
        ops.push(mk(
            TicketTextSlot::TotalsRow,
            truncate_to_width(shown, (val_w / FONT_2_W) as usize),
            val_x,
            val_w,
            y,
            TicketFace::F2,
            TicketAlign::Right,
        ));
        y += PITCH_F2_V2;
    }

    // Grand total: F4 inside a REVERSE band, framed by a double rule.
    // Owner answer Q4: bigger face + a 20-dot breathing gap above.
    ops.push(TicketOp::RuleDouble {
        x: left,
        y,
        w: full,
    });
    y += 20;
    let total_caption = format!("{} :", caps.total);
    ops.push(mk(
        TicketTextSlot::TotalTtc,
        truncate_to_width(&total_caption, (cap_w / FONT_4_W) as usize),
        left,
        cap_w,
        y + TICKET_BAND_PAD_V2,
        TicketFace::F4,
        TicketAlign::Left,
    ));
    ops.push(mk(
        TicketTextSlot::TotalTtc,
        truncate_to_width(ticket.totals.total_ttc.trim(), (val_w / FONT_4_W) as usize),
        val_x,
        val_w,
        y + TICKET_BAND_PAD_V2,
        TicketFace::F4,
        TicketAlign::Right,
    ));
    ops.push(TicketOp::Reverse {
        x: left,
        y,
        w: full,
        h: FONT_4_H + 2 * TICKET_BAND_PAD_V2,
    });
    y += FONT_4_H + 2 * TICKET_BAND_PAD_V2 + 2 * TICKET_BAND_PAD_V2;
    ops.push(TicketOp::RuleDouble {
        x: left,
        y,
        w: full,
    });
    y += 16;

    // Payment + change (caption left, figure right).
    for (slot, text, value) in [
        (
            TicketTextSlot::Payment,
            format!("{} ({}) :", caps.paid, ticket.totals.payment_method.trim()),
            ticket.totals.paid,
        ),
        (
            TicketTextSlot::Change,
            format!("{} :", caps.change),
            ticket.totals.change,
        ),
    ] {
        ops.push(mk(
            slot,
            truncate_to_width(&text, (cap_w / FONT_2_W) as usize),
            left,
            cap_w,
            y,
            TicketFace::F2,
            TicketAlign::Left,
        ));
        ops.push(mk(
            slot,
            truncate_to_width(value.trim(), (val_w / FONT_2_W) as usize),
            val_x,
            val_w,
            y,
            TicketFace::F2,
            TicketAlign::Right,
        ));
        y += PITCH_F2_V2;
    }

    // DSN-103 wholesale credit-equation witness (frozen figures only; the
    // block collapses entirely on cash-only / legacy documents).
    if let Some(credit) = ticket.totals.credit.as_ref() {
        for (caption, value) in [
            (caps.ancien_solde, credit.ancien),
            (caps.montant_document, credit.montant),
            (caps.versement, credit.versement),
            (caps.nouveau_solde, credit.nouveau),
        ] {
            ops.push(mk(
                TicketTextSlot::CreditEquation,
                truncate_to_width(&format!("{caption} :"), (cap_w / FONT_2_W) as usize),
                left,
                cap_w,
                y,
                TicketFace::F2,
                TicketAlign::Left,
            ));
            ops.push(mk(
                TicketTextSlot::CreditEquation,
                truncate_to_width(value.trim(), (val_w / FONT_2_W) as usize),
                val_x,
                val_w,
                y,
                TicketFace::F2,
                TicketAlign::Right,
            ));
            y += PITCH_F2_V2;
        }
    }

    // Barcode: 12 mm tall, narrow bars, concentric inside OUR wide frame
    // (32..568 — agreement item 5). Bars, number and frame share the box
    // center so the block reads as one coupon; the verification print
    // decides whether an exogenous frame also exists (then ours is
    // withdrawn instead of doubling it).
    y += 12;
    let barcode_width = barcode_width_dots_v2(number.chars().count()) as i64;
    let barcode_x = left + centered_x(full, barcode_width);
    let barcode_y0 = y;
    ops.push(TicketOp::Barcode {
        x_dots: barcode_x,
        y_dots: y,
        number: number.clone(),
        height_dots: TICKET_BARCODE_HEIGHT_V2,
        narrow: 1,
    });
    y += TICKET_BARCODE_HEIGHT_V2 + 10;
    let human = truncate_to_width(&number, narrow_chars);
    let human_y = y;
    ops.push(mk(
        TicketTextSlot::Human,
        human,
        left,
        full,
        human_y,
        TicketFace::F1,
        TicketAlign::Center,
    ));
    ops.push(TicketOp::Box {
        x1: left,
        y1: barcode_y0 - 8,
        x2: left + full,
        y2: human_y + FONT_1_H + 8,
    });
    y += PITCH_F1_V2 + 8;

    // Footer: owner message (collapses when blank) + constant thanks.
    let footer = truncate_to_width(ticket.footer.trim(), narrow_chars);
    if !footer.is_empty() {
        ops.push(mk(
            TicketTextSlot::Footer,
            footer,
            left,
            full,
            y,
            TicketFace::F1,
            TicketAlign::Center,
        ));
        y += PITCH_F1_V2;
    }
    ops.push(mk(
        TicketTextSlot::Thanks,
        truncate_to_width(caps.thanks, (full / FONT_2_W) as usize),
        left,
        full,
        y,
        TicketFace::F2,
        TicketAlign::Center,
    ));
    y += PITCH_F2_V2 + 16;
    (ops, y)
}
