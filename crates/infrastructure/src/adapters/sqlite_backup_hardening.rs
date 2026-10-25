//! Automatic-backup hardening persistence (TASK-BRK-092).
//!
//! Schedule cadence/time live in `backup.*` keys of the existing `settings`
//! table (zero schema changes), and the USB mirror step is a copy-verified
//! filesystem copy (copy, never move — the local snapshot is never touched).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use baraka_application::errors::ApplicationError;
use baraka_application::ports::BackupHardeningPort;
use baraka_domain::backup::{BackupCadence, BackupSchedule, UsbCopyOutcome};
use baraka_domain::errors::DomainError;
use rusqlite::Connection;

use crate::persistence::sha256::Sha256;

/// Static, non-leaking mapping for unexpected storage failures.
fn store_failure() -> ApplicationError {
    ApplicationError::Port("backup hardening store failure".to_string())
}

fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// Settings-table keys for the automatic-backup schedule.
pub mod backup_keys {
    /// Cadence wire code (`disabled` / `daily` / `weekly`).
    pub const CADENCE: &str = "backup.cadence";
    /// Local wall-clock `HH:MM`.
    pub const TIME: &str = "backup.time";
    /// Removable USB destination directory.
    pub const USB_DEST: &str = "backup.usb_destination";
}

/// Schedule + USB-mirror persistence over the shared SQLite connection.
pub struct SqliteBackupHardening {
    db: Arc<Mutex<Connection>>,
}

impl SqliteBackupHardening {
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

    fn write_key(&self, key: &str, value: &str) -> Result<(), ApplicationError> {
        let guard = self.lock()?;
        let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        guard
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                rusqlite::params![key, value, stamp],
            )
            .map_err(|_| store_failure())?;
        Ok(())
    }
}

impl BackupHardeningPort for SqliteBackupHardening {
    fn load_schedule(&self) -> Result<BackupSchedule, ApplicationError> {
        let cadence = self
            .read_key(backup_keys::CADENCE)
            .map(|code| BackupCadence::from_code(code.trim()).unwrap_or(BackupCadence::Disabled))
            .unwrap_or(BackupCadence::Disabled);
        let time_minutes = self
            .read_key(backup_keys::TIME)
            .and_then(|text| BackupSchedule::parse_time(text.trim()));
        Ok(BackupSchedule {
            cadence,
            time_minutes,
        })
    }

    fn save_schedule(&self, schedule: &BackupSchedule) -> Result<(), ApplicationError> {
        self.write_key(backup_keys::CADENCE, schedule.cadence.as_code())?;
        match (schedule.cadence, schedule.time_minutes) {
            (BackupCadence::Disabled, _) => {
                // Keep the stored time out of the due-check when disabled.
                self.write_key(backup_keys::TIME, "")?;
            }
            (_, Some(minutes)) => {
                let hhmm = BackupSchedule::format_time(minutes)
                    .ok_or_else(|| coded("INVALID_BACKUP_TIME"))?;
                self.write_key(backup_keys::TIME, &hhmm)?;
            }
            (_, None) => {
                self.write_key(backup_keys::TIME, "")?;
            }
        }
        Ok(())
    }

    fn usb_destination(&self) -> Result<Option<PathBuf>, ApplicationError> {
        let value = self
            .read_key(backup_keys::USB_DEST)
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());
        Ok(value.map(PathBuf::from))
    }

    fn set_usb_destination(&self, path: Option<&Path>) -> Result<(), ApplicationError> {
        let value = match path {
            Some(dest) => dest.to_string_lossy().trim().to_string(),
            None => String::new(),
        };
        self.write_key(backup_keys::USB_DEST, &value)
    }

    fn copy_to_usb(
        &self,
        source: &Path,
        dest_dir: &Path,
    ) -> Result<UsbCopyOutcome, ApplicationError> {
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| coded("USB_COPY_INVALID_SOURCE"))?;
        fs::create_dir_all(dest_dir).map_err(|_| coded("USB_DESTINATION_UNWRITABLE"))?;
        let dest = dest_dir.join(file_name);
        // Copy (never move): the local snapshot stays put regardless.
        fs::copy(source, &dest).map_err(|_| coded("USB_COPY_FAILED"))?;
        let source_size = fs::metadata(source)
            .map_err(|_| coded("USB_COPY_FAILED"))?
            .len();
        let dest_size = fs::metadata(&dest)
            .map_err(|_| coded("USB_COPY_FAILED"))?
            .len();
        if source_size != dest_size {
            let _ = fs::remove_file(&dest);
            return Err(coded("USB_COPY_SIZE_MISMATCH"));
        }
        // SHA-256 re-check over the exact copied bytes.
        let source_hash = hash_file(source)?;
        let dest_hash = hash_file(&dest)?;
        if source_hash != dest_hash {
            let _ = fs::remove_file(&dest);
            return Err(coded("USB_COPY_HASH_MISMATCH"));
        }
        Ok(UsbCopyOutcome {
            destination: dest_dir.to_string_lossy().to_string(),
            file_name: file_name.to_string(),
            file_size_bytes: dest_size,
            verified: true,
        })
    }
}

/// Streamed SHA-256 hex over a file's bytes (bounded RAM).
fn hash_file(path: &Path) -> Result<String, ApplicationError> {
    use std::io::Read;
    let mut file = fs::File::open(path).map_err(|_| coded("USB_COPY_FAILED"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| coded("USB_COPY_FAILED"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finish_hex())
}
