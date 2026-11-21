//! Atomic size/color exchange (BRK-125, Échange Express).
//!
//! One idempotent command returns the old sizes and sells the alternates
//! in a SINGLE atomic bundle: the return legs reuse the source historic
//! transacted price (shared `price_return_legs`), the new legs replay the
//! customer's last transacted price (0022; guests fall back to the
//! current catalog price — a guest has no memory), and the money effect
//! is exactly the net difference (0 DZD at equal price). The customer
//! statement is never touched except for an explicit credit payout.

use baraka_domain::catalog::{convert_to_base_micros, ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::primitives::{Money, QtyMicro};
use baraka_domain::role::Permission;
use baraka_domain::sales::{CustomerId, DocumentId, DocumentStatus, DocumentType};
use baraka_domain::user::UserId;

use super::returns::{price_return_legs, RefundDestination, ReturnError, ReturnItemInput};
use super::sales::{verify_manager_override, ManagerOverride};
use crate::errors::ApplicationError;
use crate::ports::{
    CashSessionRepository, CustomerRepository, DocumentRepository, PasswordHasher,
    PersistExchangeOrder, PersistExchangeSaleLine, ProductRepository, ReturnCashLeg,
    ReturnCreditLeg, ReturnRepository, SessionContext, UserRepository,
};

/// One alternate size/color sold in the exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangeItemInput {
    /// Alternate product sold.
    pub product_id: ProductId,
    /// Selling unit handled.
    pub unit_id: UnitId,
    /// Sold magnitude in selling-unit micros (must be > 0).
    pub quantity_micro: i64,
}

/// Exchange command: return legs plus alternate legs, netted atomically.
#[derive(Debug, Clone)]
pub struct ProcessExchangeCommand {
    /// Acting operator.
    pub operator_id: UserId,
    /// Exchanged-against source document id.
    pub source_document_id: DocumentId,
    /// Returned lines (non-empty).
    pub return_items: Vec<ReturnItemInput>,
    /// Alternate lines sold (non-empty).
    pub new_items: Vec<ExchangeItemInput>,
    /// Payout path when the net is negative (owed to the customer).
    pub refund: RefundDestination,
    /// Manager PIN override unlocking below-cost alternate legs for this
    /// exchange only (DSN-115 reuse). `None` locks every red leg.
    pub below_cost_override: Option<ManagerOverride>,
    /// Client idempotency key (UUID string).
    pub idempotency_key: String,
    /// Terminal identifier resolved from TerminalContext.
    pub terminal_id: String,
    /// Operator note, if any.
    pub notes: Option<String>,
}

/// Exchange outcome: both documents plus the net effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessExchangeResult {
    /// New AVOIR document id.
    pub avoir_id: DocumentId,
    /// New AVOIR gapless human number.
    pub avoir_number: String,
    /// New sale TICKET document id.
    pub sale_document_id: DocumentId,
    /// New sale TICKET gapless human number.
    pub sale_document_number: String,
    /// Return total TTC (historic prices).
    pub return_ttc_centimes: i64,
    /// Sale total TTC (last-price memory).
    pub sale_ttc_centimes: i64,
    /// Net effect `sale - return` (0 at equal price).
    pub net_centimes: i64,
}

/// Stable-code constructor for validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

/// Deterministic canonical request string for exchange idempotency.
fn canonical_exchange_request(command: &ProcessExchangeCommand, source_number: &str) -> String {
    let mut parts = vec![
        "v1".to_string(),
        "process-exchange".to_string(),
        command.idempotency_key.trim().to_string(),
        command.source_document_id.to_string(),
        source_number.to_string(),
        command.terminal_id.trim().to_string(),
    ];
    for item in &command.return_items {
        parts.push(format!(
            "R{}|{}",
            item.source_item_id,
            item.quantity_micro.micros()
        ));
    }
    for item in &command.new_items {
        parts.push(format!(
            "N{}|{}|{}",
            item.product_id, item.unit_id, item.quantity_micro
        ));
    }
    parts.join(";")
}

/// Atomic exchange: returns plus alternates in one bundle.
pub struct ProcessExchangeUseCase<'a, D, R, U, C, P, X, H>
where
    D: ?Sized,
    R: ?Sized,
    U: ?Sized,
    C: ?Sized,
    P: ?Sized,
    X: ?Sized,
    H: ?Sized,
{
    documents: &'a D,
    returns: &'a R,
    customers: &'a U,
    cash: &'a C,
    products: &'a P,
    users: &'a X,
    hasher: &'a H,
}

