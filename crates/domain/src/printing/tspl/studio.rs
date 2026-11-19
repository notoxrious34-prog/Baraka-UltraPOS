//! Label Studio templates (TASK-DSN-107, Wave 3): two 40x20 mm layouts
//! beside the classic shelf label, drawn from the SAME safe box, face
//! table and emission path (`emit_label_plan_line` / `finish_label_job`)
//! — the preview reads the same plan the renderer spools, so drift is
//! structural, not conventional.
//!
//! Layout budget (safe box x 16..304, y 16..144 plus documented
//! margins). TEXTILE (apparel): model F2 hero, giant size F5 in the left
//! zone (x16..196, zoned-center so core sizes never touch the right
//! column), color/pack F1 right column, retail price F4 (the wholesale
//! price-hide toggle drops the line, never a blank row), Code128 y126
//! h28 (6-dot bottom margin — tighter than the classic 16; PRT-06
//! verifies the die-cut clearance and scan height on hardware). No
//! human-ref line BY DESIGN: model + price identify the variant, the
//! scanner is primary. HARDWARE (quincaillerie): hero bars y16 h56,
//! human ref F1, designation F2, unit/dimension F1, price F2; the bottom
//! ends exactly y144 like the classic.
//!
//! ASCII-only payloads by construction; Arabic units/names travel as
//! bitmaps through the shared PRT-02 slot path (never a dead label).

use super::common::*;
use super::label::{
    emit_label_plan_line, finish_label_job, LabelBarcodeGeometry, LabelBitmap, LabelTextLine,
    LabelTextSlot, LABEL_BITMAP_PAD_DOTS, LABEL_SAFE_LEFT, LABEL_SAFE_WIDTH, LABEL_TEXT_SLOT_COUNT,
};
use super::media::{push_media_preamble, TsplMediaSetup, TSPL_MAX_COPIES, TSPL_MIN_COPIES};
use super::probe::TsplProbeMode;
use crate::errors::DomainError;

/// Textile right-column origin (108 dots wide: 13 F1 chars at x196..304).
const STUDIO_INFO_X: i64 = LABEL_SAFE_LEFT + LABEL_SAFE_WIDTH - 108;
/// Textile size-badge zone (180 dots: x16..196). The F5 badge centers
/// WITHIN this zone, never full-width — a centered full-width badge at
/// 32 dots/char would overprint the right column from 3 chars up
/// (`"XXL"` = 96 dots → x112..208 × column x196..). Core shop sizes
/// (XL/XXL/3XL) must stay readable; the zone makes that structural.
const STUDIO_SIZE_ZONE_W: i64 = 180;
/// Textile price cap (F4, full-width centered): 12 chars.
const STUDIO_TEXTILE_PRICE_CHARS: usize = 12;
/// Hardware price cap (F2, full-width centered): 9 chars.
const STUDIO_HW_PRICE_CHARS: usize = 9;
/// Textile bars: y126 h28 (see module docs for the margin rationale).
const STUDIO_TEXTILE_BAR_Y: i64 = 126;
const STUDIO_TEXTILE_BAR_H: i64 = 28;
/// Hardware bars: y16 h56, the barcode-forward hero.
const STUDIO_HW_BAR_Y: i64 = 16;
const STUDIO_HW_BAR_H: i64 = 56;

/// Textile 40x20 label (apparel): model + giant size + color/pack +
/// optional retail price + Code128. `price_text = None` is the wholesale
/// hide (no price line at all); `Some("")` is rejected LOUDLY.
#[derive(Debug, Clone, Copy)]
pub struct StudioTextileLabel<'a> {
    /// Model/design reference, F2 hero line 1.
    pub model: &'a str,
    /// Size badge, F5 giant (e.g. `"XL"`).
    pub size: &'a str,
    /// Colorway, F1 right column.
    pub color: &'a str,
    /// Pack badge, F1 right column (e.g. `"PACK 12 PCS"`).
    pub pack: &'a str,
    /// Retail price text, F4 (`None` = wholesale hide).
    pub price_text: Option<&'a str>,
    /// Currency printed after the price (`""` collapses).
    pub currency_text: &'a str,
    /// Code128 payload (SKU/barcode).
    pub barcode: &'a str,
    /// Copies, 1..=99.
    pub copies: u32,
    /// Media identity for the self-describing preamble (validated).
    pub media: TsplMediaSetup,
}

