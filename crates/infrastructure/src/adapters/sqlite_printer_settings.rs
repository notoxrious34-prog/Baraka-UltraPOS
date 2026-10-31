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
        // Canonical millimeters (default 80); legacy dot values migrate
        // forward: 384 reads back as 58, anything else as 80.
        let mm = self
            .read_key("printer.paper_width_mm")
            .and_then(|text| text.trim().parse::<u32>().ok())
            .or_else(|| {
                self.read_key("printer.paper_width_dots")
                    .and_then(|text| text.trim().parse::<u32>().ok())
                    .map(|dots| if dots <= 384 { 58 } else { 80 })
            })
            .unwrap_or(80);
        let width = baraka_domain::printing::PaperWidth::from_mm(mm);
        let printing = baraka_domain::printing::TsplMediaSetup::default();
        // TSPL media setup: absent keys read as defaults; out-of-range
        // stored values fall back to defaults (validated on write, but
        // never trusted on read — no silent clamps in the renderer).
        let gap_mm = self
            .read_key("tspl.label_gap_mm")
            .and_then(|text| text.trim().parse::<u32>().ok())
            .filter(|gap| *gap <= baraka_domain::printing::TSPL_GAP_MM_MAX)
            .unwrap_or(printing.gap_mm);
        let density = self
            .read_key("tspl.print_density")
            .and_then(|text| text.trim().parse::<u8>().ok())
            .filter(|density| *density <= baraka_domain::printing::TSPL_DENSITY_MAX)
            .unwrap_or(printing.density);
        let speed = self
            .read_key("tspl.print_speed")
            .and_then(|text| text.trim().parse::<u8>().ok())
            .filter(|speed| {
                *speed >= baraka_domain::printing::TSPL_SPEED_MIN
                    && *speed <= baraka_domain::printing::TSPL_SPEED_MAX
            })
            .unwrap_or(printing.speed);
        Ok(PrinterSettings {
            serial_port: self.read_key("printer.serial_port").unwrap_or_default(),
            serial_baud: baud,
            paper_width_dots: width.dot_width(),
            paper_width_mm: width.as_mm(),
            transport: self
                .read_key("printer.transport")
                .map(|value| value.trim().to_string())
                .filter(|value| {
                    value == baraka_domain::printing::TRANSPORT_SERIAL
                        || value == baraka_domain::printing::TRANSPORT_SPOOLER
                })
                .unwrap_or_else(|| baraka_domain::printing::TRANSPORT_SERIAL.to_string()),
            queue_name: self.read_key("printer.queue_name").unwrap_or_default(),
            ticket_render_mode: self
                .read_key("printer.ticket_render_mode")
                .map(|value| value.trim().to_string())
                .filter(|value| {
                    value == baraka_domain::printing::TICKET_RENDER_ESC_POS
                        || value == baraka_domain::printing::TICKET_RENDER_GRAPHIQUE
                })
                .unwrap_or_else(|| baraka_domain::printing::TICKET_RENDER_ESC_POS.to_string()),
            tspl_gap_mm: gap_mm,
            tspl_density: density,
            tspl_speed: speed,
        })
    }

    fn update_printer_settings(
        &self,
        serial_port: &str,
        serial_baud: u32,
        paper_width_mm: Option<u32>,
        transport: Option<&str>,
        queue_name: Option<&str>,
        ticket_render_mode: Option<&str>,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let port = serial_port.trim().to_string();
        let baud = serial_baud.to_string();
        let mut pairs: Vec<(&str, String)> =
            vec![("printer.serial_port", port), ("printer.serial_baud", baud)];
        // Millimeters are canonical; dots are re-derived so legacy readers
        // keep working. `None` keeps the stored width untouched.
        if let Some(mm) = paper_width_mm {
            let width = baraka_domain::printing::PaperWidth::from_mm(mm);
            pairs.push(("printer.paper_width_mm", width.as_mm().to_string()));
            pairs.push(("printer.paper_width_dots", width.dot_width().to_string()));
        }
        // Transport arm and queue name: `None` keeps the stored value.
        // Both were validated upstream before any write.
        if let Some(selection) = transport {
            pairs.push(("printer.transport", selection.trim().to_string()));
        }
        if let Some(queue) = queue_name {
            pairs.push(("printer.queue_name", queue.trim().to_string()));
        }
        // Ticket render mode: `None` keeps the stored mode. Validated
        // upstream before any write.
        if let Some(mode) = ticket_render_mode {
            pairs.push(("printer.ticket_render_mode", mode.trim().to_string()));
        }
        run_transaction(&mut guard, |tx| {
            for (key, value) in &pairs {
                write_key(tx, key, value, &stamp)?;
            }
            Ok(())
        })
        .map_err(|_| store_failure())
    }

    fn update_tspl_media_settings(
        &self,
        gap_mm: Option<u32>,
        density: Option<u8>,
        speed: Option<u8>,
    ) -> Result<(), ApplicationError> {
        // `None` keeps the stored value; `Some` was validated upstream.
        let mut guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut pairs: Vec<(&str, String)> = Vec::new();
        if let Some(gap) = gap_mm {
            pairs.push(("tspl.label_gap_mm", gap.to_string()));
        }
        if let Some(density) = density {
            pairs.push(("tspl.print_density", density.to_string()));
        }
        if let Some(speed) = speed {
            pairs.push(("tspl.print_speed", speed.to_string()));
        }
        run_transaction(&mut guard, |tx| {
            for (key, value) in &pairs {
                write_key(tx, key, value, &stamp)?;
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
