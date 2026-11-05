//! Sale commands: checkout, previews, conversions, document search.
use super::{
    error_code, parse_cart_line_dto, parse_payment_dto, parse_wire_id, parse_wire_int,
    require_session, AppState,
};
use baraka_application::errors::ApplicationError;
use baraka_application::ports::DocumentSearchFilter;
use baraka_application::use_cases::sales::{
    CartLineInput, CheckoutInput, CheckoutSaleUseCase, ClosingInput, ClosingPaymentInput,
    ConvertBlToFactureUseCase, ConvertDevisToBlInput, ConvertDevisToBlUseCase, PaymentInput,
    PreviewSaleTotalsUseCase, PreviewTotalsInput, SearchDocumentsUseCase, StockShortageWarning,
};
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;
use baraka_domain::sales::{CustomerId, DocumentId, DocumentLanguage, DocumentType, PaymentMethod};

use crate::dto::{
    CheckoutSaleRequest, CheckoutSaleResponse, ConvertBlToFactureRequest,
    ConvertBlToFactureResponse, ConvertDevisToBlRequest, ConvertDevisToBlResponse,
    DocumentSearchRequest, DocumentSearchResponse, DocumentSummaryDto, LineMarginDto,
    PreviewTotalsRequest, PreviewTotalsResponse, StockShortageWarningDto,
};

/// Parse an explicit sale-time language (`AR` or `FR` exactly).
///
/// Missing, blank, or foreign values are hard validation failures, never
/// silent fallbacks (Section 17: the language is captured, not assumed).
pub(crate) fn parse_document_language(text: &str) -> Result<DocumentLanguage, ApplicationError> {
    text.parse::<DocumentLanguage>()
        .map_err(|_| ApplicationError::Validation(DomainError::InvalidDocumentLanguage))
}

