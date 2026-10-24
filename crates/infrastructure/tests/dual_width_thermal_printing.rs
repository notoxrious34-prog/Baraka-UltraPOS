//! Dual-width thermal calibration drill (TASK-BRK-070, Debt #6).
//!
//! The raster surface was hardcoded to 576 dots: 58mm printers clipped a
//! third of every receipt. These tests prove both stocked formats render
//! byte-exact strides, wrap long Arabic designations without panic or
//! corruption, and round-trip through the persisted paper-width setting.

use std::sync::{Arc, Mutex};

use baraka_application::ports::{PrinterSettingsPort, SessionContext, SessionToken};
use baraka_application::use_cases::printing::{
    GetPrinterSettingsUseCase, UpdatePrinterSettingsUseCase,
};
use baraka_domain::printing::{
    ReceiptDocument, ReceiptLanguage, ReceiptLine, ReceiptTotals, StoreIdentity,
};
use baraka_infrastructure::adapters::SqlitePrinterSettings;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::printing::rasterizer::{
    content_width_dots, encode_gs_v0, render_canvas, PaperWidth, PAPER_WIDTH_DOTS,
};

/// Receipt with a bilingual header, three bilingual lines (one very long
/// Arabic designation), and a full totals block.
fn sample_receipt() -> ReceiptDocument {
    ReceiptDocument {
        language: ReceiptLanguage::Arabic,
        shop_name: "BARAKA QUINCAILLERIE".to_string(),
        number: "TICK-2026-000001".to_string(),
        created_at: "2026-10-16T10:00:00Z".to_string(),
        lines: vec![
            ReceiptLine {
                designation: "Vis à bois 4x40".to_string(),
                quantity_text: "10".to_string(),
                unit_price_text: "5.00".to_string(),
                total_text: "50.00".to_string(),
            },
            ReceiptLine {
                designation: "مفتاح ربط أنابيب تركي أصلي ذو مقبض مطاطي متين معزول ضد الصدمات قياس 24 بوصة فولاذ كروم فاناديوم مقسى".to_string(),
                quantity_text: "2".to_string(),
                unit_price_text: "850.00".to_string(),
                total_text: "1700.00".to_string(),
            },
            ReceiptLine {
                designation: "Peinture vinylique blanche 15kg + pinceau rouleau soie naturelle"
                    .to_string(),
                quantity_text: "1".to_string(),
                unit_price_text: "6800.00".to_string(),
                total_text: "6800.00".to_string(),
            },
        ],
        totals: ReceiptTotals {
            subtotal_ht: "8550.00".to_string(),
            tva: "0.00".to_string(),
            discount: "0.00".to_string(),
            timbre: "86.00".to_string(),
            total_ttc: "8636.00".to_string(),
            paid: "9000.00".to_string(),
            change: "364.00".to_string(),
        },
        payments: vec![("CASH".to_string(), "9000.00".to_string())],
        is_duplicata: false,
        store_identity: StoreIdentity {
            store_name: "قيد الإنشاء / Quincaillerie Générale".to_string(),
            ..StoreIdentity::default()
        },
        footer_message: String::new(),
    }
}

/// First `GS v 0` band header: row-byte stride (xL, xH) for this canvas.
fn first_band_stride(stream: &[u8]) -> usize {
    assert!(
        stream.len() >= 8,
        "stream too short for a raster band header"
    );
    assert_eq!(
        &stream[0..4],
        &[0x1D, 0x76, 0x30, 0x00],
        "raster band must open with GS v 0 m=0"
    );
    let x_low = stream[4] as usize;
    let x_high = stream[5] as usize;
    x_low + 256 * x_high
}

fn dark_pixels(canvas: &image::GrayImage) -> u64 {
    canvas.pixels().filter(|pixel| pixel.0[0] < 128).count() as u64
}

#[test]
fn render_80mm_receipt_produces_576_dot_bitmap() {
    let receipt = sample_receipt();
    let canvas = render_canvas(&receipt, PaperWidth::Width80mm).expect("80mm canvas");
    assert_eq!(canvas.width(), 576, "80mm raster is exactly 576 dots");
    assert_eq!(
        content_width_dots(PaperWidth::Width80mm),
        560.0,
        "80mm content surface excludes 8-dot margins"
    );
    assert_eq!(PaperWidth::Width80mm.char_columns(), 42);
    assert!(dark_pixels(&canvas) > 1_000, "real ink on the wide canvas");

    let stream = encode_gs_v0(&canvas);
    assert_eq!(
        first_band_stride(&stream),
        72,
        "80mm stride is 72 bytes/scanline (0x48)"
    );
    assert_eq!(&stream[4..6], &[0x48, 0x00], "stride header bytes exact");
}