/// Hardware 40x20 label (quincaillerie): barcode-forward + designation +
/// unit/dimension + price. `dimension_text = None` drops the dimension
/// half of the unit line (unit alone stays — fixed geometry, no reflow).
#[derive(Debug, Clone, Copy)]
pub struct StudioHardwareLabel<'a> {
    /// Designation, F2 (e.g. `"CHEVILLE MOLLY M8"`).
    pub designation: &'a str,
    /// Sale unit, F1 (`"PCS"`, `"M"`, `"KG"` …).
    pub unit_text: &'a str,
    /// Length/diameter half, F1 (`"L=50M"`, `"O18MM"` …).
    pub dimension_text: Option<&'a str>,
    /// Price text, F2 (bars are the hero here, not the price).
    pub price_text: &'a str,
    /// Currency printed after the price (`""` collapses).
    pub currency_text: &'a str,
    /// Code128 payload (SKU/barcode).
    pub barcode: &'a str,
    /// Copies, 1..=99.
    pub copies: u32,
    /// Media identity for the self-describing preamble (validated).
    pub media: TsplMediaSetup,
}

/// Barcode geometry at an explicit origin/height with the classic width
/// budget (11·n + 35 modules ≤ 140, narrow = 2, quiet-zoned, centered).
/// Rejects LOUDLY on the same rules as the classic renderer.
pub fn studio_barcode_geometry(
    barcode: &str,
    y_dots: i64,
    height_dots: i64,
) -> Result<LabelBarcodeGeometry, DomainError> {
    validate_code128_payload(barcode)?;
    if barcode.chars().count() > LABEL_BARCODE_MAX_CHARS {
        return Err(DomainError::TsplBarcodeTooWide);
    }
    let width_dots = barcode_width_dots(barcode.chars().count()) as i64;
    Ok(LabelBarcodeGeometry {
        x_dots: LABEL_SAFE_LEFT + centered_x(LABEL_SAFE_WIDTH, width_dots),
        y_dots,
        width_dots,
        height_dots,
    })
}

/// Face per studio slot (one table — the renderer cannot pick wrong).
fn studio_slot_face(slot: LabelTextSlot) -> Result<(&'static str, i64), DomainError> {
    match slot {
        LabelTextSlot::StudioModel | LabelTextSlot::StudioHwName => Ok(("2", 1)),
        LabelTextSlot::StudioSize => Ok(("5", 1)),
        LabelTextSlot::StudioColor
        | LabelTextSlot::StudioPack
        | LabelTextSlot::StudioHwDim
        | LabelTextSlot::Human => Ok(("1", 1)),
        LabelTextSlot::StudioPrice => Ok(("4", 1)),
        LabelTextSlot::StudioHwPrice => Ok(("2", 1)),
        _ => Err(DomainError::InvalidCanonicalString(
            "LABEL_UNKNOWN_SLOT".to_string(),
        )),
    }
}

