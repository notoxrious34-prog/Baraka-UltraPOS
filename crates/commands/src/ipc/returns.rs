//! Return commands: source summaries, execution, simplified facade.
use super::drafts::discounted_unit_price_text;
use super::{error_code, parse_wire_id, parse_wire_int, require_session, AppState};
use baraka_application::errors::ApplicationError;
use baraka_application::use_cases::returns::{
    ProcessReturnCommand, ProcessReturnUseCase, ProcessSaleReturnUseCase, RefundDestination,
    ReturnItemInput,
};
use baraka_domain::errors::DomainError;
use baraka_domain::primitives::QtyMicro;
use baraka_domain::sales::{CustomerId, DocumentId, DocumentType};
use baraka_domain::user::UserId;
use uuid::Uuid;

use crate::dto::{
    DocumentsProcessReturnRequestDto, DocumentsProcessReturnResponseDto, ProcessReturnRequestDto,
    ProcessReturnResponseDto, RefundDestinationDto, ReturnContextResponseDto,
    ReturnableDocumentSummaryDto, ReturnableItemDto, ReturnableLineDto,
};

/// `returns_get_source_summary` command: returnable state of one source
/// document (TASK-BRK-047). Read-only: original quantities, cumulative
/// prior returns, remaining returnable amounts, and converted-payment
/// eligibility — everything the return modal needs before dispatch.
#[tauri::command(rename_all = "snake_case")]
pub fn returns_get_source_summary(
    state: tauri::State<'_, AppState>,
    session_token: String,
    source_document_id: String,
) -> Result<ReturnableDocumentSummaryDto, String> {
    require_session(&state, &session_token)?;
    let id = parse_wire_id(&source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let document = state
        .documents
        .get_by_id(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| error_code(&ApplicationError::NotFound("document".to_string())))?;
    let items = state
        .documents
        .get_items(id)
        .map_err(|error| error_code(&error))?;
    let cumulative = state
        .returns
        .cumulative_returned_quantities(id)
        .map_err(|error| error_code(&error))?;
    let party_name = match document.party_id.as_deref() {
        Some(party) => parse_wire_id(party, "party_id")
            .map(CustomerId::from)
            .ok()
            .and_then(|customer_id| {
                state
                    .customers
                    .get_customer(customer_id)
                    .map_err(|error| error_code(&error))
                    .transpose()
            })
            .transpose()?
            .map(|customer| customer.name),
        None => None,
    };
    let mut lines = Vec::with_capacity(items.len());
    let mut any_remaining = false;
    for item in &items {
        let already = cumulative.get(&item.id.to_string()).copied().unwrap_or(0);
        let remaining = (item.quantity_micro - already).max(0);
        if remaining > 0 {
            any_remaining = true;
        }
        let (product_name, sku, unit_name) = match state
            .catalog
            .find_by_id(item.product_id)
            .map_err(|error| error_code(&error))?
        {
            Some((product, units)) => {
                let unit_label = units
                    .iter()
                    .find(|unit| unit.id == item.unit_id)
                    .map(|unit| unit.unit_name_fr.clone())
                    .unwrap_or_else(|| item.unit_id.to_string());
                (
                    product.name_fr.clone(),
                    product.sku.as_str().to_string(),
                    unit_label,
                )
            }
            None => (
                item.product_id.to_string(),
                String::new(),
                item.unit_id.to_string(),
            ),
        };
        // Display-aid net unit price (truncated; the engine recomputes
        // exact pro-rated lines at execution).
        let net_unit_price = if item.quantity_micro == 0 {
            0
        } else {
            (i128::from(item.line_ht_centimes) * 10_000 / i128::from(item.quantity_micro))
                .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
        };
        lines.push(ReturnableLineDto {
            source_item_id: item.id.to_string(),
            product_id: item.product_id.to_string(),
            product_name,
            sku,
            unit_id: item.unit_id.to_string(),
            unit_name,
            original_qty_micro: item.quantity_micro.to_string(),
            already_returned_qty_micro: already.to_string(),
            remaining_qty_micro: remaining.to_string(),
            unit_price_cents: item.unit_price_centimes.to_string(),
            net_unit_price_cents: net_unit_price.to_string(),
            tva_rate: item.tva_rate_micro.to_string(),
        });
    }
    let eligible = document.status == baraka_domain::sales::DocumentStatus::Finalized
        && matches!(
            document.doc_type,
            DocumentType::Ticket | DocumentType::Facture | DocumentType::Bl
        )
        && any_remaining;
    Ok(ReturnableDocumentSummaryDto {
        source_document_id: document.id.to_string(),
        source_document_number: document.number.clone(),
        document_type: document.doc_type.code().to_string(),
        party_type: document.party_id.as_ref().map(|_| "CUSTOMER".to_string()),
        party_id: document.party_id.clone(),
        party_name,
        warehouse_id: document.warehouse_id.to_string(),
        total_ht_cents: document.subtotal_ht_centimes.to_string(),
        total_ttc_cents: document.ttc_centimes.to_string(),
        is_eligible_for_return: eligible,
        items: lines,
    })
}

/// `returns_process` command: validate and atomically persist a merchandise
/// return (TASK-BRK-047). Terminal affinity resolves from TerminalContext
/// (`state.terminal_id`, never client routing); the DTO `terminal_id` is
/// accepted for wire compatibility and must be non-blank.
#[tauri::command(rename_all = "snake_case")]
pub fn returns_process(
    state: tauri::State<'_, AppState>,
    request: ProcessReturnRequestDto,
) -> Result<ProcessReturnResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    if request.terminal_id.trim().is_empty() {
        return Err(error_code(&ApplicationError::Validation(
            DomainError::InvalidCanonicalString("INVALID_TERMINAL_ID".to_string()),
        )));
    }
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let source_number = state
        .documents
        .get_by_id(source_id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| error_code(&ApplicationError::NotFound("document".to_string())))?
        .number
        .clone();
    let mut items = Vec::with_capacity(request.items.len());
    for line in &request.items {
        items.push(ReturnItemInput {
            source_item_id: parse_wire_id(&line.source_item_id, "source_item_id")
                .map_err(|error| error_code(&error))?,
            quantity_micro: QtyMicro::from_micros(
                parse_wire_int(&line.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
            ),
            reason: line.reason.clone(),
        });
    }
    let refund = match &request.refund {
        RefundDestinationDto::Cash { cash_session_id } => RefundDestination::Cash {
            cash_session_id: cash_session_id.clone(),
        },
        RefundDestinationDto::CustomerCredit { customer_id } => RefundDestination::CustomerCredit {
            customer_id: parse_wire_id(customer_id, "customer_id")
                .map(CustomerId::from)
                .map_err(|error| error_code(&error))?,
        },
        RefundDestinationDto::NoRefund => RefundDestination::NoRefund,
    };
    let refund_type = match &request.refund {
        RefundDestinationDto::Cash { .. } => "CASH",
        RefundDestinationDto::CustomerCredit { .. } => "CUSTOMER_CREDIT",
        RefundDestinationDto::NoRefund => "NO_REFUND",
    }
    .to_string();
    let result = ProcessReturnUseCase::new(
        state.documents.as_ref(),
        state.returns.as_ref(),
        state.customers.as_ref(),
        state.cash.as_ref(),
    )
    .execute(
        &session,
        ProcessReturnCommand {
            operator_id: parse_wire_id(&request.operator_id, "operator_id")
                .map(UserId::from)
                .map_err(|error| error_code(&error))?,
            source_document_id: source_id,
            items,
            refund,
            idempotency_key: request.idempotency_key,
            terminal_id: state.terminal_id.clone(),
            notes: request.notes,
        },
    )
    .map_err(|error| error_code(&error))?;
    Ok(ProcessReturnResponseDto {
        return_document_id: result.return_document_id.to_string(),
        return_document_number: result.return_document_number,
        source_document_id: source_id.to_string(),
        source_document_number: source_number,
        total_ht_cents: result.total_refund_ht.centimes().to_string(),
        total_tva_cents: result.total_refund_tva.centimes().to_string(),
        total_ttc_cents: result.total_refund_ttc.centimes().to_string(),
        refund_type,
        lines_count: result.items_returned_count,
    })
}

/// Wire-code translator for the simplified return facade (TASK-BRK-046
/// re-issue): the use case emits conversion-era stable codes, while this
/// facade contracts the dedicated return taxonomy.
fn return_wire_code(error: &ApplicationError) -> String {
    match error_code(error).as_str() {
        "INVALID_CONVERSION_SOURCE_STATUS" | "INVALID_CONVERSION_SOURCE_TYPE" => {
            "INVALID_SOURCE_DOCUMENT_STATE".to_string()
        }
        "NO_OPEN_CASH_SESSION_FOR_TERMINAL" => "NO_ACTIVE_CASH_SESSION".to_string(),
        "CREDIT_CUSTOMER_REQUIRED" => "MISSING_CUSTOMER_FOR_CREDIT_REFUND".to_string(),
        "EMPTY_CART" => "EMPTY_RETURN".to_string(),
        _ => error_code(error),
    }
}

/// `documents_get_return_context` command: returnable state of one source
/// document (TASK-BRK-046 re-issue). Read-only facade over the same reads
/// as `returns_get_source_summary`, shaped for the return workstation.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_get_return_context(
    state: tauri::State<'_, AppState>,
    session_token: String,
    source_document_id: String,
) -> Result<ReturnContextResponseDto, String> {
    require_session(&state, &session_token)?;
    let id = parse_wire_id(&source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let document = state
        .documents
        .get_by_id(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| error_code(&ApplicationError::NotFound("document".to_string())))?;
    let items = state
        .documents
        .get_items(id)
        .map_err(|error| error_code(&error))?;
    let cumulative = state
        .returns
        .cumulative_returned_quantities(id)
        .map_err(|error| error_code(&error))?;
    let party_name = match document.party_id.as_deref() {
        Some(party) => parse_wire_id(party, "party_id")
            .map(CustomerId::from)
            .ok()
            .and_then(|customer_id| {
                state
                    .customers
                    .get_customer(customer_id)
                    .map_err(|error| error_code(&error))
                    .transpose()
            })
            .transpose()?
            .map(|customer| customer.name),
        None => None,
    };
    let kind_returnable = matches!(
        document.doc_type,
        DocumentType::Ticket | DocumentType::Facture | DocumentType::Bl
    );
    let mut lines = Vec::with_capacity(items.len());
    let mut any_remaining = false;
    for item in &items {
        let already = cumulative.get(&item.id.to_string()).copied().unwrap_or(0);
        let remaining = (item.quantity_micro - already).max(0);
        if remaining > 0 {
            any_remaining = true;
        }
        let (product_name, barcode, unit_name) = match state
            .catalog
            .find_by_id(item.product_id)
            .map_err(|error| error_code(&error))?
        {
            Some((product, units)) => {
                let unit = units.iter().find(|unit| unit.id == item.unit_id);
                (
                    product.name_fr.clone(),
                    unit.and_then(|unit| {
                        unit.barcode.as_ref().map(|code| code.as_str().to_string())
                    }),
                    unit.map(|unit| unit.unit_name_fr.clone())
                        .unwrap_or_else(|| item.unit_id.to_string()),
                )
            }
            None => (item.product_id.to_string(), None, item.unit_id.to_string()),
        };
        lines.push(ReturnableItemDto {
            source_item_id: item.id.to_string(),
            product_id: item.product_id.to_string(),
            product_name,
            barcode,
            unit_id: item.unit_id.to_string(),
            unit_name,
            unit_price_centimes: item.unit_price_centimes.to_string(),
            discount_per_unit_centimes: item
                .discount_per_unit_centimes
                .map(|discount| discount.to_string()),
            discounted_unit_price_centimes: discounted_unit_price_text(
                item.unit_price_centimes,
                item.discount_per_unit_centimes,
            ),
            original_quantity_micro: item.quantity_micro.to_string(),
            already_returned_micro: already.to_string(),
            remaining_allowable_micro: remaining.to_string(),
            tva_rate: u32::try_from(item.tva_rate_micro).unwrap_or(0),
            discount_centimes: item.discount_centimes.to_string(),
        });
    }
    let (is_eligible, ineligibility_reason) =
        if document.status != baraka_domain::sales::DocumentStatus::Finalized {
            (false, Some("NOT_FINALIZED".to_string()))
        } else if !kind_returnable {
            (false, Some("NON_RETURNABLE_TYPE".to_string()))
        } else if !any_remaining {
            (false, Some("FULLY_RETURNED".to_string()))
        } else {
            (true, None)
        };
    Ok(ReturnContextResponseDto {
        source_document_id: document.id.to_string(),
        document_number: document.number.clone(),
        document_type: document.doc_type.code().to_string(),
        party_id: document.party_id.clone(),
        party_name,
        warehouse_id: document.warehouse_id.to_string(),
        created_at: document
            .created_at
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        is_eligible,
        ineligibility_reason,
        items: lines,
    })
}

/// `documents_process_return` command: simplified single-shot merchandise
/// return (TASK-BRK-046 re-issue). Settlement travels as a plain method
/// code; the operator resolves from the session, the terminal from
/// TerminalContext, and a fresh idempotency key is minted per call —
/// callers needing retry-safety use `returns_process` with an explicit
/// key. Error codes follow the dedicated return taxonomy.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_process_return(
    state: tauri::State<'_, AppState>,
    request: DocumentsProcessReturnRequestDto,
) -> Result<DocumentsProcessReturnResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let mut items = Vec::with_capacity(request.items.len());
    for line in &request.items {
        items.push(ReturnItemInput {
            source_item_id: parse_wire_id(&line.source_item_id, "source_item_id")
                .map_err(|error| error_code(&error))?,
            quantity_micro: QtyMicro::from_micros(
                parse_wire_int(&line.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
            ),
            reason: None,
        });
    }
    let method = request.refund_method.trim().to_uppercase();
    let refund = match method.as_str() {
        "CASH" => {
            let active = state
                .cash
                .get_active_session(&state.terminal_id)
                .map_err(|error| error_code(&error))?
                .filter(|view| view.status == "OPEN")
                .ok_or_else(|| "NO_ACTIVE_CASH_SESSION".to_string())?;
            RefundDestination::Cash {
                cash_session_id: active.id,
            }
        }
        "CUSTOMER_CREDIT" => {
            let party = state
                .documents
                .get_by_id(source_id)
                .map_err(|error| error_code(&error))?
                .and_then(|document| document.party_id.clone())
                .ok_or_else(|| "MISSING_CUSTOMER_FOR_CREDIT_REFUND".to_string())?;
            RefundDestination::CustomerCredit {
                customer_id: parse_wire_id(&party, "party_id")
                    .map(CustomerId::from)
                    .map_err(|error| error_code(&error))?,
            }
        }
        "NO_REFUND" => RefundDestination::NoRefund,
        _ => {
            return Err(error_code(&ApplicationError::Validation(
                DomainError::InvalidCanonicalString("INVALID_REFUND_METHOD".to_string()),
            )));
        }
    };
    let result = ProcessSaleReturnUseCase::new(
        state.documents.as_ref(),
        state.returns.as_ref(),
        state.customers.as_ref(),
        state.cash.as_ref(),
    )
    .execute(
        &session,
        ProcessReturnCommand {
            operator_id: session.user_id,
            source_document_id: source_id,
            items,
            refund,
            idempotency_key: Uuid::new_v4().to_string(),
            terminal_id: state.terminal_id.clone(),
            notes: request.notes,
        },
    )
    .map_err(|error| return_wire_code(&error))?;
    Ok(DocumentsProcessReturnResponseDto {
        return_document_id: result.return_document_id.to_string(),
        return_document_number: result.return_document_number,
        total_refund_ttc_centimes: result.total_refund_ttc.centimes().to_string(),
        refund_method: method,
    })
}