#[test]
fn render_58mm_receipt_produces_384_dot_bitmap() {
    let receipt = sample_receipt();
    let canvas = render_canvas(&receipt, PaperWidth::Width58mm).expect("58mm canvas");
    assert_eq!(canvas.width(), 384, "58mm raster is exactly 384 dots");
    assert_eq!(
        content_width_dots(PaperWidth::Width58mm),
        368.0,
        "58mm content surface excludes 8-dot margins"
    );
    assert_eq!(PaperWidth::Width58mm.char_columns(), 32);
    assert!(
        dark_pixels(&canvas) > 1_000,
        "real ink on the narrow canvas"
    );

    // The long Arabic designation must wrap onto extra scanlines instead
    // of clipping: the narrow canvas carries MORE rows than the wide one.
    let wide = render_canvas(&receipt, PaperWidth::Width80mm).expect("wide");
    assert!(
        canvas.height() > wide.height(),
        "58mm wraps ({} rows) beyond 80mm ({} rows)",
        canvas.height(),
        wide.height()
    );

    let stream = encode_gs_v0(&canvas);
    assert_eq!(
        first_band_stride(&stream),
        48,
        "58mm stride is 48 bytes/scanline (0x30)"
    );
    assert_eq!(&stream[4..6], &[0x30, 0x00], "stride header bytes exact");

    // Every band header in the stream agrees on the 48-byte stride.
    let mut cursor = 0usize;
    let mut bands = 0usize;
    while cursor + 8 <= stream.len() && stream[cursor] == 0x1D {
        let stride = first_band_stride(&stream[cursor..]);
        assert_eq!(stride, 48, "band {bands} stride stable");
        let band_rows = stream[cursor + 6] as usize + 256 * stream[cursor + 7] as usize;
        cursor += 8 + stride * band_rows;
        bands += 1;
    }
    assert!(bands >= 1, "at least one raster band emitted");
}

#[test]
fn shaping_advances_are_physical_pixels() {
    // Regression guard (TASK-BAT-071): `shape_run` divided the rustybuzz
    // advance by an extra 64, compressing every line ~64x. rustybuzz 0.18
    // returns font units, so per-glyph advance must be a sane fraction of
    // the requested pixel size, not a rounding artifact.
    let latin = "Cle a molette professionnelle renforcee nickel chrome 24 pouces";
    let arabic =
        "مفتاح ربط أنابيب تركي أصلي ذو مقبض مطاطي متين معزول ضد الصدمات قياس 24 بوصة فولاذ كروم فاناديوم مقسى";
    for (label, text, px, min_per_char) in [
        ("latin", latin, 24.0f32, 4.0f32),
        ("arabic", arabic, 24.0f32, 4.0f32),
    ] {
        let (_direction, runs) =
            baraka_infrastructure::printing::shaping::shape_line(text, px).expect("shape");
        let width: f32 = runs.iter().map(|run| run.width_px).sum();
        let per_char = width / text.chars().count() as f32;
        assert!(
            per_char >= min_per_char,
            "{label}: {per_char:.3} px/char at {px}px is not a physical advance \
             (width={width:.2}px for {} chars)",
            text.chars().count()
        );
    }
}

#[test]
fn long_designation_wraps_on_both_canvases() {
    // The same long designation must occupy more rows on the narrow canvas.
    let long_ar = format!(
        "{} {}",
        sample_receipt().lines[1].designation,
        "مقبض مطاطي متين"
    );
    let mut receipt = sample_receipt();
    receipt.lines[1].designation = long_ar;
    let narrow = render_canvas(&receipt, PaperWidth::Width58mm).expect("58mm");
    let wide = render_canvas(&receipt, PaperWidth::Width80mm).expect("80mm");
    assert!(
        narrow.height() > wide.height(),
        "58mm must wrap ({}) above 80mm ({})",
        narrow.height(),
        wide.height()
    );
}

