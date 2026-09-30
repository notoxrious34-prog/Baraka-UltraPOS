//! `ProcessReturnUseCase` tests over in-memory port fakes (TASK-BRK-045).
//!
//! No SQLite here: every port is a deterministic fake, so these tests pin
//! the use-case validation, ceiling arithmetic, fixed-point refund math,
//! and settlement dispatch — the SQLite atomic bundle ships separately.

use std::collections::HashMap;
use std::sync::Mutex;

use baraka_application::errors::ApplicationError;
use baraka_application::ports::{
    CashMovementView, CashSessionReport, CashSessionRepository, CashSessionView,
    ConvertBlToFactureCommand, ConvertBlToFactureResult, ConvertDevisToBlCommand,
    ConvertDevisToBlResult, CustomerRepository, CustomerSummary, CustomerUpdate,
    DocumentRepository, DocumentSearchFilter, DocumentSummary, LedgerEntryRow, PaymentRecord,
    PersistReturnOrder, PersistReturnReceipt, ReturnRepository,
};
use baraka_application::use_cases::returns::{
    ProcessReturnCommand, ProcessReturnUseCase, RefundDestination, ReturnItemInput,
};
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::primitives::{Money, QtyMicro};
use baraka_domain::role::Role;
use baraka_domain::sales::{
    CreditPolicy, CreditProfile, Customer, CustomerId, Document, DocumentId, DocumentItem,
    DocumentLanguage, DocumentStatus, DocumentType,
};
use baraka_domain::user::UserId;
use uuid::Uuid;

use baraka_application::ports::SessionContext;
use baraka_application::ports::SessionToken;

const TERMINAL: &str = "TEST-TERMINAL-RET";

fn session_with(role: Role) -> SessionContext {
    SessionContext {
        token: SessionToken::new_random(),
        user_id: UserId::new(),
        role,
        expires_at: chrono::Utc::now() + chrono::TimeDelta::hours(1),
    }
}

fn source_document(party: Option<String>) -> Document {
    Document {
        id: DocumentId::new(),
        language: DocumentLanguage::Fr,
        number: "TICK-2026-000005".to_string(),
        fiscal_year: 2026,
        doc_type: DocumentType::Ticket,
        status: DocumentStatus::Finalized,
        warehouse_id: WarehouseId::new(),
        party_id: party,
        subtotal_ht_centimes: 15_500,
        vat_total_centimes: 0,
        global_discount_centimes: 0,
        timbre_centimes: 0,
        ttc_centimes: 15_500,
        paid_amount_centimes: 15_500,
        change_amount_centimes: 0,
        payment_method: "CASH".to_string(),
        notes: None,
        created_at: chrono::Utc::now(),
        created_by: UserId::new(),
    }
}

fn source_line(document: &Document, qty_units: i64, unit_price: i64) -> DocumentItem {
    DocumentItem {
        id: Uuid::new_v4(),
        document_id: document.id,
        product_id: ProductId::new(),
        unit_id: UnitId::new(),
        quantity_micro: qty_units * 10_000,
        quantity_base_micro: qty_units * 10_000,
        unit_price_centimes: unit_price,
        discount_centimes: 0,
        tva_rate_micro: 0,
        line_ht_centimes: unit_price * qty_units,
        line_tva_centimes: 0,
        line_ttc_centimes: unit_price * qty_units,
        cost_price_centimes: unit_price - 200,
        sort_order: 0,
    }
}

fn named_customer(name: &str) -> Customer {
    Customer {
        id: CustomerId::new(),
        name: name.to_string(),
        phone: None,
        address: None,
        credit_policy: CreditPolicy::Limited,
        credit_limit_centimes: 100_000,
        is_active: true,
    }
}

