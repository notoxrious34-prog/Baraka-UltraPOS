//! Spooler transport selection + validation (TASK-BRK-089-C1).
//!
//! No printer is attached in CI/dev: these tests prove selection logic,
//! settings round-trip, validation, and error mapping — never a live
//! print. Live byte delivery stays HARDWARE-PENDING (owner USB test).

use std::sync::{Arc, Mutex};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    PrinterSettingsPort, PrinterTransportPort, SessionContext, SessionToken,
};
use baraka_application::use_cases::printing::UpdatePrinterSettingsUseCase;
use baraka_infrastructure::adapters::SqlitePrinterSettings;
use baraka_infrastructure::persistence::connection::open_connection;
use baraka_infrastructure::persistence::migration::run_migrations;
use baraka_infrastructure::printing::transport_router::RoutingPrinterTransport;

/// Temp DB (migrated) + settings repo sharing one connection handle so the
/// router under test reads exactly what the test persists.
fn harness() -> (
    tempfile::TempDir,
    SqlitePrinterSettings,
    Arc<Mutex<rusqlite::Connection>>,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut conn = open_connection(&dir.path().join("spooler.db")).expect("open");
    run_migrations(&mut conn).expect("migrate");
    let shared = Arc::new(Mutex::new(conn));
    let settings = SqlitePrinterSettings::new(Arc::clone(&shared));
    (dir, settings, shared)
}

fn admin_session() -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: baraka_domain::user::UserId(
            uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid"),
        ),
        role: baraka_domain::role::Role::Admin,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    }
}

#[test]
fn defaults_select_the_serial_arm() {
    let (_dir, settings, db) = harness();
    let current = settings.get_printer_settings().expect("defaults");
    assert_eq!(current.transport, "serial");
    assert_eq!(current.queue_name, "");
    // Router falls back to serial with zero spooler configuration...
    let router = RoutingPrinterTransport::new(db);
    // ...and the failure is the serial arm's missing-port diagnostic, NOT
    // a spooler error (no queue involved).
    let error = router.send_raster(b"hello").expect_err("no COM port here");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(
                baraka_domain::errors::DomainError::PrinterTransportUnavailable
            )
        ),
        "unexpected: {error:?}"
    );
}

#[test]
fn spooler_arm_routes_to_the_configured_queue_and_names_failures() {
    let (_dir, settings, db) = harness();
    settings
        .update_printer_settings(
            "COM1",
            9_600,
            Some(80),
            Some("spooler"),
            Some("NO_SUCH_QUEUE_BARAKA_9917"),
            None,
        )
        .expect("persist spooler selection");
    let current = settings.get_printer_settings().expect("reread");
    assert_eq!(current.transport, "spooler");
    assert_eq!(current.queue_name, "NO_SUCH_QUEUE_BARAKA_9917");

    // The router now takes the spooler arm: the unknown queue fails with a
    // diagnostic that names the exact Win32 call + queue + OS code.
    let router = RoutingPrinterTransport::new(db);
    let error = router
        .send_raster(b"\x1b\x40receipt-bytes")
        .expect_err("unknown queue must fail");
    match error {
        ApplicationError::Validation(
            baraka_domain::errors::DomainError::InvalidCanonicalString(detail),
        ) => {
            assert!(detail.contains("OpenPrinterW"), "names the call: {detail}");
            assert!(
                detail.contains("NO_SUCH_QUEUE_BARAKA_9917"),
                "names the queue: {detail}"
            );
        }
        other => panic!("wrong taxonomy (must stay a Validation): {other:?}"),
    }
    // Availability probes the queue without spooling a document.
    assert!(!router.is_available());
}

#[test]
fn spooler_arm_without_queue_is_unavailable_like_serial_without_port() {
    let (_dir, settings, db) = harness();
    settings
        .update_printer_settings("COM1", 9_600, Some(80), Some("spooler"), None, None)
        .expect("spooler arm, no queue yet");
    let router = RoutingPrinterTransport::new(db);
    // Empty queue = unconfigured: same taxonomy as serial missing-port.
    let error = router.send_raster(b"x").expect_err("no queue");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(
                baraka_domain::errors::DomainError::PrinterTransportUnavailable
            )
        ),
        "unexpected: {error:?}"
    );
    assert!(!router.is_available());
}

#[test]
fn transport_and_queue_round_trip_with_none_keeping_stored_values() {
    let (_dir, settings, _db) = harness();
    settings
        .update_printer_settings(
            "COM1",
            9_600,
            Some(80),
            Some("spooler"),
            Some("XPrinter XP-410B"),
            None,
        )
        .expect("store");
    // None keeps both stored values while port/baud still update.
    settings
        .update_printer_settings("COM2", 115_200, None, None, None, None)
        .expect("keep");
    let current = settings.get_printer_settings().expect("reread");
    assert_eq!(current.serial_port, "COM2");
    assert_eq!(current.serial_baud, 115_200);
    assert_eq!(current.transport, "spooler");
    assert_eq!(current.queue_name, "XPrinter XP-410B");
}

