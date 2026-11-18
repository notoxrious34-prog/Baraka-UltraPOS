//! TSPL probe switch + lossy diagnostic fold (BRK-100, temporary).
//!
//! Pure: wire spelling parse + ASCII fold. The renderer consults the mode;
//! this module never emits bytes.

use crate::errors::DomainError;

/// TEMPORARY-DIAGNOSTIC probe switch (BRK-100 — REMOVE in the lock-in task).
/// One owner trial discriminates the TEXT-rejection cause (AUD-047 P1–P3):
/// `Normal` = today's bytes; `NoCodepage` = drop the CODEPAGE line (P1);
/// `AsciiOnly` = LOSSY fold of non-ASCII payload bytes to `?` (P2 — never a
/// default, diagnostic only); `Font1Only` = force font "1" everywhere (P3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TsplProbeMode {
    /// Production bytes (byte-identical to pre-probe output).
    #[default]
    Normal,
    /// P1: omit `CODEPAGE UTF-8` from the preamble.
    NoCodepage,
    /// P2: LOSSY — every non-ASCII scalar becomes `?`.
    AsciiOnly,
    /// P3: every TEXT line uses built-in font "1".
    Font1Only,
}

/// Canonical wire spelling of the default probe mode.
pub const TSPL_PROBE_MODE_DEFAULT: &str = "normal";

/// Parse a stored probe-mode string; unknown values reject LOUDLY with
/// `TSPL_PROBE_MODE_UNKNOWN` — never a silent fallback to `Normal`.
pub fn parse_tspl_probe_mode(raw: &str) -> Result<TsplProbeMode, DomainError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "normal" => Ok(TsplProbeMode::Normal),
        "no_codepage" => Ok(TsplProbeMode::NoCodepage),
        "ascii_only" => Ok(TsplProbeMode::AsciiOnly),
        "font1_only" => Ok(TsplProbeMode::Font1Only),
        _ => Err(DomainError::InvalidCanonicalString(
            "TSPL_PROBE_MODE_UNKNOWN".to_string(),
        )),
    }
}

impl TsplProbeMode {
    /// Canonical wire spelling (`normal|no_codepage|ascii_only|font1_only`).
    pub fn as_str(self) -> &'static str {
        match self {
            TsplProbeMode::Normal => "normal",
            TsplProbeMode::NoCodepage => "no_codepage",
            TsplProbeMode::AsciiOnly => "ascii_only",
            TsplProbeMode::Font1Only => "font1_only",
        }
    }
}

/// TEMPORARY-DIAGNOSTIC LOSSY fold (BRK-100 P2 — remove in lock-in): every
/// non-ASCII scalar becomes `?`. Layout widths stay computed on the original
/// text, so alignment may shift — acceptable: the probe asks print-vs-ignore,
/// never beauty.
pub(crate) fn ascii_probe_lossy(content: &str) -> String {
    content
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect()
}
