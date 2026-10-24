//! Minimal safe wrapper over the Win32 print spooler RAW path (ADR-103).
//!
//! This crate is the ONLY unsafe site in the Baraka UltraPOS workspace:
//! every Win32 FFI call lives here, each in a `unsafe` block with a
//! SAFETY comment, behind a safe API. All other crates keep
//! `unsafe_code = "forbid"` and use only this API.
//!
//! Sequence per job: `OpenPrinterW` → `StartDocPrinterW` (datatype RAW,
//! bytes pass unaltered) → `WritePrinter` loop → `EndDocPrinter` →
//! `ClosePrinter` (RAII [`SpoolerJob`] Drop). Any failure maps to
//! [`SpoolerError`], which always names the failing call — never a panic,
//! never a crash, even for unknown queue names.

#[cfg(windows)]
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, OpenPrinterW, StartDocPrinterW, WritePrinter, DOC_INFO_1W,
        PRINTER_ACCESS_USE, PRINTER_DEFAULTSW, PRINTER_HANDLE,
    },
};

/// Maximum queue-name length in characters (spec: non-empty, ≤ 220).
pub const MAX_QUEUE_NAME_CHARS: usize = 220;

/// Fixed spooler datatype: RAW bytes pass through unaltered.
const RAW_DATATYPE: &str = "RAW";

/// Fixed spooler document name for submitted jobs.
const DOC_NAME: &str = "Baraka UltraPOS";

/// Validated Windows print-queue name (exact spooler queue name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpoolerQueue {
    name: String,
}

/// Failure of a spooler operation. Always names the failing Win32 call
/// plus the queue and the OS code — suitable for diagnostics, never a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpoolerError {
    /// Queue name fails validation (empty or longer than 220 chars).
    /// No OS call was attempted.
    InvalidQueueName(String),
    /// A named Win32 spooler call failed. `os_code` is the raw
    /// `GetLastError` code captured at the failure site.
    OsCallFailed {
        call: &'static str,
        queue: String,
        os_code: u32,
    },
    /// This platform has no Win32 spooler (non-Windows build).
    UnsupportedPlatform,
}

impl std::fmt::Display for SpoolerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpoolerError::InvalidQueueName(detail) => {
                write!(f, "invalid spooler queue name: {detail}")
            }
            SpoolerError::OsCallFailed {
                call,
                queue,
                os_code,
            } => {
                write!(
                    f,
                    "spooler call {call} failed for queue '{queue}' (OS code {os_code})"
                )
            }
            SpoolerError::UnsupportedPlatform => {
                write!(f, "Win32 spooler unavailable on this platform")
            }
        }
    }
}

impl std::error::Error for SpoolerError {}

impl SpoolerQueue {
    /// Validate a queue name without touching the OS.
    pub fn new(name: &str) -> Result<Self, SpoolerError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SpoolerError::InvalidQueueName(
                "queue name is empty".to_string(),
            ));
        }
        if trimmed.chars().count() > MAX_QUEUE_NAME_CHARS {
            return Err(SpoolerError::InvalidQueueName(format!(
                "queue name exceeds {MAX_QUEUE_NAME_CHARS} characters"
            )));
        }
        Ok(Self {
            name: trimmed.to_string(),
        })
    }

    /// The validated queue name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Open the queue and start a RAW document. The returned job owns the
    /// printer handle; dropping it ends the document (best effort) and
    /// closes the handle (RAII — no leaks on early return).
    pub fn open(&self) -> Result<SpoolerJob, SpoolerError> {
        SpoolerJob::open(self)
    }

    /// Probe queue reachability WITHOUT starting a document: opens the
    /// printer and closes it immediately. Used by availability checks so
    /// no empty job is ever spooled.
    pub fn probe(&self) -> Result<(), SpoolerError> {
        #[cfg(windows)]
        {
            // SAFETY: `wide` is a valid NUL-terminated buffer owned below;
            // `handle` is an out-param written only on success (checked).
            let wide = wide_nul(self.name());
            let mut handle = PRINTER_HANDLE::default();
            unsafe {
                OpenPrinterW(
                    PCWSTR(wide.as_ptr()),
                    &mut handle,
                    Some(&PRINTER_DEFAULTSW {
                        pDatatype: PWSTR::null(),
                        pDevMode: std::ptr::null_mut(),
                        DesiredAccess: PRINTER_ACCESS_USE,
                    }),
                )
            }
            .map_err(|error| SpoolerError::OsCallFailed {
                call: "OpenPrinterW",
                queue: self.name().to_string(),
                os_code: win_code(&error),
            })?;
            // SAFETY: `handle` is live from the successful open above,
            // closed exactly once here.
            unsafe {
                let _ = ClosePrinter(handle);
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(SpoolerError::UnsupportedPlatform)
        }
    }
}