#[test]
fn paper_width_enum_contract_and_defaults() {
    assert_eq!(PAPER_WIDTH_DOTS, 576, "default surface stays 80mm");
    assert_eq!(PaperWidth::from_mm(58), PaperWidth::Width58mm);
    assert_eq!(PaperWidth::from_mm(80), PaperWidth::Width80mm);
    assert_eq!(PaperWidth::from_mm(0), PaperWidth::Width58mm);
    assert_eq!(PaperWidth::from_mm(59), PaperWidth::Width80mm);
    assert_eq!(PaperWidth::from_mm(10_000), PaperWidth::Width80mm);
    assert_eq!(PaperWidth::Width58mm.dot_width(), 384);
    assert_eq!(PaperWidth::Width80mm.dot_width(), 576);
    assert_eq!(PaperWidth::Width58mm.char_columns(), 32);
    assert_eq!(PaperWidth::Width80mm.char_columns(), 42);
    assert_eq!(PaperWidth::Width58mm.as_mm(), 58);
    assert_eq!(PaperWidth::Width80mm.as_mm(), 80);
}

fn settings_harness() -> (tempfile::TempDir, SqlitePrinterSettings) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("paper.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let settings = SqlitePrinterSettings::new(Arc::new(Mutex::new(conn)));
    (dir, settings)
}

fn admin_session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: baraka_domain::user::UserId(
            uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid"),
        ),
        role: baraka_domain::role::Role::Admin,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    }
}

fn gerant_session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: baraka_domain::user::UserId(
            uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid"),
        ),
        role: baraka_domain::role::Role::Gerant,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    }
}

#[test]
fn sqlite_printer_settings_repository_persists_paper_width() {
    let (_dir, settings) = settings_harness();

    // Absent keys read back as the 80mm default.
    let fresh = settings.get_printer_settings().expect("defaults");
    assert_eq!(fresh.paper_width_mm, 80);
    assert_eq!(fresh.paper_width_dots, 576);

    // 58mm persists and reads back with the derived dot width.
    settings
        .update_printer_settings("COM3", 9_600, Some(58), None, None)
        .expect("store 58mm");
    let reread = settings.get_printer_settings().expect("reread");
    assert_eq!((reread.paper_width_mm, reread.paper_width_dots), (58, 384));

    // Back to 80mm.
    settings
        .update_printer_settings("COM3", 9_600, Some(80), None, None)
        .expect("store 80mm");
    let wide = settings.get_printer_settings().expect("reread");
    assert_eq!((wide.paper_width_mm, wide.paper_width_dots), (80, 576));

    // `None` keeps the stored width while port/baud still update.
    settings
        .update_printer_settings("COM4", 115_200, None, None, None)
        .expect("keep width");
    let kept = settings.get_printer_settings().expect("reread");
    assert_eq!(kept.paper_width_mm, 80);
    assert_eq!(kept.paper_width_dots, 576);
    assert_eq!(kept.serial_port, "COM4");
    assert_eq!(kept.serial_baud, 115_200);
}

#[test]
fn printer_settings_use_cases_handle_dual_width() {
    let (_dir, settings) = settings_harness();
    UpdatePrinterSettingsUseCase::new(&settings)
        .execute(&admin_session(), "COM3", 9_600, Some(80), None, None)
        .expect("baseline");

    // Admin stores 58mm through the use case; read-back confirms.
    let narrow = UpdatePrinterSettingsUseCase::new(&settings)
        .execute(&admin_session(), "COM3", 9_600, Some(58), None, None)
        .expect("store 58mm");
    assert_eq!(narrow.paper_width_mm, 58);
    assert_eq!(narrow.paper_width_dots, 384);
    let via_getter = GetPrinterSettingsUseCase::new(&settings)
        .execute(&admin_session())
        .expect("getter");
    assert_eq!(via_getter.paper_width_mm, 58);

    let rejected = UpdatePrinterSettingsUseCase::new(&settings)
        .execute(&admin_session(), "COM9", 9_600, Some(57), None, None)
        .expect_err("57mm is not stocked");
    assert!(
        matches!(
            rejected,
            baraka_application::errors::ApplicationError::Validation(
                baraka_domain::errors::DomainError::InvalidPaperWidthMillimeters(57)
            )
        ),
        "unexpected: {rejected:?}"
    );
    // Zero writes: port and width both untouched.
    let after = settings.get_printer_settings().expect("reread");
    assert_eq!(after.serial_port, "COM3");
    assert_eq!(after.paper_width_mm, 58);
    assert_eq!(after.paper_width_dots, 384);

    // Non-admin roles cannot touch printer settings at all.
    let denied = UpdatePrinterSettingsUseCase::new(&settings)
        .execute(&gerant_session(), "COM9", 9_600, Some(58), None, None)
        .expect_err("gerant blocked");
    assert!(
        matches!(
            denied,
            baraka_application::errors::ApplicationError::Unauthorized
        ),
        "unexpected: {denied:?}"
    );
}