fn open_session_view(id: &str, status: &str) -> CashSessionView {
    CashSessionView {
        id: id.to_string(),
        warehouse_id: WarehouseId::new().to_string(),
        terminal_id: TERMINAL.to_string(),
        status: status.to_string(),
        opened_by: UserId::new().to_string(),
        opened_by_username: "gerant1".to_string(),
        opened_at: "2026-01-01T00:00:00Z".to_string(),
        opening_float_centimes: 0,
        closed_by: None,
        closed_by_username: None,
        closed_at: None,
        actual_counted_centimes: None,
        system_expected_centimes: None,
        variance_centimes: None,
        closing_notes: None,
        revision: 1,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

struct FakeDocuments {
    document: Mutex<Option<Document>>,
    items: Mutex<Vec<DocumentItem>>,
}

impl FakeDocuments {
    fn with(document: Document, items: Vec<DocumentItem>) -> Self {
        Self {
            document: Mutex::new(Some(document)),
            items: Mutex::new(items),
        }
    }

    fn empty() -> Self {
        Self {
            document: Mutex::new(None),
            items: Mutex::new(Vec::new()),
        }
    }
}

impl DocumentRepository for FakeDocuments {
    fn create_draft(
        &self,
        _document: &Document,
        _items: &[DocumentItem],
        _origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn get_by_id(&self, _id: DocumentId) -> Result<Option<Document>, ApplicationError> {
        Ok(self.document.lock().expect("document").clone())
    }

    fn get_by_number(
        &self,
        _doc_type_code: &str,
        _fiscal_year: i32,
        _number: &str,
    ) -> Result<Option<Document>, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn get_items(&self, _id: DocumentId) -> Result<Vec<DocumentItem>, ApplicationError> {
        Ok(self.items.lock().expect("items").clone())
    }

    fn get_payments(
        &self,
        _id: DocumentId,
    ) -> Result<Vec<baraka_application::ports::StoredPayment>, ApplicationError> {
        Ok(Vec::new())
    }

    fn get_print_status(&self, _id: DocumentId) -> Result<String, ApplicationError> {
        Ok("NONE".to_string())
    }

    fn finalize_document(
        &self,
        _document: &Document,
        _items: &[DocumentItem],
        _payments: &[baraka_application::ports::StoredPayment],
        _origin_device_id: &str,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn convert_devis_to_bl(
        &self,
        _command: &ConvertDevisToBlCommand,
    ) -> Result<ConvertDevisToBlResult, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn convert_bl_to_facture(
        &self,
        _command: &ConvertBlToFactureCommand,
    ) -> Result<ConvertBlToFactureResult, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn search_documents(
        &self,
        _filter: &DocumentSearchFilter,
    ) -> Result<Vec<DocumentSummary>, ApplicationError> {
        panic!("out of scope for return tests");
    }
}

struct FakeReturns {
    cumulative: Mutex<HashMap<String, i64>>,
    orders: Mutex<Vec<PersistReturnOrder>>,
}

impl FakeReturns {
    fn with_cumulative(cumulative: HashMap<String, i64>) -> Self {
        Self {
            cumulative: Mutex::new(cumulative),
            orders: Mutex::new(Vec::new()),
        }
    }

    fn orders(&self) -> Vec<PersistReturnOrder> {
        self.orders.lock().expect("orders").clone()
    }
}

impl ReturnRepository for FakeReturns {
    fn cumulative_returned_quantities(
        &self,
        _source_document_id: DocumentId,
    ) -> Result<HashMap<String, i64>, ApplicationError> {
        Ok(self.cumulative.lock().expect("cumulative").clone())
    }

    fn persist_return(
        &self,
        order: &PersistReturnOrder,
    ) -> Result<PersistReturnReceipt, ApplicationError> {
        self.orders.lock().expect("orders").push(order.clone());
        Ok(PersistReturnReceipt {
            return_document_id: Uuid::new_v4().to_string(),
            return_document_number: "AVO-2026-000001".to_string(),
        })
    }
}

struct FakeCustomers {
    customer: Mutex<Option<Customer>>,
}

impl FakeCustomers {
    fn with(customer: Option<Customer>) -> Self {
        Self {
            customer: Mutex::new(customer),
        }
    }
}

impl CustomerRepository for FakeCustomers {
    fn get_credit_profile(
        &self,
        _customer_id: CustomerId,
    ) -> Result<Option<CreditProfile>, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn record_credit_debit(
        &self,
        _customer_id: CustomerId,
        _amount_centimes: i64,
        _running_balance_centimes: i64,
        _document_id: DocumentId,
        _actor: UserId,
    ) -> Result<String, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn update_cached_balance(
        &self,
        _customer_id: CustomerId,
        _new_balance_centimes: i64,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn create_customer(
        &self,
        _customer: &Customer,
        _origin_device_id: &str,
    ) -> Result<CustomerId, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn update_customer(
        &self,
        _customer_id: CustomerId,
        _update: &CustomerUpdate,
    ) -> Result<(), ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn search_customers(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<CustomerSummary>, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn get_customer(&self, _customer_id: CustomerId) -> Result<Option<Customer>, ApplicationError> {
        Ok(self.customer.lock().expect("customer").clone())
    }

    fn list_customers(&self, _active_only: bool) -> Result<Vec<CustomerSummary>, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn record_credit_payment(
        &self,
        _customer_id: CustomerId,
        _amount_centimes: i64,
        _reference: Option<String>,
        _allow_overpayment: bool,
        _actor: UserId,
        _terminal_id: &str,
    ) -> Result<PaymentRecord, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn get_statement(
        &self,
        _customer_id: CustomerId,
        _from_date: Option<String>,
        _to_date: Option<String>,
    ) -> Result<Vec<LedgerEntryRow>, ApplicationError> {
        panic!("out of scope for return tests");
    }
}

struct FakeCash {
    session: Mutex<Option<CashSessionView>>,
}

impl FakeCash {
    fn with(session: Option<CashSessionView>) -> Self {
        Self {
            session: Mutex::new(session),
        }
    }
}

impl CashSessionRepository for FakeCash {
    fn open_session(
        &self,
        _warehouse_id: WarehouseId,
        _opening_float_centimes: i64,
        _terminal_id: &str,
        _command_id: &str,
        _request_canonical: &str,
        _actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn begin_closing(
        &self,
        _terminal_id: &str,
        _command_id: &str,
        _request_canonical: &str,
        _actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn cancel_closing(
        &self,
        _terminal_id: &str,
        _command_id: &str,
        _request_canonical: &str,
        _actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn confirm_closing(
        &self,
        _session_id: &str,
        _actual_counted_centimes: i64,
        _closing_notes: Option<String>,
        _terminal_id: &str,
        _command_id: &str,
        _request_canonical: &str,
        _actor: UserId,
    ) -> Result<CashSessionView, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn get_active_session(
        &self,
        _terminal_id: &str,
    ) -> Result<Option<CashSessionView>, ApplicationError> {
        Ok(self.session.lock().expect("session").clone())
    }

    fn record_expense(
        &self,
        _amount_centimes: i64,
        _description: &str,
        _terminal_id: &str,
        _command_id: &str,
        _request_canonical: &str,
        _actor: UserId,
    ) -> Result<CashMovementView, ApplicationError> {
        panic!("out of scope for return tests");
    }

    fn get_session_report(&self, _session_id: &str) -> Result<CashSessionReport, ApplicationError> {
        panic!("out of scope for return tests");
    }
}

struct Fixture {
    documents: FakeDocuments,
    returns: FakeReturns,
    customers: FakeCustomers,
    cash: FakeCash,
    source: Document,
    line: DocumentItem,
    customer: Customer,
    session_id: String,
}

fn fixture() -> Fixture {
    let customer = named_customer("Cherif Benz");
    let source = source_document(Some(customer.id.to_string()));
    let line = source_line(&source, 2, 10_000);
    let session_id = Uuid::new_v4().to_string();
    Fixture {
        documents: FakeDocuments::with(source.clone(), vec![line.clone()]),
        returns: FakeReturns::with_cumulative(HashMap::new()),
        customers: FakeCustomers::with(Some(customer.clone())),
        cash: FakeCash::with(Some(open_session_view(&session_id, "OPEN"))),
        source,
        line,
        customer,
        session_id,
    }
}

fn command(fixture: &Fixture, qty_units: i64, refund: RefundDestination) -> ProcessReturnCommand {
    ProcessReturnCommand {
        operator_id: UserId::new(),
        source_document_id: fixture.source.id,
        items: vec![ReturnItemInput {
            source_item_id: fixture.line.id,
            quantity_micro: QtyMicro::from_micros(qty_units * 10_000),
            reason: None,
        }],
        refund,
        idempotency_key: Uuid::new_v4().to_string(),
        terminal_id: TERMINAL.to_string(),
        notes: None,
    }
}

fn use_case(
    fixture: &Fixture,
) -> ProcessReturnUseCase<'_, FakeDocuments, FakeReturns, FakeCustomers, FakeCash> {
    ProcessReturnUseCase::new(
        &fixture.documents,
        &fixture.returns,
        &fixture.customers,
        &fixture.cash,
    )
}

fn is_return_quantity_exceeded(error: &ApplicationError) -> bool {
    matches!(
        error,
        ApplicationError::Validation(DomainError::InvalidCanonicalString(ref code))
            if code == "RETURN_QUANTITY_EXCEEDED"
    )
}

#[test]
fn partial_return_with_cash_refund_restores_stock_and_pays_out() {
    let fixture = fixture();
    let result = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            command(
                &fixture,
                1,
                RefundDestination::Cash {
                    cash_session_id: fixture.session_id.clone(),
                },
            ),
        )
        .expect("return");
    assert_eq!(result.return_document_number, "AVO-2026-000001");
    assert_eq!(result.total_refund_ht, Money::from_centimes(10_000));
    assert_eq!(result.total_refund_tva, Money::from_centimes(0));
    assert_eq!(result.total_refund_ttc, Money::from_centimes(10_000));
    assert_eq!(result.items_returned_count, 1);
    let orders = fixture.returns.orders();
    assert_eq!(orders.len(), 1);
    let order = &orders[0];
    assert_eq!(order.timbre_centimes, 0);
    assert_eq!(order.lines.len(), 1);
    assert_eq!(order.lines[0].quantity_micro, 10_000);
    assert_eq!(order.lines[0].quantity_base_micro, 10_000);
    assert_eq!(order.lines[0].source_item_id, fixture.line.id.to_string());
    let cash = order.cash_refund.as_ref().expect("cash leg");
    assert_eq!(cash.cash_session_id, fixture.session_id);
    assert_eq!(cash.amount_centimes, 10_000);
    assert!(order.credit_refund.is_none());
}

#[test]
fn partial_return_with_ledger_credit_posts_credit_leg() {
    let fixture = fixture();
    let result = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            command(
                &fixture,
                2,
                RefundDestination::CustomerCredit {
                    customer_id: fixture.customer.id,
                },
            ),
        )
        .expect("return");
    assert_eq!(result.total_refund_ttc, Money::from_centimes(20_000));
    let orders = fixture.returns.orders();
    let order = &orders[0];
    assert!(order.cash_refund.is_none());
    let credit = order.credit_refund.as_ref().expect("credit leg");
    assert_eq!(credit.customer_id, fixture.customer.id);
    assert_eq!(credit.amount_centimes, 20_000);
}

#[test]
fn subsequent_partial_return_tracks_cumulative_ceiling() {
    let line_id = Uuid::new_v4();
    let source = source_document(None);
    let line = DocumentItem {
        id: line_id,
        ..source_line(&source, 2, 10_000)
    };
    let fixture = Fixture {
        documents: FakeDocuments::with(source.clone(), vec![line.clone()]),
        returns: FakeReturns::with_cumulative(HashMap::from([(line_id.to_string(), 10_000)])),
        customers: FakeCustomers::with(None),
        cash: FakeCash::with(Some(open_session_view(&Uuid::new_v4().to_string(), "OPEN"))),
        source: source.clone(),
        line: line.clone(),
        customer: named_customer("Nobody"),
        session_id: Uuid::new_v4().to_string(),
    };
    // Exactly the remaining 1 unit succeeds.
    let ok = use_case(&fixture).execute(
        &session_with(Role::Gerant),
        ProcessReturnCommand {
            operator_id: UserId::new(),
            source_document_id: source.id,
            items: vec![ReturnItemInput {
                source_item_id: line_id,
                quantity_micro: QtyMicro::from_micros(10_000),
                reason: Some("second leg".to_string()),
            }],
            refund: RefundDestination::NoRefund,
            idempotency_key: Uuid::new_v4().to_string(),
            terminal_id: TERMINAL.to_string(),
            notes: None,
        },
    );
    assert!(ok.is_ok(), "remaining unit must succeed: {ok:?}");
    // One micro over the cumulative ceiling fails.
    let over = use_case(&fixture).execute(
        &session_with(Role::Gerant),
        ProcessReturnCommand {
            operator_id: UserId::new(),
            source_document_id: source.id,
            items: vec![ReturnItemInput {
                source_item_id: line_id,
                quantity_micro: QtyMicro::from_micros(10_001),
                reason: None,
            }],
            refund: RefundDestination::NoRefund,
            idempotency_key: Uuid::new_v4().to_string(),
            terminal_id: TERMINAL.to_string(),
            notes: None,
        },
    );
    let error = over.expect_err("ceiling breach");
    assert!(is_return_quantity_exceeded(&error), "unexpected: {error:?}");
}

#[test]
fn operator_without_process_return_is_rejected() {
    let fixture = fixture();
    for role in [Role::Vendeur, Role::Caissier] {
        let error = use_case(&fixture)
            .execute(
                &session_with(role),
                command(&fixture, 1, RefundDestination::NoRefund),
            )
            .expect_err("unauthorized");
        assert!(
            matches!(error, ApplicationError::Unauthorized),
            "unexpected for {role:?}: {error:?}"
        );
    }
    assert!(fixture.returns.orders().is_empty());
}

#[test]
fn missing_or_unreturnable_source_is_rejected() {
    let fixture = fixture();
    // The fake serves the seeded document for any id; force absence instead.
    let empty_fixture = Fixture {
        documents: FakeDocuments::empty(),
        returns: FakeReturns::with_cumulative(HashMap::new()),
        customers: FakeCustomers::with(None),
        cash: FakeCash::with(None),
        source: fixture.source.clone(),
        line: fixture.line.clone(),
        customer: fixture.customer.clone(),
        session_id: fixture.session_id.clone(),
    };
    let error = use_case(&empty_fixture)
        .execute(
            &session_with(Role::Gerant),
            command(&empty_fixture, 1, RefundDestination::NoRefund),
        )
        .expect_err("absent source");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::SourceDocumentNotFound)
        ),
        "unexpected: {error:?}"
    );
    // Draft source is not returnable.
    let mut draft_source = fixture.source.clone();
    draft_source.status = DocumentStatus::Draft;
    let draft_fixture = Fixture {
        documents: FakeDocuments::with(draft_source, vec![fixture.line.clone()]),
        returns: FakeReturns::with_cumulative(HashMap::new()),
        customers: FakeCustomers::with(None),
        cash: FakeCash::with(None),
        source: fixture.source.clone(),
        line: fixture.line.clone(),
        customer: fixture.customer.clone(),
        session_id: fixture.session_id.clone(),
    };
    let error = use_case(&draft_fixture)
        .execute(
            &session_with(Role::Gerant),
            command(&draft_fixture, 1, RefundDestination::NoRefund),
        )
        .expect_err("draft source");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidConversionSourceStatus)
        ),
        "unexpected: {error:?}"
    );
}

