//! TSPL label line rasterer (PRT-02): Arabic TEXT slots → 1-bit strips.
//!
//! The XP-410B firmware silently eats whole jobs carrying Arabic TEXT
//! (no Arabic glyphs in its built-in fonts), while French accents ride
//! `CODEPAGE UTF-8` fine. So every non-Latin label line travels as a
//! `BITMAP` chunk: shaped with the receipt pipeline's Arabic face
//! (rustybuzz + Noto), drawn with the receipt threshold (coverage ≥ 0.5),
//! trimmed to ink, and shrink-to-fit into the TEXT cell box — never
//! clipped, never silent.

use ab_glyph::{Font, FontRef, Glyph, GlyphId, Point, PxScale};
use baraka_application::errors::ApplicationError;
use baraka_application::ports::{RasterLabelLine, TsplLabelRasterPort};
use image::{GrayImage, Luma};

use crate::printing::font_cache;
use crate::printing::shaping::shape_line;

/// Ink threshold: coverage ≥ 0.5 becomes a black dot (receipt parity).
const INK_COVERAGE: f32 = 0.5;
/// Shrink-to-fit floor: below 8 px Arabic marks dissolve — fail LOUDLY.
const MIN_PX: f32 = 8.0;
/// Shrink step in pixels.
const SHRINK_STEP: f32 = 1.0;
/// Canvas headroom multiplier (Arabic stacks never reach 2.5 em).
const CANVAS_EM: f32 = 2.5;
/// Baseline position inside the scratch canvas, in em.
const BASELINE_EM: f32 = 1.7;

/// Arabic-capable label line rasterer (production port).
pub struct TsplLabelRasterer;

impl TsplLabelRasterPort for TsplLabelRasterer {
    fn rasterize_label_text(
        &self,
        text: &str,
        px: f32,
        max_width_dots: u32,
        max_height_dots: u32,
    ) -> Result<RasterLabelLine, ApplicationError> {
        if max_width_dots == 0 || max_height_dots == 0 {
            return Err(ApplicationError::Port(
                "TSPL_LABEL_BOX_MUST_BE_POSITIVE".to_string(),
            ));
        }
        let mut size = px;
        loop {
            if size < MIN_PX {
                return Err(ApplicationError::Port("TSPL_LABEL_LINE_UNFIT".to_string()));
            }
            if let Some(line) = try_raster(text, size, max_width_dots, max_height_dots)? {
                return Ok(line);
            }
            size -= SHRINK_STEP;
        }
    }
}

/// Shape + draw `text` at `size` px; `Ok(None)` = exceeds the box (caller
/// shrinks and retries — content is never clipped).
fn try_raster(
    text: &str,
    size: f32,
    max_width_dots: u32,
    max_height_dots: u32,
) -> Result<Option<RasterLabelLine>, ApplicationError> {
    let (_direction, runs) = shape_line(text, size).map_err(ApplicationError::Validation)?;
    let natural_width: f32 = runs.iter().map(|run| run.width_px).sum();
    if natural_width <= 0.0 || natural_width.ceil() as u32 > max_width_dots {
        return Ok(None);
    }
    let width = natural_width.ceil().max(1.0) as u32;
    let height = (size * CANVAS_EM).ceil().max(1.0) as u32;
    let mut canvas = GrayImage::from_pixel(width, height, Luma([255u8]));
    let baseline_y = size * BASELINE_EM;
    let mut pen_x = 0.0f32;
    for run in &runs {
        draw_strip_run(&mut canvas, run, pen_x, baseline_y, size);
        pen_x += run.width_px;
    }
    // Trim all-white borders (top/bottom/left/right) — the domain
    // bottom-aligns the tight ink box to the TEXT cell.
    let (mut top, mut bottom, mut left, mut right) = (height, 0u32, width, 0u32);
    for (x, y, pixel) in canvas.enumerate_pixels() {
        if pixel[0] < 255 {
            top = top.min(y);
            bottom = bottom.max(y);
            left = left.min(x);
            right = right.max(x);
        }
    }
    if bottom < top || right < left {
        return Err(ApplicationError::Port(
            "TSPL_LABEL_BLANK_RASTER".to_string(),
        ));
    }
    let trimmed_w = right - left + 1;
    let trimmed_h = bottom - top + 1;
    if trimmed_h > max_height_dots {
        return Ok(None);
    }
    let width_bytes = trimmed_w.div_ceil(8) as usize;
    // Pad bits init to WHITE (0xFF): every row is width_bytes*8 wide but
    // only trimmed_w columns carry ink — with TSPL polarity (1 = white)
    // a 0x00 init would burn the padding as a solid black bar whenever
    // trimmed_w is not a multiple of 8 (photo-verified LIVE-2026-11-06:
    // a 7-dot phantom bar glued to the Arabic word's right edge).
    let mut data = vec![0xFFu8; width_bytes * trimmed_h as usize];
    for y in 0..trimmed_h {
        for x in 0..trimmed_w {
            // TSPL polarity (photo-verified LIVE-2026-11-06): bit 1 = white
            // (no dot), bit 0 = black — the inverse of the ESC/POS raster
            // path, whose `GS v 0` burns bit 1 as black. Ink pixels clear
            // their bit out of the 0xFF-white init (padding stays white).
            if canvas.get_pixel(left + x, top + y)[0] < 255 {
                data[(y as usize) * width_bytes + (x as usize) / 8] &= !(0x80 >> (x % 8));
            }
        }
    }
    // A fully-blank strip means an infrastructure bug — the domain
    // rejects it, but never send one by construction (blank = all WHITE,
    // i.e. 0xFF under TSPL polarity).
    if data.iter().all(|byte| *byte == 0xFF) {
        return Err(ApplicationError::Port(
            "TSPL_LABEL_BLANK_RASTER".to_string(),
        ));
    }
    Ok(Some(RasterLabelLine {
        width_bytes,
        height_rows: trimmed_h as usize,
        data,
    }))
}

/// Draw one shaped run at `(pen_x, baseline_y)` onto the strip canvas —
/// same per-glyph pipeline as the receipt `draw_run` (face select +
/// 0.5 threshold + bounds clip), minus margins/centering.
fn draw_strip_run(
    canvas: &mut GrayImage,
    run: &crate::printing::shaping::ShapedRun,
    pen_x: f32,
    baseline_y: f32,
    px: f32,
) {
    for glyph in &run.glyphs {
        let font: FontRef<'static> = if glyph.arabic {
            font_cache::noto_raster_font()
        } else {
            font_cache::inter_raster_font()
        };
        let glyph_id = Glyph {
            id: GlyphId(glyph.glyph_id as u16),
            scale: PxScale::from(px),
            position: Point {
                x: pen_x + glyph.pen_x_px + glyph.x_offset_px,
                y: baseline_y + glyph.y_offset_px,
            },
        };
        let Some(outlined) = font.outline_glyph(glyph_id) else {
            continue;
        };
        outlined.draw(|dx, dy, coverage| {
            if coverage >= INK_COVERAGE {
                let bounds = outlined.px_bounds();
                let px_x = bounds.min.x as i32 + dx as i32;
                let px_y = bounds.min.y as i32 + dy as i32;
                if px_x >= 0
                    && px_y >= 0
                    && (px_x as u32) < canvas.width()
                    && (px_y as u32) < canvas.height()
                {
                    canvas.put_pixel(px_x as u32, px_y as u32, Luma([0u8]));
                }
            }
        });
    }
}
