//! Print outbox domain: jobs, receipt content model.

pub mod decode;
pub mod paper;
pub mod print_job;
pub mod receipt_model;
pub mod transport;
pub mod tspl;

pub use decode::escpos_decode;
pub use paper::{validate_paper_width_mm, PaperWidth};
pub use print_job::{PrintJob, PrintJobStatus, PrintJobType};
pub use receipt_model::identity_caps;
pub use receipt_model::{
    format_centimes_plain, format_qty_plain, DebtReceiptDocument, DecodeSummary, DecodedLine,
    PreviewLine, ReceiptDocument, ReceiptLanguage, ReceiptLine, ReceiptTotals, StoreIdentity,
};
pub use transport::{
    validate_printer_transport, validate_spooler_queue_name, validate_ticket_render_mode,
    MAX_QUEUE_NAME_CHARS, TICKET_RENDER_ESC_POS, TICKET_RENDER_GRAPHIQUE, TRANSPORT_SERIAL,
    TRANSPORT_SPOOLER,
};
pub use tspl::{
    render_label_40x20, render_ticket_80mm, TsplCaptionLang, TsplLabel, TsplTicket, TsplTicketLine,
    TsplTicketTotals, DOTS_PER_MM, LABEL_HEIGHT_DOTS, LABEL_WIDTH_DOTS, TICKET_WIDTH_DOTS,
    TSPL_MAX_COPIES, TSPL_MIN_COPIES,
};