#[test]
fn unknown_line_and_zero_quantity_are_rejected() {
    let fixture = fixture();
    let unknown = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            ProcessReturnCommand {
                operator_id: UserId::new(),
                source_document_id: fixture.source.id,
                items: vec![ReturnItemInput {
                    source_item_id: Uuid::new_v4(),
                    quantity_micro: QtyMicro::from_micros(10_000),
                    reason: None,
                }],
                refund: RefundDestination::NoRefund,
                idempotency_key: Uuid::new_v4().to_string(),
                terminal_id: TERMINAL.to_string(),
                notes: None,
            },
        )
        .expect_err("unknown line");
    assert!(
        matches!(
            unknown,
            ApplicationError::Validation(DomainError::SourceDocumentNotFound)
        ),
        "unexpected: {unknown:?}"
    );
    let zero = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            ProcessReturnCommand {
                operator_id: UserId::new(),
                source_document_id: fixture.source.id,
                items: vec![ReturnItemInput {
                    source_item_id: fixture.line.id,
                    quantity_micro: QtyMicro::from_micros(0),
                    reason: None,
                }],
                refund: RefundDestination::NoRefund,
                idempotency_key: Uuid::new_v4().to_string(),
                terminal_id: TERMINAL.to_string(),
                notes: None,
            },
        )
        .expect_err("zero qty");
    assert!(
        matches!(
            zero,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
        ),
        "unexpected: {zero:?}"
    );
}

