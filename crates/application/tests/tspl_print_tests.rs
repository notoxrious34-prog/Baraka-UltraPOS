//! Silent TSPL print use-case tests (TASK-BRK-097, ADR-105).
//!
//! No SQLite here: every port is a deterministic fake, so these tests pin
//! the spooler-only gate, the byte transcription (discounted PU vs legacy
//! NULL), and the exact spooled bytes. Any repository *write* method
//! panics — passing tests prove the silent jobs are zero-write.

use std::sync::Mutex;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    ConvertBlToFactureCommand, ConvertBlToFactureResult, ConvertDevisToBlCommand,
    ConvertDevisToBlResult, DocumentRepository, DocumentSearchFilter, DocumentSummary,
    PrinterSettings, PrinterSettingsPort, PrinterTransportPort, SessionContext, SessionToken,
    StoreIdentityPort, StoredPayment,
};
use baraka_application::use_cases::tspl_printing::{
    PreviewTsplLabelUseCase, PrintRawBitmapUseCase, PrintTsplLabelUseCase,
    PrintTsplTestTicketUseCase, PrintTsplTicketUseCase,
};
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::printing::TsplMediaSetup;
use baraka_domain::printing::TsplProbeMode;
use baraka_domain::role::Role;
use baraka_domain::sales::{
    Document, DocumentId, DocumentItem, DocumentItemView, DocumentLanguage, DocumentStatus,
    DocumentType,
};
use baraka_domain::user::UserId;

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    }
}

struct FakeSettings {
    transport: String,
    media: TsplMediaSetup,
}

impl PrinterSettingsPort for FakeSettings {
    fn get_printer_settings(&self) -> Result<PrinterSettings, ApplicationError> {
        Ok(PrinterSettings {
            serial_port: String::new(),
            serial_baud: 9600,
            paper_width_dots: 640,
            paper_width_mm: 80,
            transport: self.transport.clone(),
            queue_name: "XP-410B".to_string(),
            ticket_render_mode: "graphique".to_string(),
            tspl_gap_mm: self.media.gap_mm,
            tspl_density: self.media.density,
            tspl_speed: self.media.speed,
            tspl_probe_mode: self.media.probe_mode.as_str().to_string(),
        })
    }

    fn update_printer_settings(
        &self,
        _serial_port: &str,
        _serial_baud: u32,
        _paper_width_mm: Option<u32>,
        _transport: Option<&str>,
        _queue_name: Option<&str>,
        _ticket_render_mode: Option<&str>,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests: settings are read-only here");
    }

    fn update_tspl_media_settings(
        &self,
        _gap_mm: Option<u32>,
        _density: Option<u8>,
        _speed: Option<u8>,
        _probe_mode: Option<String>,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests: settings are read-only here");
    }
}

struct FakeTransport {
    sent: Mutex<Vec<Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
        }
    }

    fn payloads(&self) -> Vec<Vec<u8>> {
        self.sent.lock().expect("lock").clone()
    }
}

impl PrinterTransportPort for FakeTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        self.sent.lock().expect("lock").push(bytes.to_vec());
        Ok(())
    }

    fn is_available(&self) -> bool {
        true
    }
}

struct FakeIdentity;

impl StoreIdentityPort for FakeIdentity {
    fn load_identity(&self) -> Result<baraka_domain::printing::StoreIdentity, ApplicationError> {
        Ok(baraka_domain::printing::StoreIdentity {
            store_name: "BARAKA QUINCAILLERIE".to_string(),
            store_address: "Rue des Frères".to_string(),
            store_phone: "0550 00 00 00".to_string(),
            nif: String::new(),
            nis: String::new(),
            rc: String::new(),
            ai: String::new(),
            ticket_footer_message: "Merci".to_string(),
        })
    }

    fn save_identity(
        &self,
        _identity: &baraka_domain::printing::StoreIdentity,
        _actor: &str,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests: identity is read-only here");
    }

    fn device_kind(&self) -> Result<String, ApplicationError> {
        panic!("out of scope for TSPL tests");
    }

    fn save_device(&self, _kind: &str, _actor: &str) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests");
    }

    fn append_settings_audit(
        &self,
        _actor: &str,
        _action: &str,
        _detail: &str,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests");
    }
}

