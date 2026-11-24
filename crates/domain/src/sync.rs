//! Two-till sync package: canonical format + renumber planner (TASK-BRK-124).
//!
//! Pure integer math, zero I/O. The canonical v1 package is LF-joined JSONL:
//! a `HDR` header line, then one line per record (`DOC`, `ITEM`, `PAY`).
//! Integrity hashing (SHA-256 over the body bytes) lives in infrastructure,
//! which owns the hasher; this module owns the grammar: building lines,
//! parsing them fail-closed, and planning renumbers so imported documents
//! take the IMPORTER's sequence while preserving the origin number.
//!
//! Money = i64 centimes, quantities = i64 micros. No floats, ever.

use crate::errors::DomainError;

pub const SYNC_FORMAT_VERSION: &str = "SYNC1";
pub const MAX_PACKAGE_DOCS: usize = 5_000;
pub const MAX_PACKAGE_BYTES: usize = 8_000_000;

/// One exported document header (importer resolves catalog identity by SKU).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncDoc {
    pub doc_type_code: String,
    pub fiscal_year: i32,
    pub origin_number: String,
    pub language: String,
    pub party_code: Option<String>,
    pub subtotal_ht_centimes: i64,
    pub vat_total_centimes: i64,
    pub global_discount_centimes: i64,
    pub timbre_centimes: i64,
    pub rounding_centimes: i64,
    pub ttc_centimes: i64,
    pub paid_amount_centimes: i64,
    pub change_amount_centimes: i64,
    pub payment_method: String,
    pub created_at: String,
}

/// One exported line item (identity by SKU, never by UUID).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncItem {
    pub doc_origin_number: String,
    pub sku: String,
    /// Selling-unit micros on the origin till (display reference only).
    pub quantity_micro: i64,
    /// Base-unit micros (authoritative for the importer's stock moves).
    pub quantity_base_micro: i64,
    pub unit_price_centimes: i64,
    pub discount_centimes: i64,
    pub tva_rate_micro: i64,
    pub line_ttc_centimes: i64,
    pub cost_price_centimes: i64,
    /// Selling-unit barcode on the origin till (empty = importer's default
    /// selling unit for the SKU). Identity only — quantities are base micros.
    pub unit_barcode: String,
    pub sort_order: i64,
}

/// One exported payment leg (cash session never travels: money stays put).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPay {
    pub doc_origin_number: String,
    pub method_code: String,
    pub customer_code: Option<String>,
    pub amount_centimes: i64,
}

/// Planned import numbering for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedNumber {
    pub origin_number: String,
    pub new_number: String,
    pub fiscal_year: i32,
}

/// Escape a string field for the pipe-delimited envelope: `\` -> `\\`,
/// `|` -> `\|`, newline -> ` ` (packages are LF-joined, so raw LF is banned).
fn esc(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    for grain in field.chars() {
        match grain {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '\n' | '\r' => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

/// Split an envelope line on unescaped pipes. Fail-closed on trailing escape.
fn split_escaped(line: &str) -> Result<Vec<String>, DomainError> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars();
    while let Some(grain) = chars.next() {
        if grain == '\\' {
            match chars.next() {
                Some(escaped @ ('\\' | '|')) => current.push(escaped),
                _ => {
                    return Err(DomainError::InvalidCanonicalString(
                        "bad escape in sync package".to_string(),
                    ));
                }
            }
        } else if grain == '|' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(grain);
        }
    }
    fields.push(current);
    Ok(fields)
}

fn parse_i64(field: &str, what: &str) -> Result<i64, DomainError> {
    field.parse::<i64>().map_err(|_| {
        DomainError::InvalidCanonicalString(format!("bad integer in sync package ({what})"))
    })
}

fn parse_i32(field: &str, what: &str) -> Result<i32, DomainError> {
    field.parse::<i32>().map_err(|_| {
        DomainError::InvalidCanonicalString(format!("bad integer in sync package ({what})"))
    })
}

/// Build the header line. `body_hash_hex` covers the body bytes (infrastructure).
pub fn build_header(
    origin_terminal: &str,
    exported_at: &str,
    doc_count: usize,
    body_hash_hex: &str,
) -> String {
    format!(
        "HDR|{SYNC_FORMAT_VERSION}|{}|{}|{doc_count}|{body_hash_hex}",
        esc(origin_terminal),
        esc(exported_at)
    )
}

/// Parse + validate the header. Returns (origin_terminal, exported_at, doc_count, hash).
pub fn parse_header(line: &str) -> Result<(String, String, usize, String), DomainError> {
    let bad = || DomainError::InvalidCanonicalString("bad sync header".to_string());
    let fields = split_escaped(line).map_err(|_| bad())?;
    if fields.len() != 6 || fields[0] != "HDR" || fields[1] != SYNC_FORMAT_VERSION {
        return Err(bad());
    }
    let doc_count = fields[4].parse::<usize>().map_err(|_| bad())?;
    if doc_count > MAX_PACKAGE_DOCS {
        return Err(bad());
    }
    if fields[5].len() != 64 || !fields[5].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad());
    }
    Ok((
        fields[2].clone(),
        fields[3].clone(),
        doc_count,
        fields[5].clone(),
    ))
}

