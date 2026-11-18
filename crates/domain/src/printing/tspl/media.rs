//! TSPL media identity + geometry + self-describing preamble (BRK-099).
//!
//! Screen geometry (203 dpi ≈ 8 dots/mm EXACT per task spec):
//! label 40x20 mm -> 320 x 160 dots; ticket 80 mm wide -> 640 dots.

use super::probe::TsplProbeMode;
use crate::errors::DomainError;

/// Dots per millimeter at 203 dpi (task-mandated exact integer).
pub const DOTS_PER_MM: i64 = 8;
/// Label media width in dots (40 mm).
pub const LABEL_WIDTH_DOTS: i64 = 320;
/// Label media height in dots (20 mm).
pub const LABEL_HEIGHT_DOTS: i64 = 160;
/// Ticket media width in dots (80 mm).
pub const TICKET_WIDTH_DOTS: i64 = 640;
/// Copies floor (a label run prints at least one label).
pub const TSPL_MIN_COPIES: u32 = 1;
/// Copies ceiling (PRINT m,n single-byte field discipline, stated).
pub const TSPL_MAX_COPIES: u32 = 99;

/// Label inter-label gap default in mm — STARTING VALUE, OWNER-VERIFY with
/// a ruler against the 40x20 liner (BRK-099). The measured value wins once
/// collected; this default never silently becomes truth.
pub const TSPL_GAP_MM_DEFAULT: u32 = 3;
/// Label gap ceiling in mm (die-cut desktop stock never exceeds this).
pub const TSPL_GAP_MM_MAX: u32 = 10;
/// Print density default (TSC 0..=15 scale, mid-dark for direct-thermal).
pub const TSPL_DENSITY_DEFAULT: u8 = 8;
/// Print density ceiling (TSC scale maximum).
pub const TSPL_DENSITY_MAX: u8 = 15;
/// Print speed default, inch/sec (conservative for 203 dpi text+barcode).
pub const TSPL_SPEED_DEFAULT: u8 = 4;
/// Print speed floor, inch/sec.
pub const TSPL_SPEED_MIN: u8 = 1;
/// Print speed ceiling, inch/sec (XP-410B class maximum).
pub const TSPL_SPEED_MAX: u8 = 6;
/// Calibration range in mm — STARTING VALUES, OWNER-VERIFY with a ruler
/// against the 40x20 liner (DOC-06, same posture as the gap default).
/// ±5 mm keeps any label content printable; the renderer rejects LOUDLY
/// past the media edge instead of clipping silently.
pub const TSPL_CALIB_MM_MIN: i32 = -5;
/// Calibration ceiling in mm (see [`TSPL_CALIB_MM_MIN`]).
pub const TSPL_CALIB_MM_MAX: i32 = 5;
/// Dots per mm at 203 dpi — the calibration step unit.
pub const TSPL_DOTS_PER_MM: i64 = 8;
/// Fixed charset declaration. UTF-8 — NOT 437: our TEXT bytes ARE UTF-8
/// (French accents are multibyte; CP437 would render them as mojibake),
/// and UTF-8 keeps Arabic codepoint-accurate (unshaped per AUD-045 —
/// trial 4 decides the BITMAP fallback). Printer acceptance is
/// OWNER-VERIFY: if the XP-410B rejects the line, drop to no-CODEPAGE.
pub const TSPL_CODEPAGE_LINE: &str = "CODEPAGE UTF-8";

/// Self-describing media setup carried by every silent job — no NVRAM luck
/// (BRK-099, AUD-045). Label jobs use the stored gap; ticket jobs force
/// gap 0 (`GAP 0,0` = declared continuous).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TsplMediaSetup {
    /// Inter-label gap in mm, 0..=10 (`0` = continuous, ticket only).
    pub gap_mm: u32,
    /// Print density, 0..=15.
    pub density: u8,
    /// Print speed in inch/sec, 1..=6.
    pub speed: u8,
    /// Calibration shift in mm, -5..=+5 per axis (DOC-06): applied in
    /// software to every emitted label coordinate (both directions —
    /// TSPL REFERENCE has no signed form and stays vendor-pinned).
    /// Zero keeps every golden byte-identical.
    pub offset_x_mm: i32,
    /// Calibration shift in mm on the feed axis (see `offset_x_mm`).
    pub offset_y_mm: i32,
    /// TEMPORARY-DIAGNOSTIC probe mode (BRK-100 — remove in lock-in).
    pub probe_mode: TsplProbeMode,
}