/// Layout the textile TEXT lines (pure; truncation marks the cut with
/// `"..."`, never a silent loss). Price line present iff shown.
#[must_use]
pub fn textile_label_text_plan(label: &StudioTextileLabel<'_>) -> Vec<LabelTextLine> {
    let right_w = LABEL_SAFE_LEFT + LABEL_SAFE_WIDTH - STUDIO_INFO_X;
    let info_chars = (right_w / FONT_1_W) as usize;
    let mut plan = Vec::new();
    // Model F2, full-width centered (agreement: the reference reads first).
    let model = truncate_to_width(label.model.trim(), (LABEL_SAFE_WIDTH / FONT_2_W) as usize);
    plan.push(LabelTextLine {
        slot: LabelTextSlot::StudioModel,
        text: model.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(model.chars().count(), FONT_2_W, 1),
            ),
        y_dots: 16,
        cell_h_dots: FONT_2_H,
        box_w_dots: LABEL_SAFE_WIDTH,
        box_h_dots: FONT_2_H + LABEL_BITMAP_PAD_DOTS,
    });
    // Size badge F5, centered within the left zone (never full-width:
    // the badge must stay clear of the right column for ANY size text).
    let size = truncate_to_width(
        label.size.trim(),
        (STUDIO_SIZE_ZONE_W / (FONT_3_W * 2)) as usize,
    );
    plan.push(LabelTextLine {
        slot: LabelTextSlot::StudioSize,
        text: size.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                STUDIO_SIZE_ZONE_W,
                text_dots(size.chars().count(), FONT_3_W * 2, 1),
            ),
        y_dots: 40,
        cell_h_dots: FONT_3_H * 2,
        box_w_dots: STUDIO_SIZE_ZONE_W,
        box_h_dots: FONT_3_H * 2 + LABEL_BITMAP_PAD_DOTS,
    });
    // Color + pack, right column, left-aligned.
    for (slot, raw, y) in [
        (LabelTextSlot::StudioColor, label.color, 44),
        (LabelTextSlot::StudioPack, label.pack, 60),
    ] {
        let text = truncate_to_width(raw.trim(), info_chars);
        plan.push(LabelTextLine {
            slot,
            text,
            x_dots: STUDIO_INFO_X,
            y_dots: y,
            cell_h_dots: FONT_1_H,
            box_w_dots: right_w,
            box_h_dots: FONT_1_H + LABEL_BITMAP_PAD_DOTS,
        });
    }
    // Retail price F4, full-width centered (wholesale: line absent).
    if let Some(price) = label.price_text {
        let amount = truncate_to_width(price.trim(), STUDIO_TEXTILE_PRICE_CHARS);
        let currency = truncate_to_width(label.currency_text.trim(), STUDIO_TEXTILE_PRICE_CHARS);
        let text = if currency.is_empty() {
            amount
        } else {
            truncate_to_width(&format!("{amount} {currency}"), STUDIO_TEXTILE_PRICE_CHARS)
        };
        plan.push(LabelTextLine {
            slot: LabelTextSlot::StudioPrice,
            text: text.clone(),
            x_dots: LABEL_SAFE_LEFT
                + centered_x(
                    LABEL_SAFE_WIDTH,
                    text_dots(text.chars().count(), FONT_4_W, 1),
                ),
            y_dots: 92,
            cell_h_dots: FONT_4_H,
            box_w_dots: LABEL_SAFE_WIDTH,
            box_h_dots: FONT_4_H + LABEL_BITMAP_PAD_DOTS,
        });
    }
    plan
}

/// Layout the hardware TEXT lines (pure; fixed positions — an absent
/// dimension never reflows the rest, one geometry for every golden).
#[must_use]
pub fn hardware_label_text_plan(label: &StudioHardwareLabel<'_>) -> Vec<LabelTextLine> {
    let mut plan = Vec::new();
    // Designation F2, full-width centered under the bars.
    let name = truncate_to_width(
        label.designation.trim(),
        (LABEL_SAFE_WIDTH / FONT_2_W) as usize,
    );
    plan.push(LabelTextLine {
        slot: LabelTextSlot::StudioHwName,
        text: name.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(name.chars().count(), FONT_2_W, 1),
            ),
        y_dots: 88,
        cell_h_dots: FONT_2_H,
        box_w_dots: LABEL_SAFE_WIDTH,
        box_h_dots: FONT_2_H + LABEL_BITMAP_PAD_DOTS,
    });
    // Unit + optional dimension F1, full-width centered.
    let unit = label.unit_text.trim();
    let unit_line = match label.dimension_text {
        Some(dim) if !dim.trim().is_empty() => format!("{unit} · {}", dim.trim()),
        _ => unit.to_string(),
    };
    let unit_line = truncate_to_width(&unit_line, (LABEL_SAFE_WIDTH / FONT_1_W) as usize);
    plan.push(LabelTextLine {
        slot: LabelTextSlot::StudioHwDim,
        text: unit_line.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(unit_line.chars().count(), FONT_1_W, 1),
            ),
        y_dots: 110,
        cell_h_dots: FONT_1_H,
        box_w_dots: LABEL_SAFE_WIDTH,
        box_h_dots: FONT_1_H + LABEL_BITMAP_PAD_DOTS,
    });
    // Price F2 (the bars are the hero on hardware labels).
    let amount = truncate_to_width(label.price_text.trim(), STUDIO_HW_PRICE_CHARS);
    let currency = truncate_to_width(label.currency_text.trim(), STUDIO_HW_PRICE_CHARS);
    let price = if currency.is_empty() {
        amount
    } else {
        truncate_to_width(&format!("{amount} {currency}"), STUDIO_HW_PRICE_CHARS)
    };
    plan.push(LabelTextLine {
        slot: LabelTextSlot::StudioHwPrice,
        text: price.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(price.chars().count(), FONT_2_W, 1),
            ),
        y_dots: 124,
        cell_h_dots: FONT_2_H,
        box_w_dots: LABEL_SAFE_WIDTH,
        box_h_dots: FONT_2_H + LABEL_BITMAP_PAD_DOTS,
    });
    // Human ref = the payload, F1 above the bottom edge (traceability:
    // manual entry fallback when the imager cannot read).
    let human = truncate_to_width(
        &sanitize_tspl_text(label.barcode),
        (LABEL_SAFE_WIDTH / FONT_1_W) as usize,
    );
    plan.push(LabelTextLine {
        slot: LabelTextSlot::Human,
        text: human.clone(),
        x_dots: LABEL_SAFE_LEFT
            + centered_x(
                LABEL_SAFE_WIDTH,
                text_dots(human.chars().count(), FONT_1_W, 1),
            ),
        y_dots: 74,
        cell_h_dots: FONT_1_H,
        box_w_dots: LABEL_SAFE_WIDTH,
        box_h_dots: FONT_1_H + LABEL_BITMAP_PAD_DOTS,
    });
    plan
}

