//! Merchandise return processing (TASK-BRK-045): TICKET/FACTURE/BL source
//! lines flow back into stock (`RETURN_IN`), settle via drawer refund
//! (`REFUND_OUT`), customer-ledger CREDIT, or no financial leg, and mint an
//! immutable AVOIR linked to its source. All-or-nothing: validation runs
//! here, the single atomic bundle commits in `ReturnRepository`.

use baraka_domain::errors::DomainError;
use baraka_domain::primitives::{Money, QtyMicro};
use baraka_domain::role::Permission;
use baraka_domain::sales::{CustomerId, DocumentId, DocumentStatus, DocumentType};
use baraka_domain::user::UserId;
use uuid::Uuid;

use crate::errors::ApplicationError;
use crate::ports::{
    CashSessionRepository, CustomerRepository, DocumentRepository, PersistReturnLine,
    PersistReturnOrder, ReturnCashLeg, ReturnCreditLeg, ReturnRepository, SessionContext,
};

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// Explicit return failure taxonomy.
///
/// Every variant maps 1:1 onto the stable [`ApplicationError`] wire codes
/// asserted across the use-case and persistence suites; the `From` impl is
/// the single mapping point, so Rust callers get structured data while the
/// wire keeps its stable codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnError {
    /// Operator lacks `Permission::ProcessReturn`.
    Unauthorized,
    /// No source document under the requested id.
    SourceDocumentNotFound(DocumentId),
    /// Source is not a FINALIZED Ticket/Facture/Bl.
    InvalidSourceDocumentState {
        /// Observed lifecycle state.
        status: DocumentStatus,
        /// Observed document kind.
        doc_type: DocumentType,
    },
    /// Requested line id belongs to no source line.
    ItemNotFound(Uuid),
    /// Requested quantity exceeds the remaining returnable ceiling.
    QuantityExceeded {
        /// Requested source line id.
        item_id: Uuid,
        /// Requested magnitude in selling-unit micros.
        requested_micro: i64,
        /// Still-returnable magnitude in selling-unit micros.
        allowable_micro: i64,
    },
    /// No OPEN cash session backs a drawer refund.
    NoActiveCashSession,
    /// Ledger credit requested for an anonymous (party-less) sale.
    MissingCustomerForCreditRefund,
    /// Return carries zero lines.
    EmptyReturn,
    /// Storage-layer failure passthrough.
    Infrastructure(String),
}

impl From<ReturnError> for ApplicationError {
    fn from(error: ReturnError) -> Self {
        match error {
            ReturnError::Unauthorized => ApplicationError::Unauthorized,
            ReturnError::SourceDocumentNotFound(_) => {
                ApplicationError::Validation(DomainError::SourceDocumentNotFound)
            }
            ReturnError::InvalidSourceDocumentState { status, .. }
                if status != DocumentStatus::Finalized =>
            {
                ApplicationError::Validation(DomainError::InvalidConversionSourceStatus)
            }
            ReturnError::InvalidSourceDocumentState { .. } => {
                ApplicationError::Validation(DomainError::InvalidConversionSourceType)
            }
            ReturnError::ItemNotFound(_) => {
                ApplicationError::Validation(DomainError::SourceDocumentNotFound)
            }
            ReturnError::QuantityExceeded { .. } => coded("RETURN_QUANTITY_EXCEEDED"),
            ReturnError::NoActiveCashSession => {
                ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
            }
            ReturnError::MissingCustomerForCreditRefund => coded("CREDIT_CUSTOMER_REQUIRED"),
            ReturnError::EmptyReturn => ApplicationError::Validation(DomainError::EmptyCart),
            ReturnError::Infrastructure(message) => ApplicationError::Port(message),
        }
    }
}

/// One returned source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnItemInput {
    /// Source line id.
    pub source_item_id: Uuid,
    /// Returned magnitude in selling-unit micros (must be positive).
    pub quantity_micro: QtyMicro,
    /// Operator reason, if any.
    pub reason: Option<String>,
}

/// Financial settlement of a return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefundDestination {
    /// Drawer payout from an OPEN session (verified by id below).
    Cash { cash_session_id: String },
    /// Debt reduction for a named customer (must match the source party).
    CustomerCredit { customer_id: CustomerId },
    /// No financial leg (logistics-only return / exchange draft).
    NoRefund,
}

/// Validated return request (terminal-resolved `terminal_id` supplied by
/// the caller at the IPC edge, ADR-025 pattern — never client input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessReturnCommand {
    /// Acting operator.
    pub operator_id: UserId,
    /// Returned-against source document id.
    pub source_document_id: DocumentId,
    /// Returned lines (non-empty).
    pub items: Vec<ReturnItemInput>,
    /// Financial settlement.
    pub refund: RefundDestination,
    /// Client idempotency key (UUID string).
    pub idempotency_key: String,
    /// Terminal identifier resolved from TerminalContext.
    pub terminal_id: String,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Return outcome with fixed-point totals (no floats anywhere).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessReturnResult {
    /// New AVOIR document id.
    pub return_document_id: DocumentId,
    /// New AVOIR gapless human number.
    pub return_document_number: String,
    /// Total refunded HT.
    pub total_refund_ht: Money,
    /// Total refunded TVA.
    pub total_refund_tva: Money,
    /// Total refunded TTC (== cash/ledger leg magnitude).
    pub total_refund_ttc: Money,
    /// Returned line count.
    pub items_returned_count: usize,
}