impl Default for TsplMediaSetup {
    fn default() -> Self {
        Self {
            gap_mm: TSPL_GAP_MM_DEFAULT,
            density: TSPL_DENSITY_DEFAULT,
            speed: TSPL_SPEED_DEFAULT,
            offset_x_mm: 0,
            offset_y_mm: 0,
            probe_mode: TsplProbeMode::Normal,
        }
    }
}

/// Reject a label gap outside 0..=10 mm.
pub fn validate_tspl_gap_mm(gap_mm: u32) -> Result<(), DomainError> {
    if gap_mm > TSPL_GAP_MM_MAX {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_GAP_MM_OUT_OF_RANGE".to_string(),
        ));
    }
    Ok(())
}

/// Reject a density outside the 0..=15 TSC scale.
pub fn validate_tspl_density(density: u8) -> Result<(), DomainError> {
    if density > TSPL_DENSITY_MAX {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_DENSITY_OUT_OF_RANGE".to_string(),
        ));
    }
    Ok(())
}

/// Reject a speed outside 1..=6 inch/sec.
pub fn validate_tspl_speed(speed: u8) -> Result<(), DomainError> {
    if !(TSPL_SPEED_MIN..=TSPL_SPEED_MAX).contains(&speed) {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_SPEED_OUT_OF_RANGE".to_string(),
        ));
    }
    Ok(())
}

/// Reject a calibration shift outside -5..=+5 mm (DOC-06).
pub fn validate_tspl_calib_mm(value_mm: i32) -> Result<(), DomainError> {
    if !(TSPL_CALIB_MM_MIN..=TSPL_CALIB_MM_MAX).contains(&value_mm) {
        return Err(DomainError::InvalidCanonicalString(
            "TSPL_CALIB_MM_OUT_OF_RANGE".to_string(),
        ));
    }
    Ok(())
}

impl TsplMediaSetup {
    /// Reject any out-of-range field (codes quoted, never silent clamps).
    pub fn validate(&self) -> Result<(), DomainError> {
        validate_tspl_gap_mm(self.gap_mm)?;
        validate_tspl_density(self.density)?;
        validate_tspl_speed(self.speed)?;
        validate_tspl_calib_mm(self.offset_x_mm)?;
        validate_tspl_calib_mm(self.offset_y_mm)?;
        Ok(())
    }

    /// Calibration shift in dots (DOC-06): added to every emitted label
    /// coordinate. Zero for the default setup (goldens unaffected).
    #[must_use]
    pub fn calibration_dots(&self) -> (i64, i64) {
        (
            i64::from(self.offset_x_mm) * TSPL_DOTS_PER_MM,
            i64::from(self.offset_y_mm) * TSPL_DOTS_PER_MM,
        )
    }
}

/// Emit the vendor-mirrored preamble in pinned order (Seagull ground truth,
/// `Vendor-Test.prn` 594 B, 2026-11-01): SIZE → GAP → SET RIBBON OFF →
/// DIRECTION 0,0 → REFERENCE 0,0 → OFFSET 0 mm → SET PEEL OFF →
/// SET CUTTER OFF → SET TEAR ON → CLS. The vendor sends NO
/// DENSITY/SPEED/CODEPAGE — the printer runs its calibrated NVRAM values;
/// our overrides are no longer emitted (stored keys kept, unemitted).
/// P1 NoCodepage is a preamble no-op now (nothing to drop); P2/P3 still
/// fold TEXT payloads only.
pub(crate) fn push_media_preamble(
    out: &mut Vec<String>,
    size_line: &str,
    media: &TsplMediaSetup,
) -> Result<(), DomainError> {
    media.validate()?;
    out.push(size_line.to_string());
    if media.gap_mm == 0 {
        // Continuous stock: the manual's no-gap form (no units).
        out.push("GAP 0,0".to_string());
    } else {
        // Metric gap REQUIRES the mm suffix on both values (TSC manual:
        // `GAP 3 mm,0 mm`). Bare `GAP 3,0` reads as 3 INCHES.
        out.push(format!("GAP {} mm, 0 mm", media.gap_mm));
    }
    out.push("SET RIBBON OFF".to_string());
    out.push("DIRECTION 0,0".to_string());
    out.push("REFERENCE 0,0".to_string());
    out.push("OFFSET 0 mm".to_string());
    out.push("SET PEEL OFF".to_string());
    out.push("SET CUTTER OFF".to_string());
    out.push("SET TEAR ON".to_string());
    out.push("CLS".to_string());
    Ok(())
}
