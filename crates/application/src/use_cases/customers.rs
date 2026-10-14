//! Customer management use cases: directory, credit payments, statements.
//!
//! RBAC is enforced before any repository contact: reads need
//! `ViewCustomers`; creation/updates need `CreateCustomer` (credit fields
//! additionally need `ManageCustomerCredit`); payments need
//! `RecordCreditPayment` (overpayment additionally needs
//! `ManageCustomerCredit`).

use baraka_domain::errors::DomainError;
use baraka_domain::role::Permission;
use baraka_domain::sales::{
    validate_customer_name, validate_customer_phone, CreditPolicy, Customer, CustomerId,
};

use crate::errors::ApplicationError;
use crate::ports::{
    CustomerRepository, CustomerSummary, CustomerUpdate, LedgerEntryRow, NewCustomer,
    PaymentRecord, SessionContext,
};

/// Stable-code constructor for i18n-ready validation failures.
fn coded(code: &'static str) -> ApplicationError {
    ApplicationError::Validation(DomainError::InvalidCanonicalString(code.to_string()))
}

fn require(session: &SessionContext, permission: Permission) -> Result<(), ApplicationError> {
    if session.role.has_permission(permission) {
        Ok(())
    } else {
        Err(ApplicationError::Unauthorized)
    }
}

/// Validate creation policy/limit pairing shared by create and update.
fn validate_policy_limit(
    policy: CreditPolicy,
    limit: i64,
    has_credit_authority: bool,
) -> Result<i64, ApplicationError> {
    match policy {
        CreditPolicy::Disabled => Ok(0),
        CreditPolicy::Unlimited => {
            if !has_credit_authority {
                return Err(ApplicationError::Unauthorized);
            }
            Ok(0)
        }
        CreditPolicy::Limited => {
            if !has_credit_authority {
                return Err(ApplicationError::Unauthorized);
            }
            if limit <= 0 {
                return Err(ApplicationError::Validation(
                    DomainError::InvalidCreditLimit,
                ));
            }
            Ok(limit)
        }
    }
}

/// Register a new customer.
pub struct CreateCustomerUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> CreateCustomerUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// Validate and persist a customer.
    /// `terminal_id` is TerminalContext-resolved (never client-supplied).
    pub fn execute(
        &self,
        session: &SessionContext,
        input: NewCustomer,
        terminal_id: &str,
    ) -> Result<CustomerId, ApplicationError> {
        require(session, Permission::CreateCustomer)?;
        if terminal_id.trim().is_empty() {
            return Err(ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_TERMINAL_ID".to_string()),
            ));
        }
        let name = validate_customer_name(&input.name).map_err(ApplicationError::Validation)?;
        let phone = input
            .phone
            .as_deref()
            .map(validate_customer_phone)
            .transpose()
            .map_err(ApplicationError::Validation)?;
        let address = input
            .address
            .map(|address| {
                let trimmed = address.trim();
                if trimmed.is_empty() {
                    Err(ApplicationError::Validation(
                        DomainError::InvalidCanonicalString(
                            "customer address must not be blank".to_string(),
                        ),
                    ))
                } else {
                    Ok(trimmed.to_string())
                }
            })
            .transpose()?;
        let limit = validate_policy_limit(
            input.credit_policy,
            input.credit_limit_centimes,
            session
                .role
                .has_permission(Permission::ManageCustomerCredit),
        )?;
        let id = CustomerId::new();
        self.customers.create_customer(
            &Customer {
                id,
                name,
                phone,
                address,
                credit_policy: input.credit_policy,
                credit_limit_centimes: limit,
                customer_tier: input.customer_tier,
                is_active: true,
            },
            terminal_id.trim(),
        )?;
        Ok(id)
    }
}

