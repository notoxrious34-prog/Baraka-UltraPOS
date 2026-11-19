//! 40x20 mm shelf-label engine (DOC-03/DOC-06/PRT-02): plan + render.
//!
//! Single-sourced geometry: the preview reads the same origin as the
//! renderer. TEXT lines whose slot carries a bitmap are replaced by a
//! `BITMAP` chunk (Arabic/CJK path); all-ASCII jobs are byte-identical
//! to the plain renderer.

use super::bitmap::{BITMAP_MAX_HEIGHT_DOTS, BITMAP_MAX_WIDTH_DOTS};
use super::common::*;
use super::media::*;
use super::probe::*;
use super::ticket_model::TsplLabel;
use crate::errors::DomainError;

/// 2 mm safe box on 40x20 mm (owner agreement L-margins): x 16..304.
/// Everything prints inside; the die-cut edge never touches ink.
/// `pub(crate)`: the studio templates share the identical box (DOC-03
/// single-sourced geometry — the preview reads the same origin).
pub(crate) const LABEL_SAFE_LEFT: i64 = 16;
pub(crate) const LABEL_SAFE_WIDTH: i64 = 288;
/// Label Code128 bar height: 38 dots leaves a 2-dot gap to the human
/// line at y=146 (bars 106..144, text 146..158, 2-dot bottom margin).
const LABEL_BARCODE_HEIGHT: i64 = 38;
/// Diacritic headroom for shaped Arabic (marks above/below the cell).
/// `pub(crate)`: the studio templates size their bitmap boxes alike.
pub(crate) const LABEL_BITMAP_PAD_DOTS: i64 = 4;

/// Monospace cell width of a label TEXT face (DOC-06 containment checks).
/// `pub(crate)`: shared with the studio templates (one face table).
pub(crate) fn label_face_width_dots(font: &str, scale: i64) -> i64 {
    let cell = match font {
        // Native 32x48 face: same geometry as "3"x2 (see the old Price arm).
        "5" => FONT_3_W * 2,
        // Redesign face: native 24x32 (agreement L2 — 4 mm price).
        "4" => FONT_4_W,
        "2" => FONT_2_W,
        "3" => FONT_3_W,
        _ => FONT_1_W,
    };
    cell * scale
}

/// Reject a calibration-shifted TEXT line that leaves the 320x160 media
/// (DOC-06): a shift is a correction, never a silent clip.
/// `pub(crate)`: shared with the studio templates.
pub(crate) fn check_calibrated_text(
    x_dots: i64,
    y_dots: i64,
    width_dots: i64,
    height_dots: i64,
) -> Result<(), DomainError> {
    if x_dots < 0
        || y_dots < 0
        || x_dots + width_dots > LABEL_WIDTH_DOTS
        || y_dots + height_dots > LABEL_HEIGHT_DOTS
    {
        return Err(DomainError::InvalidCanonicalString(
            "LABEL_CALIBRATION_OUT_OF_AREA".to_string(),
        ));
    }
    Ok(())
}

/// Which TEXT line of the 40x20 label a bitmap replaces (PRT-02).
/// Studio variants (DSN-107) are appended — existing discriminants never
/// shift, so stored/derived slot indexes stay stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelTextSlot {
    Brand,
    NameLine1,
    NameLine2,
    Price,
    Currency,
    Human,
    StudioModel,
    StudioSize,
    StudioColor,
    StudioPack,
    StudioPrice,
    StudioHwName,
    StudioHwDim,
    StudioHwPrice,
}

