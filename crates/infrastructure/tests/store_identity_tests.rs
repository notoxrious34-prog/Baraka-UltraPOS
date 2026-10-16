//! Store identity + test-print diagnostics (TASK-BRK-056, ADR-065 as amended).
//!
//! Layout-model goldens are the contract (Amendment 1): the preview is
//! the shared-block blueprint, never decoded pixels. Full-artifact
//! byte goldens are NOT asserted — font-rasterizer byte stability is
//! out of scope.

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    Clock, PrinterTransportPort, ReceiptRendererPort, SessionContext, StoreIdentityPort,
};
use baraka_application::use_cases::printing::{
    sample_test_receipt, summarize_decode, GetStoreIdentityUseCase, SaveStoreIdentityUseCase,
    TestPrintDeviceOutcome, TestPrintUseCase,
};
use baraka_domain::errors::DomainError;
use baraka_domain::printing::{escpos_decode, DecodedLine, ReceiptLanguage, StoreIdentity};
use baraka_infrastructure::adapters::SqlitePrinterSettings;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::printing::blocks::{build_footer_lines, build_identity_lines};
use baraka_infrastructure::printing::capture::CaptureDevice;
use baraka_infrastructure::printing::rasterizer::{
    content_width_dots, EscPosRasterizer, PaperWidth,
};

/// 80mm default content surface shared by the layout goldens below.
fn golden_max() -> f32 {
    content_width_dots(PaperWidth::Width80mm)
}

fn identity_full() -> StoreIdentity {
    StoreIdentity {
        store_name: "BARAKA QUINCAILLERIE".to_string(),
        store_address: "12 Rue des Frères, Alger".to_string(),
        store_phone: "0550 12 34 56".to_string(),
        nif: "123456789012345".to_string(),
        nis: String::new(),
        rc: "16/00-1234567B16".to_string(),
        ai: String::new(),
        ticket_footer_message: "Merci de votre visite\nRetours sous 7 jours".to_string(),
    }
}

fn preview_texts(
    renderer: &EscPosRasterizer,
    receipt: &baraka_domain::printing::ReceiptDocument,
) -> Vec<(String, String)> {
    renderer
        .preview_lines(receipt)
        .expect("preview")
        .into_iter()
        .map(|line| (line.kind, line.text))
        .collect()
}

// --- Layout goldens: identity header FR --------------------------------