#[test]
fn anonymous_sale_cannot_credit_a_ledger() {
    let source = source_document(None);
    let line = source_line(&source, 1, 5_000);
    let fixture = Fixture {
        documents: FakeDocuments::with(source.clone(), vec![line.clone()]),
        returns: FakeReturns::with_cumulative(HashMap::new()),
        customers: FakeCustomers::with(Some(named_customer("Cherif Benz"))),
        cash: FakeCash::with(None),
        source,
        line,
        customer: named_customer("Other"),
        session_id: Uuid::new_v4().to_string(),
    };
    let error = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            ProcessReturnCommand {
                operator_id: UserId::new(),
                source_document_id: fixture.source.id,
                items: vec![ReturnItemInput {
                    source_item_id: fixture.line.id,
                    quantity_micro: QtyMicro::from_micros(10_000),
                    reason: None,
                }],
                refund: RefundDestination::CustomerCredit {
                    customer_id: fixture.customer.id,
                },
                idempotency_key: Uuid::new_v4().to_string(),
                terminal_id: TERMINAL.to_string(),
                notes: None,
            },
        )
        .expect_err("guest credit");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(ref code))
                if code == "CREDIT_CUSTOMER_REQUIRED"
        ),
        "unexpected: {error:?}"
    );
}

#[test]
fn mismatched_credit_customer_is_rejected() {
    let fixture = fixture();
    let error = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            ProcessReturnCommand {
                operator_id: UserId::new(),
                source_document_id: fixture.source.id,
                items: vec![ReturnItemInput {
                    source_item_id: fixture.line.id,
                    quantity_micro: QtyMicro::from_micros(10_000),
                    reason: None,
                }],
                refund: RefundDestination::CustomerCredit {
                    customer_id: CustomerId::new(),
                },
                idempotency_key: Uuid::new_v4().to_string(),
                terminal_id: TERMINAL.to_string(),
                notes: None,
            },
        )
        .expect_err("mismatch");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::CreditDisabled)
        ),
        "unexpected: {error:?}"
    );
}

