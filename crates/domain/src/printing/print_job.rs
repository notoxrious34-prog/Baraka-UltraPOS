//! Print job outbox entity and state codes.

use serde::{Deserialize, Serialize};

use crate::errors::DomainError;

/// Outbox job lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrintJobStatus {
    /// Awaiting a worker claim.
    Pending,
    /// Claimed by a worker run.
    Processing,
    /// Physically printed.
    Done,
    /// Exhausted retries or unrenderable.
    Failed,
}

impl PrintJobStatus {
    /// Canonical storage code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Processing => "PROCESSING",
            Self::Done => "DONE",
            Self::Failed => "FAILED",
        }
    }

    /// Parse a stored status code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "PENDING" => Ok(Self::Pending),
            "PROCESSING" => Ok(Self::Processing),
            "DONE" => Ok(Self::Done),
            "FAILED" => Ok(Self::Failed),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown print status: {other}"
            ))),
        }
    }
}

/// Outbox job kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrintJobType {
    /// Original receipt.
    Receipt,
    /// Reprint, watermarked as a copy.
    Duplicata,
}

impl PrintJobType {
    /// Canonical storage code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Receipt => "RECEIPT",
            Self::Duplicata => "DUPLICATA",
        }
    }

    /// Parse a stored job-type code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "RECEIPT" => Ok(Self::Receipt),
            "DUPLICATA" => Ok(Self::Duplicata),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown print job type: {other}"
            ))),
        }
    }
}

/// Print outbox job row view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintJob {
    /// Stored row id (TEXT UUID).
    pub id: String,
    /// Owning document id, when the job was enqueued for a sale document.
    /// `None` for non-sale jobs (debt receipts, Z-reports, test prints).
    pub document_id: Option<String>,
    /// Job kind.
    pub job_type: PrintJobType,
    /// Lifecycle state.
    pub status: PrintJobStatus,
    /// Physical send attempts so far.
    pub attempt_count: i64,
}
