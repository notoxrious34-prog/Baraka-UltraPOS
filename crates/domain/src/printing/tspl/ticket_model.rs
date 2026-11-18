//! TSPL input transcription structs: ticket + label models, captions.
//!
//! Structured input in, no bytes out. The use-cases transcribe stored
//! values into these structs; plans and renderers read them.

use super::media::TsplMediaSetup;

/// Reformat a stored RFC3339 instant (`2026-11-07T19:20:04.123+00:00`) into
/// the ticket wall-clock form `07/11/2026 19:20` (DOC-07). Strict parse of
/// the `YYYY-MM-DDTHH:MM:SS` prefix — the wall clock is restated, never
/// converted (no timezone guess). Unparseable input returns verbatim so a
/// ticket never fails to print over cosmetics.
pub fn format_ticket_instant(rfc3339: &str) -> String {
    let bytes = rfc3339.as_bytes();
    let is_digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    let sep = |i: usize, c: u8| bytes.get(i) == Some(&c);
    let mut positions_ok = bytes.len() >= 19
        && sep(4, b'-')
        && sep(7, b'-')
        && sep(10, b'T')
        && sep(13, b':')
        && sep(16, b':');
    for i in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        positions_ok = positions_ok && is_digit(i);
    }
    if !positions_ok {
        return rfc3339.to_string();
    }
    format!(
        "{}/{}/{} {}:{}",
        &rfc3339[8..10],
        &rfc3339[5..7],
        &rfc3339[0..4],
        &rfc3339[11..13],
        &rfc3339[14..16]
    )
}

/// True only for canonical zero money text (`0.00`, never `-0.00`).
/// Anything unparseable returns false so money rows fail VISIBLE (DOC-07:
///
/// a zero-hide must never swallow a real figure).
pub fn money_text_is_zero(text: &str) -> bool {
    let (whole, frac) = match text.split_once('.') {
        Some(pair) => pair,
        None => return false,
    };
    frac.len() == 2
        && !whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit())
        && whole.bytes().all(|b| b == b'0')
        && frac.bytes().all(|b| b == b'0')
}

/// A silent 40x20 mm shelf label, pre-formatted transcription.
pub struct TsplLabel<'a> {
    /// Brand micro-line (collapses when blank).
    pub brand_text: &'a str,
    /// Display name (single locale, chosen upstream), 2 lines max.
    pub name: &'a str,
    /// Plain price text, e.g. `"1234.50"`.
    pub price_text: &'a str,
    /// Currency code printed right of the price (collapses when blank).
    pub currency_text: &'a str,
    /// Code128 payload (SKU/barcode).
    pub barcode: &'a str,
    /// Copies, 1..=99 (enforced in `PRINT 1,n`, never by resending).
    pub copies: u32,
    /// Media identity for the self-describing preamble (validated).
    pub media: TsplMediaSetup,
}

/// Caption language for the ticket static labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsplCaptionLang {
    French,
    Arabic,
}

/// One ticket line, pre-formatted transcription of stored values.
pub struct TsplTicketLine<'a> {
    /// SKU as stored (`"—"` orphan fallback, graphique parity).
    pub sku: &'a str,
    /// Designation (locale resolved upstream, orphan fallback applied).
    pub designation: &'a str,
    /// Plain quantity, e.g. `"3"`.
    pub quantity_text: &'a str,
    /// Selling-unit label, e.g. `"Pièce"`.
    pub unit_text: &'a str,
    /// Net unit price text (discounted PU when stored, raw legacy PU).
    pub unit_price_text: &'a str,
    /// Stored line total text.
    pub line_total_text: &'a str,
    /// Per-unit discount badge text, or `None` for legacy NULL rows.
    pub discount_text: Option<&'a str>,
}

/// Stored ticket totals, pre-formatted (graphique-wire parity plus the
/// stored global discount — no recompute).
pub struct TsplTicketTotals<'a> {
    pub subtotal_ht: &'a str,
    /// Stored total global discount (`global_discount_centimes`).
    pub global_discount: &'a str,
    pub tva: &'a str,
    pub total_ttc: &'a str,
    pub paid: &'a str,
    pub change: &'a str,
    /// Payment method code as stored (e.g. `CASH`, `MIXED`).
    pub payment_method: &'a str,
    /// Frozen wholesale credit equation (DSN-103): `None` = cash-only or
    /// legacy document — the four witness rows collapse, never zeroed.
    pub credit: Option<TsplTicketCredit<'a>>,
}