/// One finalized ticket: line 1 carries a per-unit discount
/// (3 x (1000 - 100)), line 2 is a legacy NULL row (raw PU).
fn ticket_document() -> (Document, Vec<DocumentItemView>, Vec<StoredPayment>) {
    let document = Document {
        id: DocumentId::new(),
        language: DocumentLanguage::Fr,
        number: "T-2026-000123".to_string(),
        fiscal_year: 2026,
        doc_type: DocumentType::Ticket,
        status: DocumentStatus::Finalized,
        warehouse_id: WarehouseId::new(),
        party_id: None,
        subtotal_ht_centimes: 3_200,
        vat_total_centimes: 0,
        global_discount_centimes: 0,
        timbre_centimes: 0,
        ttc_centimes: 3_200,
        paid_amount_centimes: 3_200,
        change_amount_centimes: 0,
        payment_method: "CASH".to_string(),
        template_version: baraka_domain::printing::TSPL_TICKET_TEMPLATE_VERSION,
        notes: None,
        created_at: chrono::Utc::now(),
        created_by: UserId::new(),
    };
    let discounted = DocumentItem {
        id: uuid::Uuid::new_v4(),
        document_id: document.id,
        product_id: ProductId::new(),
        unit_id: UnitId::new(),
        quantity_micro: 3 * 10_000,
        quantity_base_micro: 3 * 10_000,
        unit_price_centimes: 1_000,
        discount_centimes: 300,
        discount_per_unit_centimes: Some(100),
        tva_rate_micro: 0,
        line_ht_centimes: 2_700,
        line_tva_centimes: 0,
        line_ttc_centimes: 2_700,
        cost_price_centimes: 800,
        sort_order: 0,
    };
    let legacy = DocumentItem {
        id: uuid::Uuid::new_v4(),
        document_id: document.id,
        product_id: ProductId::new(),
        unit_id: UnitId::new(),
        quantity_micro: 10_000,
        quantity_base_micro: 10_000,
        unit_price_centimes: 500,
        discount_centimes: 0,
        discount_per_unit_centimes: None,
        tva_rate_micro: 0,
        line_ht_centimes: 500,
        line_tva_centimes: 0,
        line_ttc_centimes: 500,
        cost_price_centimes: 400,
        sort_order: 1,
    };
    let views = vec![
        DocumentItemView {
            item: discounted,
            product_name_fr: Some("Vis 8x40".to_string()),
            product_name_ar: None,
            sku: Some("VIS-001".to_string()),
            unit_name_fr: Some("Pièce".to_string()),
            unit_name_ar: None,
            product_missing: false,
        },
        DocumentItemView {
            item: legacy,
            product_name_fr: Some("Clou 5cm".to_string()),
            product_name_ar: None,
            sku: None,
            unit_name_fr: None,
            unit_name_ar: None,
            product_missing: false,
        },
    ];
    let payments = vec![StoredPayment {
        id: "pay-1".to_string(),
        document_id: document.id,
        customer_id: None,
        amount_centimes: 3_200,
        method_code: "CASH".to_string(),
        cash_session_id: None,
    }];
    (document, views, payments)
}

struct FakeDocuments {
    document: Document,
    views: Vec<DocumentItemView>,
    payments: Vec<StoredPayment>,
}

impl FakeDocuments {
    fn ticket() -> Self {
        let (document, views, payments) = ticket_document();
        Self {
            document,
            views,
            payments,
        }
    }
}

impl DocumentRepository for FakeDocuments {
    fn create_draft(
        &self,
        _document: &Document,
        _items: &[DocumentItem],
        _origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests");
    }