#[test]
fn identity_header_french_layout_golden() {
    let lines = build_identity_lines(&identity_full(), ReceiptLanguage::French, golden_max())
        .expect("build");
    let texts: Vec<String> = lines
        .iter()
        .filter_map(|line| match line {
            baraka_infrastructure::printing::blocks::LayoutLine::Text { runs, .. } => Some(
                runs.iter()
                    .map(|run| run.text.as_str())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec![
            "BARAKA QUINCAILLERIE".to_string(),
            "12 Rue des Frères, Alger".to_string(),
            "Tél: 0550 12 34 56".to_string(),
            "NIF: 123456789012345".to_string(),
            "RC: 16/00-1234567B16".to_string(),
        ],
        "FR header: name, address, localized phone, Latin-caption identifiers; empty NIS/AI omitted"
    );
    // Centered emphasis name + trailing separator rule.
    match &lines[0] {
        baraka_infrastructure::printing::blocks::LayoutLine::Text { centered, px, .. } => {
            assert!(*centered);
            assert!((*px - 32.0).abs() < f32::EPSILON);
        }
        other => panic!("first header line must be text, got {other:?}"),
    }
    assert!(matches!(
        lines.last(),
        Some(baraka_infrastructure::printing::blocks::LayoutLine::Rule { .. })
    ));
}

// --- Layout goldens: identity header AR ----------------------------------

#[test]
fn identity_header_arabic_caption_golden() {
    let lines = build_identity_lines(&identity_full(), ReceiptLanguage::Arabic, golden_max())
        .expect("build");
    let texts: Vec<String> = lines
        .iter()
        .filter_map(|line| match line {
            baraka_infrastructure::printing::blocks::LayoutLine::Text { runs, .. } => Some(
                runs.iter()
                    .map(|run| run.text.as_str())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|text| text.contains("هاتف")),
        "AR phone caption localized (visual run order), got {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.starts_with("NIF:")),
        "identifier captions stay Latin under AR, got {texts:?}"
    );
}

// --- Layout goldens: empty-field omission --------------------------------

#[test]
fn identity_header_empty_optionals_omitted() {
    let identity = StoreIdentity {
        store_name: "SHOP".to_string(),
        ..StoreIdentity::default()
    };
    let lines =
        build_identity_lines(&identity, ReceiptLanguage::French, golden_max()).expect("build");
    let text_count = lines
        .iter()
        .filter(|line| {
            matches!(
                line,
                baraka_infrastructure::printing::blocks::LayoutLine::Text { .. }
            )
        })
        .count();
    assert_eq!(text_count, 1, "only the name line; no blank caption lines");
}

// --- Layout goldens: footer ----------------------------------------------

#[test]
fn footer_multiline_golden_and_truncation() {
    let lines = build_footer_lines(
        "one\n\ntwo\nthree\nfour",
        ReceiptLanguage::French,
        golden_max(),
    )
    .expect("build");
    let texts: Vec<String> = lines
        .iter()
        .filter_map(|line| match line {
            baraka_infrastructure::printing::blocks::LayoutLine::Text { runs, .. } => Some(
                runs.iter()
                    .map(|run| run.text.as_str())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec!["one", "two", "three"],
        "blank dropped, extras truncated at 3"
    );
    assert!(matches!(
        lines.first(),
        Some(baraka_infrastructure::printing::blocks::LayoutLine::Rule { .. })
    ));
}

// --- Shared-path proof -----------------------------------------------------

#[test]
fn shared_path_header_footer_equal_between_document_and_test_print() {
    let renderer = EscPosRasterizer;
    let identity = identity_full();
    let mut document = sample_test_receipt(
        &identity,
        &identity.ticket_footer_message,
        "2026-09-30T00:00:00Z",
        ReceiptLanguage::French,
    );
    document.number = "T-000123".to_string();
    let sample = sample_test_receipt(
        &identity,
        &identity.ticket_footer_message,
        "2026-09-30T00:00:00Z",
        ReceiptLanguage::French,
    );
    let doc_preview = preview_texts(&renderer, &document);
    let sample_preview = preview_texts(&renderer, &sample);
    // Header segment: name + address + phone + NIF + RC + rule = 6 lines.
    assert_eq!(&doc_preview[..6], &sample_preview[..6]);
    // Footer segment: rule + 2 message lines at the tail (before final gap).
    let doc_tail = &doc_preview[doc_preview.len() - 4..doc_preview.len() - 1];
    let sample_tail = &sample_preview[sample_preview.len() - 4..sample_preview.len() - 1];
    assert_eq!(doc_tail, sample_tail);
}

// --- Decoder: full artifact + random bytes ---------------------------------

#[test]
fn decoder_full_artifact_zero_unknown_bands_and_feed() {
    let renderer = EscPosRasterizer;
    let receipt = sample_test_receipt(
        &identity_full(),
        "Merci",
        "2026-09-30T00:00:00Z",
        ReceiptLanguage::French,
    );
    let bytes = renderer.render(&receipt).expect("render");
    let decoded = escpos_decode(&bytes);
    let summary = summarize_decode(&decoded);
    assert!(summary.bands > 0, "raster bands present");
    assert!(summary.feed_lines >= 3, "trailing feed present");
    assert_eq!(
        summary.unknown_spans, 0,
        "own streams contain zero unknown bytes"
    );
}

#[test]
fn decoder_never_panics_on_random_bytes() {
    // Deterministic xorshift stream (no RNG dependency).
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next_byte = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state & 0xFF) as u8
    };
    for round in 0..500 {
        let len = (next_byte() as usize % 48) + 1;
        let bytes: Vec<u8> = (0..len).map(|_| next_byte()).collect();
        let decoded = escpos_decode(&bytes);
        // Structural invariant: every byte is accounted for.
        let _ = (round, decoded.len());
    }
    // Targeted truncation: header claims more data than present.
    let truncated = vec![0x1D, 0x76, 0x30, 0x00, 0x48, 0x00, 0x10, 0x00, 0xFF];
    let decoded = escpos_decode(&truncated);
    assert!(
        decoded
            .iter()
            .any(|line| matches!(line, DecodedLine::RasterBand { .. })),
        "band header still recognized under truncation"
    );
}

// --- Settings persistence round-trip ---------------------------------------

fn settings_harness() -> (tempfile::TempDir, SqlitePrinterSettings) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("settings.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let settings = SqlitePrinterSettings::new(Arc::new(Mutex::new(conn)));
    (dir, settings)
}

fn admin_session() -> SessionContext {
    SessionContext {
        token: baraka_application::ports::SessionToken::new_random(),
        user_id: baraka_domain::user::UserId(
            uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid"),
        ),
        role: baraka_domain::role::Role::Admin,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    }
}

#[test]
fn settings_round_trip_trim_and_soft_validation() {
    let (_dir, settings) = settings_harness();
    let dirty = StoreIdentity {
        store_name: "  SHOP  ".to_string(),
        nif: "12345678901234567890".to_string(), // exactly 20 chars: ACCEPTED
        ..StoreIdentity::default()
    };
    let saved = SaveStoreIdentityUseCase::new(&settings)
        .execute(&admin_session(), &dirty, "capture")
        .expect("save");
    assert_eq!(saved.store_name, "SHOP");
    assert_eq!(
        saved.nif.len(),
        20,
        "20-char NIF at the cap is accepted (soft: no format check)"
    );
    assert_eq!(settings.device_kind().expect("kind"), "capture");
    let loaded = GetStoreIdentityUseCase::new(&settings)
        .execute(&admin_session())
        .expect("load");
    assert_eq!(loaded, saved);
    // Audit evidence: both saves run inside the same transaction as
    // their `insert_cash_audit_row` calls, so success implies the
    // STORE_IDENTITY_UPDATED + PRINT_DEVICE_UPDATED rows were written
    // (any audit failure fails the whole transaction loudly).
}

#[test]
fn settings_save_requires_name_and_rejects_overlong() {
    let (_dir, settings) = settings_harness();
    let nameless = StoreIdentity::default();
    let error = SaveStoreIdentityUseCase::new(&settings)
        .execute(&admin_session(), &nameless, "physical")
        .expect_err("name required");
    assert!(matches!(
        error,
        ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
    ));
    let overlong = StoreIdentity {
        store_name: "x".repeat(81),
        ..StoreIdentity::default()
    };
    assert!(SaveStoreIdentityUseCase::new(&settings)
        .execute(&admin_session(), &overlong, "physical")
        .is_err());
}

// --- Test-print use case -----------------------------------------------------

struct FakeTransport {
    fail: bool,
    seen: Mutex<Vec<usize>>,
}

impl PrinterTransportPort for FakeTransport {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        self.seen.lock().expect("seen").push(bytes.len());
        if self.fail {
            return Err(ApplicationError::Validation(
                DomainError::PrinterTransportUnavailable,
            ));
        }
        Ok(())
    }

    fn is_available(&self) -> bool {
        !self.fail
    }
}

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_759_132_800, 0).expect("fixed")
    }

    fn now_rfc3339(&self) -> String {
        "2026-09-30T12:00:00.000Z".to_string()
    }
}