/// Build a DOC body line (field order is the canonical order — never reorder).
pub fn build_doc_line(doc: &SyncDoc) -> String {
    format!(
        "DOC|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        esc(&doc.doc_type_code),
        doc.fiscal_year,
        esc(&doc.origin_number),
        esc(&doc.language),
        esc(doc.party_code.as_deref().unwrap_or("")),
        doc.subtotal_ht_centimes,
        doc.vat_total_centimes,
        doc.global_discount_centimes,
        doc.timbre_centimes,
        doc.rounding_centimes,
        doc.ttc_centimes,
        doc.paid_amount_centimes,
        doc.change_amount_centimes,
        esc(&doc.payment_method),
        esc(&doc.created_at),
    )
}

/// Parse a DOC line fail-closed: wrong arity, bad integers, or non-positive
/// totals abort the WHOLE import (never a partial merge).
pub fn parse_doc_line(line: &str) -> Result<SyncDoc, DomainError> {
    let bad = || DomainError::InvalidCanonicalString("bad DOC line".to_string());
    let fields = split_escaped(line).map_err(|_| bad())?;
    if fields.len() != 16 || fields[0] != "DOC" {
        return Err(bad());
    }
    let fiscal_year = parse_i32(&fields[2], "fiscal_year").map_err(|_| bad())?;
    if !(2000..=2100).contains(&fiscal_year) {
        return Err(bad());
    }
    if fields[3].is_empty() {
        return Err(bad());
    }
    let money = |index: usize| parse_i64(&fields[index], "money").map_err(|_| bad());
    let subtotal = money(6)?;
    let vat = money(7)?;
    let discount = money(8)?;
    let timbre = money(9)?;
    let rounding = money(10)?;
    let ttc = money(11)?;
    let paid = money(12)?;
    let change = money(13)?;
    for value in [subtotal, vat, discount, timbre, rounding, ttc, paid, change] {
        if value < 0 {
            return Err(bad());
        }
    }
    // Arithmetic seal: the header totals must foot (importer recomputes
    // nothing — a lying package aborts instead of posting drift).
    let footing = subtotal
        .checked_add(vat)
        .and_then(|sum| sum.checked_sub(discount))
        .and_then(|sum| sum.checked_add(timbre))
        .and_then(|sum| sum.checked_add(rounding))
        .ok_or_else(bad)?;
    if footing != ttc {
        return Err(DomainError::InvalidCanonicalString(
            "DOC totals do not foot".to_string(),
        ));
    }
    Ok(SyncDoc {
        doc_type_code: fields[1].clone(),
        fiscal_year,
        origin_number: fields[3].clone(),
        language: fields[4].clone(),
        party_code: if fields[5].is_empty() {
            None
        } else {
            Some(fields[5].clone())
        },
        subtotal_ht_centimes: subtotal,
        vat_total_centimes: vat,
        global_discount_centimes: discount,
        timbre_centimes: timbre,
        rounding_centimes: rounding,
        ttc_centimes: ttc,
        paid_amount_centimes: paid,
        change_amount_centimes: change,
        payment_method: fields[14].clone(),
        created_at: fields[15].clone(),
    })
}