impl<'a, D, R, U, C, P, X, H> ProcessExchangeUseCase<'a, D, R, U, C, P, X, H>
where
    D: DocumentRepository + ?Sized,
    R: ReturnRepository + ?Sized,
    U: CustomerRepository + ?Sized,
    C: CashSessionRepository + ?Sized,
    P: ProductRepository + ?Sized,
    X: UserRepository + ?Sized,
    H: PasswordHasher + ?Sized,
{
    /// Wire the seven ports (documents, returns, customers, cash, catalog,
    /// users, hasher).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        documents: &'a D,
        returns: &'a R,
        customers: &'a U,
        cash: &'a C,
        products: &'a P,
        users: &'a X,
        hasher: &'a H,
    ) -> Self {
        Self {
            documents,
            returns,
            customers,
            cash,
            products,
            users,
            hasher,
        }
    }

    /// Validate, price (historic + last-price memory), net, persist atomically.
    pub fn execute(
        &self,
        session: &SessionContext,
        command: ProcessExchangeCommand,
    ) -> Result<ProcessExchangeResult, ApplicationError> {
        if !session.role.has_permission(Permission::ProcessReturn) {
            return Err(ReturnError::Unauthorized.into());
        }
        if command.idempotency_key.trim().is_empty() {
            return Err(coded("INVALID_IDEMPOTENCY_KEY"));
        }
        if command.terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        if command.return_items.is_empty() || command.new_items.is_empty() {
            return Err(coded("EMPTY_EXCHANGE"));
        }
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
        let source_customer: Option<CustomerId> = source
            .party_id
            .as_deref()
            .map(|party| {
                uuid::Uuid::parse_str(party)
                    .map(CustomerId::from)
                    .map_err(|_| coded("INCOHERENT_EXCHANGE_SOURCE"))
            })
            .transpose()?;
        // Return legs at historic transacted prices (shared kernel).
        let source_items = self.documents.get_items(command.source_document_id)?;
        let cumulative = self
            .returns
            .cumulative_returned_quantities(command.source_document_id)?;
        let (return_lines, return_ht, return_tva) =
            price_return_legs(&source_items, &cumulative, &command.return_items)?;
        let return_ttc = return_ht
            .checked_add(return_tva)
            .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
        // Sale legs at last-price memory (named) or catalog price (guest).
        let mut sale_lines = Vec::with_capacity(command.new_items.len());
        let mut sale_ht: i128 = 0;
        let mut any_below_cost = false;
        for (index, item) in command.new_items.iter().enumerate() {
            if item.quantity_micro <= 0 {
                return Err(coded("INVALID_EXCHANGE_QUANTITY"));
            }
            let (_, units) = self
                .products
                .find_by_id(item.product_id)?
                .ok_or(coded("EXCHANGE_PRODUCT_UNKNOWN"))?;
            let unit = units
                .iter()
                .find(|unit| unit.id == item.unit_id)
                .ok_or(coded("EXCHANGE_UNIT_UNKNOWN"))?;
            let unit_price = match source_customer {
                Some(customer) => {
                    let memory = self.customers.get_last_price(
                        customer,
                        item.product_id.to_string(),
                        item.unit_id.to_string(),
                    )?;
                    let cell = memory.ok_or(coded("EXCHANGE_PRICE_UNKNOWN"))?;
                    cell.price_centimes
                        .checked_sub(cell.discount_per_unit_centimes)
                        .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?
                }
                None => unit.sale_price.centimes(),
            };
            let cost_price = unit.purchase_price.centimes();
            if unit_price < cost_price {
                any_below_cost = true;
            }
            let line_ht = Money::from_centimes(unit_price)
                .checked_mul_qty(QtyMicro::from_micros(item.quantity_micro))
                .map_err(ApplicationError::Validation)?
                .centimes();
            let quantity_base_micro =
                convert_to_base_micros(item.quantity_micro, unit.factor_micro)
                    .map_err(ApplicationError::Validation)?;
            sale_ht = sale_ht
                .checked_add(i128::from(line_ht))
                .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
            sale_lines.push(PersistExchangeSaleLine {
                product_id: item.product_id,
                unit_id: item.unit_id,
                quantity_micro: item.quantity_micro,
                quantity_base_micro,
                unit_price_centimes: unit_price,
                line_ht_centimes: line_ht,
                line_tva_centimes: 0,
                line_ttc_centimes: line_ht,
                cost_price_centimes: unit.purchase_price.centimes(),
                sort_order: index as i64,
            });
        }
        // Margin-guard lock (DSN-115 reuse): any alternate leg priced
        // strictly below cost locks the whole exchange unless a manager
        // PIN unlocks THIS exchange. The lock lives here, never in the UI.
        if any_below_cost {
            let unlocked = match &command.below_cost_override {
                Some(over) => {
                    verify_manager_override(self.users, self.hasher, &over.username, &over.pin)?
                }
                None => false,
            };
            if !unlocked {
                return Err(coded("MARGIN_GUARD_BELOW_COST"));
            }
        }
        let sale_ht = i64::try_from(sale_ht)
            .map_err(|_| ApplicationError::Validation(DomainError::MoneyOverflow))?;
        let net = sale_ht
            .checked_sub(return_ttc)
            .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
        // Settlement: collect the positive net, pay out the negative one.
        let mut cash_in = None;
        let mut cash_out = None;
        let mut credit_leg = None;
        if net > 0 {
            let active = self
                .cash
                .get_active_session(command.terminal_id.trim())?
                .ok_or(ReturnError::NoActiveCashSession)?;
            if active.status != "OPEN" {
                return Err(ReturnError::NoActiveCashSession.into());
            }
            cash_in = Some(ReturnCashLeg {
                cash_session_id: active.id,
                amount_centimes: net,
            });
        } else if net < 0 {
            let owed = net
                .checked_neg()
                .ok_or(ApplicationError::Validation(DomainError::MoneyOverflow))?;
            match &command.refund {
                RefundDestination::Cash { cash_session_id } => {
                    let active = self
                        .cash
                        .get_active_session(command.terminal_id.trim())?
                        .ok_or(ReturnError::NoActiveCashSession)?;
                    if active.status != "OPEN" || active.id != *cash_session_id {
                        return Err(ReturnError::NoActiveCashSession.into());
                    }
                    cash_out = Some(ReturnCashLeg {
                        cash_session_id: cash_session_id.clone(),
                        amount_centimes: owed,
                    });
                }
                RefundDestination::CustomerCredit { customer_id } => {
                    let party =
                        source_customer.ok_or(ReturnError::MissingCustomerForCreditRefund)?;
                    if party != *customer_id {
                        return Err(ApplicationError::Validation(DomainError::CreditDisabled));
                    }
                    self.customers
                        .get_customer(*customer_id)?
                        .ok_or(ApplicationError::CustomerNotFound)?;
                    credit_leg = Some(ReturnCreditLeg {
                        customer_id: *customer_id,
                        amount_centimes: owed,
                    });
                }
                RefundDestination::NoRefund => {
                    return Err(coded("EXCHANGE_REFUND_REQUIRED"));
                }
            }
        }
        let receipt = self.returns.persist_exchange(&PersistExchangeOrder {
            idempotency_key: command.idempotency_key.trim().to_string(),
            request_canonical: canonical_exchange_request(&command, &source.number),
            source_document_id: command.source_document_id,
            terminal_id: command.terminal_id.trim().to_string(),
            warehouse_id: source.warehouse_id,
            document_language: source.language,
            customer_id: source_customer,
            return_lines,
            return_ht_centimes: return_ht,
            return_tva_centimes: return_tva,
            return_ttc_centimes: return_ttc,
            sale_lines,
            sale_ht_centimes: sale_ht,
            sale_tva_centimes: 0,
            sale_ttc_centimes: sale_ht,
            net_centimes: net,
            cash_in,
            cash_out,
            credit_leg,
            actor: command.operator_id,
            notes: command.notes.map(|note| note.trim().to_string()),
        })?;
        Ok(ProcessExchangeResult {
            avoir_id: DocumentId::from(
                uuid::Uuid::parse_str(&receipt.return_document_id).map_err(|_| {
                    ReturnError::Infrastructure("exchange store failure".to_string())
                })?,
            ),
            avoir_number: receipt.return_document_number,
            sale_document_id: DocumentId::from(
                uuid::Uuid::parse_str(&receipt.sale_document_id).map_err(|_| {
                    ReturnError::Infrastructure("exchange store failure".to_string())
                })?,
            ),
            sale_document_number: receipt.sale_document_number,
            return_ttc_centimes: return_ttc,
            sale_ttc_centimes: sale_ht,
            net_centimes: net,
        })
    }
}