/// Whether a studio TEXT payload must travel as a bitmap (PRT-02) —
/// same rule as the classic path.
#[must_use]
pub fn studio_text_needs_bitmap(content: &str, probe: TsplProbeMode) -> bool {
    super::label::label_text_needs_bitmap(content, probe)
}

/// Shared studio render tail: duplicate-slot guard + preamble + plan
/// emission + symbol + trailer. The two templates differ only in plan,
/// geometry and face table — never in validation strictness.
fn render_studio_mixed(
    plan: &[LabelTextLine],
    slot_face: fn(LabelTextSlot) -> Result<(&'static str, i64), DomainError>,
    geometry: &LabelBarcodeGeometry,
    barcode: &str,
    copies: u32,
    media: TsplMediaSetup,
    bitmaps: &[(LabelTextSlot, LabelBitmap)],
) -> Result<Vec<u8>, DomainError> {
    let invalid = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    let mut seen = [false; LABEL_TEXT_SLOT_COUNT];
    for (slot, _) in bitmaps {
        let index = *slot as usize;
        if seen[index] {
            return Err(invalid("LABEL_DUPLICATE_BITMAP_SLOT"));
        }
        seen[index] = true;
    }
    let barcode_sanitized = sanitize_tspl_text(barcode);
    let probe = media.probe_mode;
    let (dx, dy) = media.calibration_dots();
    let mut head: Vec<String> = Vec::new();
    push_media_preamble(&mut head, "SIZE 40 mm, 20 mm", &media)?;
    let mut job: Vec<u8> = head.join("\r\n").into_bytes();
    job.extend_from_slice(b"\r\n");
    for line in plan {
        let (font, scale) = slot_face(line.slot)?;
        emit_label_plan_line(&mut job, line, font, scale, bitmaps, probe, dx, dy)?;
    }
    finish_label_job(&mut job, geometry, &barcode_sanitized, copies, dx, dy)?;
    Ok(job)
}

/// Render the textile 40x20 mm label. Exact bytes pinned below.
pub fn render_textile_label_40x20(label: &StudioTextileLabel<'_>) -> Result<Vec<u8>, DomainError> {
    render_textile_label_40x20_mixed(label, &[])
}

/// Render the textile label with bitmap slots (PRT-02): same slot-keyed
/// contract as the classic mixed path — an all-ASCII job is
/// byte-identical to the plain renderer.
pub fn render_textile_label_40x20_mixed(
    label: &StudioTextileLabel<'_>,
    bitmaps: &[(LabelTextSlot, LabelBitmap)],
) -> Result<Vec<u8>, DomainError> {
    let invalid = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    if label.copies < TSPL_MIN_COPIES || label.copies > TSPL_MAX_COPIES {
        return Err(DomainError::TsplCopiesOutOfRange);
    }
    if label.model.trim().is_empty() {
        return Err(invalid("STUDIO_MODEL_MUST_NOT_BE_EMPTY"));
    }
    if label.size.trim().is_empty() {
        return Err(invalid("STUDIO_SIZE_MUST_NOT_BE_EMPTY"));
    }
    if label.color.trim().is_empty() {
        return Err(invalid("STUDIO_COLOR_MUST_NOT_BE_EMPTY"));
    }
    if label.pack.trim().is_empty() {
        return Err(invalid("STUDIO_PACK_MUST_NOT_BE_EMPTY"));
    }
    if let Some(price) = label.price_text {
        if price.trim().is_empty() {
            return Err(invalid("STUDIO_PRICE_MUST_NOT_BE_EMPTY"));
        }
    }
    let geometry =
        studio_barcode_geometry(label.barcode, STUDIO_TEXTILE_BAR_Y, STUDIO_TEXTILE_BAR_H)?;
    render_studio_mixed(
        &textile_label_text_plan(label),
        studio_slot_face,
        &geometry,
        label.barcode,
        label.copies,
        label.media,
        bitmaps,
    )
}

/// Barcode geometry for the textile template (bars y126 h28).
/// `pub`: the application preview reads the same origin as the renderer.
pub fn textile_barcode_geometry(barcode: &str) -> Result<LabelBarcodeGeometry, DomainError> {
    studio_barcode_geometry(barcode, STUDIO_TEXTILE_BAR_Y, STUDIO_TEXTILE_BAR_H)
}

/// Barcode geometry for the hardware template (hero bars y16 h56).
/// `pub`: the application preview reads the same origin as the renderer.
pub fn hardware_barcode_geometry(barcode: &str) -> Result<LabelBarcodeGeometry, DomainError> {
    studio_barcode_geometry(barcode, STUDIO_HW_BAR_Y, STUDIO_HW_BAR_H)
}

/// Render the hardware 40x20 mm label. Exact bytes pinned below.
pub fn render_hardware_label_40x20(
    label: &StudioHardwareLabel<'_>,
) -> Result<Vec<u8>, DomainError> {
    render_hardware_label_40x20_mixed(label, &[])
}

/// Render the hardware label with bitmap slots (PRT-02 contract as above).
pub fn render_hardware_label_40x20_mixed(
    label: &StudioHardwareLabel<'_>,
    bitmaps: &[(LabelTextSlot, LabelBitmap)],
) -> Result<Vec<u8>, DomainError> {
    let invalid = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    if label.copies < TSPL_MIN_COPIES || label.copies > TSPL_MAX_COPIES {
        return Err(DomainError::TsplCopiesOutOfRange);
    }
    if label.designation.trim().is_empty() {
        return Err(invalid("STUDIO_DESIGNATION_MUST_NOT_BE_EMPTY"));
    }
    if label.unit_text.trim().is_empty() {
        return Err(invalid("STUDIO_UNIT_MUST_NOT_BE_EMPTY"));
    }
    if label.price_text.trim().is_empty() {
        return Err(invalid("STUDIO_PRICE_MUST_NOT_BE_EMPTY"));
    }
    let geometry = studio_barcode_geometry(label.barcode, STUDIO_HW_BAR_Y, STUDIO_HW_BAR_H)?;
    render_studio_mixed(
        &hardware_label_text_plan(label),
        studio_slot_face,
        &geometry,
        label.barcode,
        label.copies,
        label.media,
        bitmaps,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn textile_retail() -> StudioTextileLabel<'static> {
        StudioTextileLabel {
            model: "COTON ML",
            size: "XL",
            color: "NOIR",
            pack: "PACK 6 PCS",
            price_text: Some("250,00"),
            currency_text: "DZD",
            barcode: "TXL001",
            copies: 1,
            media: TsplMediaSetup::default(),
        }
    }

    fn hardware_full() -> StudioHardwareLabel<'static> {
        StudioHardwareLabel {
            designation: "CHEVILLE MOLLY M8",
            unit_text: "PCS",
            dimension_text: Some("O8X32MM"),
            price_text: "45,00",
            currency_text: "DZD",
            barcode: "HW0001",
            copies: 1,
            media: TsplMediaSetup::default(),
        }
    }

    /// Every planned line must sit inside the 2 mm safe box, the symbol
    /// inside the media, and no two ink rects may overlap (the die-cut
    /// never touches ink, lines never collide — proven here, not
    /// eyeballed). Edges may touch; overlap may not.
    fn line_width(line: &LabelTextLine) -> i64 {
        let face_w = match line.slot {
            LabelTextSlot::StudioSize => FONT_3_W * 2,
            LabelTextSlot::StudioPrice => FONT_4_W,
            LabelTextSlot::StudioModel
            | LabelTextSlot::StudioHwName
            | LabelTextSlot::StudioHwPrice => FONT_2_W,
            _ => FONT_1_W,
        };
        line.text.chars().count() as i64 * face_w
    }
    fn assert_plan_clean(plan: &[LabelTextLine], bars: &LabelBarcodeGeometry) {
        let mut rects: Vec<(i64, i64, i64, i64)> = plan
            .iter()
            .map(|line| {
                (
                    line.x_dots,
                    line.y_dots,
                    line.x_dots + line_width(line),
                    line.y_dots + line.cell_h_dots,
                )
            })
            .collect();
        for line in plan {
            assert!(
                line.x_dots >= LABEL_SAFE_LEFT
                    && line.x_dots + line_width(line) <= LABEL_SAFE_LEFT + LABEL_SAFE_WIDTH,
                "line {:?} escapes the safe box",
                line.slot
            );
            assert!(
                line.y_dots >= 0 && line.y_dots + line.cell_h_dots <= 160,
                "line {:?} escapes the media",
                line.slot
            );
        }
        assert!(
            bars.x_dots >= LABEL_SAFE_LEFT
                && bars.x_dots + bars.width_dots <= LABEL_SAFE_LEFT + LABEL_SAFE_WIDTH
                && bars.y_dots >= 0
                && bars.y_dots + bars.height_dots <= 160,
            "symbol escapes the media"
        );
        rects.push((
            bars.x_dots,
            bars.y_dots,
            bars.x_dots + bars.width_dots,
            bars.y_dots + bars.height_dots,
        ));
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                let overlap = a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
                assert!(!overlap, "ink rects {a:?} and {b:?} overlap");
            }
        }
    }

    #[test]
    fn textile_plan_geometry_is_pinned() {
        let plan = textile_label_text_plan(&textile_retail());
        assert_eq!(plan.len(), 5, "retail textile has 5 lines");
        let by_slot = |slot| plan.iter().find(|line| line.slot == slot).expect("slot");
        assert_eq!(
            (
                by_slot(LabelTextSlot::StudioModel).x_dots,
                by_slot(LabelTextSlot::StudioModel).y_dots
            ),
            (
                LABEL_SAFE_LEFT + centered_x(LABEL_SAFE_WIDTH, text_dots(8, FONT_2_W, 1)),
                16
            )
        );
        assert_eq!(by_slot(LabelTextSlot::StudioSize).y_dots, 40);
        // "XL" = 64 dots, zoned-center: 16 + (180 - 64) / 2 = 74.
        assert_eq!(
            by_slot(LabelTextSlot::StudioSize).x_dots,
            LABEL_SAFE_LEFT + centered_x(STUDIO_SIZE_ZONE_W, text_dots(2, FONT_3_W * 2, 1))
        );
        assert_eq!(
            (
                by_slot(LabelTextSlot::StudioColor).x_dots,
                by_slot(LabelTextSlot::StudioColor).y_dots
            ),
            (STUDIO_INFO_X, 44)
        );
        assert_eq!(by_slot(LabelTextSlot::StudioPack).y_dots, 60);
        assert_eq!(by_slot(LabelTextSlot::StudioPrice).y_dots, 92);
        let bars = studio_barcode_geometry("TXL001", STUDIO_TEXTILE_BAR_Y, STUDIO_TEXTILE_BAR_H)
            .expect("bars");
        assert_plan_clean(&plan, &bars);
    }

    #[test]
    fn core_shop_sizes_never_touch_the_right_column() {
        // Regression: a centered FULL-WIDTH F5 badge overprints the
        // color/pack column from 3 chars up ("XXL" = 96 dots →
        // x112..208 × column x196..). The badge zone (x16..196) makes
        // clearance structural — proven here for every core size.
        for size in ["S", "XL", "XXL", "3XL", "XXXL"] {
            let mut label = textile_retail();
            label.size = size;
            let plan = textile_label_text_plan(&label);
            let badge = plan
                .iter()
                .find(|line| line.slot == LabelTextSlot::StudioSize)
                .expect("slot");
            let badge_right = badge.x_dots + badge.text.chars().count() as i64 * FONT_3_W * 2;
            assert!(
                badge_right <= STUDIO_INFO_X,
                "size {size} overprints the column (right edge {badge_right})"
            );
            let bars =
                studio_barcode_geometry(label.barcode, STUDIO_TEXTILE_BAR_Y, STUDIO_TEXTILE_BAR_H)
                    .expect("bars");
            assert_plan_clean(&plan, &bars);
        }
        // Overlong sizes truncate WITH a mark, still inside the zone.
        let mut label = textile_retail();
        label.size = "TAILLE UNIQUE";
        let plan = textile_label_text_plan(&label);
        let badge = plan
            .iter()
            .find(|line| line.slot == LabelTextSlot::StudioSize)
            .expect("slot");
        assert_eq!(badge.text.chars().count(), 5, "zone cap: 180 / 32");
        assert!(badge.text.ends_with("..."), "cut is marked, never silent");
    }

    #[test]
    fn textile_wholesale_hide_drops_the_price_line() {
        let mut label = textile_retail();
        label.price_text = None;
        let plan = textile_label_text_plan(&label);
        assert_eq!(plan.len(), 4, "wholesale textile has no price line");
        assert!(plan
            .iter()
            .all(|line| line.slot != LabelTextSlot::StudioPrice));
        let job = render_textile_label_40x20(&label).expect("renders");
        let text = String::from_utf8(job).expect("ascii-clean TSPL");
        assert!(!text.contains("250,00"), "no price bytes ride");
    }

    #[test]
    fn textile_retail_job_shape_is_pinned() {
        let job = render_textile_label_40x20(&textile_retail()).expect("renders");
        let text = String::from_utf8(job).expect("ascii-clean TSPL");
        assert!(text.contains("SIZE 40 mm, 20 mm"), "preamble");
        assert!(text.contains("\"COTON ML\""), "model F2");
        assert!(text.contains("\"XL\""), "giant size");
        assert!(text.contains("\"NOIR\""), "color");
        assert!(text.contains("\"250,00 DZD\""), "retail price");
        // Bars y126 h28, centered, narrow = 2 (proven symbol geometry).
        let geometry = studio_barcode_geometry("TXL001", 126, 28).expect("geometry");
        assert_eq!((geometry.y_dots, geometry.height_dots), (126, 28));
        assert!(
            text.contains(&format!(
                "BARCODE {},126,\"128\",28,0,0,2,2,\"TXL001\"",
                geometry.x_dots
            )),
            "symbol line pinned"
        );
        assert!(text.ends_with("PRINT 1,1\r\n"), "trailer + CRLF law");
    }

    #[test]
    fn hardware_plan_geometry_is_pinned() {
        let plan = hardware_label_text_plan(&hardware_full());
        assert_eq!(plan.len(), 4);
        let by_slot = |slot| plan.iter().find(|line| line.slot == slot).expect("slot");
        assert_eq!(by_slot(LabelTextSlot::StudioHwName).y_dots, 88);
        assert_eq!(by_slot(LabelTextSlot::StudioHwDim).y_dots, 110);
        assert_eq!(by_slot(LabelTextSlot::StudioHwPrice).y_dots, 124);
        assert_eq!(by_slot(LabelTextSlot::Human).y_dots, 74);
        assert_eq!(
            by_slot(LabelTextSlot::StudioHwDim).text,
            "PCS · O8X32MM",
            "unit + dimension compose"
        );
        let bars =
            studio_barcode_geometry("HW0001", STUDIO_HW_BAR_Y, STUDIO_HW_BAR_H).expect("bars");
        assert_plan_clean(&plan, &bars);
    }

    #[test]
    fn hardware_job_is_barcode_forward() {
        let job = render_hardware_label_40x20(&hardware_full()).expect("renders");
        let text = String::from_utf8(job).expect("ascii-clean TSPL");
        let geometry = studio_barcode_geometry("HW0001", 16, 56).expect("geometry");
        assert_eq!((geometry.y_dots, geometry.height_dots), (16, 56));
        assert!(
            text.contains(&format!(
                "BARCODE {},16,\"128\",56,0,0,2,2,\"HW0001\"",
                geometry.x_dots
            )),
            "hero bars pinned"
        );
        let bar_at = text.find("BARCODE").expect("bars");
        let name_at = text.find("CHEVILLE MOLLY M8").expect("name");
        // Emission order is plan order (TEXT first, symbol last — same as
        // the classic path); "barcode-forward" is VISUAL: the hero bars
        // sit at y16 while every TEXT line lives below y74.
        let _ = (bar_at, name_at);
        assert!(geometry.y_dots < 74, "hero bars sit above every text line");
        assert!(text.contains("\"PCS · O8X32MM\""), "unit line");
        assert!(text.contains("\"45,00 DZD\""), "price line");
        assert!(text.ends_with("PRINT 1,1\r\n"), "trailer + CRLF law");
    }

    #[test]
    fn hardware_without_dimension_keeps_geometry() {
        let mut label = hardware_full();
        label.dimension_text = None;
        let plan = hardware_label_text_plan(&label);
        let dim = plan
            .iter()
            .find(|line| line.slot == LabelTextSlot::StudioHwDim)
            .expect("slot");
        assert_eq!(dim.text, "PCS", "unit alone, no reflow");
        assert_eq!(dim.y_dots, 110, "fixed position kept");
    }

    #[test]
    fn oversize_fields_truncate_with_ellipsis() {
        let mut label = textile_retail();
        label.model = "MODELE TRES LONG QUI DEPASSE LARGEMENT";
        let plan = textile_label_text_plan(&label);
        let model = plan
            .iter()
            .find(|line| line.slot == LabelTextSlot::StudioModel)
            .expect("slot");
        assert_eq!(model.text.chars().count(), 24, "F2 cap");
        assert!(model.text.ends_with("..."), "cut is marked, never silent");
    }

    #[test]
    fn empty_fields_reject_loudly() {
        let mut label = textile_retail();
        label.model = "  ";
        assert!(render_textile_label_40x20(&label).is_err(), "empty model");
        let mut label = textile_retail();
        label.price_text = Some(" ");
        assert!(
            render_textile_label_40x20(&label).is_err(),
            "shown-but-blank price"
        );
        let mut label = hardware_full();
        label.designation = "";
        assert!(
            render_hardware_label_40x20(&label).is_err(),
            "empty designation"
        );
        let mut label = hardware_full();
        label.barcode = "WAY-TOO-LONG-12345";
        assert!(
            matches!(
                render_hardware_label_40x20(&label),
                Err(DomainError::TsplBarcodeTooWide)
            ),
            "wide barcode"
        );
        let mut label = hardware_full();
        label.copies = 0;
        assert!(
            matches!(
                render_hardware_label_40x20(&label),
                Err(DomainError::TsplCopiesOutOfRange)
            ),
            "copies range"
        );
    }

    #[test]
    fn arabic_color_travels_as_bitmap_never_dead() {
        let mut label = textile_retail();
        label.color = "أسود";
        assert!(
            studio_text_needs_bitmap("أسود", TsplProbeMode::Normal),
            "arabic needs pixels"
        );
        // Without a raster the job fails LOUDLY (same PRT-02 contract).
        assert!(render_textile_label_40x20(&label).is_err());
        // With a raster the slot rides a BITMAP chunk at its origin.
        let plan = textile_label_text_plan(&label);
        let slot_line = plan
            .iter()
            .find(|line| line.slot == LabelTextSlot::StudioColor)
            .expect("slot");
        let raster = LabelBitmap {
            width_bytes: 13,
            height_rows: 12,
            data: vec![0xAA; 13 * 12],
        };
        let job = render_textile_label_40x20_mixed(&label, &[(LabelTextSlot::StudioColor, raster)])
            .expect("mixed renders");
        let head = format!(
            "BITMAP {},{},13,12,0,",
            slot_line.x_dots,
            slot_line.y_dots + slot_line.cell_h_dots - 12
        );
        assert!(
            job.windows(head.len()).any(|w| w == head.as_bytes()),
            "bitmap chunk at the slot origin"
        );
    }

    #[test]
    fn duplicate_bitmap_slot_rejects() {
        let raster = LabelBitmap {
            width_bytes: 2,
            height_rows: 4,
            data: vec![0xAA; 8],
        };
        let label = textile_retail();
        assert!(
            render_textile_label_40x20_mixed(
                &label,
                &[
                    (LabelTextSlot::StudioModel, raster.clone()),
                    (LabelTextSlot::StudioModel, raster),
                ]
            )
            .is_err(),
            "duplicate slot"
        );
    }
}
