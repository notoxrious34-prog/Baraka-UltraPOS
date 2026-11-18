//! Shared TSPL geometry + text helpers (crate-internal).
//!
//! Font metrics, Code128 budgets and the TEXT emission primitives used by
//! the ticket plans, the label engine and the ticket renderer. All items
//! are `pub(crate)`: the hub re-exports them for the pinned tests, but no
//! downstream crate addresses them directly.

use super::media::{LABEL_WIDTH_DOTS, TICKET_WIDTH_DOTS};
use super::probe::TsplProbeMode;
use crate::errors::DomainError;

/// Assumed bitmap font widths in dots (TSC TSPL reference) — see module
/// docs. Heights are captured in the PITCH_* line pitches below.
pub(crate) const FONT_1_W: i64 = 8;
pub(crate) const FONT_2_W: i64 = 12;
pub(crate) const FONT_3_W: i64 = 16;
/// TSC bitmap cell heights (dots): font "1" is 8x12, "2" is 12x20,
/// "3" is 16x24 — confirmed by the label geometry itself (price at 2x24
/// ends at y=106 where the barcode starts; human at 148+12 ends at the
/// 160-dot bottom edge).
pub(crate) const FONT_1_H: i64 = 12;
pub(crate) const FONT_2_H: i64 = 20;
pub(crate) const FONT_3_H: i64 = 24;
/// DOC-07 cosmetics: TSC native 24x32 face ("4") for the store name (F4)
/// and the grand total — the 4 mm middle the owner asked for, without the
/// F5 slab that fills the paper edge to edge.
pub(crate) const FONT_4_W: i64 = 24;
pub(crate) const FONT_4_H: i64 = 32;

/// Code128 auto-subset density: 11 modules per character plus 35 modules
/// of start/checksum/stop overhead (stated approximation).
pub(crate) const CODE128_MODULES_PER_CHAR: usize = 11;
pub(crate) const CODE128_OVERHEAD_MODULES: usize = 35;
/// Module width in dots (narrow = wide = 2 = 0.25 mm ≈ 9.8 mil @203 dpi).
pub(crate) const BARCODE_NARROW_DOTS: usize = 2;
/// Quiet zone reserved per side: 10 narrow modules × 2 dots (ISO minimum).
pub(crate) const QUIET_ZONE_DOTS: i64 = 10 * BARCODE_NARROW_DOTS as i64;
/// Maximum ticket barcode payload: 11·n + 35 modules must fit the media
/// minus both quiet zones (640 − 2×20 = 600 dots → 300 modules → n ≤ 24).
pub(crate) const TICKET_BARCODE_MAX_CHARS: usize =
    ((TICKET_WIDTH_DOTS as usize - 2 * QUIET_ZONE_DOTS as usize) / BARCODE_NARROW_DOTS
        - CODE128_OVERHEAD_MODULES)
        / CODE128_MODULES_PER_CHAR;
/// Maximum label barcode payload: 320 − 2×20 = 280 dots → 140 modules
/// → 11·n + 35 ≤ 140 → n ≤ 9.
pub(crate) const LABEL_BARCODE_MAX_CHARS: usize =
    ((LABEL_WIDTH_DOTS as usize - 2 * QUIET_ZONE_DOTS as usize) / BARCODE_NARROW_DOTS
        - CODE128_OVERHEAD_MODULES)
        / CODE128_MODULES_PER_CHAR;
/// Ticket side margin.
pub(crate) const TICKET_MARGIN_X: i64 = 16;

/// Code128 width for the v2 barcode (1-dot narrow bar, 2-dot wide bar:
/// modules are 11 per character + 35 overhead, average module 1.5 dots —
/// rounded UP to 2 so the centering reserve is conservative).
pub(crate) fn barcode_width_dots_v2(payload_len_chars: usize) -> usize {
    let modules = payload_len_chars * CODE128_MODULES_PER_CHAR + CODE128_OVERHEAD_MODULES;
    modules * 2
}

pub(crate) fn sanitize_tspl_text(value: &str) -> String {
    value.replace('"', "'").replace(['\r', '\n'], " ")
}

/// Code128 payload guard: printable ASCII only (0x20..=0x7E), excluding
/// `"` (0x22, unquotable in TSPL — see module docs). Never mojibake.
pub(crate) fn validate_code128_payload(payload: &str) -> Result<(), DomainError> {
    let ok = payload
        .bytes()
        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
        && !payload.bytes().any(|byte| byte == b'"');
    if ok {
        Ok(())
    } else {
        Err(DomainError::TsplBarcodeUnencodable)
    }
}

/// Barcode width in dots for a payload at the fixed narrow width.
pub(crate) fn barcode_width_dots(payload_len_chars: usize) -> usize {
    (payload_len_chars * CODE128_MODULES_PER_CHAR + CODE128_OVERHEAD_MODULES) * BARCODE_NARROW_DOTS
}

/// Truncate to a field width in characters (dot budget / font width),
/// marking with ASCII "...". Character-based so multi-byte scripts are
/// not penalized.
pub(crate) fn truncate_to_width(value: &str, max_chars: usize) -> String {
    let clean = sanitize_tspl_text(value);
    if clean.chars().count() <= max_chars {
        return clean;
    }
    if max_chars <= 3 {
        return clean.chars().take(max_chars).collect();
    }
    let kept: String = clean.chars().take(max_chars - 3).collect();
    format!("{kept}...")
}

/// Dots a text run occupies (monospace approximation, byte-pinned).
pub(crate) fn text_dots(chars: usize, font_w: i64, scale: i64) -> i64 {
    chars as i64 * font_w * scale
}

/// Centered x inside the media width, clamped at 0.
pub(crate) fn centered_x(media: i64, dots: i64) -> i64 {
    ((media - dots) / 2).max(0)
}

/// x such that the run ends at the right edge, clamped at 0.
pub(crate) fn right_x(right_edge: i64, dots: i64) -> i64 {
    (right_edge - dots).max(0)
}

/// Emit one TEXT command. TEMPORARY-DIAGNOSTIC (BRK-100): `Font1Only` forces
/// face "1"; `AsciiOnly` folds the payload LOSSY (see `ascii_probe_lossy`).
pub(crate) fn push_text(
    out: &mut Vec<String>,
    x: i64,
    y: i64,
    font: &str,
    scale: i64,
    content: &str,
    probe: TsplProbeMode,
) {
    let face = if probe == TsplProbeMode::Font1Only {
        "1"
    } else {
        font
    };
    if probe == TsplProbeMode::AsciiOnly {
        let lossy = super::probe::ascii_probe_lossy(content);
        out.push(format!(
            "TEXT {x},{y},\"{face}\",0,{scale},{scale},\"{lossy}\""
        ));
    } else {
        out.push(format!(
            "TEXT {x},{y},\"{face}\",0,{scale},{scale},\"{content}\""
        ));
    }
}