#[test]
fn cash_refund_without_open_session_is_rejected() {
    let fixture = fixture();
    // No session at all.
    let closed = Fixture {
        documents: FakeDocuments::with(fixture.source.clone(), vec![fixture.line.clone()]),
        returns: FakeReturns::with_cumulative(HashMap::new()),
        customers: FakeCustomers::with(None),
        cash: FakeCash::with(None),
        source: fixture.source.clone(),
        line: fixture.line.clone(),
        customer: fixture.customer.clone(),
        session_id: fixture.session_id.clone(),
    };
    let error = use_case(&closed)
        .execute(
            &session_with(Role::Gerant),
            command(
                &closed,
                1,
                RefundDestination::Cash {
                    cash_session_id: closed.session_id.clone(),
                },
            ),
        )
        .expect_err("no session");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
        ),
        "unexpected: {error:?}"
    );
    // Wrong session id (e.g. CLOSING session or another terminal's drawer).
    let other = Fixture {
        documents: FakeDocuments::with(fixture.source.clone(), vec![fixture.line.clone()]),
        returns: FakeReturns::with_cumulative(HashMap::new()),
        customers: FakeCustomers::with(None),
        cash: FakeCash::with(Some(open_session_view(&fixture.session_id, "CLOSING"))),
        source: fixture.source.clone(),
        line: fixture.line.clone(),
        customer: fixture.customer.clone(),
        session_id: fixture.session_id.clone(),
    };
    let error = use_case(&other)
        .execute(
            &session_with(Role::Gerant),
            command(
                &other,
                1,
                RefundDestination::Cash {
                    cash_session_id: Uuid::new_v4().to_string(),
                },
            ),
        )
        .expect_err("wrong session");
    assert!(
        matches!(
            error,
            ApplicationError::Validation(DomainError::NoOpenCashSessionForTerminal)
        ),
        "unexpected: {error:?}"
    );
    assert!(closed.returns.orders().is_empty());
    assert!(other.returns.orders().is_empty());
}

