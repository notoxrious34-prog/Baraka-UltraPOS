//! Pre-flight gate: bundled font assets parse and cover required glyphs.
//!
//! This test guards the `include_bytes!` inputs of the whole raster
//! pipeline. Rendering tests below build on these faces.

use baraka_domain::printing::{
    ReceiptDocument, ReceiptLanguage, ReceiptLine, ReceiptTotals, StoreIdentity,
};
use baraka_infrastructure::printing::font_cache;
use baraka_infrastructure::printing::rasterizer::{encode_gs_v0, render_canvas, PAPER_WIDTH_DOTS};

#[test]
fn font_assets_parse_and_cover_required_glyphs() {
    // Structural validity of both embedded faces.
    let noto = font_cache::noto_ttf_face();
    let inter = font_cache::inter_ttf_face();
    assert!(noto.units_per_em() > 0);
    assert!(inter.units_per_em() > 0);
    // ARABIC LETTER BEH must exist in the Noto face.
    assert!(
        noto.glyph_index('ب').is_some(),
        "Noto face lacks U+0628 BEH"
    );
    // Arabic presentation coverage for shaped output.
    assert!(
        noto.glyph_index('م').is_some(),
        "Noto face lacks U+0645 MEEM"
    );
    // Latin 'A' and ASCII digits must exist in the Inter face.
    assert!(inter.glyph_index('A').is_some(), "Inter face lacks 'A'");
    assert!(inter.glyph_index('5').is_some(), "Inter face lacks '5'");
    // rustybuzz shaping faces build from the same bytes.
    let _ = font_cache::noto_shaping_face();
    let _ = font_cache::inter_shaping_face();
    // ab_glyph raster faces build from the same bytes.
    let _ = font_cache::noto_raster_font();
    let _ = font_cache::inter_raster_font();
}

fn sample_receipt(language: ReceiptLanguage, designation: &str) -> ReceiptDocument {
    ReceiptDocument {
        language,
        shop_name: "BARAKA QUINCAILLERIE".to_string(),
        number: "TICK-2026-000001".to_string(),
        created_at: "2026-09-26T10:00:00Z".to_string(),
        lines: vec![ReceiptLine {
            designation: designation.to_string(),
            quantity_text: "2".to_string(),
            unit_price_text: "122.50".to_string(),
            total_text: "245.00".to_string(),
        }],
        totals: ReceiptTotals {
            subtotal_ht: "245.00".to_string(),
            tva: "0.00".to_string(),
            discount: "0.00".to_string(),
            timbre: "5.00".to_string(),
            total_ttc: "250.00".to_string(),
            paid: "250.00".to_string(),
            change: "0.00".to_string(),
        },
        payments: vec![("CASH".to_string(), "250.00".to_string())],
        is_duplicata: false,
        store_identity: StoreIdentity::default(),
        footer_message: String::new(),
    }
}

/// Count dark pixels in a canvas (must be non-trivial for real content).
fn dark_pixels(canvas: &image::GrayImage) -> u64 {
    canvas.pixels().filter(|pixel| pixel.0[0] < 128).count() as u64
}

#[test]
fn arabic_receipt_renders_576_dot_raster() {
    let receipt = sample_receipt(ReceiptLanguage::Arabic, "برغي حديد 8مم");
    let canvas = render_canvas(&receipt).expect("arabic canvas");
    assert_eq!(canvas.width(), PAPER_WIDTH_DOTS);
    assert!(canvas.height() > 100, "receipt has body height");
    assert!(
        dark_pixels(&canvas) > 500,
        "arabic shaping produced visible ink"
    );
    let bytes = encode_gs_v0(&canvas);
    assert!(bytes.len() > 100);
    // GS v 0 header, 72 bytes per row at 576 dots.
    assert_eq!(&bytes[0..4], &[0x1D, 0x76, 0x30, 0x00]);
    assert_eq!((bytes[4], bytes[5]), (72, 0));
}

#[test]
fn french_receipt_renders_576_dot_raster() {
    let receipt = sample_receipt(ReceiptLanguage::French, "Vis acier 8x40 (boite)");
    let canvas = render_canvas(&receipt).expect("french canvas");
    assert_eq!(canvas.width(), PAPER_WIDTH_DOTS);
    assert!(dark_pixels(&canvas) > 500);
    let bytes = encode_gs_v0(&canvas);
    assert!(bytes.len() > 100);
}

#[test]
fn empty_designation_never_panics() {
    let receipt = sample_receipt(ReceiptLanguage::Arabic, "");
    let canvas = render_canvas(&receipt).expect("empty designation canvas");
    assert_eq!(canvas.width(), PAPER_WIDTH_DOTS);
    // A very long name must wrap-free render without panic or truncation crash.
    let long = sample_receipt(ReceiptLanguage::French, &"Vis ".repeat(200));
    let wide = render_canvas(&long).expect("long designation canvas");
    assert_eq!(wide.width(), PAPER_WIDTH_DOTS);
    assert!(dark_pixels(&wide) > dark_pixels(&canvas));
}

#[test]
fn duplicata_watermark_changes_output() {
    let plain = sample_receipt(ReceiptLanguage::French, "Vis acier");
    let mut stamped = plain.clone();
    stamped.is_duplicata = true;
    let plain_bytes = encode_gs_v0(&render_canvas(&plain).expect("plain"));
    let stamped_bytes = encode_gs_v0(&render_canvas(&stamped).expect("stamped"));
    assert_ne!(
        plain_bytes, stamped_bytes,
        "DUPLICATA watermark must alter raster bytes"
    );
    assert!(stamped_bytes.len() > plain_bytes.len());
}
