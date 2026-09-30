//! Embedded font loading (process-wide singletons, never per-job).
//!
//! Both faces parse from `include_bytes!` data exactly once. The byte
//! slices are `'static`, so every derived face borrows for the whole
//! process lifetime with zero per-job parsing cost.

use std::sync::OnceLock;

/// Noto Naskh Arabic (SIL OFL) — Arabic-script runs.
pub const NOTO_NASKH_ARABIC_TTF: &[u8] =
    include_bytes!("../../../../assets/fonts/NotoNaskhArabic-Regular.ttf");

/// Inter Regular (SIL OFL) — Latin/digit runs.
pub const INTER_REGULAR_TTF: &[u8] = include_bytes!("../../../../assets/fonts/Inter-Regular.ttf");

static NOTO_TTF_FACE: OnceLock<ttf_parser::Face<'static>> = OnceLock::new();
static INTER_TTF_FACE: OnceLock<ttf_parser::Face<'static>> = OnceLock::new();
static NOTO_RUSTYBUZZ_FACE: OnceLock<rustybuzz::Face<'static>> = OnceLock::new();
static INTER_RUSTYBUZZ_FACE: OnceLock<rustybuzz::Face<'static>> = OnceLock::new();
static NOTO_GLYPH_FONT: OnceLock<ab_glyph::FontRef<'static>> = OnceLock::new();
static INTER_GLYPH_FONT: OnceLock<ab_glyph::FontRef<'static>> = OnceLock::new();

/// Parsed Noto face (metrics queries, pre-flight checks).
pub fn noto_ttf_face() -> ttf_parser::Face<'static> {
    NOTO_TTF_FACE
        .get_or_init(|| {
            ttf_parser::Face::parse(NOTO_NASKH_ARABIC_TTF, 0).expect("bundled Noto font parses")
        })
        .clone()
}

/// Parsed Inter face (metrics queries, pre-flight checks).
pub fn inter_ttf_face() -> ttf_parser::Face<'static> {
    INTER_TTF_FACE
        .get_or_init(|| {
            ttf_parser::Face::parse(INTER_REGULAR_TTF, 0).expect("bundled Inter font parses")
        })
        .clone()
}

/// Shaping face for Arabic-script runs.
pub fn noto_shaping_face() -> rustybuzz::Face<'static> {
    NOTO_RUSTYBUZZ_FACE
        .get_or_init(|| {
            rustybuzz::Face::from_slice(NOTO_NASKH_ARABIC_TTF, 0).expect("bundled Noto font shapes")
        })
        .clone()
}

/// Shaping face for Latin/digit runs.
pub fn inter_shaping_face() -> rustybuzz::Face<'static> {
    INTER_RUSTYBUZZ_FACE
        .get_or_init(|| {
            rustybuzz::Face::from_slice(INTER_REGULAR_TTF, 0).expect("bundled Inter font shapes")
        })
        .clone()
}

/// Raster face for Arabic-script runs.
pub fn noto_raster_font() -> ab_glyph::FontRef<'static> {
    NOTO_GLYPH_FONT
        .get_or_init(|| {
            ab_glyph::FontRef::try_from_slice(NOTO_NASKH_ARABIC_TTF)
                .expect("bundled Noto font rasterizes")
        })
        .clone()
}

/// Raster face for Latin/digit runs.
pub fn inter_raster_font() -> ab_glyph::FontRef<'static> {
    INTER_GLYPH_FONT
        .get_or_init(|| {
            ab_glyph::FontRef::try_from_slice(INTER_REGULAR_TTF)
                .expect("bundled Inter font rasterizes")
        })
        .clone()
}
