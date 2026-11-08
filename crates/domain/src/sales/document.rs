//! Sale documents and their immutable line snapshots.

use std::fmt::{Display, Formatter, Result as FmtResult};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::catalog::{ProductId, UnitId};
use crate::errors::DomainError;
use crate::inventory::WarehouseId;
use crate::user::UserId;

/// Sale document kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentType {
    /// Retail ticket.
    Ticket,
    /// Invoice.
    Facture,
    /// Delivery note.
    Bl,
    /// Quote.
    Devis,
    /// Credit note / return.
    Avoir,
    /// Purchase order.
    Bc,
    /// Reception note.
    Br,
    /// Supplier invoice.
    Ff,
}

impl DocumentType {
    /// Canonical storage code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Ticket => "TICKET",
            Self::Facture => "FACTURE",
            Self::Bl => "BL",
            Self::Devis => "DEVIS",
            Self::Avoir => "AVOIR",
            Self::Bc => "BC",
            Self::Br => "BR",
            Self::Ff => "FF",
        }
    }

    /// Gapless number prefix.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Ticket => "TICK",
            Self::Facture => "FAC",
            Self::Bl => "BL",
            Self::Devis => "DEV",
            Self::Avoir => "AVO",
            Self::Bc => "BC",
            Self::Br => "BR",
            Self::Ff => "FF",
        }
    }

    /// Parse a wire/storage code.
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "TICKET" => Ok(Self::Ticket),
            "FACTURE" => Ok(Self::Facture),
            "BL" => Ok(Self::Bl),
            "DEVIS" => Ok(Self::Devis),
            "AVOIR" => Ok(Self::Avoir),
            "BC" => Ok(Self::Bc),
            "BR" => Ok(Self::Br),
            "FF" => Ok(Self::Ff),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown document type: {other}"
            ))),
        }
    }
}

/// Document lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentStatus {
    /// Editable park state (mutable).
    Draft,
    /// Sealed state (database-immutable).
    Finalized,
    /// Voided state.
    Cancelled,
    /// Purchase-order rollup: some ordered quantity received (ADR-034).
    PartiallyReceived,
    /// Purchase-order rollup: every ordered line fully received (ADR-034).
    FullyReceived,
    /// Single-shot conversion seal (ADR-040): a DEVIS or BL consumed by
    /// exactly one conversion. Terminal: no outgoing transition exists.
    Converted,
}

impl DocumentStatus {
    /// Canonical storage code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Finalized => "FINALIZED",
            Self::Cancelled => "CANCELLED",
            Self::PartiallyReceived => "PARTIALLY_RECEIVED",
            Self::FullyReceived => "FULLY_RECEIVED",
            Self::Converted => "CONVERTED",
        }
    }

    /// Parse a stored status code (includes post-migration values).
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        match code {
            "DRAFT" => Ok(Self::Draft),
            "FINALIZED" => Ok(Self::Finalized),
            "CANCELLED" => Ok(Self::Cancelled),
            "PARTIALLY_RECEIVED" => Ok(Self::PartiallyReceived),
            "FULLY_RECEIVED" => Ok(Self::FullyReceived),
            "CONVERTED" => Ok(Self::Converted),
            other => Err(DomainError::InvalidCanonicalString(format!(
                "unknown document status: {other}"
            ))),
        }
    }

    /// Validate a status transition for single-shot document conversion.
    ///
    /// ADR-040: `Finalized -> Converted` is the only legal entry into
    /// `Converted`, and only for DEVIS or BL source documents. `Converted`
    /// is terminal: nothing may leave it, and no other status may target
    /// it. Returns the specific domain error for the rejected transition.
    pub fn validate_conversion_transition(
        from: Self,
        doc_type: DocumentType,
    ) -> Result<(), DomainError> {
        match (from, doc_type) {
            (Self::Finalized, DocumentType::Devis | DocumentType::Bl) => Ok(()),
            (Self::Converted, _) => Err(DomainError::DocumentAlreadyConverted),
            (Self::Finalized, _) => Err(DomainError::InvalidConversionSourceType),
            (Self::Draft | Self::Cancelled, _) => Err(DomainError::InvalidConversionSourceStatus),
            (Self::PartiallyReceived | Self::FullyReceived, _) => {
                Err(DomainError::InvalidConversionSourceType)
            }
        }
    }
}

/// Stable document identifier (TEXT UUID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    /// Fresh random identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for DocumentId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for DocumentId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Language captured at document creation (Section 17: stored once, never
/// reinterpreted later from the UI locale).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentLanguage {
    /// Arabic receipt layout.
    Ar,
    /// French receipt layout.
    Fr,
}

impl Display for DocumentLanguage {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Ar => write!(f, "AR"),
            Self::Fr => write!(f, "FR"),
        }
    }
}

impl std::str::FromStr for DocumentLanguage {
    type Err = DomainError;

    fn from_str(code: &str) -> Result<Self, Self::Err> {
        match code {
            "AR" => Ok(Self::Ar),
            "FR" => Ok(Self::Fr),
            _ => Err(DomainError::InvalidDocumentLanguage),
        }
    }
}

