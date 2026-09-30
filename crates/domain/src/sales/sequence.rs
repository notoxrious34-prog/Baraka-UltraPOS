//! Gapless document numbering (ADR-008).
//!
//! Numbers allocate inside the sale transaction (`BEGIN IMMEDIATE`), so a
//! rolled-back sale never consumes a number: committed numbers are gapless
//! by construction.

use crate::errors::DomainError;

/// Hard cap of the 6-digit display format.
pub const MAX_SEQUENCE_NUMBER: i64 = 999_999;

/// Next sequence value after `last`, rejecting display overflow.
pub fn next_number(last: i64) -> Result<i64, DomainError> {
    if !(0..MAX_SEQUENCE_NUMBER).contains(&last) {
        return Err(DomainError::GaplessSequenceOverflow);
    }
    Ok(last + 1)
}

/// Human number: `{PREFIX}-{YEAR}-{NUMBER:06}` (e.g. `TICK-2026-000001`).
#[must_use]
pub fn format_number(prefix: &str, year: i32, number: i64) -> String {
    format!("{prefix}-{year}-{number:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_shapes() {
        assert_eq!(next_number(0), Ok(1));
        assert_eq!(next_number(41), Ok(42));
        assert_eq!(
            next_number(MAX_SEQUENCE_NUMBER),
            Err(DomainError::GaplessSequenceOverflow)
        );
        assert_eq!(format_number("TICK", 2026, 1), "TICK-2026-000001");
        assert_eq!(format_number("FAC", 2026, 123456), "FAC-2026-123456");
    }
}
