//! ESC/POS raster renderer: layout + ab_glyph raster + `GS v 0` encoding.
//!
//! Canvas is 576 dots wide (80mm @ 203dpi). Bands carry at most 255 dot
//! rows each, per the ESC/POS raster graphics specification.

use ab_glyph::{Font, FontRef, Glyph, GlyphId, Point, PxScale};
use baraka_application::errors::ApplicationError;
use baraka_application::ports::ReceiptRendererPort;
use baraka_domain::errors::DomainError;
use baraka_domain::printing::{DebtReceiptDocument, ReceiptDocument};
use image::{GrayImage, Luma};

use crate::printing::blocks::{
    build_footer_lines, build_identity_lines, label, layout_text, LayoutLine,
};
use crate::printing::font_cache;
use crate::printing::shaping::LineDirection;

/// Paper width in dots (80mm @ 203dpi).
pub const PAPER_WIDTH_DOTS: u32 = 576;
/// Body text pixel em size.
pub const BODY_PX: f32 = 24.0;
/// Title pixel em size.
pub const TITLE_PX: f32 = 32.0;
/// Maximum raster band height per `GS v 0` command.
pub const MAX_BAND_HEIGHT: u32 = 255;
/// Side margin in dots.
pub const MARGIN_DOTS: f32 = 8.0;

/// Build the ordered logical lines of a receipt (content only, no drawing).
fn build_lines(receipt: &ReceiptDocument) -> Result<Vec<LayoutLine>, DomainError> {
    let language = receipt.language;
    let mut lines = Vec::new();
    if receipt.store_identity.store_name.trim().is_empty() {
        lines.push(layout_text(&receipt.shop_name, TITLE_PX, true)?);
    } else {
        lines.extend(build_identity_lines(&receipt.store_identity, language)?);
    }
    if receipt.is_duplicata {
        lines.push(layout_text(
            &label(language, "*** نسخة مكررة ***", "*** DUPLICATA ***"),
            TITLE_PX,
            true,
        )?);
    }
    lines.push(layout_text(
        &format!("{}  {}", receipt.number, receipt.created_at),
        BODY_PX,
        true,
    )?);
    lines.push(LayoutLine::Rule { height_px: 2.0 });
    for line in &receipt.lines {
        lines.push(layout_text(&line.designation, BODY_PX, false)?);
        lines.push(layout_text(
            &format!(
                "{} x {} = {}",
                line.quantity_text, line.unit_price_text, line.total_text
            ),
            BODY_PX,
            false,
        )?);
    }
    lines.push(LayoutLine::Rule { height_px: 2.0 });
    let totals: [(&str, &str, &str); 6] = [
        ("المجموع الفرعي", "Sous-total", &receipt.totals.subtotal_ht),
        ("التخفيض", "Remise", &receipt.totals.discount),
        ("الضريبة", "TVA", &receipt.totals.tva),
        ("الطابع", "Timbre", &receipt.totals.timbre),
        ("المجموع", "TOTAL", &receipt.totals.total_ttc),
        ("المدفوع", "Payé", &receipt.totals.paid),
    ];
    for (arabic, french, value) in totals {
        lines.push(layout_text(
            &format!("{}: {}", label(language, arabic, french), value),
            BODY_PX,
            false,
        )?);
    }
    if receipt.totals.change != "0.00" {
        lines.push(layout_text(
            &format!(
                "{}: {}",
                label(language, "الباقي", "Rendu"),
                receipt.totals.change
            ),
            BODY_PX,
            false,
        )?);
    }
    for (method, amount) in &receipt.payments {
        lines.push(layout_text(&format!("{method}: {amount}"), BODY_PX, false)?);
    }
    lines.push(LayoutLine::Rule { height_px: 2.0 });
    if receipt.footer_message.trim().is_empty() {
        lines.push(layout_text(
            &label(language, "شكرا لزيارتكم", "Merci de votre visite"),
            BODY_PX,
            true,
        )?);
    } else {
        lines.extend(build_footer_lines(&receipt.footer_message, language)?);
    }
    lines.push(LayoutLine::Gap { height_px: 24.0 });
    Ok(lines)
}

