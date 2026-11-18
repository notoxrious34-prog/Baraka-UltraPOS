//! Silent raster-only jobs (BRK-101): pure parameters in, bytes out.
//!
//! The TEXT path is DEAD here — raster only. Refuses, never truncates.

use super::media::*;
use crate::errors::DomainError;

/// BITMAP raster bounds (BRK-101; architect-verified TSC reference
/// `BITMAP X,Y,width,height,mode,data`: width IN BYTES, height IN DOTS,
/// mode 0 OVERWRITE, raw binary data after the comma).
/// Width ceiling = 80 mm ticket (640 dots); height ceiling guards the
/// spooler transfer; total ceiling = 128 KiB (ticket ~52 KiB typical).
pub const BITMAP_MAX_WIDTH_DOTS: usize = 640;
pub const BITMAP_MAX_HEIGHT_DOTS: usize = 1638;
pub const BITMAP_MAX_BYTES: usize = 131072;

/// Render a silent raster-only job (BRK-101 — TEXT path is DEAD, owner
/// all-red on P1–P3): self-describing preamble
/// SIZE → GAP → DENSITY → SPEED → DIRECTION → CLS (**NO CODEPAGE — zero
/// text bytes anywhere**), then `BITMAP x,y,w,h,0,<raw>`, then `PRINT 1,n`.
/// SIZE is derived from the raster geometry (millimeters, integer math).
/// Pure: parameters in, bytes out. Refuses, never truncates.
#[allow(clippy::too_many_arguments)]
pub fn render_bitmap_job(
    media: &TsplMediaSetup,
    x: i64,
    y: i64,
    width_bytes: usize,
    height_rows: usize,
    data: &[u8],
    copies: u32,
) -> Result<Vec<u8>, DomainError> {
    media.validate()?;
    let code = |name: &str| DomainError::InvalidCanonicalString(name.to_string());
    if x < 0 || y < 0 {
        return Err(code("BITMAP_ORIGIN_MUST_BE_NON_NEGATIVE"));
    }
    if width_bytes == 0 || height_rows == 0 {
        return Err(code("BITMAP_DIMENSIONS_MUST_BE_POSITIVE"));
    }
    if width_bytes
        .checked_mul(8)
        .filter(|dots| *dots <= BITMAP_MAX_WIDTH_DOTS)
        .is_none()
    {
        return Err(code("BITMAP_WIDTH_TOO_WIDE"));
    }
    if height_rows > BITMAP_MAX_HEIGHT_DOTS {
        return Err(code("BITMAP_TOO_TALL"));
    }
    let expected = width_bytes
        .checked_mul(height_rows)
        .ok_or_else(|| code("BITMAP_TOO_LARGE"))?;
    if expected > BITMAP_MAX_BYTES {
        return Err(code("BITMAP_TOO_LARGE"));
    }
    if data.len() != expected {
        return Err(code("BITMAP_LENGTH_MISMATCH"));
    }
    // Anti-blank guard: an all-white or all-black frame is a rasterizer bug
    // or a dead transfer — never worth spooling.
    if data.iter().all(|byte| *byte == 0x00) || data.iter().all(|byte| *byte == 0xFF) {
        return Err(code("BITMAP_BLANK_IMAGE"));
    }
    if !(TSPL_MIN_COPIES..=TSPL_MAX_COPIES).contains(&copies) {
        return Err(code("BITMAP_COPIES_OUT_OF_RANGE"));
    }
    // Width is byte-exact so dots (bytes×8) divide evenly into whole mm.
    let width_mm = width_bytes;
    let height_mm = height_rows.div_ceil(DOTS_PER_MM as usize);
    let mut job: Vec<u8> = Vec::new();
    for line in [
        format!("SIZE {width_mm} mm, {height_mm} mm"),
        if media.gap_mm == 0 {
            // Continuous stock: the manual's no-gap form (no units).
            "GAP 0,0".to_string()
        } else {
            // Metric gap REQUIRES the mm suffix (TSC manual: `GAP 3 mm,0 mm`).
            format!("GAP {} mm, 0 mm", media.gap_mm)
        },
        // Vendor-mirrored init (Seagull ground truth): no DENSITY/SPEED/
        // CODEPAGE — the printer runs its calibrated NVRAM values.
        "SET RIBBON OFF".to_string(),
        "DIRECTION 0,0".to_string(),
        "REFERENCE 0,0".to_string(),
        "OFFSET 0 mm".to_string(),
        "SET PEEL OFF".to_string(),
        "SET CUTTER OFF".to_string(),
        "SET TEAR ON".to_string(),
        "CLS".to_string(),
        format!("BITMAP {x},{y},{width_bytes},{height_rows},0,"),
    ] {
        job.extend_from_slice(line.as_bytes());
        if !line.starts_with("BITMAP ") {
            job.extend_from_slice(b"\r\n");
        }
    }
    job.extend_from_slice(data);
    job.extend_from_slice(format!("\r\nPRINT 1,{copies}\r\n").as_bytes());
    Ok(job)
}
