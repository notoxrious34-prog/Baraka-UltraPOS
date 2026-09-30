//! Bounded ESC/POS decoder: interprets ONLY the command subset our
//! raster renderer emits (`GS v 0` bands + trailing `LF` feed).
//!
//! Pure byte parsing (no dependencies): anything else becomes an
//! [`DecodedLine::Unknown`] hex marker — the decoder reports structure,
//! never errors, and never panics on any input. Honesty boundary: this
//! verifies OUR bytes and layout deterministically; it does not verify
//! physical hardware.

use crate::printing::DecodedLine;

/// Flush the pending unknown span as one hex marker.
fn flush_unknown(
    unknown_start: &mut Option<usize>,
    bytes: &[u8],
    up_to: usize,
    lines: &mut Vec<DecodedLine>,
) {
    if let Some(start) = unknown_start.take() {
        let span = &bytes[start..up_to.min(bytes.len())];
        if !span.is_empty() {
            lines.push(DecodedLine::Unknown {
                hex: span
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<Vec<_>>()
                    .join(" "),
            });
        }
    }
}

/// Decode a captured byte stream into structural lines.
#[must_use]
pub fn escpos_decode(bytes: &[u8]) -> Vec<DecodedLine> {
    let mut lines = Vec::new();
    let mut cursor = 0usize;
    let mut unknown_start: Option<usize> = None;

    while cursor < bytes.len() {
        // `GS v 0 m xL xH yL yH` raster band header (8 bytes).
        if bytes[cursor] == 0x1D
            && cursor + 8 <= bytes.len()
            && bytes[cursor + 1] == 0x76
            && bytes[cursor + 2] == 0x30
            && bytes[cursor + 3] == 0x00
        {
            flush_unknown(&mut unknown_start, bytes, cursor, &mut lines);
            let row_bytes = u16::from_le_bytes([bytes[cursor + 4], bytes[cursor + 5]]) as usize;
            let band_rows = u16::from_le_bytes([bytes[cursor + 6], bytes[cursor + 7]]) as usize;
            let data_len = row_bytes.saturating_mul(band_rows);
            let data_start = cursor + 8;
            let data_end = data_start.saturating_add(data_len).min(bytes.len());
            let data = &bytes[data_start..data_end];
            let total_bits = data.len().saturating_mul(8);
            let dark = data
                .iter()
                .map(|byte| byte.count_ones() as usize)
                .sum::<usize>();
            let dark_per_mille = if total_bits == 0 {
                0
            } else {
                ((dark as u64).saturating_mul(1000) / total_bits as u64).min(1000) as u32
            };
            lines.push(DecodedLine::RasterBand {
                width_dots: row_bytes.saturating_mul(8) as u32,
                height_dots: band_rows as u32,
                dark_per_mille,
            });
            // A short read means truncation: the tail becomes Unknown.
            cursor = data_end;
            if data_end - data_start < data_len {
                continue;
            }
            continue;
        }
        // Trailing paper feed (`LF` run).
        if bytes[cursor] == 0x0A {
            flush_unknown(&mut unknown_start, bytes, cursor, &mut lines);
            let mut run = 0u32;
            while cursor < bytes.len() && bytes[cursor] == 0x0A {
                run += 1;
                cursor += 1;
            }
            lines.push(DecodedLine::Feed { lines: run });
            continue;
        }
        if unknown_start.is_none() {
            unknown_start = Some(cursor);
        }
        cursor += 1;
    }
    flush_unknown(&mut unknown_start, bytes, bytes.len(), &mut lines);
    lines
}