/// Update mutable customer fields.
pub struct UpdateCustomerUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> UpdateCustomerUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// Validate and persist the sparse update.
    pub fn execute(
        &self,
        session: &SessionContext,
        customer_id: CustomerId,
        input: CustomerUpdate,
    ) -> Result<(), ApplicationError> {
        require(session, Permission::CreateCustomer)?;
        let mut update = CustomerUpdate::default();
        if let Some(name) = input.name.as_deref() {
            update.name = Some(validate_customer_name(name).map_err(ApplicationError::Validation)?);
        }
        if let Some(phone) = input.phone.as_ref() {
            update.phone = Some(
                phone
                    .as_deref()
                    .map(validate_customer_phone)
                    .transpose()
                    .map_err(ApplicationError::Validation)?,
            );
        }
        if let Some(address) = input.address.as_ref() {
            update.address = Some(
                address
                    .as_deref()
                    .map(|text| {
                        let trimmed = text.trim();
                        if trimmed.is_empty() {
                            Err(ApplicationError::Validation(
                                DomainError::InvalidCanonicalString(
                                    "customer address must not be blank".to_string(),
                                ),
                            ))
                        } else {
                            Ok(trimmed.to_string())
                        }
                    })
                    .transpose()?,
            );
        }
        if input.credit_policy.is_some() || input.credit_limit_centimes.is_some() {
            let stored = self
                .customers
                .get_customer(customer_id)?
                .ok_or(ApplicationError::CustomerNotFound)?;
            let policy = input.credit_policy.unwrap_or(stored.credit_policy);
            let limit = input
                .credit_limit_centimes
                .unwrap_or(stored.credit_limit_centimes);
            let resolved = validate_policy_limit(
                policy,
                limit,
                session
                    .role
                    .has_permission(Permission::ManageCustomerCredit),
            )?;
            update.credit_policy = Some(policy);
            update.credit_limit_centimes = Some(resolved);
        }
        if let Some(is_active) = input.is_active {
            update.is_active = Some(is_active);
        }
        if let Some(tier) = input.customer_tier {
            update.customer_tier = Some(tier);
        }
        self.customers.update_customer(customer_id, &update)
    }
}

/// Search the customer directory.
pub struct SearchCustomersUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> SearchCustomersUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// Prefix/multilingual search (blank queries return no rows).
    pub fn execute(
        &self,
        session: &SessionContext,
        query: &str,
        limit: usize,
    ) -> Result<Vec<CustomerSummary>, ApplicationError> {
        require(session, Permission::ViewCustomers)?;
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        self.customers
            .search_customers(trimmed, limit.clamp(1, 100))
    }
}

/// Fetch one full customer view.
pub struct GetCustomerUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> GetCustomerUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// Load the detail view or fail with `CustomerNotFound`.
    pub fn execute(
        &self,
        session: &SessionContext,
        customer_id: CustomerId,
    ) -> Result<Customer, ApplicationError> {
        require(session, Permission::ViewCustomers)?;
        self.customers
            .get_customer(customer_id)?
            .ok_or(ApplicationError::CustomerNotFound)
    }
}

/// List the customer directory.
pub struct ListCustomersUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> ListCustomersUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// List customers ordered by name.
    pub fn execute(
        &self,
        session: &SessionContext,
        active_only: bool,
    ) -> Result<Vec<CustomerSummary>, ApplicationError> {
        require(session, Permission::ViewCustomers)?;
        self.customers.list_customers(active_only)
    }
}

/// Record a customer credit payment (CREDIT ledger entry).
pub struct RecordCreditPaymentUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> RecordCreditPaymentUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// Validate, authorize, and append the payment atomically.
    /// ADR-027: `terminal_id` binds the cash drawer collection (never client input).
    pub fn execute(
        &self,
        session: &SessionContext,
        customer_id: CustomerId,
        amount_centimes: i64,
        reference: Option<String>,
        allow_overpayment: bool,
        terminal_id: &str,
    ) -> Result<PaymentRecord, ApplicationError> {
        require(session, Permission::RecordCreditPayment)?;
        if amount_centimes <= 0 {
            return Err(coded("INVALID_PAYMENT_AMOUNT"));
        }
        if terminal_id.trim().is_empty() {
            return Err(coded("INVALID_TERMINAL_ID"));
        }
        if allow_overpayment
            && !session
                .role
                .has_permission(Permission::ManageCustomerCredit)
        {
            return Err(ApplicationError::OverpaymentUnauthorized);
        }
        self.customers.record_credit_payment(
            customer_id,
            amount_centimes,
            reference,
            allow_overpayment,
            session.user_id,
            terminal_id.trim(),
        )
    }
}

/// Read a customer statement with server-side running balances.
pub struct GetCustomerStatementUseCase<'a, C>
where
    C: ?Sized,
{
    customers: &'a C,
}

impl<'a, C> GetCustomerStatementUseCase<'a, C>
where
    C: CustomerRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(customers: &'a C) -> Self {
        Self { customers }
    }

    /// Load ledger rows in ledger order.
    pub fn execute(
        &self,
        session: &SessionContext,
        customer_id: CustomerId,
        from_date: Option<String>,
        to_date: Option<String>,
    ) -> Result<Vec<LedgerEntryRow>, ApplicationError> {
        require(session, Permission::ViewCustomers)?;
        if self.customers.get_customer(customer_id)?.is_none() {
            return Err(ApplicationError::CustomerNotFound);
        }
        self.customers
            .get_statement(customer_id, from_date, to_date)
    }
}
