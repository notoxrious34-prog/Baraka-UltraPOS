//! Print outbox consumer: pure orchestration, no threads, no I/O drivers.
//!
//! `process_one_job` claims a single PENDING job, hydrates its receipt from
//! persisted rows, renders raster bytes, and transmits them. Every failure
//! path marks the JOB row only; committed accounting rows are never touched.

use std::time::Duration;

use baraka_domain::errors::DomainError;
use baraka_domain::printing::{
    format_centimes_plain, format_qty_plain, DebtReceiptDocument, ReceiptDocument, ReceiptLanguage,
    ReceiptLine, ReceiptTotals,
};

use crate::errors::ApplicationError;
use crate::ports::{
    DocumentRepository, PrintJobRepository, PrinterTransportPort, ProductRepository,
    ReceiptRendererPort, PRINT_MAX_ATTEMPTS,
};

/// Backoff delays indexed by failed physical attempt (1-based).
pub fn retry_delay(failed_attempt: i64) -> Duration {
    match failed_attempt {
        1 => Duration::from_secs(1),
        2 => Duration::from_secs(2),
        _ => Duration::from_secs(4),
    }
}

/// Single worker-cycle outcome (the thread maps these to sleeps).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobOutcome {
    /// Queue empty, nothing to do.
    Idle,
    /// Job physically printed.
    Done {
        /// Completed job id.
        job_id: String,
    },
    /// Send failed; sleep, then poll again.
    RetryAfter {
        /// Retried job id.
        job_id: String,
        /// Backoff sleep.
        delay: Duration,
    },
    /// Job exhausted retries or is unrenderable.
    Failed {
        /// Failed job id.
        job_id: String,
        /// Stable reason code.
        reason: String,
    },
    /// Transport unreachable; job released untouched for the next cycle.
    Deferred {
        /// Deferred job id.
        job_id: String,
    },
}

/// Build a receipt model verbatim from persisted rows (no recomputation).
#[allow(clippy::too_many_arguments)]
fn hydrate_receipt<P>(
    products: &P,
    document: &baraka_domain::sales::Document,
    language: ReceiptLanguage,
    items: &[baraka_domain::sales::DocumentItem],
    payments: &[crate::ports::StoredPayment],
    is_duplicata: bool,
    store_identity: baraka_domain::printing::StoreIdentity,
    footer_message: String,
) -> Result<ReceiptDocument, ApplicationError>
where
    P: ProductRepository + ?Sized,
{
    let mut lines = Vec::with_capacity(items.len());
    for item in items {
        let designation = products
            .find_by_id(item.product_id)?
            .map(|(product, _)| product.name_fr)
            .unwrap_or_else(|| item.product_id.to_string());
        lines.push(ReceiptLine {
            designation,
            quantity_text: format_qty_plain(item.quantity_micro),
            unit_price_text: format_centimes_plain(item.unit_price_centimes),
            total_text: format_centimes_plain(item.line_ttc_centimes),
        });
    }
    let mut tender: Vec<(String, String)> = Vec::with_capacity(payments.len());
    for payment in payments {
        tender.push((
            payment.method_code.clone(),
            format_centimes_plain(payment.amount_centimes),
        ));
    }
    Ok(ReceiptDocument {
        language,
        shop_name: "BARAKA QUINCAILLERIE".to_string(),
        number: document.number.clone(),
        created_at: document
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        lines,
        totals: ReceiptTotals {
            subtotal_ht: format_centimes_plain(document.subtotal_ht_centimes),
            tva: format_centimes_plain(document.vat_total_centimes),
            discount: format_centimes_plain(document.global_discount_centimes),
            timbre: format_centimes_plain(document.timbre_centimes),
            total_ttc: format_centimes_plain(document.ttc_centimes),
            paid: format_centimes_plain(document.paid_amount_centimes),
            change: format_centimes_plain(document.change_amount_centimes),
        },
        payments: tender,
        is_duplicata,
        store_identity,
        footer_message,
    })
}

