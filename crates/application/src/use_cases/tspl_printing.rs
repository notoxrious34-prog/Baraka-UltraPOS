//! Silent TSPL print use-cases (TASK-BRK-097, ADR-105).
//!
//! Two button-triggered commands emitting TSPL bytes to the spooler queue
//! — no dialog, no auto-print, no window.print path. Both jobs route the
//! spooler transport ONLY: when `printer.transport` selects another arm
//! the job is rejected with [`DomainError::TsplTransportRequiresSpooler`]
//! (stated code `TSPL_REQUIRES_SPOOLER`).
//!
//! Transcription rules (graphique parity, DSN-097 semantics):
//! - label price: kernel centimes formatted via `format_centimes_plain`.
//! - ticket lines: designation by document language with the graphique
//!   fallback chain (`fr ?? ar ?? "—"` mirrored per locale); net unit
//!   price = stored `discount_per_unit_centimes` replayed exactly as the
//!   sales wire (`checked_sub`, no floor — legacy `None` keeps raw PU);
//!   line totals, header totals, and payment rows transcribed from stored
//!   values only (wire parity: five totals + `method_code.join(" + ")`).
//! - dates: RFC3339 millis, worker parity.

use baraka_domain::errors::DomainError;
use baraka_domain::printing::{
    format_centimes_plain, format_qty_plain, render_bitmap_job, render_label_40x20,
    render_ticket_80mm, TsplCaptionLang, TsplLabel, TsplMediaSetup, TsplTicket, TsplTicketLine,
    TsplTicketTotals, TRANSPORT_SPOOLER,
};
use baraka_domain::role::Permission;
use baraka_domain::sales::{DocumentId, DocumentLanguage};

use crate::errors::ApplicationError;
use crate::ports::{
    DocumentRepository, PrinterSettingsPort, PrinterTransportPort, SessionContext,
    StoreIdentityPort,
};
use crate::use_cases::sales::GetSaleUseCase;

/// Bytes-outcome of a silent TSPL job (count only — no content echo).
#[derive(Debug)]
pub struct TsplPrintOutcome {
    /// Bytes handed to the spooler transport.
    pub bytes_sent: usize,
}

/// Reject unless the `printer.transport` selector points at the spooler.
fn require_spooler_transport<P>(settings: &P) -> Result<(), ApplicationError>
where
    P: PrinterSettingsPort + ?Sized,
{
    let selector = settings.get_printer_settings()?;
    if selector.transport.trim() == TRANSPORT_SPOOLER {
        Ok(())
    } else {
        Err(ApplicationError::Validation(
            DomainError::TsplTransportRequiresSpooler,
        ))
    }
}

/// Load the stored TSPL media setup (defaults when keys are absent; the
/// renderer re-validates — out-of-range stored values reject loudly there).
fn media_from_settings<P>(settings: &P) -> Result<TsplMediaSetup, ApplicationError>
where
    P: PrinterSettingsPort + ?Sized,
{
    let stored = settings.get_printer_settings()?;
    // TEMPORARY-DIAGNOSTIC (BRK-100): a hand-edited stored spelling fails
    // the print LOUDLY here — never a silent fallback to `Normal`.
    let probe_mode = baraka_domain::printing::parse_tspl_probe_mode(&stored.tspl_probe_mode)?;
    Ok(TsplMediaSetup {
        gap_mm: stored.tspl_gap_mm,
        density: stored.tspl_density,
        speed: stored.tspl_speed,
        probe_mode,
    })
}

/// Silent 40x20 mm shelf-label print (brand + name + price + Code128).
pub struct PrintTsplLabelUseCase<'a, S, I, T>
where
    S: ?Sized,
    I: ?Sized,
    T: ?Sized,
{
    settings: &'a S,
    identity: &'a I,
    transport: &'a T,
}