#[test]
fn empty_items_and_blank_keys_are_rejected_before_any_write() {
    let fixture = fixture();
    let empty = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            ProcessReturnCommand {
                operator_id: UserId::new(),
                source_document_id: fixture.source.id,
                items: Vec::new(),
                refund: RefundDestination::NoRefund,
                idempotency_key: Uuid::new_v4().to_string(),
                terminal_id: TERMINAL.to_string(),
                notes: None,
            },
        )
        .expect_err("empty");
    assert!(
        matches!(empty, ApplicationError::Validation(DomainError::EmptyCart)),
        "unexpected: {empty:?}"
    );
    let blank = use_case(&fixture)
        .execute(
            &session_with(Role::Gerant),
            ProcessReturnCommand {
                operator_id: UserId::new(),
                source_document_id: fixture.source.id,
                items: vec![ReturnItemInput {
                    source_item_id: fixture.line.id,
                    quantity_micro: QtyMicro::from_micros(10_000),
                    reason: None,
                }],
                refund: RefundDestination::NoRefund,
                idempotency_key: "   ".to_string(),
                terminal_id: TERMINAL.to_string(),
                notes: None,
            },
        )
        .expect_err("blank key");
    assert!(
        matches!(
            blank,
            ApplicationError::Validation(DomainError::InvalidCanonicalString(_))
        ),
        "unexpected: {blank:?}"
    );
    assert!(fixture.returns.orders().is_empty());
}
