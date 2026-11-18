//! 80 mm ticket renderers (DOC-01/DOC-02/DOC-04/DOC-07): plan-driven emit.
//!
//! Every renderer resolves a plan, substitutes bitmap slots, then emits.
//! Template versions pin reprint fidelity (DOC-04).

use super::bitmap::*;
use super::common::*;
use super::label::LabelBitmap;
use super::media::*;
use super::ticket_model::*;
use super::ticket_plan::*;
use crate::errors::DomainError;

/// Print-template version stamped on every new document (DOC-04) and
/// pinned on reprint: v1 is the renderer as of the DOC-01/DOC-02
/// redesign — frozen so old documents reprint byte-identical forever.
/// v2 (DOC-07) is the owner receipt redesign: narrower printable box,
/// three-column lines, REVERSE grand total, 12 mm barcode. New documents
/// stamp v2; v1 reprints keep the exact bytes they were issued with.
pub const TSPL_TICKET_TEMPLATE_VERSION: u32 = 2;
/// First frozen layout (DOC-01/DOC-02/DOC-05 geometry, transcription as of
/// DOC-05). Kept so pre-v2 documents reprint byte-identical.
pub const TSPL_TICKET_TEMPLATE_VERSION_V1: u32 = 1;

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
        // DOC-07 final: ONE layout for every document. v1 bytes were
        // never correct (right-clipped money figures fail the receipt's
        // legal purpose), so there is no faithful geometry to preserve —
        // fidelity is the transcribed figures, identical either way.
        // `ticket_text_plan` stays as the historical reference + pins.
        TSPL_TICKET_TEMPLATE_VERSION_V1 | TSPL_TICKET_TEMPLATE_VERSION => {
            render_ticket_mixed_for(ticket, bitmaps, TicketLayout::V2)
        }
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
    render_ticket_mixed_for(ticket, bitmaps, TicketLayout::V1)
}

/// DOC-07 v2 layout (owner receipt redesign): same guarantees as v1 —
/// plan-driven, bitmap-substituted, refusals instead of truncation.
pub fn render_ticket_80mm_v2(ticket: &TsplTicket<'_>) -> Result<Vec<u8>, DomainError> {
    render_ticket_mixed_for(ticket, &[], TicketLayout::V2)
}

/// v2 layout with Arabic bitmap substitution.
pub fn render_ticket_80mm_mixed_v2(
    ticket: &TsplTicket<'_>,
    bitmaps: &[(usize, LabelBitmap)],
) -> Result<Vec<u8>, DomainError> {
    render_ticket_mixed_for(ticket, bitmaps, TicketLayout::V2)
}

/// Layout selector: the plan and the x-reanchoring differ, everything
/// else (header, SIZE math, refusals) is shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TicketLayout {
    V1,
    V2,
}

