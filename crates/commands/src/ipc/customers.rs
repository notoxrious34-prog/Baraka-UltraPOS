//! Customer commands: directory, credit ledger, statements.
use super::drafts::parse_optional_int;
use super::{
    error_code, parse_customer_tier, parse_wire_id, parse_wire_int, require_session, AppState,
};
use baraka_application::errors::ApplicationError;
use baraka_application::use_cases::customers::{
    CreateCustomerUseCase, GetCustomerStatementUseCase, GetCustomerUseCase, ListCustomersUseCase,
    RecordCreditPaymentUseCase, SearchCustomersUseCase, UpdateCustomerUseCase,
};
use baraka_application::use_cases::deletion::DeleteCustomerUseCase;
use baraka_domain::sales::{CreditPolicy, CustomerId};

use crate::dto::{
    CustomerDetailDto, CustomerSummaryDto, DeleteEntityRequestDto, DeleteEntityResultDto,
    GetCustomerRequestDto, GetStatementRequest, GetStatementResponse, NewCustomerRequest,
    RecordCreditPaymentRequest, RecordCreditPaymentResponse, SearchCustomersRequest,
    SearchCustomersResponse, StatementEntryDto, UpdateCustomerRequest,
};

/// `customers_create` command: register a customer (RBAC-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_create(
    state: tauri::State<'_, AppState>,
    request: NewCustomerRequest,
) -> Result<CustomerDetailDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let policy = CreditPolicy::from_code(request.credit_policy.trim())
        .map_err(|error| error_code(&ApplicationError::Validation(error)))?;
    let id = CreateCustomerUseCase::new(state.customers.as_ref())
        .execute(
            &session,
            baraka_application::ports::NewCustomer {
                name: request.name,
                phone: request.phone,
                address: request.address,
                credit_policy: policy,
                credit_limit_centimes: request
                    .credit_limit_centimes
                    .as_deref()
                    .map(|text| parse_wire_int(text, "credit_limit_centimes"))
                    .transpose()
                    .map_err(|error| error_code(&error))?
                    .unwrap_or(0),
                customer_tier: parse_customer_tier(
                    request.customer_tier.as_deref(),
                    "customer_tier",
                )
                .map_err(|error| error_code(&error))?,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    customers_get(
        state,
        GetCustomerRequestDto {
            session_token: request.session_token,
            customer_id: id.to_string(),
        },
    )
}

/// `customers_update` command: sparse customer update (RBAC-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_update(
    state: tauri::State<'_, AppState>,
    request: UpdateCustomerRequest,
) -> Result<CustomerDetailDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    UpdateCustomerUseCase::new(state.customers.as_ref())
        .execute(
            &session,
            id,
            baraka_application::ports::CustomerUpdate {
                name: request.name,
                phone: request.phone,
                address: request.address,
                credit_policy: request
                    .credit_policy
                    .as_deref()
                    .map(CreditPolicy::from_code)
                    .transpose()
                    .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
                credit_limit_centimes: parse_optional_int(
                    request.credit_limit_centimes.as_deref(),
                    "credit_limit_centimes",
                )?,
                customer_tier: match request.customer_tier.as_deref().map(str::trim) {
                    None | Some("") => None,
                    Some(code) => Some(
                        parse_customer_tier(Some(code), "customer_tier")
                            .map_err(|error| error_code(&error))?,
                    ),
                },
                is_active: request.is_active,
            },
        )
        .map_err(|error| error_code(&error))?;
    customers_get(
        state,
        GetCustomerRequestDto {
            session_token: request.session_token,
            customer_id: id.to_string(),
        },
    )
}

/// `customers_delete` command: guarded dual-track safe deletion (ADR-092).
/// Non-zero balances reject; history deactivates. Credit-manage roles only.
#[tauri::command(rename_all = "snake_case")]
pub fn customers_delete(
    state: tauri::State<'_, AppState>,
    request: DeleteEntityRequestDto,
) -> Result<DeleteEntityResultDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.id, "id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    let outcome = DeleteCustomerUseCase::new(state.customers.as_ref())
        .execute(&session, id, &request.reason)
        .map_err(|error| error_code(&error))?;
    Ok(DeleteEntityResultDto {
        id: outcome.id,
        mode: outcome.mode.to_string(),
        message: outcome.message.to_string(),
    })
}

