//! Baraka UltraPOS desktop harness (Tauri v2).
//!
//! Thin shell: resolves the OS-standard application data directory, opens
//! the local database there, runs embedded migrations, resolves the
//! sovereign terminal identifier, wires every backend into command state,
//! spawns the print outbox worker, and registers the typed handlers. No
//! business logic here.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use baraka_application::use_cases::backup_hardening::ScheduledBackupUseCase;
use baraka_commands::ipc::AppState;
use baraka_infrastructure::adapters::{
    Argon2PasswordHasher, InMemorySessionStore, SqliteBackupHardening, SqliteBackupManager,
    SqliteCashSessionRepository, SqliteCatalogRepository, SqliteCheckoutRepository,
    SqliteCustomerRepository, SqliteDocumentRepository, SqliteFiscalSettings,
    SqlitePrintJobRepository, SqlitePrinterSettings, SqlitePurchaseRepository,
    SqliteReportingRepository, SqliteReturnRepository, SqliteStockRepository,
    SqliteSupplierRepository, SqliteUserRepository, SqliteWarehouseRepository, SystemClock,
};
use baraka_infrastructure::persistence::connection::{
    ensure_database_dir, open_connection, resolve_database_path,
};
use baraka_infrastructure::persistence::data_dir::{
    prepare_data_dir, resolve_data_dir, DataDirError, DATA_DIR_ENV,
};
use baraka_infrastructure::persistence::migration::{probe_newer_database, run_migrations};
use baraka_infrastructure::persistence::terminal_context::resolve_device_id;
use baraka_infrastructure::printing::capture::CaptureDevice;
use baraka_infrastructure::printing::label_raster::TsplLabelRasterer;
use baraka_infrastructure::printing::rasterizer::EscPosRasterizer;
use baraka_infrastructure::printing::transport_router::RoutingPrinterTransport;
use baraka_infrastructure::printing::worker_thread::{spawn_print_worker, WorkerDeps};
use tauri::Manager;

/// Upper bound for worker-thread join during shutdown.
const SHUTDOWN_JOIN_TIMEOUT: Duration = Duration::from_secs(10);

/// Map a startup failure to a clear, logged, non-panicking setup error.
///
/// Every fallible startup I/O operation flows through here: the message
/// goes to stderr and aborts `setup` with a plain `std::io::Error` (no new
/// error-type family, no `unwrap`/`expect` on startup I/O).
fn startup_error(context: &str, detail: impl std::fmt::Display) -> std::io::Error {
    let message = format!("Baraka UltraPOS startup failed at {context}: {detail}");
    eprintln!("{message}");
    std::io::Error::other(message)
}

/// Map an invalid `BARAKA_DATA_DIR` override to the exact developer diagnostic.
///
/// There is no fallback: the caller aborts startup, the default (live)
/// directory is never used, and the database is never opened.
fn data_dir_invalid(error: DataDirError) -> std::io::Error {
    let message = format!(
        "Baraka UltraPOS cannot start: {DATA_DIR_ENV} is invalid ({}).",
        error.code()
    );
    eprintln!("{message}");
    std::io::Error::other(message)
}