    fn cancel_draft(&self, _id: DocumentId) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests");
    }

    fn get_by_id(&self, _id: DocumentId) -> Result<Option<Document>, ApplicationError> {
        Ok(Some(self.document.clone()))
    }

    fn get_by_number(
        &self,
        _doc_type_code: &str,
        _fiscal_year: i32,
        _number: &str,
    ) -> Result<Option<Document>, ApplicationError> {
        panic!("out of scope for TSPL tests");
    }

    fn get_items(&self, _id: DocumentId) -> Result<Vec<DocumentItem>, ApplicationError> {
        Ok(self.views.iter().map(|view| view.item.clone()).collect())
    }

    fn get_items_with_names(
        &self,
        _id: DocumentId,
    ) -> Result<Vec<DocumentItemView>, ApplicationError> {
        Ok(self.views.clone())
    }

    fn get_payments(&self, _id: DocumentId) -> Result<Vec<StoredPayment>, ApplicationError> {
        Ok(self.payments.clone())
    }

    fn get_print_status(&self, _id: DocumentId) -> Result<String, ApplicationError> {
        Ok("NONE".to_string())
    }

    fn finalize_document(
        &self,
        _document: &Document,
        _items: &[DocumentItem],
        _payments: &[StoredPayment],
        _origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for TSPL tests: zero writes");
    }

    fn convert_devis_to_bl(
        &self,
        _command: &ConvertDevisToBlCommand,
    ) -> Result<ConvertDevisToBlResult, ApplicationError> {
        panic!("out of scope for TSPL tests");
    }

    fn convert_bl_to_facture(
        &self,
        _command: &ConvertBlToFactureCommand,
    ) -> Result<ConvertBlToFactureResult, ApplicationError> {
        panic!("out of scope for TSPL tests");
    }

    fn search_documents(
        &self,
        _filter: &DocumentSearchFilter,
    ) -> Result<Vec<DocumentSummary>, ApplicationError> {
        panic!("out of scope for TSPL tests");
    }
}

fn spooler() -> FakeSettings {
    FakeSettings {
        transport: "spooler".to_string(),
        media: TsplMediaSetup::default(),
    }
}

#[test]
fn label_spools_exact_bytes_and_reports_count() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let outcome = PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
        .execute(
            &session_with(Role::Caissier),
            "Vis 8x40",
            1_000,
            "VIS-001",
            3,
        )
        .expect("spooler label prints");
    let payloads = transport.payloads();
    assert_eq!(payloads.len(), 1, "exactly one spooler submission");
    assert_eq!(outcome.bytes_sent, payloads[0].len());
    let text = String::from_utf8(payloads[0].clone()).expect("ascii-clean TSPL");
    assert!(text.starts_with("SIZE 40 mm, 20 mm\r\n"));
    // BRK-098 anatomy: brand micro-line from the identity wire, native
    // 32x48 price + right-aligned currency, Code128 + human-readable.
    assert!(text.contains("\"BARAKA QUINCAILLERIE\""));
    assert!(text.contains("TEXT 8,58,\"5\",0,1,1,\"10.00\""));
    assert!(text.contains("\"DZD\""));
    assert!(text.contains("BARCODE 8,106,\"128\",38,0,0,2,2,\"VIS-001\""));
    assert!(text.ends_with("PRINT 1,3\r\n"));
}

#[test]
fn label_preamble_uses_stored_media_setup() {
    let settings = FakeSettings {
        transport: "spooler".to_string(),
        media: TsplMediaSetup {
            gap_mm: 7,
            density: 12,
            speed: 2,
            probe_mode: TsplProbeMode::Normal,
        },
    };
    let transport = FakeTransport::new();
    PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
        .execute(
            &session_with(Role::Caissier),
            "Vis 8x40",
            1_000,
            "VIS-001",
            1,
        )
        .expect("spooler label prints");
    let payloads = transport.payloads();
    let text = String::from_utf8(payloads[0].clone()).expect("ascii-clean TSPL");
    assert!(
        text.starts_with("SIZE 40 mm, 20 mm\r\nGAP 7 mm, 0 mm\r\nSET RIBBON OFF\r\nDIRECTION 0,0\r\nREFERENCE 0,0\r\nOFFSET 0 mm\r\nSET PEEL OFF\r\nSET CUTTER OFF\r\nSET TEAR ON\r\nCLS\r\n"),
        "stored media rides the preamble, order pinned"
    );
}

// TEMPORARY-DIAGNOSTIC (BRK-100): stored probe mode rides the spooled bytes.
#[test]
fn label_preamble_uses_stored_probe_mode() {
    let settings = FakeSettings {
        transport: "spooler".to_string(),
        media: TsplMediaSetup {
            probe_mode: TsplProbeMode::NoCodepage,
            ..TsplMediaSetup::default()
        },
    };
    let transport = FakeTransport::new();
    PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
        .execute(
            &session_with(Role::Caissier),
            "Vis 8x40",
            1_000,
            "VIS-001",
            1,
        )
        .expect("spooler label prints");
    let payloads = transport.payloads();
    let text = String::from_utf8(payloads[0].clone()).expect("ascii-clean TSPL");
    assert!(
        text.starts_with(
            "SIZE 40 mm, 20 mm\r\nGAP 3 mm, 0 mm\r\nSET RIBBON OFF\r\nDIRECTION 0,0\r\nREFERENCE 0,0\r\nOFFSET 0 mm\r\nSET PEEL OFF\r\nSET CUTTER OFF\r\nSET TEAR ON\r\nCLS\r\n"
        ),
        "stored NoCodepage drops CODEPAGE from spooled bytes (vendor preamble carries none)"
    );
}

