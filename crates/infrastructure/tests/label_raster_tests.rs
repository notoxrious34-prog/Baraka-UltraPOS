//! PRT-02 gate: the label line rasterer shapes Arabic into 1-bit strips
//! that fit the TEXT cell box (shrink-to-fit, never clip, never blank).

use baraka_application::ports::TsplLabelRasterPort;
use baraka_infrastructure::printing::label_raster::TsplLabelRasterer;

#[test]
fn arabic_line_rasterizes_into_name_box() {
    let rasterer = TsplLabelRasterer;
    // NameLine1 box: 300 dots wide, 24 rows tall (20-dot cell + pad).
    let line = rasterer
        .rasterize_label_text("ابيض", 20.0, 300, 24)
        .expect("arabic rasterizes");
    assert!(
        line.width_bytes * 8 <= 300,
        "strip fits the box width, got {} bytes",
        line.width_bytes
    );
    assert!(
        line.height_rows <= 24,
        "strip fits the box height, got {} rows",
        line.height_rows
    );
    assert_eq!(
        line.data.len(),
        line.width_bytes * line.height_rows,
        "packed rows are complete"
    );
    assert!(
        line.data.iter().any(|byte| *byte != 0xFF),
        "the strip carries ink (cleared bits = black, TSPL polarity)"
    );
}

#[test]
fn mixed_arabic_latin_line_rasterizes_into_name_box() {
    let rasterer = TsplLabelRasterer;
    let line = rasterer
        .rasterize_label_text("JILI ISPANA ابيض", 20.0, 300, 24)
        .expect("mixed line rasterizes");
    assert_eq!(
        line.data.len(),
        line.width_bytes * line.height_rows,
        "packed rows are complete"
    );
    assert!(
        line.data.iter().any(|byte| *byte != 0xFF),
        "the strip carries ink (cleared bits = black, TSPL polarity)"
    );
}

#[test]
fn degenerate_box_rejects_loudly() {
    let rasterer = TsplLabelRasterer;
    let error = rasterer
        .rasterize_label_text("ابيض", 20.0, 0, 24)
        .expect_err("zero-width box rejects");
    assert!(
        matches!(
            error,
            baraka_application::errors::ApplicationError::Port(ref code)
            if code == "TSPL_LABEL_BOX_MUST_BE_POSITIVE"
        ),
        "stable loud code, got: {error:?}"
    );
}

/// Padding bits beyond the trimmed width must stay WHITE (0xFF init).
/// Regression: with TSPL polarity (1 = white) a 0x00 init burned the
/// padding as a solid black bar whenever trimmed_w % 8 != 0
/// (photo-verified LIVE-2026-11-06: 7-dot phantom bar).
#[test]
fn strip_padding_bits_stay_white() {
    use baraka_application::ports::TsplLabelRasterPort;
    let rasterer = TsplLabelRasterer;
    let word = "ابيض";
    let line = rasterer
        .rasterize_label_text(word, 20.0, 300, 24)
        .expect("rasterizes");
    assert_eq!(line.width_bytes, 5);
    for (i, row) in line.data.as_chunks::<5>().0.iter().enumerate() {
        assert_eq!(row[4] & 0x0F, 0x0F, "row {i}: padding bits must be white");
    }
    assert!(line.data.iter().any(|b| *b != 0xFF));
}