/// Build an ITEM body line.
pub fn build_item_line(item: &SyncItem) -> String {
    format!(
        "ITEM|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        esc(&item.doc_origin_number),
        esc(&item.sku),
        item.quantity_micro,
        item.quantity_base_micro,
        item.unit_price_centimes,
        item.discount_centimes,
        item.tva_rate_micro,
        item.line_ttc_centimes,
        item.cost_price_centimes,
        esc(&item.unit_barcode),
        item.sort_order,
    )
}

/// Parse an ITEM line fail-closed (non-positive quantity, negative money).
pub fn parse_item_line(line: &str) -> Result<SyncItem, DomainError> {
    let bad = || DomainError::InvalidCanonicalString("bad ITEM line".to_string());
    let fields = split_escaped(line).map_err(|_| bad())?;
    if fields.len() != 12 || fields[0] != "ITEM" {
        return Err(bad());
    }
    if fields[1].is_empty() || fields[2].is_empty() {
        return Err(bad());
    }
    let quantity = parse_i64(&fields[3], "qty").map_err(|_| bad())?;
    let quantity_base = parse_i64(&fields[4], "qty_base").map_err(|_| bad())?;
    if quantity <= 0 || quantity_base <= 0 {
        return Err(bad());
    }
    let money = |index: usize| parse_i64(&fields[index], "money").map_err(|_| bad());
    let price = money(5)?;
    let discount = money(6)?;
    let tva = parse_i64(&fields[7], "tva").map_err(|_| bad())?;
    let line_ttc = money(8)?;
    let cost = money(9)?;
    let sort = parse_i64(&fields[11], "sort").map_err(|_| bad())?;
    if price < 0 || discount < 0 || tva < 0 || line_ttc < 0 || cost < 0 || sort < 0 {
        return Err(bad());
    }
    Ok(SyncItem {
        doc_origin_number: fields[1].clone(),
        sku: fields[2].clone(),
        quantity_micro: quantity,
        quantity_base_micro: quantity_base,
        unit_price_centimes: price,
        discount_centimes: discount,
        tva_rate_micro: tva,
        line_ttc_centimes: line_ttc,
        cost_price_centimes: cost,
        unit_barcode: fields[10].clone(),
        sort_order: sort,
    })
}

/// Build a PAY body line.
pub fn build_pay_line(pay: &SyncPay) -> String {
    format!(
        "PAY|{}|{}|{}|{}",
        esc(&pay.doc_origin_number),
        esc(&pay.method_code),
        esc(pay.customer_code.as_deref().unwrap_or("")),
        pay.amount_centimes,
    )
}

/// Parse a PAY line fail-closed (non-positive amount aborts).
pub fn parse_pay_line(line: &str) -> Result<SyncPay, DomainError> {
    let bad = || DomainError::InvalidCanonicalString("bad PAY line".to_string());
    let fields = split_escaped(line).map_err(|_| bad())?;
    if fields.len() != 5 || fields[0] != "PAY" {
        return Err(bad());
    }
    if fields[1].is_empty() || fields[2].is_empty() {
        return Err(bad());
    }
    let amount = parse_i64(&fields[4], "amount").map_err(|_| bad())?;
    if amount <= 0 {
        return Err(bad());
    }
    Ok(SyncPay {
        doc_origin_number: fields[1].clone(),
        method_code: fields[2].clone(),
        customer_code: if fields[3].is_empty() {
            None
        } else {
            Some(fields[3].clone())
        },
        amount_centimes: amount,
    })
}