fn to_summary(summary: &baraka_application::ports::CustomerSummary) -> CustomerSummaryDto {
    CustomerSummaryDto {
        id: summary.id.to_string(),
        name: summary.name.clone(),
        phone: summary.phone.clone(),
        credit_policy: summary.credit_policy.code().to_string(),
        customer_tier: summary.customer_tier.as_str().to_string(),
    }
}

/// `customers_search` command: directory search (any authenticated role).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_search(
    state: tauri::State<'_, AppState>,
    request: SearchCustomersRequest,
) -> Result<SearchCustomersResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let limit = request.limit.unwrap_or(20) as usize;
    let results = SearchCustomersUseCase::new(state.customers.as_ref())
        .execute(&session, &request.query, limit)
        .map_err(|error| error_code(&error))?;
    Ok(SearchCustomersResponse {
        customers: results.iter().map(to_summary).collect(),
    })
}

/// Shared detail assembler for get/update/create responses.
fn assemble_detail(
    state: &tauri::State<'_, AppState>,
    session: &baraka_application::ports::SessionContext,
    id: CustomerId,
) -> Result<CustomerDetailDto, String> {
    let customer = GetCustomerUseCase::new(state.customers.as_ref())
        .execute(session, id)
        .map_err(|error| error_code(&error))?;
    let profile = state
        .customers
        .get_credit_profile(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| "CUSTOMER_NOT_FOUND".to_string())?;
    Ok(CustomerDetailDto {
        id: customer.id.to_string(),
        name: customer.name.clone(),
        phone: customer.phone.clone(),
        address: customer.address.clone(),
        credit_policy: customer.credit_policy.code().to_string(),
        credit_limit_centimes: if customer.credit_policy == CreditPolicy::Limited {
            Some(customer.credit_limit_centimes.to_string())
        } else {
            None
        },
        customer_tier: customer.customer_tier.as_str().to_string(),
        current_balance_centimes: profile.current_balance_centimes.to_string(),
        is_active: customer.is_active,
    })
}

/// `customers_get` command: full detail view (any authenticated role).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_get(
    state: tauri::State<'_, AppState>,
    request: GetCustomerRequestDto,
) -> Result<CustomerDetailDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    assemble_detail(&state, &session, id)
}

/// `customers_list` command: directory listing (any authenticated role).
#[tauri::command(rename_all = "snake_case")]
pub fn customers_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<SearchCustomersResponse, String> {
    let session = require_session(&state, &session_token)?;
    let results = ListCustomersUseCase::new(state.customers.as_ref())
        .execute(&session, true)
        .map_err(|error| error_code(&error))?;
    Ok(SearchCustomersResponse {
        customers: results.iter().map(to_summary).collect(),
    })
}

/// `credit_record_payment` command: append a CREDIT ledger line.
#[tauri::command(rename_all = "snake_case")]
pub fn credit_record_payment(
    state: tauri::State<'_, AppState>,
    request: RecordCreditPaymentRequest,
) -> Result<RecordCreditPaymentResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    let record = RecordCreditPaymentUseCase::new(state.customers.as_ref())
        .execute(
            &session,
            id,
            parse_wire_int(&request.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
            request.reference,
            request.allow_overpayment,
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(RecordCreditPaymentResponse {
        ledger_entry_id: record.ledger_entry_id,
        new_balance_centimes: record.new_balance_centimes.to_string(),
    })
}

/// `credit_get_statement` command: ledger rows with running balances.
#[tauri::command(rename_all = "snake_case")]
pub fn credit_get_statement(
    state: tauri::State<'_, AppState>,
    request: GetStatementRequest,
) -> Result<GetStatementResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.customer_id, "customer_id")
        .map(CustomerId::from)
        .map_err(|error| error_code(&error))?;
    let rows = GetCustomerStatementUseCase::new(state.customers.as_ref())
        .execute(&session, id, request.from_date, request.to_date)
        .map_err(|error| error_code(&error))?;
    Ok(GetStatementResponse {
        entries: rows
            .into_iter()
            .map(|row| StatementEntryDto {
                entry_type: row.entry_type,
                amount_centimes: row.amount_centimes.to_string(),
                reference_type: row.reference_type,
                reference_id: row.reference_id,
                created_at: row.created_at,
                balance_after_centimes: row.balance_after_centimes.to_string(),
            })
            .collect(),
    })
}