impl<'a, S, I, T> PrintTsplLabelUseCase<'a, S, I, T>
where
    S: PrinterSettingsPort + ?Sized,
    I: StoreIdentityPort + ?Sized,
    T: PrinterTransportPort + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(settings: &'a S, identity: &'a I, transport: &'a T) -> Self {
        Self {
            settings,
            identity,
            transport,
        }
    }

    /// Validate, render, and spool one label run.
    pub fn execute(
        &self,
        session: &SessionContext,
        name: &str,
        price_centimes: i64,
        barcode: &str,
        copies: u32,
    ) -> Result<TsplPrintOutcome, ApplicationError> {
        if !session.role.has_permission(Permission::PosSell) {
            return Err(ApplicationError::Unauthorized);
        }
        require_spooler_transport(self.settings)?;
        if price_centimes < 0 {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("TSPL_PRICE_MUST_BE_NON_NEGATIVE".to_string()),
            ));
        }
        let identity = self.identity.load_identity()?;
        let bytes = render_label_40x20(&TsplLabel {
            brand_text: &identity.store_name,
            name,
            price_text: &format_centimes_plain(price_centimes),
            currency_text: "DZD",
            barcode,
            copies,
            media: media_from_settings(self.settings)?,
        })?;
        self.transport.send_raster(&bytes)?;
        Ok(TsplPrintOutcome {
            bytes_sent: bytes.len(),
        })
    }
}

/// Silent 80 mm ticket print (graphique content model, TSPL bytes).
pub struct PrintTsplTicketUseCase<'a, D, S, I, T>
where
    D: ?Sized,
    S: ?Sized,
    I: ?Sized,
    T: ?Sized,
{
    documents: &'a D,
    settings: &'a S,
    identity: &'a I,
    transport: &'a T,
}

impl<'a, D, S, I, T> PrintTsplTicketUseCase<'a, D, S, I, T>
where
    D: DocumentRepository + ?Sized,
    S: PrinterSettingsPort + ?Sized,
    I: StoreIdentityPort + ?Sized,
    T: PrinterTransportPort + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(documents: &'a D, settings: &'a S, identity: &'a I, transport: &'a T) -> Self {
        Self {
            documents,
            settings,
            identity,
            transport,
        }
    }

    /// Transcribe the stored document and spool one ticket.
    pub fn execute(
        &self,
        session: &SessionContext,
        document_id: DocumentId,
    ) -> Result<TsplPrintOutcome, ApplicationError> {
        if !session.role.has_permission(Permission::ReprintDocument) {
            return Err(ApplicationError::Unauthorized);
        }
        require_spooler_transport(self.settings)?;
        let detail = GetSaleUseCase::new(self.documents).execute(session, document_id)?;
        let document = detail.document;
        let identity = self.identity.load_identity()?;
        let french = matches!(document.language, DocumentLanguage::Fr);

        // Phase 1: own every formatted string (transcription only — stored
        // values in, display strings out, no recompute).
        let mut owned: Vec<OwnedTicketLine> = Vec::with_capacity(detail.items_view.len());
        for view in &detail.items_view {
            let item = &view.item;
            // DSN-097 wire rule, replayed verbatim: discounted PU when a
            // per-unit discount is stored, raw PU for legacy NULL rows.
            let net_unit_price = item
                .discount_per_unit_centimes
                .and_then(|discount| item.unit_price_centimes.checked_sub(discount))
                .unwrap_or(item.unit_price_centimes);
            owned.push(OwnedTicketLine {
                sku: view.sku.clone().unwrap_or_else(|| "—".to_string()),
                designation: pick_name(&view.product_name_fr, &view.product_name_ar, french, "—"),
                quantity: format_qty_plain(item.quantity_micro),
                unit: pick_name(&view.unit_name_fr, &view.unit_name_ar, french, ""),
                unit_price: format_centimes_plain(net_unit_price),
                line_total: format_centimes_plain(item.line_ttc_centimes),
                discount: item.discount_per_unit_centimes.map(format_centimes_plain),
            });
        }
        // Phase 2: borrow the owned strings into the pure renderer input.
        let lines: Vec<TsplTicketLine<'_>> = owned
            .iter()
            .map(|line| TsplTicketLine {
                sku: &line.sku,
                designation: &line.designation,
                quantity_text: &line.quantity,
                unit_text: &line.unit,
                unit_price_text: &line.unit_price,
                line_total_text: &line.line_total,
                discount_text: line.discount.as_deref(),
            })
            .collect();

        let payment_label = detail
            .payments
            .iter()
            .map(|payment| payment.method_code.clone())
            .collect::<Vec<_>>()
            .join(" + ");
        let created_at = document
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let subtotal_ht = format_centimes_plain(document.subtotal_ht_centimes);
        let global_discount = format_centimes_plain(document.global_discount_centimes);
        let tva = format_centimes_plain(document.vat_total_centimes);
        let total_ttc = format_centimes_plain(document.ttc_centimes);
        let paid = format_centimes_plain(document.paid_amount_centimes);
        let change = format_centimes_plain(document.change_amount_centimes);
        // Cashier + depot ride the stored identifiers verbatim (no lookup
        // port on this path — collapse, never invent).
        let cashier_text = document.created_by.to_string();
        let warehouse_text = document.warehouse_id.to_string();
        let tax_rows = [
            ("NIF", identity.nif.as_str()),
            ("NIS", identity.nis.as_str()),
            ("RC", identity.rc.as_str()),
            ("AI", identity.ai.as_str()),
        ];
        let ticket = TsplTicket {
            store_name: &identity.store_name,
            store_address: &identity.store_address,
            store_phone: &identity.store_phone,
            tax_rows: &tax_rows,
            number: &document.number,
            created_at: &created_at,
            cashier_text: &cashier_text,
            warehouse_text: &warehouse_text,
            captions: if french {
                TsplCaptionLang::French
            } else {
                TsplCaptionLang::Arabic
            },
            lines: &lines,
            totals: TsplTicketTotals {
                subtotal_ht: &subtotal_ht,
                global_discount: &global_discount,
                tva: &tva,
                total_ttc: &total_ttc,
                paid: &paid,
                change: &change,
                payment_method: &payment_label,
            },
            footer: &identity.ticket_footer_message,
            media: media_from_settings(self.settings)?,
        };
        let bytes = render_ticket_80mm(&ticket)?;
        self.transport.send_raster(&bytes)?;
        Ok(TsplPrintOutcome {
            bytes_sent: bytes.len(),
        })
    }
}