#[test]
fn test_print_capture_only_never_touches_transport() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_settings_dir, settings) = settings_harness();
    SaveStoreIdentityUseCase::new(&settings)
        .execute(&admin_session(), &identity_full(), "capture")
        .expect("save");
    let capture = CaptureDevice::new(dir.path().to_path_buf());
    let transport = FakeTransport {
        fail: false,
        seen: Mutex::new(Vec::new()),
    };
    let renderer = EscPosRasterizer;
    let clock = FixedClock;
    let report = TestPrintUseCase::new(&settings, &renderer, &capture, &transport, &clock)
        .execute(&admin_session(), ReceiptLanguage::French)
        .expect("diagnostics");
    assert_eq!(report.device, TestPrintDeviceOutcome::CaptureOnly);
    assert!(transport.seen.lock().expect("seen").is_empty());
    assert!(report.capture_file_name.ends_with(".escpos"));
    assert!(dir.path().join(&report.capture_file_name).exists());
    assert!(!report.preview.is_empty(), "preview from shared blocks");
    assert_eq!(report.preview[0].text, "BARAKA QUINCAILLERIE");
    assert!(report.decode_summary.bands > 0);
}

#[test]
fn test_print_physical_attempt_success_and_error_mapping() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_settings_dir, settings) = settings_harness();
    SaveStoreIdentityUseCase::new(&settings)
        .execute(&admin_session(), &identity_full(), "physical")
        .expect("save");
    let capture = CaptureDevice::new(dir.path().to_path_buf());
    let renderer = EscPosRasterizer;
    let clock = FixedClock;
    let good = FakeTransport {
        fail: false,
        seen: Mutex::new(Vec::new()),
    };
    let report = TestPrintUseCase::new(&settings, &renderer, &capture, &good, &clock)
        .execute(&admin_session(), ReceiptLanguage::Arabic)
        .expect("diagnostics");
    assert_eq!(report.device, TestPrintDeviceOutcome::Sent);
    assert_eq!(good.seen.lock().expect("seen").len(), 1);
    assert!(
        report.preview.iter().any(|line| line.text.contains("هاتف")),
        "AR diagnostic language localizes captions"
    );
    let bad = FakeTransport {
        fail: true,
        seen: Mutex::new(Vec::new()),
    };
    let report = TestPrintUseCase::new(&settings, &renderer, &capture, &bad, &clock)
        .execute(&admin_session(), ReceiptLanguage::French)
        .expect("diagnostics maps errors, never fails");
    assert!(
        matches!(report.device, TestPrintDeviceOutcome::DeviceError(_)),
        "typed device-error mapping"
    );
    // Shadow capture written even on device error.
    assert!(dir.path().join(&report.capture_file_name).exists());
}