// BRK-101: raster-only print through the existing spooler transport.
#[test]
fn raw_bitmap_spools_validated_bytes_and_rejects_before_send() {
    let settings = spooler();
    let transport = FakeTransport::new();
    // 2x1 mixed frame: preamble + BITMAP + PRINT.
    let outcome = PrintRawBitmapUseCase::new(&settings, &transport)
        .execute(&session_with(Role::Caissier), &[0xAA, 0x55], 2, 1, 1)
        .expect("raster spools");
    let payloads = transport.payloads();
    assert_eq!(payloads.len(), 1, "one job spooled");
    assert_eq!(outcome.bytes_sent, payloads[0].len());
    assert!(
        payloads[0].starts_with(b"SIZE 2 mm, 1 mm\r\n"),
        "size from geometry"
    );
    assert!(
        !payloads[0].windows(8).any(|w| w == b"CODEPAGE"),
        "no CODEPAGE in raster job"
    );
    // Oversize frame: rejected pre-send, transport untouched.
    let before = transport.payloads().len();
    let huge = vec![0xAA_u8; 81 * 1638];
    PrintRawBitmapUseCase::new(&settings, &transport)
        .execute(&session_with(Role::Caissier), &huge, 81, 1638, 1)
        .expect_err("too wide rejected");
    assert_eq!(transport.payloads().len(), before, "nothing sent on reject");
    // Blank frame: rejected pre-send.
    PrintRawBitmapUseCase::new(&settings, &transport)
        .execute(&session_with(Role::Caissier), &[0x00, 0x00], 2, 1, 1)
        .expect_err("blank rejected");
    assert_eq!(transport.payloads().len(), before, "nothing sent on blank");
}

#[test]
fn label_rejects_non_spooler_transport_without_touching_the_queue() {
    for arm in ["serial", "escpos"] {
        let settings = FakeSettings {
            transport: arm.to_string(),
            media: TsplMediaSetup::default(),
        };
        let transport = FakeTransport::new();
        let error = PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
            .execute(
                &session_with(Role::Caissier),
                "Vis 8x40",
                1_000,
                "VIS-001",
                1,
            )
            .expect_err("non-spooler arm rejected");
        assert!(
            matches!(
                error,
                ApplicationError::Validation(DomainError::TsplTransportRequiresSpooler)
            ),
            "arm: {arm}"
        );
        assert!(transport.payloads().is_empty(), "queue untouched");
    }
}

#[test]
fn ticket_rejects_non_spooler_transport_without_touching_the_queue() {
    let settings = FakeSettings {
        transport: "serial".to_string(),
        media: TsplMediaSetup::default(),
    };
    let transport = FakeTransport::new();
    let documents = FakeDocuments::ticket();
    let error = PrintTsplTicketUseCase::new(&documents, &settings, &FakeIdentity, &transport)
        .execute(&session_with(Role::Caissier), documents.document.id)
        .expect_err("non-spooler arm rejected");
    assert!(matches!(
        error,
        ApplicationError::Validation(DomainError::TsplTransportRequiresSpooler)
    ));
    assert!(transport.payloads().is_empty(), "queue untouched");
}

