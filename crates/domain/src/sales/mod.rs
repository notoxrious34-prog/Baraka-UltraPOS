//! Sales domain: documents, payments, sequences, pricing kernel.

pub mod colisage;
pub mod credit_equation;
pub mod customer;
pub mod document;
pub mod payment;
pub mod pricing;
pub mod sequence;

pub use colisage::{breakdown as colisage_breakdown, ColisageRung, ColisageShare};
pub use credit_equation::CreditSnapshot;

pub use customer::{
    assess_credit_risk, validate_customer_name, validate_customer_phone, CreditPolicy,
    CreditProfile, CreditRisk, Customer, CustomerId, CustomerLedgerEntryType, CustomerTier,
    CREDIT_NEAR_LIMIT_PERCENT,
};
pub use document::{
    Document, DocumentId, DocumentItem, DocumentItemView, DocumentLanguage, DocumentStatus,
    DocumentType,
};
pub use payment::{PaymentLine, PaymentMethod, PaymentStatus};
pub use pricing::{
    allocate_global_discount, calculate_margin_basis_points, compute_line_tva, compute_timbre,
    price_document, price_line_gross, settle_payments, DiscountShare, PaymentShare, PricedDocument,
    PricedLine, PricedLineInput, TenderResult,
};
pub use sequence::{format_number, next_number};