#[test]
fn use_case_rejects_bad_transport_and_queue_before_any_write() {
    let (_dir, settings, _db) = harness();
    // Baseline first so "zero writes" is observable.
    UpdatePrinterSettingsUseCase::new(&settings)
        .execute(
            &admin_session(),
            "COM1",
            9_600,
            Some(80),
            Some("spooler"),
            Some("Q"),
            None,
        )
        .expect("baseline");

    let bad_transport = UpdatePrinterSettingsUseCase::new(&settings)
        .execute(
            &admin_session(),
            "COM1",
            9_600,
            Some(80),
            Some("usb"),
            None,
            None,
        )
        .expect_err("usb is not an arm");
    assert!(
        matches!(
            bad_transport,
            ApplicationError::Validation(
                baraka_domain::errors::DomainError::InvalidCanonicalString(_)
            )
        ),
        "unexpected: {bad_transport:?}"
    );

    let empty_queue = UpdatePrinterSettingsUseCase::new(&settings)
        .execute(
            &admin_session(),
            "COM1",
            9_600,
            Some(80),
            None,
            Some("   "),
            None,
        )
        .expect_err("empty queue rejected");
    assert!(
        matches!(
            empty_queue,
            ApplicationError::Validation(
                baraka_domain::errors::DomainError::InvalidCanonicalString(_)
            )
        ),
        "unexpected: {empty_queue:?}"
    );

    let long: String = "x".repeat(221);
    let long_queue = UpdatePrinterSettingsUseCase::new(&settings)
        .execute(
            &admin_session(),
            "COM1",
            9_600,
            Some(80),
            None,
            Some(&long),
            None,
        )
        .expect_err("overlong queue rejected");
    assert!(
        matches!(
            long_queue,
            ApplicationError::Validation(
                baraka_domain::errors::DomainError::InvalidCanonicalString(_)
            )
        ),
        "unexpected: {long_queue:?}"
    );

    // All three rejections wrote nothing: baseline intact.
    let after = settings.get_printer_settings().expect("reread");
    assert_eq!(
        (after.transport.as_str(), after.queue_name.as_str()),
        ("spooler", "Q")
    );
}

#[test]
fn ticket_render_mode_defaults_to_escpos_and_round_trips() {
    let (_dir, settings, _db) = harness();
    // Absent key reads as the escpos default (existing shops unchanged).
    assert_eq!(
        settings
            .get_printer_settings()
            .expect("defaults")
            .ticket_render_mode,
        "escpos"
    );
    settings
        .update_printer_settings("COM1", 9_600, None, None, None, Some("graphique"))
        .expect("store graphique");
    assert_eq!(
        settings
            .get_printer_settings()
            .expect("reread")
            .ticket_render_mode,
        "graphique"
    );
    // `None` keeps the stored mode while port/baud still update.
    settings
        .update_printer_settings("COM2", 115_200, None, None, None, None)
        .expect("keep");
    let kept = settings.get_printer_settings().expect("reread");
    assert_eq!(kept.ticket_render_mode, "graphique");
    assert_eq!(kept.serial_port, "COM2");
}

#[test]
fn use_case_rejects_bad_render_mode_before_any_write() {
    let (_dir, settings, _db) = harness();
    UpdatePrinterSettingsUseCase::new(&settings)
        .execute(
            &admin_session(),
            "COM1",
            9_600,
            Some(80),
            None,
            None,
            Some("graphique"),
        )
        .expect("baseline");

    let bad_mode = UpdatePrinterSettingsUseCase::new(&settings)
        .execute(
            &admin_session(),
            "COM1",
            9_600,
            Some(80),
            None,
            None,
            Some("tspl"),
        )
        .expect_err("tspl is not a render mode");
    assert!(
        matches!(
            &bad_mode,
            ApplicationError::Validation(
                baraka_domain::errors::DomainError::InvalidCanonicalString(_)
            )
        ),
        "unexpected: {bad_mode:?}"
    );
    assert!(
        format!("{bad_mode}").contains("ticket_render_mode must be 'escpos' or 'graphique'"),
        "rejection names the contract: {bad_mode}"
    );

    // The rejection wrote nothing: baseline intact.
    assert_eq!(
        settings
            .get_printer_settings()
            .expect("reread")
            .ticket_render_mode,
        "graphique"
    );
}
