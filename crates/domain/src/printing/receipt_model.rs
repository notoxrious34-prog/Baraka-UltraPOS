//! Printer-agnostic receipt content model.
//!
//! Built verbatim from already-persisted `Document`/`DocumentItem` rows.
//! Zero financial recomputation happens here: every amount string is
//! transcribed from storage (Article 6.2.4). Numeric presentation uses
//! plain decimal strings with Western digits in both languages.

use serde::{Deserialize, Serialize};

/// Receipt language (from the stored `document_language`, never the UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReceiptLanguage {
    /// Right-to-left Arabic layout.
    Arabic,
    /// Left-to-right French layout.
    French,
}

impl ReceiptLanguage {
    /// Resolve from a stored `document_language` value.
    #[must_use]
    pub fn from_document_language(code: &str) -> Self {
        if code.trim().to_ascii_uppercase().starts_with("AR") {
            Self::Arabic
        } else {
            Self::French
        }
    }
}

/// One printable item row (transcribed, never recomputed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptLine {
    /// Product designation in the receipt language.
    pub designation: String,
    /// Quantity display text (e.g. `"2"`, `"2.5"`).
    pub quantity_text: String,
    /// Unit price display text (e.g. `"122.50"`).
    pub unit_price_text: String,
    /// Line total display text.
    pub total_text: String,
}

/// Receipt totals block (transcribed centime strings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptTotals {
    /// HT subtotal after all discounts.
    pub subtotal_ht: String,
    /// Total TVA.
    pub tva: String,
    /// Total global discount allocated.
    pub discount: String,
    /// Timbre fiscal.
    pub timbre: String,
    /// Grand total.
    pub total_ttc: String,
    /// Total tendered.
    pub paid: String,
    /// Change owed.
    pub change: String,
}

/// Store identity + ticket footer (TASK-BRK-056, ADR-065).
///
/// Persisted as namespaced `store.*` keys in the existing key-value
/// `settings` table — zero schema changes. Identifier FORMAT enforcement
/// is deferred to the ADR-013 tax/legal gate; these are free-text values
/// with length caps only (soft validation in the application layer).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StoreIdentity {
    /// Shop display name (required, trimmed).
    pub store_name: String,
    /// Street address line.
    pub store_address: String,
    /// Phone number.
    pub store_phone: String,
    /// Numéro d'Identification Fiscale.
    pub nif: String,
    /// Numéro d'Identification Statistique.
    pub nis: String,
    /// Numéro de Registre de Commerce.
    pub rc: String,
    /// Article d'Imposition.
    pub ai: String,
    /// Free-text footer (up to 3 `\n`-separated lines).
    pub ticket_footer_message: String,
}

/// Length caps (soft validation; see [`StoreIdentity`]).
pub mod identity_caps {
    /// Maximum chars for [`super::StoreIdentity::store_name`].
    pub const STORE_NAME: usize = 80;
    /// Maximum chars for [`super::StoreIdentity::store_address`].
    pub const STORE_ADDRESS: usize = 120;
    /// Maximum chars for [`super::StoreIdentity::store_phone`].
    pub const STORE_PHONE: usize = 40;
    /// Maximum chars for each fiscal identifier.
    pub const FISCAL_ID: usize = 20;
    /// Maximum chars for [`super::StoreIdentity::ticket_footer_message`].
    pub const FOOTER_MESSAGE: usize = 160;
    /// Maximum footer lines.
    pub const FOOTER_LINES: usize = 3;
}

/// Monospace preview line: the shared layout blueprint before raster.
///
/// Rendered from the SAME block functions the renderer consumes
/// (Amendment 1: preview source = layout model, never decoded pixels).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewLine {
    /// `text` | `rule` | `gap`.
    pub kind: String,
    /// Centered (headers) or edge-aligned (body).
    pub centered: bool,
    /// Pixel em size used for shaping.
    pub size_px: f32,
    /// Concatenated run texts (`""` for rules/gaps).
    pub text: String,
}

/// Structural verdict over a captured artifact (decoder scope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecodeSummary {
    /// Raster band count (must be > 0 for our streams).
    pub bands: u32,
    /// Trailing feed lines.
    pub feed_lines: u32,
    /// Unknown-byte spans (must be 0 for our streams).
    pub unknown_spans: u32,
}
/// One decoded ESC/POS structural line (bounded decoder output).
///
/// The raster renderer emits graphics only (`GS v 0` bands + feed), so
/// the line model is structural, never textual: band geometry replaces
/// text lines. Unknown bytes surface as hex markers, never errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecodedLine {
    /// `GS v 0` raster band: width/height in dots + dark-pixel ratio
    /// (0–1000, permille) so previews can show density bars.
    RasterBand {
        /// Band width in dots.
        width_dots: u32,
        /// Band height in dots.
        height_dots: u32,
        /// Dark pixels per mille.
        dark_per_mille: u32,
    },
    /// Line-feed run (trailing paper feed).
    Feed {
        /// Number of `LF` bytes.
        lines: u32,
    },
    /// Bytes outside the emitted subset (`[HEX xx ..]`).
    Unknown {
        /// Lowercase hex of the uninterpreted span.
        hex: String,
    },
}
/// Full printable receipt content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptDocument {
    /// Layout language.
    pub language: ReceiptLanguage,
    /// Shop header line.
    pub shop_name: String,
    /// Gapless human number.
    pub number: String,
    /// Capture instant display text.
    pub created_at: String,
    /// Item rows in display order.
    pub lines: Vec<ReceiptLine>,
    /// Totals block.
    pub totals: ReceiptTotals,
    /// Tender lines as `(method code, amount text)`.
    pub payments: Vec<(String, String)>,
    /// True for reprints (DUPLICATA watermark rendered).
    pub is_duplicata: bool,
    /// Store identity block (TASK-BRK-056). Empty `store_name` selects
    /// the legacy single-line `shop_name` header (byte-stable fallback).
    pub store_identity: StoreIdentity,
    /// Free-text footer (TASK-BRK-056). Empty selects the legacy thanks
    /// line (byte-stable fallback).
    pub footer_message: String,
}

/// Format integer centimes as plain `"122.50"` display text.
#[must_use]
pub fn format_centimes_plain(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// Format integer micros as trimmed decimal display text.
#[must_use]
pub fn format_qty_plain(micros: i64) -> String {
    let sign = if micros < 0 { "-" } else { "" };
    let abs = micros.unsigned_abs();
    let whole = abs / 10_000;
    let frac = format!("{:04}", abs % 10_000);
    let trimmed = frac.trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_formatters() {
        assert_eq!(format_centimes_plain(12_250), "122.50");
        assert_eq!(format_centimes_plain(5), "0.05");
        assert_eq!(format_centimes_plain(-12_505), "-125.05");
        assert_eq!(format_qty_plain(20_000), "2");
        assert_eq!(format_qty_plain(25_000), "2.5");
        assert_eq!(format_qty_plain(10_001), "1.0001");
        assert_eq!(
            ReceiptLanguage::from_document_language("ar-DZ"),
            ReceiptLanguage::Arabic
        );
        assert_eq!(
            ReceiptLanguage::from_document_language("FR"),
            ReceiptLanguage::French
        );
    }
}
