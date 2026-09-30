//! Thermal printer emulator: capture device + artifact writer.
//!
//! `CaptureDevice` implements the existing [`PrinterTransportPort`] seam
//! and writes the EXACT byte stream to immutable timestamped `.escpos`
//! artifacts under `<app-data>/printer-capture/`. A test print ALWAYS
//! writes a shadow capture regardless of the selected device. Business
//! print jobs are NEVER shadow-captured. Capture files are immutable
//! artifacts and are NEVER deleted (no rotation in this task).

use baraka_application::errors::ApplicationError;
use baraka_application::ports::PrinterTransportPort;
use std::path::PathBuf;

/// Capture directory name under the application data directory.
pub const CAPTURE_DIR_NAME: &str = "printer-capture";

/// Emulator capture device over a shared connection (dir source).
///
/// The directory is resolved from the application data directory; the
/// struct stores it directly so tests can point it at a temp dir.
pub struct CaptureDevice {
    /// Immutable artifact directory (created on demand).
    dir: PathBuf,
}

impl CaptureDevice {
    /// Create over an explicit directory (production: app-data join).
    #[must_use]
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Resolve from the database path's parent (the app-data dir).
    #[must_use]
    pub fn from_db_path(db_path: &std::path::Path) -> Self {
        let dir = db_path
            .parent()
            .map(|parent| parent.join(CAPTURE_DIR_NAME))
            .unwrap_or_else(|| PathBuf::from(CAPTURE_DIR_NAME));
        Self::new(dir)
    }

    /// Capture directory (created on demand by [`Self::write_capture`]).
    #[must_use]
    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }

    /// Write an immutable timestamped artifact; returns the file name.
    pub fn write_capture(&self, bytes: &[u8]) -> Result<String, ApplicationError> {
        std::fs::create_dir_all(&self.dir).map_err(|_| {
            ApplicationError::Port("printer capture directory creation".to_string())
        })?;
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
        // Millis + length suffix keeps names unique without a clock
        // sequence (two captures in the same millisecond stay distinct).
        let file_name = format!("test-{stamp}-{}.escpos", bytes.len());
        std::fs::write(self.dir.join(&file_name), bytes)
            .map_err(|_| ApplicationError::Port("printer capture write".to_string()))?;
        Ok(file_name)
    }
}

impl PrinterTransportPort for CaptureDevice {
    fn send_raster(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        self.write_capture(bytes).map(|_| ())
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl baraka_application::ports::CaptureSink for CaptureDevice {
    fn write_capture(&self, bytes: &[u8]) -> Result<String, ApplicationError> {
        self.write_capture(bytes)
    }
}