/// Slot universe size (bitmap-slot duplicate guards).
pub(crate) const LABEL_TEXT_SLOT_COUNT: usize = 14;

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
            LabelTextSlot::StudioModel => "studio_model",
            LabelTextSlot::StudioSize => "studio_size",
            LabelTextSlot::StudioColor => "studio_color",
            LabelTextSlot::StudioPack => "studio_pack",
            LabelTextSlot::StudioPrice => "studio_price",
            LabelTextSlot::StudioHwName => "studio_hw_name",
            LabelTextSlot::StudioHwDim => "studio_hw_dim",
            LabelTextSlot::StudioHwPrice => "studio_hw_price",
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
    let width_dots = barcode_width_dots(barcode.chars().count()) as i64;
    Ok(LabelBarcodeGeometry {
        // Centered in the 2 mm safe box (16..304), never at x=8.
        x_dots: LABEL_SAFE_LEFT + centered_x(LABEL_SAFE_WIDTH, width_dots),
        // Price bottom (44+32=76) + 10 dots of scanner air.
        y_dots: 86,
        width_dots,
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
    // 2 mm safe box: x 16..304 (owner agreement L-margins). Every element
    // is centered; the die-cut edge never touches ink. Vertical budget
    // (y 16..144 = 128 dots): name 20 + 8 + price 32 + 10 + bars 38 + 8
    // + ref 12 = 128 — the box is exactly full, no room for a brand line
    // (agreement L1: brand dropped).
    let right_edge = LABEL_SAFE_LEFT + LABEL_SAFE_WIDTH;
    // Small-face budget inside the safe box (288 / 8).
    let small_chars = (LABEL_SAFE_WIDTH / FONT_1_W) as usize;
    // Name: F2, single centered line (agreement L4 — no vertical room for
    // a second row, so the overflow is cut at 24, documented).
    let name_chars = (LABEL_SAFE_WIDTH / FONT_2_W) as usize;
    // Price: F4 (agreement L2). Cap 9, not 7: a centered 9x24 row ends at
    // x=268, still 12 dots clear of DZD at x=280 — while a 7-cap would eat
    // the last digit of any price >= 10000.00 (deviation from the signed
    // draft, proven by the collision math, flagged in the report).
    let price_chars = 9usize;
    let mut plan = Vec::new();
    // truncate_to_width marks the cut with "..." (never a silent loss).
    let first = truncate_to_width(label.name.trim(), name_chars);
    plan.push(LabelTextLine {
        slot: LabelTextSlot::NameLine1,
        text: first.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(first.chars().count(), FONT_2_W, 1),
            ),
        y_dots: 16,
        cell_h_dots: FONT_2_H,
        box_w_dots: LABEL_SAFE_WIDTH,
        box_h_dots: FONT_2_H + LABEL_BITMAP_PAD_DOTS,
    });
    // Price F4 (4 mm — large, a step under the old 6 mm slab).
    let price = truncate_to_width(label.price_text.trim(), price_chars);
    plan.push(LabelTextLine {
        slot: LabelTextSlot::Price,
        text: price.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(price.chars().count(), FONT_4_W, 1),
            ),
        y_dots: 44,
        cell_h_dots: FONT_4_H,
        box_w_dots: LABEL_SAFE_WIDTH,
        box_h_dots: FONT_4_H + LABEL_BITMAP_PAD_DOTS,
    });
    let currency = truncate_to_width(label.currency_text.trim(), small_chars);
    if !currency.is_empty() {
        let x = right_x(right_edge, text_dots(currency.chars().count(), FONT_1_W, 1));
        plan.push(LabelTextLine {
            slot: LabelTextSlot::Currency,
            text: currency,
            x_dots: x,
            y_dots: 54,
            cell_h_dots: FONT_1_H,
            box_w_dots: i64::from(small_chars as u32) * FONT_1_W,
            box_h_dots: FONT_1_H + LABEL_BITMAP_PAD_DOTS,
        });
    }
    // Reference = the barcode payload, F1, centered above the bottom edge.
    let human = truncate_to_width(&sanitize_tspl_text(label.barcode), small_chars);
    plan.push(LabelTextLine {
        slot: LabelTextSlot::Human,
        text: human.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(human.chars().count(), FONT_1_W, 1),
            ),
        y_dots: 132,
        cell_h_dots: FONT_1_H,
        box_w_dots: i64::from(small_chars as u32) * FONT_1_W,
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
/// `pub(crate)`: shared with the studio templates.
pub(crate) fn check_label_text_printable(
    content: &str,
    probe: TsplProbeMode,
) -> Result<(), DomainError> {
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

/// Emit one planned TEXT line into the job — a `BITMAP` chunk when the
/// slot carries a raster (PRT-02 Arabic/CJK path, bottom-aligned to the
/// TEXT cell), otherwise the plain `TEXT` command. One emission path for
/// the classic and studio renderers (DSN-107): the preview, the plain
/// and the mixed jobs all share it, so byte-identity is structural.
/// `pub(crate)`: the studio templates call it per planned line.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_label_plan_line(
    job: &mut Vec<u8>,
    line: &LabelTextLine,
    font: &str,
    scale: i64,
    bitmaps: &[(LabelTextSlot, LabelBitmap)],
    probe: TsplProbeMode,
    dx: i64,
    dy: i64,
) -> Result<(), DomainError> {
    let invalid = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    if let Some((_, raster)) = bitmaps.iter().find(|(slot, _)| slot == &line.slot) {
        if raster.data.len() != raster.width_bytes * raster.height_rows {
            return Err(invalid("LABEL_BITMAP_LENGTH_MISMATCH"));
        }
        if raster.width_bytes == 0 || raster.height_rows == 0 {
            return Err(invalid("BITMAP_DIMENSIONS_MUST_BE_POSITIVE"));
        }
        if raster
            .width_bytes
            .checked_mul(8)
            .filter(|dots| *dots <= BITMAP_MAX_WIDTH_DOTS)
            .is_none()
        {
            return Err(invalid("BITMAP_WIDTH_TOO_WIDE"));
        }
        if raster.height_rows > BITMAP_MAX_HEIGHT_DOTS {
            return Err(invalid("BITMAP_TOO_TALL"));
        }
        if raster.data.iter().all(|byte| *byte == 0x00)
            || raster.data.iter().all(|byte| *byte == 0xFF)
        {
            return Err(invalid("BITMAP_BLANK_IMAGE"));
        }
        let width_dots =
            i64::try_from(raster.width_bytes * 8).map_err(|_| invalid("BITMAP_TOO_LARGE"))?;
        let height_dots =
            i64::try_from(raster.height_rows).map_err(|_| invalid("BITMAP_TOO_LARGE"))?;
        if width_dots > line.box_w_dots || height_dots > line.box_h_dots {
            return Err(invalid("LABEL_BITMAP_EXCEEDS_SLOT"));
        }
        // Bottom-align to the TEXT cell (taller rasters grow upward).
        let bitmap_y = line.y_dots + line.cell_h_dots - height_dots;
        let (sx, sby) = (line.x_dots + dx, bitmap_y + dy);
        if sx < 0
            || sby < 0
            || sx + width_dots > LABEL_WIDTH_DOTS
            || sby + height_dots > LABEL_HEIGHT_DOTS
        {
            return Err(invalid("LABEL_CALIBRATION_OUT_OF_AREA"));
        }
        job.extend_from_slice(
            format!(
                "BITMAP {},{},{},{},0,",
                sx, sby, raster.width_bytes, raster.height_rows
            )
            .as_bytes(),
        );
        job.extend_from_slice(&raster.data);
        job.extend_from_slice(b"\r\n");
    } else {
        check_label_text_printable(&line.text, probe)?;
        let mut text_out: Vec<String> = Vec::new();
        let (sx, sy) = (line.x_dots + dx, line.y_dots + dy);
        let width_dots = i64::try_from(line.text.chars().count()).unwrap_or(i64::MAX)
            * label_face_width_dots(font, scale);
        check_calibrated_text(sx, sy, width_dots, line.cell_h_dots)?;
        push_text(&mut text_out, sx, sy, font, scale, &line.text, probe);
        job.extend_from_slice(text_out.join("\r\n").as_bytes());
        job.extend_from_slice(b"\r\n");
    }
    Ok(())
}

/// Emit the centered Code128 symbol plus the `PRINT 1,n` trailer (one
/// path for classic + studio; the symbol always spools, decoupled from
/// any currency row). `pub(crate)`: the studio templates finish alike.
pub(crate) fn finish_label_job(
    job: &mut Vec<u8>,
    geometry: &LabelBarcodeGeometry,
    barcode_sanitized: &str,
    copies: u32,
    dx: i64,
    dy: i64,
) -> Result<(), DomainError> {
    let (sbx, sby) = (geometry.x_dots + dx, geometry.y_dots + dy);
    if sbx < 0
        || sby < 0
        || sbx + geometry.width_dots > LABEL_WIDTH_DOTS
        || sby + geometry.height_dots > LABEL_HEIGHT_DOTS
    {
        return Err(DomainError::InvalidCanonicalString(
            "LABEL_CALIBRATION_OUT_OF_AREA".to_string(),
        ));
    }
    job.extend_from_slice(
        format!(
            "BARCODE {sbx},{sby},\"128\",{},0,0,2,2,\"{barcode_sanitized}\"\r\n",
            geometry.height_dots
        )
        .as_bytes(),
    );
    job.extend_from_slice(format!("PRINT 1,{copies}\r\n").as_bytes());
    Ok(())
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
    let mut seen = [false; LABEL_TEXT_SLOT_COUNT];
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
    let probe = label.media.probe_mode;
    // DOC-06: software calibration — every emitted coordinate below is
    // shifted; zero keeps the goldens byte-identical.
    let (dx, dy) = label.media.calibration_dots();
    let mut head: Vec<String> = Vec::new();
    push_media_preamble(&mut head, "SIZE 40 mm, 20 mm", &label.media)?;
    let mut job: Vec<u8> = head.join("\r\n").into_bytes();
    job.extend_from_slice(b"\r\n");
    for line in &label_text_plan(label) {
        let (font, scale) = match line.slot {
            LabelTextSlot::Brand | LabelTextSlot::Currency | LabelTextSlot::Human => ("1", 1),
            LabelTextSlot::NameLine1 | LabelTextSlot::NameLine2 => ("2", 1),
            // Redesign price: native 24x32 face (agreement L2).
            LabelTextSlot::Price => ("4", 1),
            // Studio slots never occur in the classic plan — LOUD, never a
            // silent wrong face.
            _ => return Err(code("LABEL_UNKNOWN_SLOT")),
        };
        emit_label_plan_line(&mut job, line, font, scale, bitmaps, probe, dx, dy)?;
    }
    // The symbol always spools (agreement L5: centered, quiet-zoned,
    // decoupled from the currency row).
    finish_label_job(&mut job, &geometry, &barcode, label.copies, dx, dy)?;
    Ok(job)
}

/// Render the 40x20 mm calibration pattern (DOC-06): a ruler-readable
/// printout for the Printer page. Full-bleed border at the 320x160 media
/// bounds, 1 mm ticks along the top edge (8 dots = 1 mm at 203 dpi),
/// center crosshair, and self-documenting captions carrying the applied
/// correction (`X±nMM Y±mMM`). Every element shifts WITH the calibration
/// under test — the owner prints, measures the border inset with a ruler,
/// and enters the correction; the next printout proves it.
///
/// Deliberately unchecked against the media bounds (unlike labels): a
/// correction under test may push the border off-media, and that clip IS
/// the measurement signal. `media.validate()` still rejects out-of-range
/// calibration LOUDLY. ASCII-only payload by construction.
pub fn render_calibration_pattern_40x20(media: &TsplMediaSetup) -> Result<Vec<u8>, DomainError> {
    media.validate()?;
    let (dx, dy) = media.calibration_dots();
    let mut head: Vec<String> = Vec::new();
    push_media_preamble(&mut head, "SIZE 40 mm, 20 mm", media)?;
    let mut job: Vec<u8> = head.join("\r\n").into_bytes();
    job.extend_from_slice(b"\r\n");
    let mut emit = |line: String| {
        job.extend_from_slice(line.as_bytes());
        job.extend_from_slice(b"\r\n");
    };
    // Border at the media bounds (shifts with the calibration under test).
    emit(format!(
        "BOX {},{},{},{},2",
        dx,
        dy,
        LABEL_WIDTH_DOTS - 1 + dx,
        LABEL_HEIGHT_DOTS - 1 + dy
    ));
    // 1 mm ruler ticks along the top edge (long every 5 mm).
    for mm in 1..40 {
        let x = i64::from(mm) * TSPL_DOTS_PER_MM + dx;
        let h = if mm % 5 == 0 { 8 } else { 4 };
        emit(format!("BAR {x},{dy},1,{h}"));
    }
    // Center crosshair.
    emit(format!(
        "BOX {},{},{},{},1",
        159 + dx,
        dy,
        160 + dx,
        LABEL_HEIGHT_DOTS - 1 + dy
    ));
    emit(format!(
        "BOX {},{},{},{},1",
        dx,
        79 + dy,
        LABEL_WIDTH_DOTS - 1 + dx,
        80 + dy
    ));
    // Self-documenting captions (ASCII-only: the pattern spools anywhere).
    let mut text_out: Vec<String> = Vec::new();
    push_text(
        &mut text_out,
        8 + dx,
        16 + dy,
        "1",
        1,
        "CALIBRATION",
        media.probe_mode,
    );
    push_text(
        &mut text_out,
        8 + dx,
        40 + dy,
        "1",
        1,
        &format!("X{:+}MM Y{:+}MM", media.offset_x_mm, media.offset_y_mm),
        media.probe_mode,
    );
    push_text(
        &mut text_out,
        8 + dx,
        64 + dy,
        "1",
        1,
        "40X20 RULER=MM",
        media.probe_mode,
    );
    for line in text_out {
        emit(line);
    }
    job.extend_from_slice(b"PRINT 1,1\r\n");
    Ok(job)
}
