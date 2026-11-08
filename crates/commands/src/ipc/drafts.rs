//! Draft commands: parking, discarding, and full sale reads.
use super::sales::parse_document_language;
use super::{error_code, parse_wire_id, parse_wire_int, require_session, AppState};
use baraka_application::use_cases::sales::{
    CartLineInput, CreateDraftSaleUseCase, DiscardDraftSaleUseCase, DraftInput, GetSaleUseCase,
};
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::inventory::WarehouseId;

use crate::dto::{
    CreateDraftSaleRequest, DocumentDto, DocumentItemDto, DraftSaleResponse, GetSaleResponse,
    PaymentDto,
};

/// `sales_create_draft` command (`sales.create_draft`): park an editable draft.
#[tauri::command(rename_all = "snake_case")]
pub fn sales_create_draft(
    state: tauri::State<'_, AppState>,
    request: CreateDraftSaleRequest,
) -> Result<DraftSaleResponse, String> {
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
    let draft_id = CreateDraftSaleUseCase::new(
        state.catalog.as_ref(),
        state.documents.as_ref(),
        state.warehouses.as_ref(),
        state.customers.as_ref(),
        state.clock.as_ref(),
    )
    .execute(
        &session,
        DraftInput {
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
            language: parse_document_language(&request.document_language)
                .map_err(|error| error_code(&error))?,
            lines,
            notes: request.notes,
        },
        &state.terminal_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(DraftSaleResponse {
        draft_id: draft_id.to_string(),
        created_at: state.clock.now_rfc3339(),
    })
}

/// `sales_discard_draft` command (`sales.discard_draft`): discard a parked draft.
#[derive(Debug, serde::Deserialize)]
pub struct DiscardDraftRequestDto {
    pub session_token: String,
    pub draft_id: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn sales_discard_draft(
    state: tauri::State<'_, AppState>,
    request: DiscardDraftRequestDto,
) -> Result<(), String> {
    let session = require_session(&state, &request.session_token)?;
    DiscardDraftSaleUseCase::new(state.documents.as_ref())
        .execute(&session, &request.draft_id)
        .map_err(|error| error_code(&error))
}

/// Display transcription of the stored per-unit discount (DSN-097):
/// exact `unit_price - per_unit`, `None` when the line predates migration
/// 0018 (legacy display keeps the raw unit price). Not kernel math — the
/// engine validated `per_unit <= unit_price` at commit time; the fallback
/// only keeps a corrupt row from printing a misleading ticket.
pub(crate) fn discounted_unit_price_text(
    unit_price_centimes: i64,
    discount_per_unit_centimes: Option<i64>,
) -> Option<String> {
    discount_per_unit_centimes.map(|discount| {
        unit_price_centimes
            .checked_sub(discount)
            .unwrap_or(unit_price_centimes)
            .to_string()
    })
}

/// `sales_get` command (`sales.get`): full sale view.
#[tauri::command(rename_all = "snake_case")]
pub fn sales_get(
    state: tauri::State<'_, AppState>,
    session_token: String,
    document_id: String,
) -> Result<GetSaleResponse, String> {
    let session = require_session(&state, &session_token)?;
    let id = parse_wire_id(&document_id, "document_id")
        .map(baraka_domain::sales::DocumentId::from)
        .map_err(|error| error_code(&error))?;
    let detail = GetSaleUseCase::new(state.documents.as_ref())
        .execute(&session, id)
        .map_err(|error| error_code(&error))?;
    Ok(GetSaleResponse {
        document: DocumentDto {
            document_id: detail.document.id.to_string(),
            number: detail.document.number.clone(),
            doc_type: detail.document.doc_type.code().to_string(),
            status: detail.document.status.code().to_string(),
            warehouse_id: detail.document.warehouse_id.to_string(),
            subtotal_ht_centimes: detail.document.subtotal_ht_centimes.to_string(),
            global_discount_centimes: detail.document.global_discount_centimes.to_string(),
            tva_total_centimes: detail.document.vat_total_centimes.to_string(),
            total_ttc_centimes: detail.document.ttc_centimes.to_string(),
            paid_amount_centimes: detail.document.paid_amount_centimes.to_string(),
            change_amount_centimes: detail.document.change_amount_centimes.to_string(),
            created_at: detail
                .document
                .created_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            created_by: detail.document.created_by.to_string(),
            payment_status: match detail.payment_status {
                baraka_domain::sales::PaymentStatus::Paid => "PAID",
                baraka_domain::sales::PaymentStatus::Partial => "PARTIAL",
                baraka_domain::sales::PaymentStatus::Unpaid => "UNPAID",
            }
            .to_string(),
        },
        items: detail
            .items_view
            .iter()
            .map(|view| DocumentItemDto {
                product_id: view.item.product_id.to_string(),
                unit_id: view.item.unit_id.to_string(),
                quantity_micro: view.item.quantity_micro.to_string(),
                unit_price_centimes: view.item.unit_price_centimes.to_string(),
                discount_per_unit_centimes: view
                    .item
                    .discount_per_unit_centimes
                    .map(|discount| discount.to_string()),
                discounted_unit_price_centimes: discounted_unit_price_text(
                    view.item.unit_price_centimes,
                    view.item.discount_per_unit_centimes,
                ),
                line_tva_centimes: view.item.line_tva_centimes.to_string(),
                line_ttc_centimes: view.item.line_ttc_centimes.to_string(),
                product_name_fr: view.product_name_fr.clone(),
                product_name_ar: view.product_name_ar.clone(),
                sku: view.sku.clone(),
                unit_name_fr: view.unit_name_fr.clone(),
                unit_name_ar: view.unit_name_ar.clone(),
                product_missing: view.product_missing,
            })
            .collect(),
        payments: detail
            .payments
            .iter()
            .map(|payment| PaymentDto {
                method_code: payment.method_code.clone(),
                amount_centimes: payment.amount_centimes.to_string(),
            })
            .collect(),
        print_status: detail.print_status,
    })
}

/// Parse an optional wire integer (`None` stays `None`).
pub(crate) fn parse_optional_int(
    text: Option<&str>,
    field: &'static str,
) -> Result<Option<i64>, String> {
    text.map(|value| parse_wire_int(value, field))
        .transpose()
        .map_err(|error| error_code(&error))
}
