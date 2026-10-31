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
    render_label_40x20, render_ticket_80mm, validate_tspl_density, validate_tspl_gap_mm,
    validate_tspl_speed, TsplCaptionLang, TsplLabel, TsplMediaSetup, TsplTicket, TsplTicketLine,
    TsplTicketTotals, DOTS_PER_MM, LABEL_HEIGHT_DOTS, LABEL_WIDTH_DOTS, TICKET_WIDTH_DOTS,
    TSPL_CODEPAGE_LINE, TSPL_DENSITY_DEFAULT, TSPL_DENSITY_MAX, TSPL_GAP_MM_DEFAULT,
    TSPL_GAP_MM_MAX, TSPL_MAX_COPIES, TSPL_MIN_COPIES, TSPL_SPEED_DEFAULT, TSPL_SPEED_MAX,
    TSPL_SPEED_MIN,
};