/// Consume a single outbox job with the configured store identity.
///
/// Production entry: the worker loads the identity per job so settings
/// changes apply without restart. Identity-unaware `process_one_job`
/// keeps the legacy header/footer fallback (existing tests unchanged).
pub fn process_one_job_with_identity<D, J, P, T, R, S, PR>(
    documents: &D,
    jobs: &J,
    products: &P,
    transport: &T,
    renderer: &R,
    identity: &S,
    printer: &PR,
) -> Result<JobOutcome, ApplicationError>
where
    D: DocumentRepository + ?Sized,
    J: PrintJobRepository + ?Sized,
    P: ProductRepository + ?Sized,
    T: PrinterTransportPort + ?Sized,
    R: ReceiptRendererPort + ?Sized,
    S: crate::ports::StoreIdentityPort + ?Sized,
    PR: crate::ports::PrinterSettingsPort + ?Sized,
{
    let claimed = match jobs.claim_next_pending()? {
        Some(job) => job,
        None => return Ok(JobOutcome::Idle),
    };
    jobs.mark_processing(&claimed.job_id)?;
    let store_identity = identity.load_identity().unwrap_or_default();
    let footer_message = store_identity.ticket_footer_message.clone();
    // TASK-BAT-072: resolve the configured paper per job; any settings
    // failure falls back to the 80mm default (never fell the job).
    let paper = printer
        .get_printer_settings()
        .map(|settings| baraka_domain::printing::PaperWidth::from_mm(settings.paper_width_mm))
        .unwrap_or(baraka_domain::printing::PaperWidth::Width80mm);
    let outcome = process_claimed(
        documents,
        jobs,
        products,
        transport,
        renderer,
        &claimed,
        store_identity,
        footer_message,
        paper,
    );
    match outcome {
        Ok(done) => Ok(done),
        Err(error) => {
            let reason = match &error {
                ApplicationError::Validation(domain) => format!("{domain}"),
                _ => "WORKER_ERROR".to_string(),
            };
            let _ = jobs.mark_failed(&claimed.job_id, &reason);
            Ok(JobOutcome::Failed {
                job_id: claimed.job_id,
                reason,
            })
        }
    }
}

/// Consume a single outbox job; never touches accounting state.
pub fn process_one_job<D, J, P, T, R>(
    documents: &D,
    jobs: &J,
    products: &P,
    transport: &T,
    renderer: &R,
) -> Result<JobOutcome, ApplicationError>
where
    D: DocumentRepository + ?Sized,
    J: PrintJobRepository + ?Sized,
    P: ProductRepository + ?Sized,
    T: PrinterTransportPort + ?Sized,
    R: ReceiptRendererPort + ?Sized,
{
    let claimed = match jobs.claim_next_pending()? {
        Some(job) => job,
        None => return Ok(JobOutcome::Idle),
    };
    jobs.mark_processing(&claimed.job_id)?;
    let outcome = process_claimed(
        documents,
        jobs,
        products,
        transport,
        renderer,
        &claimed,
        baraka_domain::printing::StoreIdentity::default(),
        String::new(),
        // Legacy entry: no settings port available; 80mm default surface.
        baraka_domain::printing::PaperWidth::Width80mm,
    );
    match outcome {
        Ok(done) => Ok(done),
        Err(error) => {
            // The worker loop must survive any failure shape.
            let reason = match &error {
                ApplicationError::Validation(domain) => format!("{domain}"),
                _ => "WORKER_ERROR".to_string(),
            };
            let _ = jobs.mark_failed(&claimed.job_id, &reason);
            Ok(JobOutcome::Failed {
                job_id: claimed.job_id,
                reason,
            })
        }
    }
}

