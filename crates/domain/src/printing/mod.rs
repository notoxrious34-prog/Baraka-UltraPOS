//! Print outbox domain: jobs, receipt content model.

pub mod decode;
pub mod paper;
pub mod print_job;
pub mod receipt_model;

pub use decode::escpos_decode;
pub use paper::{validate_paper_width_mm, PaperWidth};
pub use print_job::{PrintJob, PrintJobStatus, PrintJobType};
pub use receipt_model::identity_caps;
pub use receipt_model::{
    format_centimes_plain, format_qty_plain, DebtReceiptDocument, DecodeSummary, DecodedLine,
    PreviewLine, ReceiptDocument, ReceiptLanguage, ReceiptLine, ReceiptTotals, StoreIdentity,
};