/// `sales_checkout` command (`sales.checkout`): atomic sale commit.
#[tauri::command(rename_all = "snake_case")]
pub fn sales_checkout(
    state: tauri::State<'_, AppState>,
    request: CheckoutSaleRequest,
) -> Result<CheckoutSaleResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut lines = Vec::with_capacity(request.lines.len());
    for line in &request.lines {
        lines.push(CartLineInput {
            product_id: parse_wire_id(&line.product_id, "product_id")
                .map(ProductId::from)
                .map_err(|error| error_code(&error))?,
            unit_id: parse_wire_id(&line.unit_id, "unit_id")
                .map(UnitId::from)
                .map_err(|error| error_code(&error))?,
            quantity_micro: parse_wire_int(&line.quantity_micro, "quantity_micro")
                .map_err(|error| error_code(&error))?,
            unit_price_override: line
                .unit_price_centimes
                .as_deref()
                .map(|text| parse_wire_int(text, "unit_price_centimes"))
                .transpose()
                .map_err(|error| error_code(&error))?,
            line_discount_centimes: parse_wire_int(
                &line.line_discount_centimes,
                "line_discount_centimes",
            )
            .map_err(|error| error_code(&error))?,
        });
    }
    let mut payments = Vec::with_capacity(request.payments.len());
    for payment in &request.payments {
        payments.push(PaymentInput {
            method: PaymentMethod::from_code(&payment.payment_method)
                .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
            amount_centimes: parse_wire_int(&payment.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
        });
    }
    let input = CheckoutInput {
        command_id: request.command_id,
        warehouse_id: parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        customer_id: request
            .customer_id
            .as_deref()
            .map(|text| {
                parse_wire_id(text, "customer_id").map(baraka_domain::sales::CustomerId::from)
            })
            .transpose()
            .map_err(|error| error_code(&error))?,
        doc_type: DocumentType::from_code(&request.document_type)
            .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
        language: parse_document_language(&request.document_language)
            .map_err(|error| error_code(&error))?,
        lines,
        global_discount_centimes: parse_wire_int(
            &request.global_discount_centimes,
            "global_discount_centimes",
        )
        .map_err(|error| error_code(&error))?,
        payments,
        allow_negative_stock: request.allow_negative_stock,
        notes: request.notes,
        timbre_enabled: state.fiscal.timbre_enabled().unwrap_or(true),
    };
    let receipt = CheckoutSaleUseCase::new(
        state.catalog.as_ref(),
        state.customers.as_ref(),
        state.checkout.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(&session, input, &state.terminal_id)
    .map_err(|error| error_code(&error))?;
    Ok(CheckoutSaleResponse {
        document_id: receipt.document_id,
        document_number: receipt.document_number,
        subtotal_ht_centimes: receipt.subtotal_ht_centimes,
        tva_total_centimes: receipt.tva_total_centimes,
        discount_total_centimes: receipt.discount_total_centimes,
        timbre_centimes: receipt.timbre_centimes,
        total_ttc_centimes: receipt.total_ttc_centimes,
        paid_amount_centimes: receipt.paid_amount_centimes,
        change_amount_centimes: receipt.change_amount_centimes,
        print_job_id: receipt.print_job_id,
    })
}

/// Map one application shortage warning to its wire DTO.
fn to_shortage_dto(warning: &StockShortageWarning) -> StockShortageWarningDto {
    StockShortageWarningDto {
        product_id: warning.product_id.to_string(),
        product_name: warning.product_name.clone(),
        sku: warning.sku.clone(),
        available_qty_micro: warning.available_qty_micro.to_string(),
        requested_qty_micro: warning.requested_qty_micro.to_string(),
    }
}

/// `sales_preview_totals` command (`sales.preview_totals`): read-only,
/// commit-identical totals preview (TASK-BRK-031).
///
/// Prices the current cart through the exact shared path the atomic
/// `sales_checkout` commit travels (`verify_and_price_cart` → domain
/// `price_document`) with zero persistence, sequence, stock, ledger, or
/// print side effects. The checkout modal renders ONLY these values; it
/// never recomputes totals client-side, so the figures the cashier
/// confirms before payment are byte-for-byte the figures persisted.
///
/// TASK-BRK-033 additionally surfaces leaf-level stock shortages against
/// the requested warehouse (advisory: the atomic engine re-checks under
/// its own transaction with role-gated enforcement).
#[tauri::command(rename_all = "snake_case")]
pub fn sales_preview_totals(
    state: tauri::State<'_, AppState>,
    request: PreviewTotalsRequest,
) -> Result<PreviewTotalsResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let mut lines = Vec::with_capacity(request.lines.len());
    for line in &request.lines {
        lines.push(parse_cart_line_dto(line).map_err(|error| error_code(&error))?);
    }
    let mut payments = Vec::with_capacity(request.payments.len());
    for payment in &request.payments {
        payments.push(parse_payment_dto(payment).map_err(|error| error_code(&error))?);
    }
    let output = PreviewSaleTotalsUseCase::new(
        state.catalog.as_ref(),
        state.stock.as_ref(),
        state.warehouses.as_ref(),
        state.customers.as_ref(),
    )
    .execute(
        &session,
        PreviewTotalsInput {
            warehouse_id: parse_wire_id(&request.warehouse_id, "warehouse_id")
                .map(WarehouseId::from)
                .map_err(|error| error_code(&error))?,
            doc_type: DocumentType::from_code(&request.document_type)
                .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
            lines,
            customer_id: request
                .customer_id
                .as_deref()
                .map(|text| {
                    parse_wire_id(text, "customer_id")
                        .map(CustomerId::from)
                        .map_err(|error| error_code(&error))
                })
                .transpose()?,
            global_discount_centimes: parse_wire_int(
                &request.global_discount_centimes,
                "global_discount_centimes",
            )
            .map_err(|error| error_code(&error))?,
            payments,
            timbre_enabled: state.fiscal.timbre_enabled().unwrap_or(true),
        },
    )
    .map_err(|error| error_code(&error))?;
    Ok(PreviewTotalsResponse {
        subtotal_ht_centimes: output.subtotal_ht.to_string(),
        tva_total_centimes: output.total_tva.to_string(),
        global_discount_centimes: output.global_discount.to_string(),
        timbre_centimes: output.timbre.to_string(),
        total_ttc_centimes: output.total_ttc.to_string(),
        paid_amount_centimes: output.paid_total.to_string(),
        change_amount_centimes: output.change.to_string(),
        remaining_due_centimes: output.remaining_due.to_string(),
        stock_warnings: output.stock_warnings.iter().map(to_shortage_dto).collect(),
        line_margins: output
            .line_margins
            .iter()
            .map(|margin| LineMarginDto {
                line_index: margin.line_index,
                margin_basis_points: margin.margin_basis_points,
                is_below_cost: margin.is_below_cost,
            })
            .collect(),
    })
}