/// Half-up `a * b / denominator` over `i128` (non-negative operands).
fn mul_div_half_up(a: i64, b: i64, denominator: i64) -> Result<i64, ApplicationError> {
    let product = (i128::from(a))
        .checked_mul(i128::from(b))
        .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
    let rounded = product
        .checked_add(i128::from(denominator) / 2)
        .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?
        / i128::from(denominator);
    i64::try_from(rounded).map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))
}

/// Merchandise return orchestrator (pre-validates, then issues one atomic call).
pub struct ProcessReturnUseCase<'a, D, R, U, C>
where
    D: ?Sized,
    R: ?Sized,
    U: ?Sized,
    C: ?Sized,
{
    documents: &'a D,
    returns: &'a R,
    customers: &'a U,
    cash: &'a C,
}

/// Spec-name alias for the merchandise return orchestrator: the return
/// engine's canonical entry point is [`ProcessReturnUseCase`]; both names
/// address the identical type and behavior.
pub type ProcessSaleReturnUseCase<'a, D, R, U, C> = ProcessReturnUseCase<'a, D, R, U, C>;

impl<'a, D, R, U, C> ProcessReturnUseCase<'a, D, R, U, C>
where
    D: DocumentRepository + ?Sized,
    R: ReturnRepository + ?Sized,
    U: CustomerRepository + ?Sized,
    C: CashSessionRepository + ?Sized,
{
    /// Wire the use case to its ports.
    pub fn new(documents: &'a D, returns: &'a R, customers: &'a U, cash: &'a C) -> Self {
        Self {
            documents,
            returns,
            customers,
            cash,
        }
    }

    /// Validate the return and commit it atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        command: ProcessReturnCommand,
    ) -> Result<ProcessReturnResult, ApplicationError> {
        // Step 1: authorization (Gerant/Admin hold ProcessReturn).
        if !session.role.has_permission(Permission::ProcessReturn) {
            return Err(ReturnError::Unauthorized.into());
        }
        if command.idempotency_key.trim().is_empty() {
            return Err(coded("INVALID_IDEMPOTENCY_KEY"));
        }
        if command.terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        if command.items.is_empty() {
            return Err(ReturnError::EmptyReturn.into());
        }
        // Step 2: source identity, status, and kind.
        let source = self
            .documents
            .get_by_id(command.source_document_id)?
            .ok_or(ReturnError::SourceDocumentNotFound(
                command.source_document_id,
            ))?;
        if source.status != DocumentStatus::Finalized
            || !matches!(
                source.doc_type,
                DocumentType::Ticket | DocumentType::Facture | DocumentType::Bl
            )
        {
            return Err(ReturnError::InvalidSourceDocumentState {
                status: source.status,
                doc_type: source.doc_type,
            }
            .into());
        }
        // Step 2b: refund-destination preconditions.
        let cash_session_id = match &command.refund {
            RefundDestination::Cash { cash_session_id } => {
                let active = self
                    .cash
                    .get_active_session(command.terminal_id.trim())?
                    .ok_or(ReturnError::NoActiveCashSession)?;
                if active.status != "OPEN" || active.id != *cash_session_id {
                    return Err(ReturnError::NoActiveCashSession.into());
                }
                Some(cash_session_id.clone())
            }
            RefundDestination::CustomerCredit { customer_id } => {
                let party = source
                    .party_id
                    .as_deref()
                    .ok_or(ReturnError::MissingCustomerForCreditRefund)?;
                if party != customer_id.to_string() {
                    return Err(ApplicationError::Validation(DomainError::CreditDisabled));
                }
                self.customers
                    .get_customer(*customer_id)?
                    .ok_or(ApplicationError::CustomerNotFound)?;
                None
            }
            RefundDestination::NoRefund => None,
        };
        // Step 3: per-line ceiling validation against cumulative returns.
        let source_items = self.documents.get_items(command.source_document_id)?;
        let cumulative = self
            .returns
            .cumulative_returned_quantities(command.source_document_id)?;
        let mut lines: Vec<PersistReturnLine> = Vec::with_capacity(command.items.len());
        let mut seen: Vec<Uuid> = Vec::with_capacity(command.items.len());
        let mut total_ht: i128 = 0;
        let mut total_tva: i128 = 0;
        for (index, item) in command.items.iter().enumerate() {
            if item.quantity_micro.micros() <= 0 {
                return Err(coded("INVALID_PAYMENT_AMOUNT"));
            }
            if seen.contains(&item.source_item_id) {
                return Err(coded("DUPLICATE_RETURN_LINE"));
            }
            seen.push(item.source_item_id);
            let original = source_items
                .iter()
                .find(|line| line.id == item.source_item_id)
                .ok_or(ReturnError::ItemNotFound(item.source_item_id))?;
            let already = cumulative
                .get(&item.source_item_id.to_string())
                .copied()
                .unwrap_or(0);
            let remaining = original
                .quantity_micro
                .checked_sub(already)
                .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
            if item.quantity_micro.micros() > remaining {
                return Err(ReturnError::QuantityExceeded {
                    item_id: item.source_item_id,
                    requested_micro: item.quantity_micro.micros(),
                    allowable_micro: remaining,
                }
                .into());
            }
            // Step 4: fixed-point line math (half-up, i128-checked).
            let ret_qty = item.quantity_micro.micros();
            let line_ht = Money::from_centimes(original.unit_price_centimes)
                .checked_mul_qty(QtyMicro::from_micros(ret_qty))
                .map_err(ApplicationError::Validation)?
                .centimes();
            let discount =
                mul_div_half_up(original.discount_centimes, ret_qty, original.quantity_micro)?;
            let net_ht = line_ht
                .checked_sub(discount)
                .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
            let line_tva = if original.line_ht_centimes == 0 {
                0
            } else {
                mul_div_half_up(
                    net_ht,
                    original.line_tva_centimes,
                    original.line_ht_centimes,
                )?
            };
            let line_ttc = net_ht
                .checked_add(line_tva)
                .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
            // Base-unit restock magnitude, proportional to the source line.
            let quantity_base_micro = if original.quantity_micro == 0 {
                0
            } else {
                mul_div_half_up(
                    ret_qty,
                    original.quantity_base_micro,
                    original.quantity_micro,
                )?
            };
            total_ht = total_ht
                .checked_add(i128::from(net_ht))
                .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
            total_tva = total_tva
                .checked_add(i128::from(line_tva))
                .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
            lines.push(PersistReturnLine {
                source_item_id: item.source_item_id.to_string(),
                product_id: original.product_id,
                unit_id: original.unit_id,
                quantity_micro: ret_qty,
                quantity_base_micro,
                unit_price_centimes: original.unit_price_centimes,
                discount_centimes: discount,
                line_ht_centimes: net_ht,
                line_tva_centimes: line_tva,
                line_ttc_centimes: line_ttc,
                cost_price_centimes: original.cost_price_centimes,
                sort_order: index as i64,
                reason: item.reason.clone(),
            });
        }
        let total_ht = i64::try_from(total_ht)
            .map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))?;
        let total_tva = i64::try_from(total_tva)
            .map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))?;
        let total_ttc = total_ht
            .checked_add(total_tva)
            .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
        // Step 6: settlement legs (timbre is never refunded: always 0).
        let cash_refund = cash_session_id.map(|session_id| ReturnCashLeg {
            cash_session_id: session_id,
            amount_centimes: total_ttc,
        });
        let credit_refund = match &command.refund {
            RefundDestination::CustomerCredit { customer_id } => Some(ReturnCreditLeg {
                customer_id: *customer_id,
                amount_centimes: total_ttc,
            }),
            _ => None,
        };
        // Step 7+8: single atomic bundle (header, items, RETURN_IN legs,
        // financial legs, audit, command log).
        let receipt = self.returns.persist_return(&PersistReturnOrder {
            idempotency_key: command.idempotency_key.trim().to_string(),
            request_canonical: canonical_return_request(&command, &source.number),
            source_document_id: command.source_document_id,
            terminal_id: command.terminal_id.trim().to_string(),
            warehouse_id: source.warehouse_id,
            document_language: source.language,
            subtotal_ht_centimes: total_ht,
            vat_total_centimes: total_tva,
            timbre_centimes: 0,
            ttc_centimes: total_ttc,
            cash_refund,
            credit_refund,
            lines,
            actor: command.operator_id,
            notes: command.notes.map(|note| note.trim().to_string()),
        })?;
        Ok(ProcessReturnResult {
            return_document_id: DocumentId::from(
                Uuid::parse_str(&receipt.return_document_id)
                    .map_err(|_| ReturnError::Infrastructure("return store failure".to_string()))?,
            ),
            return_document_number: receipt.return_document_number,
            total_refund_ht: Money::from_centimes(total_ht),
            total_refund_tva: Money::from_centimes(total_tva),
            total_refund_ttc: Money::from_centimes(total_ttc),
            items_returned_count: command.items.len(),
        })
    }
}

/// Deterministic canonical request string for return idempotency.
fn canonical_return_request(command: &ProcessReturnCommand, source_number: &str) -> String {
    let mut parts = vec![
        "v1".to_string(),
        "process-return".to_string(),
        command.idempotency_key.trim().to_string(),
        command.source_document_id.to_string(),
        source_number.to_string(),
        command.terminal_id.trim().to_string(),
    ];
    for item in &command.items {
        parts.push(format!(
            "{}|{}",
            item.source_item_id,
            item.quantity_micro.micros()
        ));
    }
    parts.join(";")
}