/// Run the Tauri application (called from `main.rs`).
pub fn run() {
    // Outbox worker lifecycle: stop flag shared with the setup closure and
    // the shutdown handler; the spawned handle escapes via this cell so the
    // bounded join below still applies after `run` returns.
    let worker_stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&worker_stop);
    let stop_for_setup = Arc::clone(&worker_stop);
    let worker_handle_cell: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    let handle_cell = Arc::clone(&worker_handle_cell);
    tauri::Builder::default()
        .manage(Arc::clone(&worker_stop))
        // BRK-069: single-instance guard — a second launch focuses the
        // existing main window instead of starting a rival instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        // Auto-update (GitHub Releases): signature-checked, dialog-free —
        // all UX lives in the Settings Update Center.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            // ADR-049: the SQLite file lives in the OS-standard per-user
            // application data directory (Tauri official path API), under
            // the `default_db_file_name()` file name. No CWD-relative
            // fallback exists: the launch working directory can never fork
            // or orphan business data again.
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| startup_error("app data directory resolution", error))?;
            // BRK-058: opt-in isolated data directory (developer/test facility).
            // Unset or blank keeps the default paths byte-for-byte; an invalid
            // value aborts here, before any database is opened, with no fallback.
            let data_dir_override_value = std::env::var_os(DATA_DIR_ENV);
            let resolved = resolve_data_dir(&app_data_dir, data_dir_override_value.as_deref())
                .map_err(data_dir_invalid)?;
            prepare_data_dir(&resolved, &app_data_dir).map_err(data_dir_invalid)?;
            let data_dir = resolved.path.clone();
            let data_dir_override = resolved.is_override;
            ensure_database_dir(&data_dir)
                .map_err(|error| startup_error("application data directory creation", error))?;
            let db_path = resolve_database_path(&data_dir);
            let mut conn =
                open_connection(&db_path).map_err(|error| startup_error("database open", error))?;
            run_migrations(&mut conn)
                .map_err(|error| startup_error("database migration", error))?;
            // TASK-BRK-104 / ADR-106: refuse a database last written by a
            // newer binary (downgrade attempt). The probe runs after
            // migrations succeed and never writes: a future schema version
            // aborts launch here instead of opening an unreadable database.
            probe_newer_database(&conn)
                .map_err(|error| startup_error("database version", error))?;
            // Section 34 startup protocol: resolve the sovereign terminal identifier
            // after schema verification and before UI initialization. The same value
            // serves as origin_device_id and terminal_id system-wide.
            let terminal_id = resolve_device_id(&conn)
                .map_err(|error| startup_error("terminal identifier resolution", error))?;
            let db = Arc::new(Mutex::new(conn));
            let clock = Arc::new(SystemClock);
            let catalog = Arc::new(SqliteCatalogRepository::new(Arc::clone(&db)));
            let documents = Arc::new(SqliteDocumentRepository::new(Arc::clone(&db)));
            let print_jobs = Arc::new(SqlitePrintJobRepository::new(Arc::clone(&db)));
            let transport = Arc::new(RoutingPrinterTransport::new(Arc::clone(&db)));
            let renderer = Arc::new(EscPosRasterizer);
            let printer_settings = Arc::new(SqlitePrinterSettings::new(Arc::clone(&db)));
            let fiscal_settings = Arc::new(SqliteFiscalSettings::new(Arc::clone(&db)));
            let backups = Arc::new(SqliteBackupManager::new(Arc::clone(&db), db_path.clone()));
            let hardening = Arc::new(SqliteBackupHardening::new(Arc::clone(&db)));
            // TASK-BRK-092 on-launch due-check: silent when not due (never a
            // startup failure); a due failure is also silent here — the
            // admin-gated `backup_schedule_run` command surfaces it on
            // action, and the durable last-status row records every attempt.
            let _ =
                ScheduledBackupUseCase::new(backups.as_ref(), hardening.as_ref(), clock.as_ref())
                    .run_system(
                        &data_dir.join("backups"),
                        30,
                        &terminal_id,
                        env!("CARGO_PKG_VERSION"),
                    );
            let state = AppState {
                users: Arc::new(SqliteUserRepository::new(Arc::clone(&db))),
                sessions: Arc::new(InMemorySessionStore::new(clock.clone())),
                clock,
                hasher: Arc::new(Argon2PasswordHasher),
                catalog: catalog.clone(),
                imports: catalog.clone(),
                categories: catalog.clone(),
                warehouses: Arc::new(SqliteWarehouseRepository::new(Arc::clone(&db))),
                stock: Arc::new(SqliteStockRepository::new(Arc::clone(&db))),
                customers: Arc::new(SqliteCustomerRepository::new(Arc::clone(&db))),
                checkout: Arc::new(SqliteCheckoutRepository::new(Arc::clone(&db))),
                documents: documents.clone(),
                print_jobs: print_jobs.clone(),
                printer_settings: printer_settings.clone(),
                store_identity: printer_settings.clone(),
                fiscal: fiscal_settings,
                printer_transport: transport.clone(),
                printer_capture: Arc::new(CaptureDevice::from_db_path(&db_path)),
                printer_renderer: renderer.clone(),
                label_raster: Arc::new(TsplLabelRasterer),
                cash: Arc::new(SqliteCashSessionRepository::new(Arc::clone(&db))),
                suppliers: Arc::new(SqliteSupplierRepository::new(Arc::clone(&db))),
                purchases: Arc::new(SqlitePurchaseRepository::new(Arc::clone(&db))),
                reports: Arc::new(SqliteReportingRepository::new(Arc::clone(&db))),
                returns: Arc::new(SqliteReturnRepository::new(Arc::clone(&db))),
                backups: backups.clone(),
                backup_hardening: hardening,
                factory_reset: backups,
                backup_dir: data_dir.join("backups"),
                terminal_id,
            };
            app.manage(state);
            let worker_handle = spawn_print_worker(
                WorkerDeps {
                    documents,
                    jobs: print_jobs,
                    products: catalog,
                    transport,
                    renderer,
                    settings: printer_settings.clone(),
                    printer: printer_settings.clone(),
                },
                stop_for_setup,
            );
            if let Ok(mut cell) = handle_cell.lock() {
                *cell = Some(worker_handle);
            }
            // BRK-058: the main window is created here (config `create: false`)
            // so override mode can redirect the WebView2 profile directory and
            // mark the title. Default mode rebuilds the same configured window.
            let window_config = app
                .handle()
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .ok_or_else(|| {
                    startup_error("main window configuration", "missing label \"main\"")
                })?
                .clone();
            let handle = app.handle().clone();
            let mut builder = tauri::WebviewWindowBuilder::from_config(&handle, &window_config)
                .map_err(|error| startup_error("main window creation", error))?;
            if data_dir_override {
                let webview_dir = data_dir.join("webview2");
                std::fs::create_dir_all(&webview_dir)
                    .map_err(|error| startup_error("override webview directory creation", error))?;
                builder = builder.data_directory(webview_dir);
            }
            let window = builder
                .build()
                .map_err(|error| startup_error("main window creation", error))?;
            if data_dir_override {
                window
                    .set_title("Baraka UltraPOS [BARAKA_DATA_DIR]")
                    .map_err(|error| startup_error("override window title", error))?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            baraka_commands::ipc::app_health,
            baraka_commands::ipc::auth_login,
            baraka_commands::ipc::auth_logout,
            baraka_commands::ipc::check_first_run_status,
            baraka_commands::ipc::bootstrap_first_admin,
            baraka_commands::ipc::users_list,
            baraka_commands::ipc::users_create,
            baraka_commands::ipc::users_update,
            baraka_commands::ipc::users_reset_credentials,
            baraka_commands::ipc::catalog_create_product,
            baraka_commands::ipc::catalog_update_product,
            baraka_commands::ipc::catalog_delete_product,
            baraka_commands::ipc::catalog_get_product,
            baraka_commands::ipc::catalog_search,
            baraka_commands::ipc::catalog_get_metrics,
            baraka_commands::ipc::catalog_define_kit,
            baraka_commands::ipc::catalog_import_products,
            baraka_commands::ipc::category_list,
            baraka_commands::ipc::category_create,
            baraka_commands::ipc::category_update,
            baraka_commands::ipc::category_seed_starter,
            baraka_commands::ipc::warehouses_create,
            baraka_commands::ipc::warehouses_list,
            baraka_commands::ipc::warehouses_update,
            baraka_commands::ipc::warehouses_delete,
            baraka_commands::ipc::warehouses_stock_presence,
            baraka_commands::ipc::stock_get,
            baraka_commands::ipc::stock_adjust,
            baraka_commands::ipc::stock_transfer,
            baraka_commands::ipc::sales_checkout,
            baraka_commands::ipc::sales_preview_totals,
            baraka_commands::ipc::convert_devis_to_bl,
            baraka_commands::ipc::convert_bl_to_facture,
            baraka_commands::ipc::sales_create_draft,
            baraka_commands::ipc::sales_discard_draft,
            baraka_commands::ipc::sales_get,
            baraka_commands::ipc::documents_search,
            baraka_commands::ipc::returns_get_source_summary,
            baraka_commands::ipc::returns_process,
            baraka_commands::ipc::documents_get_return_context,
            baraka_commands::ipc::documents_process_return,
            baraka_commands::ipc::backups_create,
            baraka_commands::ipc::backups_list,
            baraka_commands::ipc::backups_verify,
            baraka_commands::ipc::backups_restore_precheck,
            baraka_commands::ipc::backups_restore,
            baraka_commands::ipc::backup_hardening_status,
            baraka_commands::ipc::backup_hardening_configure,
            baraka_commands::ipc::backup_schedule_run,
            baraka_commands::ipc::factory_reset_precheck,
            baraka_commands::ipc::factory_reset_execute,
            baraka_commands::ipc::customers_create,
            baraka_commands::ipc::customers_update,
            baraka_commands::ipc::customers_delete,
            baraka_commands::ipc::customers_search,
            baraka_commands::ipc::customers_get,
            baraka_commands::ipc::customers_list,
            baraka_commands::ipc::credit_record_payment,
            baraka_commands::ipc::credit_get_statement,
            baraka_commands::ipc::documents_get_print_status,
            baraka_commands::ipc::documents_reprint,
            baraka_commands::ipc::print_tspl_label,
            baraka_commands::ipc::print_tspl_ticket,
            baraka_commands::ipc::print_raw_bitmap,
            baraka_commands::ipc::printer_get_settings,
            baraka_commands::ipc::printer_update_settings,
            baraka_commands::ipc::printer_test_print,
            baraka_commands::ipc::printer_test_tspl_label,
            baraka_commands::ipc::printer_test_tspl_ticket,
            baraka_commands::ipc::store_identity_get,
            baraka_commands::ipc::store_identity_save,
            baraka_commands::ipc::fiscal_settings_get,
            baraka_commands::ipc::fiscal_settings_update,
            baraka_commands::ipc::cash_open,
            baraka_commands::ipc::cash_begin_closing,
            baraka_commands::ipc::cash_cancel_closing,
            baraka_commands::ipc::cash_confirm_closing,
            baraka_commands::ipc::cash_get_active_session,
            baraka_commands::ipc::cash_record_expense,
            baraka_commands::ipc::cash_get_session_report,
            baraka_commands::ipc::suppliers_create,
            baraka_commands::ipc::suppliers_update,
            baraka_commands::ipc::suppliers_delete,
            baraka_commands::ipc::suppliers_get,
            baraka_commands::ipc::suppliers_search,
            baraka_commands::ipc::suppliers_list,
            baraka_commands::ipc::suppliers_get_statement,
            baraka_commands::ipc::purchases_create_order,
            baraka_commands::ipc::purchases_update_order,
            baraka_commands::ipc::purchases_receive_goods,
            baraka_commands::ipc::purchases_record_invoice,
            baraka_commands::ipc::purchases_record_payment,
            baraka_commands::ipc::purchases_get_document,
            baraka_commands::ipc::reports_daily_sales,
            baraka_commands::ipc::reports_sales_by_product,
            baraka_commands::ipc::reports_sales_by_category,
            baraka_commands::ipc::reports_gross_margin,
            baraka_commands::ipc::reports_credit_outstanding,
            baraka_commands::ipc::reports_stock_movement_history,
            baraka_commands::ipc::reports_cash_sessions_history,
            baraka_commands::ipc::reports_low_stock,
            baraka_commands::ipc::reports_user_activity
        ])
        .build(tauri::generate_context!())
        .expect("build Baraka UltraPOS")
        .run(move |_app, event| {
            // Section 35 shutdown protocol: stop the worker before exit.
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                stop_signal.store(true, Ordering::SeqCst);
            }
        });
    // Bounded join: responsive sleeps inside the worker bound this well
    // under the timeout; a hung join must never block process exit here
    // (the OS reclaims the thread; DB state stays consistent via txn).
    // A missing handle (setup never completed) simply skips the join.
    let started = Instant::now();
    loop {
        let finished = worker_handle_cell
            .lock()
            .map(|cell| cell.as_ref().is_none_or(JoinHandle::is_finished))
            .unwrap_or(true);
        if finished {
            break;
        }
        if started.elapsed() >= SHUTDOWN_JOIN_TIMEOUT {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
