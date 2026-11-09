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
    format_ticket_instant, label_barcode_geometry, label_text_needs_bitmap, label_text_plan,
    money_text_is_zero, parse_tspl_probe_mode, render_bitmap_job, render_calibration_pattern_40x20,
    render_label_40x20, render_label_40x20_mixed, render_ticket_80mm, render_ticket_80mm_mixed,
    render_ticket_80mm_mixed_v2, render_ticket_80mm_mixed_versioned, render_ticket_80mm_v2,
    render_ticket_80mm_versioned, render_ticket_ruler_80mm, ticket_text_plan, ticket_text_plan_v2,
    validate_tspl_calib_mm, validate_tspl_density, validate_tspl_gap_mm, validate_tspl_speed,
    LabelBarcodeGeometry, LabelBitmap, LabelTextLine, LabelTextSlot, TicketAlign, TicketFace,
    TicketOp, TicketTextLine, TicketTextSlot, TsplCaptionLang, TsplLabel, TsplMediaSetup,
    TsplProbeMode, TsplTicket, TsplTicketLine, TsplTicketTotals, BITMAP_MAX_BYTES,
    BITMAP_MAX_HEIGHT_DOTS, BITMAP_MAX_WIDTH_DOTS, DOTS_PER_MM, LABEL_HEIGHT_DOTS,
    LABEL_WIDTH_DOTS, TICKET_BARCODE_HEIGHT_V2, TICKET_BARCODE_MAX_CHARS_V2,
    TICKET_MARGIN_LEFT_DOTS, TICKET_MARGIN_RIGHT_DOTS, TICKET_PRINTABLE_DOTS, TICKET_WIDTH_DOTS,
    TSPL_CALIB_MM_MAX, TSPL_CALIB_MM_MIN, TSPL_CODEPAGE_LINE, TSPL_DENSITY_DEFAULT,
    TSPL_DENSITY_MAX, TSPL_DOTS_PER_MM, TSPL_GAP_MM_DEFAULT, TSPL_GAP_MM_MAX, TSPL_MAX_COPIES,
    TSPL_MIN_COPIES, TSPL_PROBE_MODE_DEFAULT, TSPL_SPEED_DEFAULT, TSPL_SPEED_MAX, TSPL_SPEED_MIN,
    TSPL_TICKET_TEMPLATE_VERSION, TSPL_TICKET_TEMPLATE_VERSION_V1,
};