/// An open spooler job: printer handle + started RAW document.
/// Drop closes the document and the handle (RAII).
#[derive(Debug)]
pub struct SpoolerJob {
    #[cfg(windows)]
    handle: PRINTER_HANDLE,
    #[cfg(windows)]
    queue: String,
    #[cfg(windows)]
    document_open: bool,
}

#[cfg(windows)]
impl SpoolerJob {
    fn open(queue: &SpoolerQueue) -> Result<Self, SpoolerError> {
        use windows::core::Error as WinError;

        // Wide, NUL-terminated buffers owned by this scope; the raw
        // PWSTR/PCWSTR values never outlive them.
        let queue_wide = wide_nul(queue.name());
        let doc_wide = wide_nul(DOC_NAME);
        let raw_wide = wide_nul(RAW_DATATYPE);

        // SAFETY: `queue_wide` is a valid NUL-terminated UTF-16 buffer
        // owned below; `handle` is an uninitialized out-param written only
        // on success (checked via the returned Result).
        let mut handle = PRINTER_HANDLE::default();
        unsafe {
            OpenPrinterW(
                PCWSTR(queue_wide.as_ptr()),
                &mut handle,
                Some(&PRINTER_DEFAULTSW {
                    pDatatype: PWSTR::null(),
                    pDevMode: std::ptr::null_mut(),
                    DesiredAccess: PRINTER_ACCESS_USE,
                }),
            )
        }
        .map_err(|error| SpoolerError::OsCallFailed {
            call: "OpenPrinterW",
            queue: queue.name().to_string(),
            os_code: win_code(&error),
        })?;

        let mut job = Self {
            handle,
            queue: queue.name().to_string(),
            document_open: false,
        };

        let doc_info = DOC_INFO_1W {
            // SAFETY: `doc_wide`/`raw_wide` are valid NUL-terminated
            // buffers owned by this scope; the callee only reads them
            // during the call. Empty output file = default port routing.
            pDocName: PWSTR(doc_wide.as_ptr() as *mut u16),
            pOutputFile: PWSTR::null(),
            pDatatype: PWSTR(raw_wide.as_ptr() as *mut u16),
        };
        // SAFETY: `handle` is a live printer handle from the successful
        // OpenPrinterW above; `doc_info` borrows scope-owned buffers for
        // the duration of this call only. A zero job id means failure.
        let job_id = unsafe { StartDocPrinterW(handle, 1, &doc_info) };
        if job_id == 0 {
            // from_thread(): captures the GetLastError code at the failure site.
            let code = WinError::from_thread().code().0 as u32;
            return Err(SpoolerError::OsCallFailed {
                call: "StartDocPrinterW",
                queue: queue.name().to_string(),
                os_code: code,
            });
        }
        job.document_open = true;
        Ok(job)
    }

