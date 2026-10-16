//! Bidi run splitting + rustybuzz shaping into positioned glyphs.
//!
//! Each logical line is split into visual directional runs (Unicode Bidi
//! Algorithm); each run is shaped with the script-appropriate embedded
//! face. Output runs are already in visual order with positive advances,
//! so the rasterizer lays them out left to right from a computed origin.

use baraka_domain::errors::DomainError;
use unicode_bidi::{BidiInfo, Level};

use crate::printing::font_cache;

/// True for characters shaped with the Arabic face.
fn is_arabic_char(value: char) -> bool {
    matches!(
        value,
        '\u{0590}'..='\u{08FF}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}'
    )
}

/// One shaped glyph with run-relative pen geometry (pixels).
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    /// Font glyph id (same numbering as the raster faces).
    pub glyph_id: u32,
    /// Byte index of the source character inside the shaped run.
    pub cluster: u32,
    /// True when shaped (and rasterized) with the Arabic face.
    pub arabic: bool,
    /// Pen advance in pixels (always non-negative here).
    pub x_advance_px: f32,
    /// Pen offset in pixels.
    pub x_offset_px: f32,
    /// Baseline offset in pixels (marks and diacritics).
    pub y_offset_px: f32,
    /// Pen position of this glyph relative to the run start.
    pub pen_x_px: f32,
}

/// One visual run: glyphs in display order plus metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRun {
    /// Source slice backing this run (cluster indices address it).
    pub text: String,
    /// Glyphs left to right as displayed.
    pub glyphs: Vec<ShapedGlyph>,
    /// True for right-to-left runs.
    pub is_rtl: bool,
    /// Total advance width in pixels.
    pub width_px: f32,
}

/// Paragraph base direction of a logical line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineDirection {
    /// Left-to-right layout from the paper's left edge.
    Ltr,
    /// Right-to-left layout from the paper's right edge.
    Rtl,
}

/// Shape one logical line into visual runs at `px_size` pixel em size.
///
/// Empty text yields zero runs (never an error, never a panic).
pub fn shape_line(
    text: &str,
    px_size: f32,
) -> Result<(LineDirection, Vec<ShapedRun>), DomainError> {
    if text.is_empty() {
        return Ok((LineDirection::Ltr, Vec::new()));
    }
    let info = BidiInfo::new(text, None);
    let mut runs_out = Vec::new();
    let mut base_rtl = false;
    for paragraph in &info.paragraphs {
        if paragraph.level.is_rtl() {
            base_rtl = true;
        }
        let line_range = paragraph.range.clone();
        // unicode-bidi 0.3 returns parallel (levels, byte-ranges) vectors.
        let (levels, ranges) = info.visual_runs(paragraph, line_range);
        for (level, range) in levels.into_iter().zip(ranges) {
            let run_text = text.get(range).ok_or_else(|| {
                DomainError::InvalidCanonicalString("line splits mid-character".to_string())
            })?;
            if run_text.is_empty() {
                continue;
            }
            let (glyphs, width_px, is_rtl) = shape_run(run_text, level, px_size)?;
            runs_out.push(ShapedRun {
                text: run_text.to_string(),
                glyphs,
                is_rtl,
                width_px,
            });
        }
    }
    Ok((
        if base_rtl {
            LineDirection::Rtl
        } else {
            LineDirection::Ltr
        },
        runs_out,
    ))
}

/// Shape a single directional run with the script-appropriate face.
///
/// Positions come back in 26.6 units at units-per-em scale (HarfBuzz
/// convention mirrored by rustybuzz): pixels = advance / 64 / upem * size.
/// Returns glyphs, total width, and the run direction flag.
fn shape_run(
    run_text: &str,
    level: Level,
    px_size: f32,
) -> Result<(Vec<ShapedGlyph>, f32, bool), DomainError> {
    let arabic = run_text.chars().any(is_arabic_char);
    let upem: f32 = if arabic {
        font_cache::noto_ttf_face().units_per_em() as f32
    } else {
        font_cache::inter_ttf_face().units_per_em() as f32
    };
    if upem <= 0.0 {
        return Err(DomainError::PrintRenderingFailed);
    }
    let face = if arabic {
        font_cache::noto_shaping_face()
    } else {
        font_cache::inter_shaping_face()
    };
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(run_text);
    let output = rustybuzz::shape(&face, &[], buffer);
    let infos = output.glyph_infos();
    let positions = output.glyph_positions();
    // rustybuzz reports advances in font units (verified against a 1000-upem
    // Noto face: `BEH`-class glyphs advance ~636 units), so one division by
    // upem yields pixels. Dividing again by 64 (the old code) compressed
    // every line ~64x, which silently defeated wrapping on narrow paper.
    let scale = px_size / upem;
    let mut glyphs = Vec::with_capacity(infos.len());
    let mut pen = 0.0f32;
    for (info, position) in infos.iter().zip(positions.iter()) {
        let advance = position.x_advance as f32 * scale;
        glyphs.push(ShapedGlyph {
            glyph_id: info.glyph_id,
            cluster: info.cluster,
            arabic,
            x_advance_px: if advance < 0.0 { 0.0 } else { advance },
            x_offset_px: position.x_offset as f32 * scale,
            y_offset_px: position.y_offset as f32 * scale,
            pen_x_px: pen,
        });
        pen += if advance < 0.0 { 0.0 } else { advance };
    }
    Ok((glyphs, pen, level.is_rtl()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_line_shapes_rtl_with_glyphs() {
        let (direction, runs) = shape_line("منتج تجريبي", 24.0).expect("shape");
        assert_eq!(direction, LineDirection::Rtl);
        assert!(!runs.is_empty());
        assert!(runs.iter().all(|run| run.width_px >= 0.0));
    }

    #[test]
    fn digits_inside_arabic_keep_ltr_order() {
        // Price embedded in an Arabic line: recover each glyph's source
        // character through its cluster byte index and assert the digit
        // sequence reads left to right ("12250"), never reversed.
        let (_, runs) = shape_line("المجموع: 122.50 د.ج", 24.0).expect("shape");
        let mut digits = String::new();
        for run in &runs {
            for glyph in &run.glyphs {
                let at = glyph.cluster as usize;
                if let Some(tail) = run.text.get(at..) {
                    if let Some(character) = tail.chars().next() {
                        if character.is_ascii_digit() {
                            digits.push(character);
                        }
                    }
                }
            }
        }
        assert_eq!(digits, "12250", "digit order in visual runs");
    }

    #[test]
    fn empty_text_never_fails() {
        let (direction, runs) = shape_line("", 24.0).expect("empty");
        assert_eq!(direction, LineDirection::Ltr);
        assert!(runs.is_empty());
    }
}