/// Four frozen credit-equation figures, pre-formatted (DSN-103).
pub struct TsplTicketCredit<'a> {
    /// Customer balance before this document.
    pub ancien: &'a str,
    /// Document grand total.
    pub montant: &'a str,
    /// Non-credit tender (`Montant − |delta|`, never the paid column).
    pub versement: &'a str,
    /// Customer balance after this document.
    pub nouveau: &'a str,
}

/// A silent 80 mm ticket with the full BRK-098 receipt anatomy.
pub struct TsplTicket<'a> {
    pub store_name: &'a str,
    pub store_address: &'a str,
    pub store_phone: &'a str,
    /// Tax-id rows `(prefix, value)`; blank values collapse the row.
    pub tax_rows: &'a [(&'a str, &'a str)],
    /// Ticket number (also the barcode payload, charset-guarded).
    pub number: &'a str,
    /// Creation instant, pre-formatted (RFC3339 millis, worker parity).
    pub created_at: &'a str,
    /// Acting operator identifier (stored value, verbatim).
    pub cashier_text: &'a str,
    /// Selling warehouse identifier (stored value, verbatim).
    pub warehouse_text: &'a str,
    pub captions: TsplCaptionLang,
    pub lines: &'a [TsplTicketLine<'a>],
    pub totals: TsplTicketTotals<'a>,
    /// Footer message (empty lines skipped).
    pub footer: &'a str,
    /// Reprint stamp (DOC-05): when true the plan carries a DUPLICATA band
    /// (FR TEXT, AR BITMAP) so a reprint can never pass as an original.
    pub is_reprint: bool,
    /// Media identity for the self-describing preamble (validated;
    /// gap is forced to 0 = continuous for tickets).
    pub media: TsplMediaSetup,
}

/// Static captions per language (Arabic shaping caveat: see module docs).
pub(crate) struct Captions {
    pub(crate) subtotal: &'static str,
    pub(crate) discount_total: &'static str,
    pub(crate) tva: &'static str,
    pub(crate) total: &'static str,
    pub(crate) paid: &'static str,
    pub(crate) change: &'static str,
    pub(crate) thanks: &'static str,
    pub(crate) column_header: &'static str,
    pub(crate) discount_word: &'static str,
    pub(crate) depot_word: &'static str,
    pub(crate) cashier_word: &'static str,
    /// Reprint band word (DOC-05): FR TEXT, AR BITMAP via the rasterer.
    pub(crate) reprint_mark: &'static str,
    /// DOC-07 v2 three-column header segments.
    pub(crate) column_article: &'static str,
    pub(crate) column_qty_price: &'static str,
    pub(crate) column_total: &'static str,
    /// DSN-103 wholesale credit-equation captions.
    pub(crate) ancien_solde: &'static str,
    pub(crate) montant_document: &'static str,
    pub(crate) versement: &'static str,
    pub(crate) nouveau_solde: &'static str,
}

pub(crate) fn captions(lang: TsplCaptionLang) -> Captions {
    match lang {
        TsplCaptionLang::French => Captions {
            subtotal: "Sous-total",
            discount_total: "Remise totale",
            tva: "TVA",
            total: "TOTAL TTC",
            paid: "Payé",
            change: "Rendu",
            thanks: "Merci de votre visite",
            column_header: "Qté x PU = Total",
            discount_word: "remise",
            depot_word: "Depot",
            cashier_word: "Caissier",
            reprint_mark: "DUPLICATA",
            column_article: "Article",
            column_qty_price: "Qté x P.U",
            column_total: "Total",
            ancien_solde: "Ancien solde",
            montant_document: "Montant document",
            versement: "Versement",
            nouveau_solde: "Nouveau solde",
        },
        TsplCaptionLang::Arabic => Captions {
            subtotal: "المجموع الفرعي",
            discount_total: "الخصم الإجمالي",
            tva: "الضريبة",
            total: "الإجمالي",
            paid: "المدفوع",
            change: "الباقي",
            thanks: "شكراً لزيارتكم",
            column_header: "الكمية × سعر = المجموع",
            discount_word: "خصم",
            depot_word: "المخزن",
            cashier_word: "الكاشير",
            reprint_mark: "نسخة",
            column_article: "الصنف",
            column_qty_price: "الكمية × السعر",
            column_total: "المجموع",
            ancien_solde: "الرصيد السابق",
            montant_document: "مبلغ الوثيقة",
            versement: "الدفعة",
            nouveau_solde: "الرصيد الجديد",
        },
    }
}
