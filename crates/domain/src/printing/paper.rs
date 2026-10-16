//! Thermal paper format (TASK-BRK-070): Engineering Debt #6.
//!
//! The ESC/POS raster surface was hardcoded to 576 dots (80mm @ 203dpi);
//! 58mm printers clipped ~33% of every receipt. Paper width is a domain
//! fact (millimeters configured by the merchant), dot geometry a pure
//! function of it. Validation accepts only the two stocked formats.

use crate::errors::DomainError;

/// Stocked thermal paper formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperWidth {
    /// 80mm @ 203dpi (default).
    Width80mm,
    /// 58mm @ 203dpi.
    Width58mm,
}

impl PaperWidth {
    /// Printable raster width in dots.
    pub const fn dot_width(&self) -> u32 {
        match self {
            Self::Width80mm => 576,
            Self::Width58mm => 384,
        }
    }

    /// Monospace text columns the format comfortably holds.
    pub const fn char_columns(&self) -> usize {
        match self {
            Self::Width80mm => 42,
            Self::Width58mm => 32,
        }
    }

    /// Configured millimeters resolve to the nearest stocked format:
    /// anything up to 58mm prints on the narrow canvas, above on wide.
    pub fn from_mm(mm: u32) -> Self {
        if mm <= 58 {
            Self::Width58mm
        } else {
            Self::Width80mm
        }
    }

    /// Canonical millimeters for persistence and display.
    pub const fn as_mm(&self) -> u32 {
        match self {
            Self::Width80mm => 80,
            Self::Width58mm => 58,
        }
    }
}

/// Strict validation for the persisted setting: only 58 or 80 are
/// stocked; anything else is rejected before any write.
pub fn validate_paper_width_mm(mm: u32) -> Result<PaperWidth, DomainError> {
    match mm {
        58 => Ok(PaperWidth::Width58mm),
        80 => Ok(PaperWidth::Width80mm),
        other => Err(DomainError::InvalidPaperWidthMillimeters(other)),
    }
}