/// Owned ticket-line transcription (phase 1 of the ticket render).
struct OwnedTicketLine {
    sku: String,
    designation: String,
    quantity: String,
    unit: String,
    unit_price: String,
    line_total: String,
    /// Per-unit discount badge (stored value), `None` for legacy NULL rows.
    discount: Option<String>,
}

/// Graphique fallback chain (`fr ?? ar ?? fallback`, locale-first).
fn pick_name(
    name_fr: &Option<String>,
    name_ar: &Option<String>,
    french: bool,
    fallback: &str,
) -> String {
    let (first, second) = if french {
        (name_fr, name_ar)
    } else {
        (name_ar, name_fr)
    };
    first
        .clone()
        .or_else(|| second.clone())
        .unwrap_or_else(|| fallback.to_string())
}

/// Silent raster-only print (BRK-101 — TEXT path is DEAD, owner all-red).
/// Same auth seam as the `print_tspl_*` use-cases: POS sell permission +
/// spooler transport required; opaque packed pixels in, spooler bytes out.
/// NO infrastructure change — the existing
/// [`PrinterTransportPort::send_raster`] moves the bytes.
pub struct PrintRawBitmapUseCase<'a, S, T>
where
    S: ?Sized,
    T: ?Sized,
{
    settings: &'a S,
    transport: &'a T,
}

impl<'a, S, T> PrintRawBitmapUseCase<'a, S, T>
where
    S: PrinterSettingsPort + ?Sized,
    T: PrinterTransportPort + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(settings: &'a S, transport: &'a T) -> Self {
        Self {
            settings,
            transport,
        }
    }

    /// Validate, render the raster job, and spool it. All caps enforced
    /// pre-send by [`render_bitmap_job`] — the transport only ever sees a
    /// fully validated job (never truncated, never blank).
    pub fn execute(
        &self,
        session: &SessionContext,
        pixels: &[u8],
        width_bytes: usize,
        height_rows: usize,
        copies: u32,
    ) -> Result<TsplPrintOutcome, ApplicationError> {
        if !session.role.has_permission(Permission::PosSell) {
            return Err(ApplicationError::Unauthorized);
        }
        require_spooler_transport(self.settings)?;
        let bytes = render_bitmap_job(
            &media_from_settings(self.settings)?,
            0,
            0,
            width_bytes,
            height_rows,
            pixels,
            copies,
        )?;
        self.transport.send_raster(&bytes)?;
        Ok(TsplPrintOutcome {
            bytes_sent: bytes.len(),
        })
    }
}
