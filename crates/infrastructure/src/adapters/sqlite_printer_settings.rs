//! SQLite printer configuration repository (`settings` table).

use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{PrinterSettings, PrinterSettingsPort};
use rusqlite::Connection;

use crate::persistence::transaction::run_transaction;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("printer settings store failure".to_string())
}

/// Printer configuration persistence over a shared SQLite connection.
pub struct SqlitePrinterSettings {
    db: Arc<Mutex<Connection>>,
}

impl SqlitePrinterSettings {
    /// Wrap the shared connection.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, ApplicationError> {
        self.db
            .lock()
            .map_err(|_| ApplicationError::Port("database lock poisoned".to_string()))
    }

    fn read_key(&self, key: &str) -> Option<String> {
        let guard = self.lock().ok()?;
        guard
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .ok()
    }
}

impl PrinterSettingsPort for SqlitePrinterSettings {
    fn get_printer_settings(&self) -> Result<PrinterSettings, ApplicationError> {
        let baud = self
            .read_key("printer.serial_baud")
            .and_then(|text| text.trim().parse::<u32>().ok())
            .unwrap_or(9_600);
        let width = self
            .read_key("printer.paper_width_dots")
            .and_then(|text| text.trim().parse::<u32>().ok())
            .unwrap_or(576);
        Ok(PrinterSettings {
            serial_port: self.read_key("printer.serial_port").unwrap_or_default(),
            serial_baud: baud,
            paper_width_dots: width,
        })
    }

    fn update_printer_settings(
        &self,
        serial_port: &str,
        serial_baud: u32,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let port = serial_port.trim().to_string();
        let baud = serial_baud.to_string();
        run_transaction(&mut guard, |tx| {
            for (key, value) in [
                ("printer.serial_port", port.as_str()),
                ("printer.serial_baud", baud.as_str()),
            ] {
                tx.execute(
                    "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                    rusqlite::params![key, value, stamp],
                )?;
            }
            Ok(())
        })
        .map_err(|_| store_failure())
    }
}

/// Namespaced keys for store identity + diagnostics device selection.
pub mod store_keys {
    /// Shop display name.
    pub const NAME: &str = "store.name";
    /// Street address.
    pub const ADDRESS: &str = "store.address";
    /// Phone.
    pub const PHONE: &str = "store.phone";
    /// Fiscal identifiers.
    pub const NIF: &str = "store.nif";
    /// Fiscal identifiers.
    pub const NIS: &str = "store.nis";
    /// Fiscal identifiers.
    pub const RC: &str = "store.rc";
    /// Fiscal identifiers.
    pub const AI: &str = "store.ai";
    /// Free-text footer.
    pub const FOOTER: &str = "store.footer_message";
    /// `"physical"` (default) or `"capture"` (diagnostics-scoped).
    pub const DEVICE_KIND: &str = "printing.device.kind";
}

fn write_key(
    tx: &rusqlite::Transaction,
    key: &str,
    value: &str,
    stamp: &str,
) -> Result<(), rusqlite::Error> {
    tx.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![key, value, stamp],
    )?;
    Ok(())
}

fn audit_settings(
    tx: &rusqlite::Transaction,
    action: &str,
    detail: &str,
    actor: &str,
    stamp: &str,
) -> Result<(), crate::persistence::InfrastructureError> {
    crate::adapters::sqlite_audit_log::insert_cash_audit_row(
        tx,
        action,
        "settings",
        "store",
        None,
        &format!("{{\"detail\":\"{}\"}}", detail.replace('"', "'")),
        actor,
        stamp,
    )
    .map_err(|error| crate::persistence::InfrastructureError::Config(format!("{error}")))
}

impl baraka_application::ports::StoreIdentityPort for SqlitePrinterSettings {
    fn load_identity(&self) -> Result<baraka_domain::printing::StoreIdentity, ApplicationError> {
        Ok(baraka_domain::printing::StoreIdentity {
            store_name: self.read_key(store_keys::NAME).unwrap_or_default(),
            store_address: self.read_key(store_keys::ADDRESS).unwrap_or_default(),
            store_phone: self.read_key(store_keys::PHONE).unwrap_or_default(),
            nif: self.read_key(store_keys::NIF).unwrap_or_default(),
            nis: self.read_key(store_keys::NIS).unwrap_or_default(),
            rc: self.read_key(store_keys::RC).unwrap_or_default(),
            ai: self.read_key(store_keys::AI).unwrap_or_default(),
            ticket_footer_message: self.read_key(store_keys::FOOTER).unwrap_or_default(),
        })
    }

    fn save_identity(
        &self,
        identity: &baraka_domain::printing::StoreIdentity,
        actor: &str,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            for (key, value) in [
                (store_keys::NAME, identity.store_name.as_str()),
                (store_keys::ADDRESS, identity.store_address.as_str()),
                (store_keys::PHONE, identity.store_phone.as_str()),
                (store_keys::NIF, identity.nif.as_str()),
                (store_keys::NIS, identity.nis.as_str()),
                (store_keys::RC, identity.rc.as_str()),
                (store_keys::AI, identity.ai.as_str()),
                (store_keys::FOOTER, identity.ticket_footer_message.as_str()),
            ] {
                write_key(tx, key, value, &stamp)?;
            }
            audit_settings(
                tx,
                "STORE_IDENTITY_UPDATED",
                &identity.store_name,
                actor,
                &stamp,
            )
        })
        .map_err(|_| store_failure())
    }

    fn device_kind(&self) -> Result<String, ApplicationError> {
        Ok(match self.read_key(store_keys::DEVICE_KIND).as_deref() {
            Some("capture") => "capture".to_string(),
            _ => "physical".to_string(),
        })
    }

    fn save_device(&self, kind: &str, actor: &str) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            write_key(tx, store_keys::DEVICE_KIND, kind, &stamp)?;
            audit_settings(tx, "PRINT_DEVICE_UPDATED", kind, actor, &stamp)
        })
        .map_err(|_| store_failure())
    }

    fn append_settings_audit(
        &self,
        actor: &str,
        action: &str,
        detail: &str,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        run_transaction(&mut guard, |tx| {
            audit_settings(tx, action, detail, actor, &stamp)
        })
        .map_err(|_| store_failure())
    }
}