#[test]
fn ticket_transcribes_discounted_and_legacy_lines_verbatim() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let documents = FakeDocuments::ticket();
    let outcome = PrintTsplTicketUseCase::new(&documents, &settings, &FakeIdentity, &transport)
        .execute(&session_with(Role::Caissier), documents.document.id)
        .expect("spooler ticket prints");
    let payloads = transport.payloads();
    assert_eq!(payloads.len(), 1);
    assert_eq!(outcome.bytes_sent, payloads[0].len());
    let text = String::from_utf8(payloads[0].clone()).expect("bytes kept");
    assert!(text.starts_with("SIZE 80 mm,"));
    // Per-unit line: stored net PU 900, never the raw 1000.
    assert!(text.contains("3 Pièce x 9.00 = 27.00"));
    assert!(
        !text.contains("10.00"),
        "raw PU never leaks into a discounted line"
    );
    // Legacy NULL line: raw PU transcribed verbatim.
    assert!(text.contains("5.00"));
    // Per-unit discount badge (stored value), absent for legacy NULL rows.
    assert!(text.contains("remise 1.00"));
    // BRK-098 doc meta: cashier + depot ride the stored identifiers.
    assert!(text.contains("Caissier: "));
    assert!(text.contains("Depot: "));
    // Graphique-parity totals hierarchy + stored global discount row.
    // DOC-02 (owner 1.b): the duplicated HT row is gone — Sous-total stays.
    assert!(text.contains("Sous-total: 32.00"));
    assert!(!text.contains("HT:"));
    assert!(text.contains("Remise totale: 0.00"));
    assert!(text.contains("TVA: 0.00"));
    assert!(text.contains("TOTAL TTC: 32.00"));
    assert!(text.contains("Payé (CASH): 32.00"));
    assert!(text.contains("Rendu: 0.00"));
    // Doc-number Code128 with human-readable number.
    assert!(text.contains("BARCODE"));
    assert!(text.ends_with("PRINT 1,1\r\n"));
}

#[test]
fn ticket_gate_reprint_document_but_label_allows_vendeur() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let documents = FakeDocuments::ticket();
    // Vendeur lacks ReprintDocument: ticket refused before any read.
    let error = PrintTsplTicketUseCase::new(&documents, &settings, &FakeIdentity, &transport)
        .execute(&session_with(Role::Vendeur), documents.document.id)
        .expect_err("vendeur cannot print tickets");
    assert!(matches!(error, ApplicationError::Unauthorized));
    assert!(transport.payloads().is_empty());
    // Vendeur holds PosSell: shelf labels allowed.
    PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
        .execute(
            &session_with(Role::Vendeur),
            "Vis 8x40",
            1_000,
            "VIS-001",
            1,
        )
        .expect("vendeur prints labels");
    assert_eq!(transport.payloads().len(), 1);
}

#[test]
fn label_copies_and_price_guards_reject_before_spooling() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let use_case = PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport);
    let session = session_with(Role::Caissier);
    assert!(matches!(
        use_case
            .execute(&session, "Vis 8x40", 1_000, "VIS-001", 0)
            .expect_err("0 copies rejected"),
        ApplicationError::Validation(DomainError::TsplCopiesOutOfRange)
    ));
    assert!(matches!(
        use_case
            .execute(&session, "Vis 8x40", -1, "VIS-001", 1)
            .expect_err("negative price rejected"),
        ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
    ));
    assert!(matches!(
        use_case
            .execute(&session, "Vis 8x40", 1_000, "VIS-é-001", 1)
            .expect_err("non-ascii barcode rejected"),
        ApplicationError::Validation(DomainError::TsplBarcodeUnencodable)
    ));
    assert!(transport.payloads().is_empty(), "queue untouched");
}