/// Persisted sale document with frozen financial snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// Stable identifier.
    pub id: DocumentId,
    /// Sale-time language selection (immutable after creation).
    pub language: DocumentLanguage,
    /// Gapless human number (`PREFIX-YEAR-000001`) or draft placeholder.
    pub number: String,
    /// Fiscal year of the number.
    pub fiscal_year: i32,
    /// Document kind.
    pub doc_type: DocumentType,
    /// Lifecycle state.
    pub status: DocumentStatus,
    /// Selling warehouse.
    pub warehouse_id: WarehouseId,
    /// Customer id for credit sales, if any.
    pub party_id: Option<String>,
    /// HT subtotal after all discounts.
    pub subtotal_ht_centimes: i64,
    /// Total TVA.
    pub vat_total_centimes: i64,
    /// Total global discount allocated.
    pub global_discount_centimes: i64,
    /// Timbre fiscal.
    pub timbre_centimes: i64,
    /// Grand total.
    pub ttc_centimes: i64,
    /// Total tendered.
    pub paid_amount_centimes: i64,
    /// Change given back.
    pub change_amount_centimes: i64,
    /// Primary payment method code, or `MIXED` / `UNPAID`.
    pub payment_method: String,
    /// Print-template version stamped at creation (DOC-04): a reprint pins
    /// this version so old documents keep old geometry forever. Legacy rows
    /// (pre-migration-0019) read back as 1.
    pub template_version: u32,
    /// Operator note, if any.
    pub notes: Option<String>,
    /// Capture instant.
    pub created_at: DateTime<Utc>,
    /// Acting operator.
    pub created_by: UserId,
}

/// One frozen document line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentItem {
    /// Stable identifier.
    pub id: Uuid,
    /// Owning document.
    pub document_id: DocumentId,
    /// Product sold (leaf or kit header as displayed).
    pub product_id: ProductId,
    /// Selling unit handled.
    pub unit_id: UnitId,
    /// Quantity in selling-unit micros.
    pub quantity_micro: i64,
    /// Quantity converted to base micros.
    pub quantity_base_micro: i64,
    /// Agreed unit price in centimes.
    pub unit_price_centimes: i64,
    /// Line discount in centimes.
    pub discount_centimes: i64,
    /// Per-unit line discount in centimes (BRK-096: exact, no division).
    /// `None` = legacy pre-BRK-096 line: the lump `discount_centimes`
    /// applies to the line gross and no discounted PU is recoverable.
    pub discount_per_unit_centimes: Option<i64>,
    /// TVA rate in micros (basis points x 100).
    pub tva_rate_micro: i64,
    /// Line HT after all discounts.
    pub line_ht_centimes: i64,
    /// Line TVA.
    pub line_tva_centimes: i64,
    /// Line TTC.
    pub line_ttc_centimes: i64,
    /// Purchase price snapshot (margin base).
    pub cost_price_centimes: i64,
    /// Display order.
    pub sort_order: i64,
}

/// One document line enriched with live product designation for display
/// (TASK-BRK-088). The frozen [`DocumentItem`] stays ID-immutable; these
/// extra fields are display-only and resolved with LEFT JOINs at read time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentItemView {
    /// Frozen line snapshot (ids, quantities, money).
    pub item: DocumentItem,
    /// Commercial designation (FR), absent when the product row is gone.
    pub product_name_fr: Option<String>,
    /// Commercial designation (AR), absent when the product row is gone.
    pub product_name_ar: Option<String>,
    /// Stock-keeping reference, absent when the product row is gone.
    pub sku: Option<String>,
    /// Selling-unit label (FR), absent when the unit row is gone.
    pub unit_name_fr: Option<String>,
    /// Selling-unit label (AR), absent when the unit row is gone.
    pub unit_name_ar: Option<String>,
    /// True when the product row is missing (orphaned line degrades).
    pub product_missing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converted_status_round_trips_and_gates_transitions() {
        assert_eq!(DocumentStatus::Converted.code(), "CONVERTED");
        assert_eq!(
            DocumentStatus::from_code("CONVERTED"),
            Ok(DocumentStatus::Converted)
        );
        // Only Finalized DEVIS/BL may enter Converted.
        assert!(DocumentStatus::validate_conversion_transition(
            DocumentStatus::Finalized,
            DocumentType::Devis
        )
        .is_ok());
        assert!(DocumentStatus::validate_conversion_transition(
            DocumentStatus::Finalized,
            DocumentType::Bl
        )
        .is_ok());
        // Terminal: Converted converts to nothing.
        for doc_type in [
            DocumentType::Ticket,
            DocumentType::Facture,
            DocumentType::Bl,
            DocumentType::Devis,
            DocumentType::Avoir,
            DocumentType::Bc,
            DocumentType::Br,
            DocumentType::Ff,
        ] {
            assert_eq!(
                DocumentStatus::validate_conversion_transition(DocumentStatus::Converted, doc_type),
                Err(DomainError::DocumentAlreadyConverted)
            );
        }
        // Finalized non-DEVIS/BL is a type error, not a status error.
        assert_eq!(
            DocumentStatus::validate_conversion_transition(
                DocumentStatus::Finalized,
                DocumentType::Ticket
            ),
            Err(DomainError::InvalidConversionSourceType)
        );
        assert_eq!(
            DocumentStatus::validate_conversion_transition(
                DocumentStatus::Finalized,
                DocumentType::Facture
            ),
            Err(DomainError::InvalidConversionSourceType)
        );
        // Draft/Cancelled are status errors for every type.
        for status in [DocumentStatus::Draft, DocumentStatus::Cancelled] {
            for doc_type in [DocumentType::Devis, DocumentType::Ticket] {
                assert_eq!(
                    DocumentStatus::validate_conversion_transition(status, doc_type),
                    Err(DomainError::InvalidConversionSourceStatus)
                );
            }
        }
        // Purchase rollup statuses are never convertible.
        for status in [
            DocumentStatus::PartiallyReceived,
            DocumentStatus::FullyReceived,
        ] {
            assert_eq!(
                DocumentStatus::validate_conversion_transition(status, DocumentType::Bl),
                Err(DomainError::InvalidConversionSourceType)
            );
        }
    }
}