fn render_ticket_mixed_for(
    ticket: &TsplTicket<'_>,
    bitmaps: &[(usize, LabelBitmap)],
    layout: TicketLayout,
) -> Result<Vec<u8>, DomainError> {
    if ticket.number.trim().is_empty() {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_NUMBER_MUST_NOT_BE_EMPTY".to_string(),
        ));
    }
    validate_code128_payload(ticket.number)?;
    // v2's narrower printable box carries a tighter payload budget (DOC-07);
    // v1 keeps its frozen 24-char ceiling.
    let max_chars = match layout {
        TicketLayout::V1 => TICKET_BARCODE_MAX_CHARS,
        TicketLayout::V2 => TICKET_BARCODE_MAX_CHARS_V2,
    };
    if ticket.number.chars().count() > max_chars {
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
    let (ops, end_y) = match layout {
        TicketLayout::V1 => ticket_text_plan(ticket),
        TicketLayout::V2 => ticket_text_plan_v2(ticket),
    };
    // v2 plans are segment boxes (x = segment start, align decides the
    // in-box position); v1 plans pre-bake the absolute x.
    let box_aligned = layout == TicketLayout::V2;
    let code = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    let mut body: Vec<u8> = Vec::new();
    let mut text_idx: usize = 0;
    for op in &ops {
        match op {
            TicketOp::Rule { x, y, w } => {
                body.extend_from_slice(format!("BAR {x},{y},{w},2\r\n").as_bytes());
            }
            TicketOp::RuleDouble { x, y, w } => {
                body.extend_from_slice(format!("BAR {x},{y},{w},2\r\n").as_bytes());
                body.extend_from_slice(format!("BAR {x},{},{w},2\r\n", y + 6).as_bytes());
            }
            TicketOp::Reverse { x, y, w, h } => {
                body.extend_from_slice(format!("REVERSE {x},{y},{w},{h}\r\n").as_bytes());
            }
            TicketOp::Box { x1, y1, x2, y2 } => {
                body.extend_from_slice(format!("BOX {x1},{y1},{x2},{y2},2\r\n").as_bytes());
            }
            TicketOp::Barcode {
                x_dots,
                y_dots,
                number,
                height_dots,
                narrow,
            } => {
                body.extend_from_slice(
                    format!(
                        "BARCODE {x_dots},{y_dots},\"128\",{height_dots},0,0,{narrow},2,\"{number}\"\r\n"
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
                    let bitmap_x = if box_aligned {
                        match line.align {
                            TicketAlign::Left => line.x_dots,
                            TicketAlign::Center => {
                                line.x_dots + centered_x(line.box_w_dots, width_dots)
                            }
                            TicketAlign::Right => line.x_dots + line.box_w_dots - width_dots,
                        }
                    } else {
                        match line.align {
                            TicketAlign::Left => line.x_dots,
                            TicketAlign::Center => (TICKET_WIDTH_DOTS - width_dots) / 2,
                            TicketAlign::Right => {
                                (TICKET_WIDTH_DOTS - TICKET_MARGIN_X) - width_dots
                            }
                        }
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
                    let text_x = if box_aligned {
                        let text_w = text_dots(
                            line.text.chars().count(),
                            line.face.width(),
                            line.face.scale(),
                        );
                        match line.align {
                            TicketAlign::Left => line.x_dots,
                            TicketAlign::Center => {
                                line.x_dots + centered_x(line.box_w_dots, text_w)
                            }
                            TicketAlign::Right => line.x_dots + line.box_w_dots - text_w,
                        }
                    } else {
                        line.x_dots
                    };
                    push_text(
                        &mut text_out,
                        text_x,
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

/// DOC-07 final: 80 mm ticket-printer ruler — measures the printer's true
/// right dead zone instead of guessing it from clipped receipts.
/// Full-width 1 mm ticks (every 8 dots) from x=0 to x=632, tall ticks +
/// millimetre numbers every 10 mm, framed by plain French instructions.
/// The owner prints it once and reports the last visible number; ticket
/// margins move only from that reading. ASCII-only by construction.
pub fn render_ticket_ruler_80mm(media: &TsplMediaSetup) -> Result<Vec<u8>, DomainError> {
    let mut head: Vec<String> = Vec::new();
    push_media_preamble(&mut head, "SIZE 80 mm, 40 mm", media)?;
    let mut body: Vec<String> = head;
    body.push("TEXT 16,8,\"2\",0,1,1,\"TICKET RULER 80MM - LIRE LE DERNIER TRAIT\"".to_string());
    body.push(
        "TEXT 16,32,\"1\",0,1,1,\"IMPRIMER - NOTER LE DERNIER CHIFFRE VISIBLE A DROITE\""
            .to_string(),
    );
    let mut x: i64 = 0;
    while x <= 632 {
        let tall = x % 80 == 0;
        body.push(format!("BAR {x},56,2,{}", if tall { 48 } else { 24 }));
        x += 8;
    }
    let mut mm: i64 = 0;
    while mm <= 70 {
        let tick_x = mm * 8;
        let text_x = if mm == 0 { 0 } else { tick_x - 8 };
        body.push(format!("TEXT {text_x},108,\"1\",0,1,1,\"{mm}\""));
        mm += 10;
    }
    body.push("TEXT 16,132,\"1\",0,1,1,\"REPORTER CE CHIFFRE AU SUPPORT\"".to_string());
    let mut job: Vec<u8> = body.join("\r\n").into_bytes();
    job.extend_from_slice(b"\r\nPRINT 1,1\r\n");
    Ok(job)
}