/// Plan renumbers for parsed docs against the importer's next sequences.
///
/// `next_sequences` maps `(doc_type_code, fiscal_year)` to the importer's
/// next free sequence (read inside the import transaction). Numbers are
/// assigned in package order per (type, year) bucket; the origin number is
/// preserved verbatim for the audit trail. Unknown types are NOT defaulted —
/// the caller resolves them before planning (fail closed upstream).
pub fn plan_renumbers(
    docs: &[SyncDoc],
    next_sequences: &std::collections::HashMap<(String, i32), i64>,
    prefix_of: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<PlannedNumber>, DomainError> {
    let mut counters: std::collections::HashMap<(String, i32), i64> =
        std::collections::HashMap::new();
    let mut planned = Vec::with_capacity(docs.len());
    for doc in docs {
        let key = (doc.doc_type_code.clone(), doc.fiscal_year);
        let prefix = prefix_of(&doc.doc_type_code).ok_or_else(|| {
            DomainError::InvalidCanonicalString(format!(
                "unknown document type {}",
                doc.doc_type_code
            ))
        })?;
        let next = counters
            .entry(key.clone())
            .or_insert_with(|| next_sequences.get(&key).copied().unwrap_or(1).max(1));
        if *next > 999_999 {
            return Err(DomainError::InvalidCanonicalString(
                "local sequence exhausted".to_string(),
            ));
        }
        planned.push(PlannedNumber {
            origin_number: doc.origin_number.clone(),
            new_number: crate::sales::sequence::format_number(&prefix, doc.fiscal_year, *next),
            fiscal_year: doc.fiscal_year,
        });
        *next += 1;
    }
    Ok(planned)
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    fn sample_doc() -> SyncDoc {
        SyncDoc {
            doc_type_code: "TICKET".to_string(),
            fiscal_year: 2026,
            origin_number: "TICK-2026-000041".to_string(),
            language: "FR".to_string(),
            party_code: None,
            subtotal_ht_centimes: 10_000,
            vat_total_centimes: 0,
            global_discount_centimes: 0,
            timbre_centimes: 0,
            rounding_centimes: 0,
            ttc_centimes: 10_000,
            paid_amount_centimes: 10_000,
            change_amount_centimes: 0,
            payment_method: "CASH".to_string(),
            created_at: "2026-11-20T10:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn doc_line_round_trips_verbatim() {
        let doc = sample_doc();
        let parsed = parse_doc_line(&build_doc_line(&doc)).expect("round trip");
        assert_eq!(parsed, doc);
    }

    #[test]
    fn doc_line_with_drifted_totals_aborts() {
        let mut doc = sample_doc();
        doc.ttc_centimes = 9_999;
        assert!(parse_doc_line(&build_doc_line(&doc)).is_err());
    }

    #[test]
    fn doc_line_with_negative_money_aborts() {
        let mut doc = sample_doc();
        doc.subtotal_ht_centimes = -1;
        doc.ttc_centimes = -1;
        assert!(parse_doc_line(&build_doc_line(&doc)).is_err());
    }

    #[test]
    fn item_line_round_trips_and_rejects_bad_qty() {
        let item = SyncItem {
            doc_origin_number: "TICK-2026-000041".to_string(),
            sku: "SLIP-M/NOIR".to_string(),
            quantity_micro: 2_000_000,
            unit_price_centimes: 5_000,
            discount_centimes: 0,
            tva_rate_micro: 0,
            line_ttc_centimes: 10_000,
            cost_price_centimes: 3_000,
            quantity_base_micro: 2_000_000,
            unit_barcode: "6111250000013".to_string(),
            sort_order: 0,
        };
        let parsed = parse_item_line(&build_item_line(&item)).expect("round trip");
        assert_eq!(parsed, item);
        let bad = format!(
            "ITEM|{}|{}|0|2000000|5000|0|0|0|3000||0",
            item.doc_origin_number, item.sku
        );
        assert!(parse_item_line(&bad).is_err());
    }

    #[test]
    fn renumber_takes_importer_sequence_and_preserves_origin() {
        let docs = vec![sample_doc()];
        let mut next = std::collections::HashMap::new();
        next.insert(("TICKET".to_string(), 2026), 12);
        let planned = plan_renumbers(&docs, &next, &|code| {
            (code == "TICKET").then(|| "TICK".to_string())
        })
        .expect("plan");
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].origin_number, "TICK-2026-000041");
        assert_eq!(planned[0].new_number, "TICK-2026-000012");
    }

    #[test]
    fn renumber_increments_within_one_package_bucket() {
        let docs = vec![sample_doc(), sample_doc()];
        let mut next = std::collections::HashMap::new();
        next.insert(("TICKET".to_string(), 2026), 7);
        let planned = plan_renumbers(&docs, &next, &|code| {
            (code == "TICKET").then(|| "TICK".to_string())
        })
        .expect("plan");
        assert_eq!(planned[0].new_number, "TICK-2026-000007");
        assert_eq!(planned[1].new_number, "TICK-2026-000008");
    }
}