/// `convert_devis_to_bl` command: single-shot DEVIS-to-BL conversion
/// (ADR-040, TASK-BRK-014A). Copies the locked quote snapshot into a new
/// FINALIZED BL, decrements stock, and seals the source as CONVERTED —
/// all atomically. No recalculation, no partial conversion.
#[tauri::command(rename_all = "snake_case")]
pub fn convert_devis_to_bl(
    state: tauri::State<'_, AppState>,
    request: ConvertDevisToBlRequest,
) -> Result<ConvertDevisToBlResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let result = ConvertDevisToBlUseCase::new(state.catalog.as_ref(), state.documents.as_ref())
        .execute(
            &session,
            ConvertDevisToBlInput {
                command_id: request.command_id,
                source_document_id: source_id,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(ConvertDevisToBlResponse {
        new_document_id: result.new_document_id,
        new_document_number: result.new_document_number,
        new_document_status: result.new_document_status,
        total_ttc_centimes: result.total_ttc_centimes.to_string(),
        items_count: result.items_count,
        warnings: result.warnings,
    })
}

/// `convert_bl_to_facture` command: single-shot BL-to-FACTURE financial
/// closing (ADR-040/041/046, TASK-BRK-038). Settles the locked BL snapshot
/// into a new FINALIZED FACTURE with exact tender settlement (no change
/// concept), posts the credit ledger leg when CREDIT tenders are present,
/// and seals the source as CONVERTED — all atomically. Zero stock writes
/// by invariant (physical stock left at BL finalization). Mirrors the
/// `convert_devis_to_bl` edge conventions: session extraction, wire-int
/// parsing, `command_id` idempotency threading, and `error_code` mapping.
#[tauri::command(rename_all = "snake_case")]
pub fn convert_bl_to_facture(
    state: tauri::State<'_, AppState>,
    request: ConvertBlToFactureRequest,
) -> Result<ConvertBlToFactureResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let source_id = parse_wire_id(&request.source_document_id, "source_document_id")
        .map(DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let mut payments = Vec::with_capacity(request.payments.len());
    for payment in &request.payments {
        payments.push(ClosingPaymentInput {
            method: PaymentMethod::from_code(&payment.payment_method)
                .map_err(|error| error_code(&ApplicationError::Validation(error)))?,
            amount_centimes: parse_wire_int(&payment.amount_centimes, "amount_centimes")
                .map_err(|error| error_code(&error))?,
        });
    }
    let result = ConvertBlToFactureUseCase::new(state.customers.as_ref(), state.documents.as_ref())
        .execute(
            &session,
            ClosingInput {
                command_id: request.command_id,
                source_document_id: source_id,
                language: parse_document_language(&request.document_language)
                    .map_err(|error| error_code(&error))?,
                payments,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(ConvertBlToFactureResponse {
        new_document_id: result.new_document_id,
        new_document_number: result.new_document_number,
        new_document_status: result.new_document_status,
        total_ttc_centimes: result.total_ttc_centimes.to_string(),
        items_count: result.items_count,
        customer_new_balance_centimes: result
            .customer_new_balance_centimes
            .map(|balance| balance.to_string()),
    })
}

/// `documents_search` command: read-only document lookup for conversion
/// candidate flows (TASK-BRK-042). Filters by kind/status with partial
/// human-number or customer-name match, newest first, limit clamped
/// to 1..=100. Any operator who may sell or report may search.
#[tauri::command(rename_all = "snake_case")]
pub fn documents_search(
    state: tauri::State<'_, AppState>,
    request: DocumentSearchRequest,
) -> Result<DocumentSearchResponse, String> {
    let session = require_session(&state, &request.session_token)?;
    let summaries = SearchDocumentsUseCase::new(state.documents.as_ref())
        .execute(
            &session,
            DocumentSearchFilter {
                doc_type: request.document_type,
                status: request.status,
                query: request.query,
                limit: request.limit,
            },
        )
        .map_err(|error| error_code(&error))?;
    let mut documents = Vec::with_capacity(summaries.len());
    for summary in summaries {
        documents.push(DocumentSummaryDto {
            id: summary.id,
            document_number: summary.number,
            document_type: summary.doc_type,
            status: summary.status,
            party_id: summary.party_id,
            party_name: summary.party_name,
            total_ttc_centimes: summary.ttc_centimes.to_string(),
            items_count: u32::try_from(summary.items_count).map_err(|_| {
                error_code(&ApplicationError::Port("line count overflow".to_string()))
            })?,
            created_at: summary.created_at,
        });
    }
    Ok(DocumentSearchResponse { documents })
}
