//! Shared ESC/POS template blocks: store identity header + ticket footer.
//!
//! These constructors are the SINGLE composition path (duplication
//! forbidden): the document renderer (`build_lines`) and the test-print
//! diagnostics command both consume them, so a golden test can prove
//! byte-identical header/footer blocks for the same settings. Identity
//! VALUES print as stored; Arabic values flow through the existing
//! glyph-shaping path via `layout_text`. Captions for the phone line
//! follow the document language (Baseline §17); identifier captions
//! stay Latin (NIF / NIS / RC / AI) on both languages, as on Algerian
//! documents. Empty optional fields are omitted (no blank caption
//! lines). Identifier FORMAT enforcement is deferred to ADR-013.

use baraka_domain::errors::DomainError;
use baraka_domain::printing::{ReceiptLanguage, StoreIdentity};

use crate::printing::font_cache;
use crate::printing::shaping::{LineDirection, ShapedRun};

/// Emphasis size for the store name (existing renderer mechanism — the
/// title size; no new rendering primitives introduced).
pub const STORE_NAME_PX: f32 = super::rasterizer::TITLE_PX;
/// Body size for identity detail lines and the footer.
pub const BLOCK_BODY_PX: f32 = super::rasterizer::BODY_PX;

/// One laid-out logical line.
///
/// Public so golden tests can assert block structure (run texts,
/// centering, rule placement) without rasterizing.
#[derive(Debug, Clone)]
pub enum LayoutLine {
    /// Shaped text with direction and vertical metrics.
    Text {
        /// Visual runs already ordered.
        runs: Vec<ShapedRun>,
        /// Base direction of the paragraph.
        direction: LineDirection,
        /// Centered (headers) or edge-aligned (body).
        centered: bool,
        /// Pixel em size used for shaping.
        px: f32,
        /// Ascent in pixels (max over runs).
        ascent_px: f32,
        /// Full line height in pixels.
        height_px: f32,
    },
    /// Full-width rule bar of `height_px` dots.
    Rule {
        /// Bar thickness in dots.
        height_px: f32,
    },
    /// Blank spacer of `height_px` dots.
    Gap {
        /// Spacer height in dots.
        height_px: f32,
    },
}

fn ascent_of(arabic: bool, px: f32) -> f32 {
    let font = if arabic {
        font_cache::noto_raster_font()
    } else {
        font_cache::inter_raster_font()
    };
    use ab_glyph::PxScale;
    use ab_glyph::{Font, ScaleFont};
    font.as_scaled(PxScale::from(px)).ascent()
}

fn line_height_of(arabic: bool, px: f32) -> f32 {
    let font = if arabic {
        font_cache::noto_raster_font()
    } else {
        font_cache::inter_raster_font()
    };
    use ab_glyph::PxScale;
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(PxScale::from(px));
    (scaled.ascent() - scaled.descent() + scaled.line_gap()).max(px) + 6.0
}

/// Shape one non-empty fragment (single visual line, no wrapping).
fn shape_single(text: &str, px: f32, centered: bool) -> Result<LayoutLine, DomainError> {
    let (direction, runs) = crate::printing::shaping::shape_line(text, px)?;
    let arabic = runs
        .iter()
        .any(|run| run.glyphs.iter().any(|glyph| glyph.arabic));
    Ok(LayoutLine::Text {
        runs,
        direction,
        centered,
        px,
        ascent_px: ascent_of(arabic, px),
        height_px: line_height_of(arabic, px),
    })
}

/// Line width in dots of an already-shaped logical line.
fn shaped_width(line: &LayoutLine) -> f32 {
    match line {
        LayoutLine::Text { runs, .. } => runs.iter().map(|run| run.width_px).sum(),
        _ => 0.0,
    }
}

/// Shape logical lines wrapped to `max_content_dots` (TASK-BRK-070).
///
/// Short text yields exactly one line, identical to [`layout_text`].
/// Overflow wraps on whitespace boundaries by re-shaping word groups, so
/// Arabic intra-word ligatures are never split. A single word wider than
/// the canvas is emitted alone (clipped downstream exactly as before —
/// graceful, never a panic).
pub(crate) fn layout_lines(
    text: &str,
    px: f32,
    centered: bool,
    max_content_dots: f32,
) -> Result<Vec<LayoutLine>, DomainError> {
    if text.is_empty() {
        return Ok(vec![LayoutLine::Gap {
            height_px: line_height_of(false, px),
        }]);
    }
    let whole = shape_single(text, px, centered)?;
    if shaped_width(&whole) <= max_content_dots {
        return Ok(vec![whole]);
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut pending: Option<String> = None;
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        let probe = shape_single(&candidate, px, centered)?;
        if shaped_width(&probe) <= max_content_dots {
            current = candidate;
        } else {
            if !current.is_empty() {
                out.push(shape_single(&current, px, centered)?);
            } else {
                pending = Some(candidate);
                break;
            }
            current = word.to_string();
        }
    }
    if let Some(oversize) = pending {
        out.push(shape_single(&oversize, px, centered)?);
    } else if !current.is_empty() {
        out.push(shape_single(&current, px, centered)?);
    }
    if out.is_empty() {
        out.push(whole);
    }
    Ok(out)
}

/// Bilingual label pair selected by receipt language.
pub(crate) fn label(language: ReceiptLanguage, arabic: &str, french: &str) -> String {
    match language {
        ReceiptLanguage::Arabic => arabic.to_string(),
        ReceiptLanguage::French => french.to_string(),
    }
}

/// Identity header block (top of every ESC/POS document).
///
/// Layout: store name (emphasis size, centered) → address (if present)
/// → localized phone line (if present) → one line per present
/// identifier → separator rule in the template style.
pub fn build_identity_lines(
    identity: &StoreIdentity,
    language: ReceiptLanguage,
    max_content_dots: f32,
) -> Result<Vec<LayoutLine>, DomainError> {
    let mut lines = Vec::new();
    lines.extend(layout_lines(
        &identity.store_name,
        STORE_NAME_PX,
        true,
        max_content_dots,
    )?);
    if !identity.store_address.is_empty() {
        lines.extend(layout_lines(
            &identity.store_address,
            BLOCK_BODY_PX,
            true,
            max_content_dots,
        )?);
    }
    if !identity.store_phone.is_empty() {
        lines.extend(layout_lines(
            &format!(
                "{}: {}",
                label(language, "هاتف", "Tél"),
                identity.store_phone
            ),
            BLOCK_BODY_PX,
            true,
            max_content_dots,
        )?);
    }
    for (caption, value) in [
        ("NIF", identity.nif.as_str()),
        ("NIS", identity.nis.as_str()),
        ("RC", identity.rc.as_str()),
        ("AI", identity.ai.as_str()),
    ] {
        if !value.is_empty() {
            lines.extend(layout_lines(
                &format!("{caption}: {value}"),
                BLOCK_BODY_PX,
                true,
                max_content_dots,
            )?);
        }
    }
    lines.push(LayoutLine::Rule { height_px: 2.0 });
    Ok(lines)
}

/// Footer block (after totals, before cut): separator + centered
/// message lines (up to 3 `\n`-separated lines; extras truncated).
pub fn build_footer_lines(
    message: &str,
    _language: ReceiptLanguage,
    max_content_dots: f32,
) -> Result<Vec<LayoutLine>, DomainError> {
    let mut lines = vec![LayoutLine::Rule { height_px: 2.0 }];
    for line in message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(baraka_domain::printing::identity_caps::FOOTER_LINES)
    {
        lines.extend(layout_lines(line, BLOCK_BODY_PX, true, max_content_dots)?);
    }
    Ok(lines)
}