#[test]
fn ticket_missing_document_surfaces_not_found() {
    struct EmptyDocuments;
    impl DocumentRepository for EmptyDocuments {
        fn create_draft(
            &self,
            _document: &Document,
            _items: &[DocumentItem],
            _origin_device_id: &str,
        ) -> Result<(), ApplicationError> {
            panic!("out of scope")
        }
        fn cancel_draft(&self, _id: DocumentId) -> Result<(), ApplicationError> {
            panic!("out of scope")
        }
        fn get_by_id(&self, _id: DocumentId) -> Result<Option<Document>, ApplicationError> {
            Ok(None)
        }
        fn get_by_number(
            &self,
            _doc_type_code: &str,
            _fiscal_year: i32,
            _number: &str,
        ) -> Result<Option<Document>, ApplicationError> {
            panic!("out of scope")
        }
        fn get_items(&self, _id: DocumentId) -> Result<Vec<DocumentItem>, ApplicationError> {
            panic!("out of scope")
        }
        fn get_items_with_names(
            &self,
            _id: DocumentId,
        ) -> Result<Vec<DocumentItemView>, ApplicationError> {
            panic!("out of scope")
        }
        fn get_payments(&self, _id: DocumentId) -> Result<Vec<StoredPayment>, ApplicationError> {
            panic!("out of scope")
        }
        fn get_print_status(&self, _id: DocumentId) -> Result<String, ApplicationError> {
            panic!("out of scope")
        }
        fn finalize_document(
            &self,
            _document: &Document,
            _items: &[DocumentItem],
            _payments: &[StoredPayment],
            _origin_device_id: &str,
        ) -> Result<(), ApplicationError> {
            panic!("out of scope")
        }
        fn convert_devis_to_bl(
            &self,
            _command: &ConvertDevisToBlCommand,
        ) -> Result<ConvertDevisToBlResult, ApplicationError> {
            panic!("out of scope")
        }
        fn convert_bl_to_facture(
            &self,
            _command: &ConvertBlToFactureCommand,
        ) -> Result<ConvertBlToFactureResult, ApplicationError> {
            panic!("out of scope")
        }
        fn search_documents(
            &self,
            _filter: &DocumentSearchFilter,
        ) -> Result<Vec<DocumentSummary>, ApplicationError> {
            panic!("out of scope")
        }
    }
    let settings = spooler();
    let transport = FakeTransport::new();
    let error = PrintTsplTicketUseCase::new(&EmptyDocuments, &settings, &FakeIdentity, &transport)
        .execute(&session_with(Role::Caissier), DocumentId::new())
        .expect_err("missing document rejected");
    assert!(matches!(error, ApplicationError::NotFound(_)));
}

// TASK-PRT-02: a deterministic fake rasterer — every text becomes one
// solid row of `0xAA`-bytes (never blank, never oversized).
struct FakeLabelRaster;

impl baraka_application::ports::TsplLabelRasterPort for FakeLabelRaster {
    fn rasterize_label_text(
        &self,
        text: &str,
        _px: f32,
        max_width_dots: u32,
        _max_height_dots: u32,
    ) -> Result<baraka_application::ports::RasterLabelLine, ApplicationError> {
        let dots = (text.chars().count().max(1) * 8).min(max_width_dots as usize);
        let width_bytes = dots.div_ceil(8);
        Ok(baraka_application::ports::RasterLabelLine {
            data: vec![0xAA; width_bytes],
            width_bytes,
            height_rows: 1,
        })
    }
}

// TASK-PRT-02: an Arabic name routes through the rasterer — one BITMAP
// chunk per non-Latin line, every Latin line still TEXT.
#[test]
fn label_arabic_name_rasterizes_to_bitmap() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let outcome = PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
        .with_label_raster(&FakeLabelRaster)
        .execute(
            &session_with(Role::Admin),
            "JILI ISPANA ابيض",
            35_000,
            "AR02",
            1,
        )
        .expect("arabic label spools");
    let payloads = transport.payloads();
    assert_eq!(payloads.len(), 1);
    assert_eq!(outcome.bytes_sent, payloads[0].len());
    let bytes = &payloads[0];
    assert!(
        bytes.windows(b"BITMAP ".len()).any(|w| w == b"BITMAP "),
        "arabic line travels as BITMAP"
    );
    assert!(
        !bytes
            .windows(b"TEXT 8,16,".len())
            .any(|w| w == b"TEXT 8,16,"),
        "rasterized name line leaves no TEXT behind"
    );
    assert!(bytes.ends_with(b"\r\nPRINT 1,1\r\n"), "tail intact");
}

// TASK-PRT-02: without a rasterer the Arabic label fails LOUDLY with the
// stable code — never a silently-eaten dead label.
#[test]
fn label_arabic_name_without_rasterer_rejects_loudly() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let error = PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
        .execute(
            &session_with(Role::Admin),
            "JILI ISPANA ابيض",
            35_000,
            "AR02",
            1,
        )
        .expect_err("arabic without rasterer rejects");
    assert!(
        matches!(
            error,
            ApplicationError::Port(ref code) if code == "TSPL_LABEL_RASTER_UNAVAILABLE"
        ),
        "stable loud code, got: {error:?}"
    );
    assert!(transport.payloads().is_empty(), "nothing spooled");
}