/// Draw one shaped run at `(origin_x, baseline_y)` onto the canvas.
fn draw_run(
    canvas: &mut GrayImage,
    run: &crate::printing::shaping::ShapedRun,
    origin_x: f32,
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
                x: origin_x + glyph.pen_x_px + glyph.x_offset_px,
                y: baseline_y + glyph.y_offset_px,
            },
        };
        let Some(outlined) = font.outline_glyph(glyph_id) else {
            continue;
        };
        outlined.draw(|dx, dy, coverage| {
            if coverage >= 0.5 {
                let bounds = outlined.px_bounds();
                let px_x = (bounds.min.x as i32) + dx as i32;
                let px_y = (bounds.min.y as i32) + dy as i32;
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

/// Per-line pixel size (titles render larger than body).
fn line_px(line: &LayoutLine) -> f32 {
    match line {
        LayoutLine::Text { px, .. } => *px,
        LayoutLine::Rule { .. } | LayoutLine::Gap { .. } => BODY_PX,
    }
}

/// Compose a monochrome canvas from pre-built logical lines.
///
/// Shared by the document renderer and the test-print diagnostics path:
/// both travel this exact function, so layout behavior cannot diverge.
pub fn render_lines(lines: &[LayoutLine]) -> Result<GrayImage, DomainError> {
    let mut heights: Vec<f32> = Vec::with_capacity(lines.len());
    let mut total: f32 = 0.0;
    for line in lines {
        let height = match line {
            LayoutLine::Text { height_px, .. } => *height_px,
            LayoutLine::Rule { height_px } | LayoutLine::Gap { height_px } => *height_px,
        };
        heights.push(height);
        total += height;
    }
    let canvas_height = total.ceil().max(1.0) as u32;
    let mut canvas = GrayImage::from_pixel(PAPER_WIDTH_DOTS, canvas_height, Luma([255u8]));
    let mut cursor_y = 0.0f32;
    for (line, height) in lines.iter().zip(heights.iter()) {
        match line {
            LayoutLine::Rule { height_px } => {
                let thickness = (*height_px as u32).max(1);
                let y0 = cursor_y as u32;
                for y in y0..(y0 + thickness).min(canvas_height) {
                    for x in 0..PAPER_WIDTH_DOTS {
                        canvas.put_pixel(x, y, Luma([0u8]));
                    }
                }
            }
            LayoutLine::Gap { .. } => {}
            LayoutLine::Text {
                runs,
                direction,
                centered,
                ascent_px,
                ..
            } => {
                let px = line_px(line);
                let content_width: f32 = runs.iter().map(|run| run.width_px).sum();
                let origin_x = if *centered {
                    ((PAPER_WIDTH_DOTS as f32 - content_width) / 2.0).max(MARGIN_DOTS)
                } else {
                    match direction {
                        LineDirection::Rtl => {
                            (PAPER_WIDTH_DOTS as f32 - MARGIN_DOTS - content_width).max(MARGIN_DOTS)
                        }
                        LineDirection::Ltr => MARGIN_DOTS,
                    }
                };
                let baseline_y = cursor_y + ascent_px;
                let mut pen_x = origin_x;
                for run in runs {
                    draw_run(&mut canvas, run, pen_x, baseline_y, px);
                    pen_x += run.width_px;
                }
            }
        }
        cursor_y += height;
    }
    Ok(canvas)
}

/// Compose the full monochrome canvas for a receipt.
pub fn render_canvas(receipt: &ReceiptDocument) -> Result<GrayImage, DomainError> {
    let lines = build_lines(receipt)?;
    render_lines(&lines)
}

/// Build the ordered logical lines of a debt-payment receipt.
///
/// Separate pipeline from [`build_lines`]: a credit payment is not a
/// sale, so sales-specific totals rows (HT/TVA/Timbre) never appear
/// here. Only display-ready strings from [`DebtReceiptDocument`] are
/// transcribed; no financial computation happens in this function.
fn build_debt_receipt_lines(receipt: &DebtReceiptDocument) -> Result<Vec<LayoutLine>, DomainError> {
    let language = receipt.language;
    let mut lines = Vec::new();
    if receipt.store_identity.store_name.trim().is_empty() {
        lines.push(layout_text(&receipt.shop_name, TITLE_PX, true)?);
    } else {
        lines.extend(build_identity_lines(&receipt.store_identity, language)?);
    }
    lines.push(layout_text(
        &label(language, "وصل تسديد دَين", "Reçu de paiement"),
        TITLE_PX,
        true,
    )?);
    lines.push(layout_text(
        &format!("{}  {}", receipt.receipt_number, receipt.created_at),
        BODY_PX,
        true,
    )?);
    lines.push(LayoutLine::Rule { height_px: 2.0 });
    lines.push(layout_text(
        &format!(
            "{}: {}",
            label(language, "الزبون", "Client"),
            receipt.customer_name
        ),
        BODY_PX,
        false,
    )?);
    lines.push(layout_text(
        &format!(
            "{}: {}",
            label(language, "المبلغ المحصَّل", "Montant encaissé"),
            receipt.amount_text
        ),
        BODY_PX,
        false,
    )?);
    lines.push(layout_text(
        &format!(
            "{}: {}",
            label(language, "الرصيد الجديد", "Nouveau solde"),
            receipt.new_balance_text
        ),
        BODY_PX,
        false,
    )?);
    if let Some(reference) = receipt.reference_text.as_deref() {
        if !reference.trim().is_empty() {
            lines.push(layout_text(
                &format!("{}: {}", label(language, "المرجع", "Référence"), reference),
                BODY_PX,
                false,
            )?);
        }
    }
    lines.push(LayoutLine::Rule { height_px: 2.0 });
    if receipt.footer_message.trim().is_empty() {
        lines.push(layout_text(
            &label(language, "شكرا لثقتكم", "Merci de votre confiance"),
            BODY_PX,
            true,
        )?);
    } else {
        lines.extend(build_footer_lines(&receipt.footer_message, language)?);
    }
    lines.push(LayoutLine::Gap { height_px: 24.0 });
    Ok(lines)
}

/// Compose the full monochrome canvas for a debt-payment receipt.
///
/// Shares the single pixel-drawing core [`render_lines`] with the sales
/// path, so layout behavior cannot diverge between the two pipelines.
pub fn render_debt_canvas(receipt: &DebtReceiptDocument) -> Result<GrayImage, DomainError> {
    let lines = build_debt_receipt_lines(receipt)?;
    render_lines(&lines)
}

/// Build the ordered logical lines of a receipt (shared with test print).
pub fn receipt_lines(receipt: &ReceiptDocument) -> Result<Vec<LayoutLine>, DomainError> {
    build_lines(receipt)
}

/// Encode a monochrome canvas as `GS v 0` raster bands + feed.
///
/// Bit 1 = printed dot, MSB-first, row-major. Bands carry at most 255 rows.
pub fn encode_gs_v0(canvas: &GrayImage) -> Vec<u8> {
    let width = canvas.width();
    let height = canvas.height();
    let row_bytes = width.div_ceil(8);
    let mut out = Vec::new();
    let mut y = 0u32;
    while y < height {
        let band = (height - y).min(MAX_BAND_HEIGHT);
        out.extend_from_slice(&[0x1D, 0x76, 0x30, 0x00]);
        out.push((row_bytes & 0xFF) as u8);
        out.push(((row_bytes >> 8) & 0xFF) as u8);
        out.push((band & 0xFF) as u8);
        out.push(((band >> 8) & 0xFF) as u8);
        for row in y..(y + band) {
            for byte_index in 0..row_bytes {
                let mut byte = 0u8;
                for bit in 0..8 {
                    let x = byte_index * 8 + bit;
                    let dark = x < width && canvas.get_pixel(x, row).0[0] < 128;
                    if dark {
                        byte |= 0x80 >> bit;
                    }
                }
                out.push(byte);
            }
        }
        y += band;
    }
    out.extend_from_slice(&[0x0A, 0x0A, 0x0A]);
    out
}

/// Receipt raster renderer (ESC/POS transport edge).
pub struct EscPosRasterizer;

impl ReceiptRendererPort for EscPosRasterizer {
    fn render(&self, receipt: &ReceiptDocument) -> Result<Vec<u8>, ApplicationError> {
        render_canvas(receipt)
            .map(|canvas| encode_gs_v0(&canvas))
            .map_err(|_| ApplicationError::Validation(DomainError::PrintRenderingFailed))
    }

    fn render_debt_receipt(
        &self,
        receipt: &baraka_domain::printing::DebtReceiptDocument,
    ) -> Result<Vec<u8>, ApplicationError> {
        render_debt_canvas(receipt)
            .map(|canvas| encode_gs_v0(&canvas))
            .map_err(|_| ApplicationError::Validation(DomainError::PrintRenderingFailed))
    }

    fn preview_lines(
        &self,
        receipt: &ReceiptDocument,
    ) -> Result<Vec<baraka_domain::printing::PreviewLine>, ApplicationError> {
        use baraka_domain::printing::PreviewLine;
        build_lines(receipt)
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| match line {
                        LayoutLine::Text {
                            runs, centered, px, ..
                        } => PreviewLine {
                            kind: "text".to_string(),
                            centered: *centered,
                            size_px: *px,
                            text: runs
                                .iter()
                                .map(|run| run.text.as_str())
                                .collect::<Vec<_>>()
                                .join(""),
                        },
                        LayoutLine::Rule { .. } => PreviewLine {
                            kind: "rule".to_string(),
                            centered: true,
                            size_px: 0.0,
                            text: String::new(),
                        },
                        LayoutLine::Gap { .. } => PreviewLine {
                            kind: "gap".to_string(),
                            centered: true,
                            size_px: 0.0,
                            text: String::new(),
                        },
                    })
                    .collect()
            })
            .map_err(|_| ApplicationError::Validation(DomainError::PrintRenderingFailed))
    }
}