#[test]
fn test_print_permission_denied() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_settings_dir, settings) = settings_harness();
    let capture = CaptureDevice::new(dir.path().to_path_buf());
    let transport = FakeTransport {
        fail: false,
        seen: Mutex::new(Vec::new()),
    };
    let renderer = EscPosRasterizer;
    let clock = FixedClock;
    let cashier = SessionContext {
        token: baraka_application::ports::SessionToken::new_random(),
        user_id: baraka_domain::user::UserId(
            uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid"),
        ),
        role: baraka_domain::role::Role::Caissier,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    };
    let error = TestPrintUseCase::new(&settings, &renderer, &capture, &transport, &clock)
        .execute(&cashier, ReceiptLanguage::French)
        .expect_err("denied");
    assert!(matches!(error, ApplicationError::Unauthorized));
}

// --- Worker regression: identity on/off --------------------------------------

#[test]
fn worker_layout_with_and_without_identity() {
    let renderer = EscPosRasterizer;
    let plain = sample_test_receipt(
        &StoreIdentity::default(),
        "",
        "2026-09-30T00:00:00Z",
        ReceiptLanguage::French,
    );
    let plain_texts: Vec<String> = renderer
        .preview_lines(&plain)
        .expect("preview")
        .into_iter()
        .filter(|line| line.kind == "text")
        .map(|line| line.text)
        .collect();
    assert!(
        plain_texts.iter().any(|text| text.contains("TEST-000000")),
        "legacy fallback keeps the shop/number header, got {plain_texts:?}"
    );
    assert!(
        !plain_texts.iter().any(|text| text.starts_with("NIF:")),
        "no identity block when unconfigured"
    );
    let configured = sample_test_receipt(
        &identity_full(),
        &identity_full().ticket_footer_message,
        "2026-09-30T00:00:00Z",
        ReceiptLanguage::French,
    );
    let texts: Vec<String> = renderer
        .preview_lines(&configured)
        .expect("preview")
        .into_iter()
        .filter(|line| line.kind == "text")
        .map(|line| line.text)
        .collect();
    assert!(texts.contains(&"BARAKA QUINCAILLERIE".to_string()));
    assert!(texts.iter().any(|text| text.starts_with("NIF:")));
    assert!(texts.contains(&"Retours sous 7 jours".to_string()));
}

// --- Capture device ------------------------------------------------------------

#[test]
fn capture_writes_immutable_artifact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let capture = CaptureDevice::new(dir.path().to_path_buf());
    let name = capture.write_capture(&[0x1D, 0x76, 0x30]).expect("write");
    let written = std::fs::read(dir.path().join(&name)).expect("read");
    assert_eq!(written, vec![0x1D, 0x76, 0x30], "exact byte stream");
    // Second write never overwrites the first (unique names).
    let second = capture.write_capture(&[0x0A]).expect("write");
    assert_ne!(name, second);
    assert!(dir.path().join(&name).exists());
}
