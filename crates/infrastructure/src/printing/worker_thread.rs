//! Print outbox worker thread (plain OS thread, no async runtime).
//!
//! Polls `claim_next_pending()` every 500ms, drives `process_one_job`,
//! and sleeps backoff delays in 100ms slices so the stop signal stays
//! responsive (worst-case shutdown latency ≈ 600ms on idle, ≈ backoff +
//! 100ms mid-retry). Stale PROCESSING rows are reclaimed once at startup.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use baraka_application::ports::{
    DocumentRepository, PrintJobRepository, PrinterTransportPort, ProductRepository,
    ReceiptRendererPort, StoreIdentityPort, PRINT_STALE_PENDING_SECS,
};
use baraka_application::services::print_worker_service::{
    process_one_job_with_identity, JobOutcome,
};

/// Empty-queue poll interval.
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// Stale-claim age triggering a reset to PENDING (crash recovery).
pub const STALE_CLAIM_TIMEOUT_SECS: u64 = 60;
/// Stop-check granularity inside every sleep.
const STOP_SLICE: Duration = Duration::from_millis(100);

/// Shared worker dependencies (all `'static` for the spawned thread).
pub struct WorkerDeps {
    /// Document hydration source.
    pub documents: Arc<dyn DocumentRepository>,
    /// Outbox state machine.
    pub jobs: Arc<dyn PrintJobRepository>,
    /// Catalog source for receipt designations.
    pub products: Arc<dyn ProductRepository>,
    /// Physical transport edge.
    pub transport: Arc<dyn PrinterTransportPort>,
    /// Raster renderer.
    pub renderer: Arc<dyn ReceiptRendererPort>,
    /// Store identity source (loaded per job; worker prints the
    /// configured header/footer on every document).
    pub settings: Arc<dyn StoreIdentityPort>,
}

/// Sleep `total`, waking every 100ms to honor `stop` promptly.
fn sleep_responsive(stop: &AtomicBool, total: Duration) {
    let mut remaining = total;
    while remaining > Duration::ZERO {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let slice = remaining.min(STOP_SLICE);
        thread::sleep(slice);
        remaining = remaining.saturating_sub(slice);
    }
}

/// Spawn the worker thread; returns its join handle.
///
/// The caller owns `stop`: set it, then join the handle for a bounded,
/// clean shutdown.
pub fn spawn_print_worker(deps: WorkerDeps, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || {
        // Startup recovery, in order: first release crashed PROCESSING claims,
        // then abandon orphaned PENDING rows (exhausted budget or older than
        // one hour). Both are best-effort; failures must not stop the worker.
        let _ = deps.jobs.reclaim_stale_processing(STALE_CLAIM_TIMEOUT_SECS);
        let _ = deps.jobs.reconcile_stale_pending(PRINT_STALE_PENDING_SECS);
        loop {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            let outcome = process_one_job_with_identity(
                deps.documents.as_ref(),
                deps.jobs.as_ref(),
                deps.products.as_ref(),
                deps.transport.as_ref(),
                deps.renderer.as_ref(),
                deps.settings.as_ref(),
            );
            match outcome {
                Ok(JobOutcome::Idle) | Err(_) => sleep_responsive(&stop, POLL_INTERVAL),
                Ok(JobOutcome::Done { .. }) | Ok(JobOutcome::Failed { .. }) => {}
                // Deferred (legacy) and retry paths sleep to avoid tight
                // polling loops and log storms when no printer is present.
                Ok(JobOutcome::Deferred { .. }) => sleep_responsive(&stop, POLL_INTERVAL),
                Ok(JobOutcome::RetryAfter { delay, .. }) => sleep_responsive(&stop, delay),
            }
        }
    })
}