// DOC-01: an Arabic ticket designation routes through the rasterer —
// BITMAP chunks for non-Latin lines, Latin lines still TEXT, tail intact.
#[test]
fn ticket_arabic_designation_rasterizes_to_bitmap() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let mut documents = FakeDocuments::ticket();
    documents.document.language = DocumentLanguage::Ar;
    documents.views[0].product_name_ar = Some("مسمار ابيض".to_string());
    let outcome = PrintTsplTicketUseCase::new(&documents, &settings, &FakeIdentity, &transport)
        .with_ticket_raster(&FakeLabelRaster)
        .execute(&session_with(Role::Admin), documents.document.id)
        .expect("arabic ticket spools");
    let payloads = transport.payloads();
    assert_eq!(payloads.len(), 1);
    assert_eq!(outcome.bytes_sent, payloads[0].len());
    let bytes = &payloads[0];
    assert!(
        bytes.windows(b"BITMAP ".len()).any(|w| w == b"BITMAP "),
        "arabic line travels as BITMAP"
    );
    assert!(
        bytes.windows(b"TEXT ".len()).any(|w| w == b"TEXT "),
        "latin lines (number, instant, cashier) stay TEXT"
    );
    assert!(bytes.ends_with(b"\r\nPRINT 1,1\r\n"), "tail intact");
}

// DOC-01: without a rasterer the Arabic ticket fails LOUDLY with the
// stable code — never a silently-eaten dead ticket.
#[test]
fn ticket_arabic_designation_without_rasterer_rejects_loudly() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let mut documents = FakeDocuments::ticket();
    documents.document.language = DocumentLanguage::Ar;
    documents.views[0].product_name_ar = Some("مسمار ابيض".to_string());
    let error = PrintTsplTicketUseCase::new(&documents, &settings, &FakeIdentity, &transport)
        .execute(&session_with(Role::Admin), documents.document.id)
        .expect_err("arabic without rasterer rejects");
    assert!(
        matches!(
            error,
            ApplicationError::Port(ref code) if code == "TSPL_LABEL_RASTER_UNAVAILABLE"
        ),
        "stable loud code, got: {error:?}"
    );
    assert!(transport.payloads().is_empty(), "nothing spooled");
}

// DOC-04: the stored template version pins the reprint — a tampered or
// future version rejects LOUDLY instead of silently drifting geometry.
#[test]
fn ticket_unknown_template_version_rejects_loudly_without_spooling() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let mut documents = FakeDocuments::ticket();
    documents.document.template_version = 99;
    let error = PrintTsplTicketUseCase::new(&documents, &settings, &FakeIdentity, &transport)
        .execute(&session_with(Role::Caissier), documents.document.id)
        .expect_err("unknown template rejects");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(ref code))
                if code == "TSPL_TEMPLATE_VERSION_UNKNOWN"
        ),
        "stable loud code, got: {error:?}"
    );
    assert!(transport.payloads().is_empty(), "nothing spooled");
}

// DOC-03: the preview IS the engine plan — same lines, same order,
// same exact texts; job_len predicts the spooled bytes byte-for-byte.
#[test]
fn label_preview_is_the_engine_plan() {
    let settings = spooler();
    let preview = PreviewTsplLabelUseCase::new(&settings, &FakeIdentity)
        .execute(&session_with(Role::Admin), "JILI ISPANA", 35_000, "AR02")
        .expect("latin preview renders");
    let slots: Vec<&str> = preview
        .lines
        .iter()
        .map(|line| line.slot.as_str())
        .collect();
    assert_eq!(slots, vec!["brand", "name1", "price", "currency", "human"]);
    let texts: Vec<&str> = preview
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert!(
        texts.contains(&"350.00"),
        "engine price spelling, got: {texts:?}"
    );
    assert!(texts.contains(&"DZD"));
    assert_eq!(
        (
            preview.barcode.x_dots,
            preview.barcode.y_dots,
            preview.barcode.height_dots
        ),
        (8, 106, 38),
        "engine barcode origin"
    );
    // Byte-identity proof: predicted length == the actual spooled job
    // (same brand the identity port supplies — the print path's input).
    let label = baraka_domain::printing::TsplLabel {
        brand_text: "BARAKA QUINCAILLERIE",
        name: "JILI ISPANA",
        price_text: "350.00",
        currency_text: "DZD",
        barcode: "AR02",
        copies: 1,
        media: baraka_domain::printing::TsplMediaSetup::default(),
    };
    let job = baraka_domain::printing::render_label_40x20(&label).expect("renders");
    assert_eq!(preview.job_len, job.len());
}