    /// Write the full byte slice as RAW, then end the document so the
    /// failure (if any) surfaces here instead of in Drop.
    pub fn write_raw(&mut self, bytes: &[u8]) -> Result<(), SpoolerError> {
        use windows::core::{Error as WinError, BOOL};

        let mut written_total: usize = 0;
        while written_total < bytes.len() {
            let mut written: u32 = 0;
            // SAFETY: `handle` is live (Drop closes it exactly once);
            // the buffer slice is valid for `remaining` bytes for the
            // duration of the call; `written` is a valid out-param.
            let remaining = &bytes[written_total..];
            let ok: BOOL = unsafe {
                WritePrinter(
                    self.handle,
                    remaining.as_ptr() as *const core::ffi::c_void,
                    remaining.len() as u32,
                    &mut written,
                )
            };
            if !ok.as_bool() {
                let code = WinError::from_thread().code().0 as u32;
                return Err(SpoolerError::OsCallFailed {
                    call: "WritePrinter",
                    queue: self.queue.clone(),
                    os_code: code,
                });
            }
            if written == 0 {
                // Spooler accepted nothing and reported success: avoid an
                // infinite loop; surface as a write failure with code 0.
                return Err(SpoolerError::OsCallFailed {
                    call: "WritePrinter",
                    queue: self.queue.clone(),
                    os_code: 0,
                });
            }
            written_total += written as usize;
        }
        // SAFETY: `handle` is live with an open document started above.
        let ended: BOOL = unsafe { EndDocPrinter(self.handle) };
        self.document_open = false;
        if !ended.as_bool() {
            let code = WinError::from_thread().code().0 as u32;
            return Err(SpoolerError::OsCallFailed {
                call: "EndDocPrinter",
                queue: self.queue.clone(),
                os_code: code,
            });
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for SpoolerJob {
    fn drop(&mut self) {
        // Best effort only: errors cannot propagate from Drop. The
        // normal path already ended the document in `write_raw`.
        // SAFETY: `handle` is live exactly once — `write_raw` never
        // closes it, and Drop runs once per owned job.
        unsafe {
            if self.document_open {
                let _ = EndDocPrinter(self.handle);
                self.document_open = false;
            }
            let _ = ClosePrinter(self.handle);
        }
    }
}

#[cfg(not(windows))]
impl SpoolerJob {
    fn open(_queue: &SpoolerQueue) -> Result<Self, SpoolerError> {
        Err(SpoolerError::UnsupportedPlatform)
    }

    /// Unreachable on this platform (`open` always fails first).
    pub fn write_raw(&mut self, _bytes: &[u8]) -> Result<(), SpoolerError> {
        Err(SpoolerError::UnsupportedPlatform)
    }
}

/// One-shot helper: open the queue, write all bytes RAW, end + close.
pub fn submit_raw(queue: &SpoolerQueue, bytes: &[u8]) -> Result<(), SpoolerError> {
    let mut job = queue.open()?;
    job.write_raw(bytes)
}

/// Encode a string as NUL-terminated UTF-16 for Win32 wide APIs.
#[cfg(windows)]
fn wide_nul(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Extract the raw code from a windows-core error.
#[cfg(windows)]
fn win_code(error: &windows::core::Error) -> u32 {
    error.code().0 as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_queue_names() {
        for name in ["", "   ", "\t\n "] {
            let error = SpoolerQueue::new(name).expect_err("must reject");
            assert_eq!(
                error,
                SpoolerError::InvalidQueueName("queue name is empty".to_string())
            );
        }
    }

    #[test]
    fn rejects_overlong_queue_names() {
        let long: String = "ن".repeat(MAX_QUEUE_NAME_CHARS + 1);
        let error = SpoolerQueue::new(&long).expect_err("must reject");
        assert!(matches!(error, SpoolerError::InvalidQueueName(_)));
        // Boundary: exactly 220 chars is accepted (chars, not bytes).
        let boundary: String = "ن".repeat(MAX_QUEUE_NAME_CHARS);
        assert!(SpoolerQueue::new(&boundary).is_ok());
    }

    #[test]
    fn accepts_exact_queue_names_verbatim() {
        let queue = SpoolerQueue::new("  XPrinter XP-410B  ").expect("valid");
        assert_eq!(queue.name(), "XPrinter XP-410B");
        // Unicode names pass through unaltered (wide-string path).
        let arabic = SpoolerQueue::new("طابعة المطبخ").expect("valid");
        assert_eq!(arabic.name(), "طابعة المطبخ");
    }

    #[test]
    fn unknown_queue_fails_cleanly_without_panic() {
        let queue = SpoolerQueue::new("NO_SUCH_QUEUE_BARAKA_9917").expect("valid name");
        let error = queue.open().expect_err("unknown queue must fail");
        // No printer in the test environment: any failure is fine as long
        // as it is a clean diagnostic, never a panic or crash.
        #[cfg(windows)]
        assert!(
            matches!(
                &error,
                SpoolerError::OsCallFailed { call, .. } if *call == "OpenPrinterW"
            ),
            "must name OpenPrinterW, got: {error}"
        );
        #[cfg(not(windows))]
        assert_eq!(error, SpoolerError::UnsupportedPlatform);
    }

    #[test]
    fn error_display_names_call_queue_and_code() {
        let error = SpoolerError::OsCallFailed {
            call: "OpenPrinterW",
            queue: "Q".to_string(),
            os_code: 1801,
        };
        let text = error.to_string();
        assert!(text.contains("OpenPrinterW"));
        assert!(text.contains("'Q'"));
        assert!(text.contains("1801"));
    }
}