/// Consume a non-sale (`document_id == None`) outbox job through the
/// independent debt receipt pipeline (ADR-083): deserialize the stored
/// `payload_json` into a [`DebtReceiptDocument`], render it with
/// [`ReceiptRendererPort::render_debt_receipt`], and transmit the bytes
/// over the same transport edge (same bounded retry budget). Never
/// touches the sales receipt pipeline or accounting state.
fn process_debt_claimed<J, T, R>(
    jobs: &J,
    transport: &T,
    renderer: &R,
    claimed: &crate::ports::PrintJobRecord,
    paper: baraka_domain::printing::PaperWidth,
) -> Result<JobOutcome, ApplicationError>
where
    J: PrintJobRepository + ?Sized,
    T: PrinterTransportPort + ?Sized,
    R: ReceiptRendererPort + ?Sized,
{
    let debt_receipt: DebtReceiptDocument =
        serde_json::from_str(&claimed.payload_json).map_err(|_| {
            ApplicationError::Validation(DomainError::InvalidCanonicalString(
                "DEBT_PAYLOAD_INVALID".to_string(),
            ))
        })?;
    // Defense in depth: a renderer panic must fell the job, never the thread.
    let bytes = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.render_debt_receipt_with_paper(&debt_receipt, paper)
    })) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(error)) => {
            jobs.mark_failed(&claimed.job_id, "RENDER_FAILED")?;
            return Ok(JobOutcome::Failed {
                job_id: claimed.job_id.clone(),
                reason: format!("{error}"),
            });
        }
        Err(_) => {
            jobs.mark_failed(&claimed.job_id, "RENDER_PANIC")?;
            return Ok(JobOutcome::Failed {
                job_id: claimed.job_id.clone(),
                reason: "RENDER_PANIC".to_string(),
            });
        }
    };
    if !transport.is_available() {
        let attempt = claimed.attempt_count + 1;
        if attempt >= PRINT_MAX_ATTEMPTS {
            let reason = "EXHAUSTED_MAX_RETRIES: NO_PRINTER_CONFIGURED".to_string();
            jobs.mark_failed(&claimed.job_id, &reason)?;
            return Ok(JobOutcome::Failed {
                job_id: claimed.job_id.clone(),
                reason,
            });
        }
        jobs.release_for_retry(&claimed.job_id, "NO_PRINTER_CONFIGURED")?;
        return Ok(JobOutcome::RetryAfter {
            job_id: claimed.job_id.clone(),
            delay: retry_delay(attempt),
        });
    }
    match transport.send_raster(&bytes) {
        Ok(()) => {
            jobs.mark_done(&claimed.job_id)?;
            Ok(JobOutcome::Done {
                job_id: claimed.job_id.clone(),
            })
        }
        Err(error) => {
            let attempt = claimed.attempt_count + 1;
            let reason = match &error {
                ApplicationError::Validation(
                    baraka_domain::errors::DomainError::PrinterTransportUnavailable,
                ) => "PRINTER_OFFLINE".to_string(),
                ApplicationError::Validation(domain) => format!("{domain}"),
                _ => "TRANSPORT_ERROR".to_string(),
            };
            if attempt >= PRINT_MAX_ATTEMPTS {
                let terminal = format!("EXHAUSTED_MAX_RETRIES: {reason}");
                jobs.mark_failed(&claimed.job_id, &terminal)?;
                Ok(JobOutcome::Failed {
                    job_id: claimed.job_id.clone(),
                    reason: terminal,
                })
            } else {
                jobs.release_for_retry(&claimed.job_id, &reason)?;
                Ok(JobOutcome::RetryAfter {
                    job_id: claimed.job_id.clone(),
                    delay: retry_delay(attempt),
                })
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_claimed<D, J, P, T, R>(
    documents: &D,
    jobs: &J,
    products: &P,
    transport: &T,
    renderer: &R,
    claimed: &crate::ports::PrintJobRecord,
    store_identity: baraka_domain::printing::StoreIdentity,
    footer_message: String,
    paper: baraka_domain::printing::PaperWidth,
) -> Result<JobOutcome, ApplicationError>
where
    D: DocumentRepository + ?Sized,
    J: PrintJobRepository + ?Sized,
    P: ProductRepository + ?Sized,
    T: PrinterTransportPort + ?Sized,
    R: ReceiptRendererPort + ?Sized,
{
    let document_id = match claimed.document_id {
        Some(document_id) => document_id,
        // Non-sale jobs (debt receipts, Z-reports, test prints) carry no
        // row in `documents`: route them to the independent debt receipt
        // pipeline (ADR-083). The sales flow below stays verbatim.
        None => {
            return process_debt_claimed(jobs, transport, renderer, claimed, paper);
        }
    };
    let document = documents
        .get_by_id(document_id)?
        .ok_or(ApplicationError::Validation(DomainError::PrintJobNotFound))?;
    let language = match document.language {
        baraka_domain::sales::DocumentLanguage::Ar => ReceiptLanguage::Arabic,
        baraka_domain::sales::DocumentLanguage::Fr => ReceiptLanguage::French,
    };
    let items = documents.get_items(document_id)?;
    let payments = documents.get_payments(document_id)?;
    // Retried physical attempts render watermarked: explicit DUPLICATA jobs
    // plus any job that already consumed >= 1 attempt.
    let is_duplicata = claimed.job_type == "DUPLICATA" || claimed.attempt_count > 0;
    let receipt = hydrate_receipt(
        products,
        &document,
        language,
        &items,
        &payments,
        is_duplicata,
        store_identity,
        footer_message,
    )?;
    // Defense in depth: a renderer panic must fell the job, never the thread.
    let bytes = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.render_with_paper(&receipt, paper)
    })) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(error)) => {
            jobs.mark_failed(&claimed.job_id, "RENDER_FAILED")?;
            return Ok(JobOutcome::Failed {
                job_id: claimed.job_id.clone(),
                reason: format!("{error}"),
            });
        }
        Err(_) => {
            jobs.mark_failed(&claimed.job_id, "RENDER_PANIC")?;
            return Ok(JobOutcome::Failed {
                job_id: claimed.job_id.clone(),
                reason: "RENDER_PANIC".to_string(),
            });
        }
    };
    // Bounded unavailable-printer path: an unconfigured or unreachable
    // transport consumes retry budget instead of spinning forever. After
    // PRINT_MAX_ATTEMPTS the job fails terminally with NO_PRINTER_CONFIGURED.
    if !transport.is_available() {
        let attempt = claimed.attempt_count + 1;
        if attempt >= PRINT_MAX_ATTEMPTS {
            let reason = "EXHAUSTED_MAX_RETRIES: NO_PRINTER_CONFIGURED".to_string();
            jobs.mark_failed(&claimed.job_id, &reason)?;
            return Ok(JobOutcome::Failed {
                job_id: claimed.job_id.clone(),
                reason,
            });
        }
        jobs.release_for_retry(&claimed.job_id, "NO_PRINTER_CONFIGURED")?;
        return Ok(JobOutcome::RetryAfter {
            job_id: claimed.job_id.clone(),
            delay: retry_delay(attempt),
        });
    }
    match transport.send_raster(&bytes) {
        Ok(()) => {
            jobs.mark_done(&claimed.job_id)?;
            Ok(JobOutcome::Done {
                job_id: claimed.job_id.clone(),
            })
        }
        Err(error) => {
            let attempt = claimed.attempt_count + 1;
            let reason = match &error {
                ApplicationError::Validation(
                    baraka_domain::errors::DomainError::PrinterTransportUnavailable,
                ) => "PRINTER_OFFLINE".to_string(),
                ApplicationError::Validation(domain) => format!("{domain}"),
                _ => "TRANSPORT_ERROR".to_string(),
            };
            if attempt >= PRINT_MAX_ATTEMPTS {
                let terminal = format!("EXHAUSTED_MAX_RETRIES: {reason}");
                jobs.mark_failed(&claimed.job_id, &terminal)?;
                Ok(JobOutcome::Failed {
                    job_id: claimed.job_id.clone(),
                    reason: terminal,
                })
            } else {
                jobs.release_for_retry(&claimed.job_id, &reason)?;
                Ok(JobOutcome::RetryAfter {
                    job_id: claimed.job_id.clone(),
                    delay: retry_delay(attempt),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_schedule() {
        assert_eq!(retry_delay(1), Duration::from_secs(1));
        assert_eq!(retry_delay(2), Duration::from_secs(2));
        assert_eq!(retry_delay(3), Duration::from_secs(4));
        assert_eq!(retry_delay(99), Duration::from_secs(4));
    }

    #[test]
    fn plain_formatters_cover_signs() {
        assert_eq!(format_centimes_plain(0), "0.00");
        assert_eq!(format_centimes_plain(-1), "-0.01");
        assert_eq!(format_qty_plain(0), "0");
    }
}