// DOC-03: Arabic preview preserves the verbatim text (the modal renders
// it shaped); without a rasterer the preview rejects LOUDLY — it never
// previews a job the printer would eat.
#[test]
fn label_preview_arabic_preserves_and_rejects_loudly() {
    let settings = spooler();
    let error = PreviewTsplLabelUseCase::new(&settings, &FakeIdentity)
        .execute(
            &session_with(Role::Admin),
            "JILI ISPANA ابيض",
            35_000,
            "AR02",
        )
        .expect_err("arabic preview without rasterer rejects");
    assert!(
        matches!(
            error,
            ApplicationError::Port(ref code) if code == "TSPL_LABEL_RASTER_UNAVAILABLE"
        ),
        "stable loud code, got: {error:?}"
    );
    let preview = PreviewTsplLabelUseCase::new(&settings, &FakeIdentity)
        .with_preview_raster(&FakeLabelRaster)
        .execute(
            &session_with(Role::Admin),
            "JILI ISPANA ابيض",
            35_000,
            "AR02",
        )
        .expect("arabic preview renders with rasterer");
    assert!(
        preview.lines.iter().any(|line| line.text.contains("ابيض")),
        "arabic text preserved verbatim"
    );
}

// TASK-PRT-02: the pure-ASCII path never touches the rasterer and stays
// byte-identical to the domain renderer.
#[test]
fn label_ascii_path_never_touches_rasterer() {
    use baraka_domain::printing::{render_label_40x20, TsplLabel};
    struct PanicRaster;
    impl baraka_application::ports::TsplLabelRasterPort for PanicRaster {
        fn rasterize_label_text(
            &self,
            _text: &str,
            _px: f32,
            _max_width_dots: u32,
            _max_height_dots: u32,
        ) -> Result<baraka_application::ports::RasterLabelLine, ApplicationError> {
            panic!("ascii path must not rasterize")
        }
    }
    let settings = spooler();
    let transport = FakeTransport::new();
    PrintTsplLabelUseCase::new(&settings, &FakeIdentity, &transport)
        .with_label_raster(&PanicRaster)
        .execute(
            &session_with(Role::Admin),
            "JILI ISPANA",
            35_000,
            "VIS-001",
            1,
        )
        .expect("ascii label spools");
    let payloads = transport.payloads();
    assert_eq!(payloads.len(), 1);
    let expected = render_label_40x20(&TsplLabel {
        brand_text: "BARAKA QUINCAILLERIE",
        name: "JILI ISPANA",
        price_text: "350.00",
        currency_text: "DZD",
        barcode: "VIS-001",
        copies: 1,
        media: TsplMediaSetup::default(),
    })
    .expect("domain renders");
    assert_eq!(payloads[0], expected, "ascii bytes pinned");
}

// TASK-PRT-03: the ticket self-test speaks the production 80 mm renderer
// with stored media — fixed ASCII payload, one spooler submission, and a
// non-ADMIN session is rejected before any byte is rendered.
#[test]
fn test_ticket_spools_fixed_ascii_ticket_and_reports_count() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let outcome = PrintTsplTestTicketUseCase::new(&settings, &transport)
        .execute(&session_with(Role::Admin))
        .expect("spooler ticket self-test prints");
    let payloads = transport.payloads();
    assert_eq!(payloads.len(), 1, "exactly one spooler submission");
    assert_eq!(outcome.bytes_sent, payloads[0].len());
    let text = String::from_utf8(payloads[0].clone()).expect("ascii-clean TSPL");
    assert!(text.contains("\"BARAKA\""), "fixed store brand");
    assert!(text.contains("TEST-000001"), "fixed ticket number");
    assert!(text.contains("ARTICLE TEST 2"), "both fixed lines ride");
    assert!(
        text.ends_with("\r\n"),
        "job terminated (TASK-PRT-01 CRLF law)"
    );
}

#[test]
fn test_ticket_rejects_non_admin_before_render() {
    let settings = spooler();
    let transport = FakeTransport::new();
    let result = PrintTsplTestTicketUseCase::new(&settings, &transport)
        .execute(&session_with(Role::Caissier));
    assert!(
        matches!(result, Err(ApplicationError::Unauthorized)),
        "ticket self-test is an ADMIN seam"
    );
    assert!(
        transport.payloads().is_empty(),
        "nothing spooled on refusal"
    );
}
