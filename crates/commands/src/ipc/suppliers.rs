//! Supplier commands: directory, statements, purchase documents, payments.
use super::{error_code, parse_wire_id, parse_wire_int, require_session, AppState};
use baraka_application::ports::{NewSupplier, PurchaseLineInput, SupplierUpdate};
use baraka_application::use_cases::deletion::DeleteSupplierUseCase;
use baraka_application::use_cases::purchases::{
    CreatePurchaseOrderUseCase, GetPurchaseDocumentUseCase, ReceiveGoodsUseCase,
    RecordSupplierInvoiceUseCase, RecordSupplierPaymentUseCase, UpdatePurchaseOrderUseCase,
};
use baraka_application::use_cases::suppliers::{
    CreateSupplierUseCase, GetSupplierStatementUseCase, GetSupplierUseCase, ListSuppliersUseCase,
    SearchSuppliersUseCase, UpdateSupplierUseCase,
};
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::inventory::WarehouseId;

use crate::dto::{
    CreatePurchaseOrderRequestDto, CreateSupplierRequestDto, DeleteEntityRequestDto,
    DeleteEntityResultDto, PurchaseLineRequestDto, PurchaseOrderDto, PurchaseOrderLineDto,
    ReceiveGoodsRequestDto, RecordSupplierInvoiceRequestDto, RecordSupplierPaymentRequestDto,
    SearchSuppliersRequestDto, SearchSuppliersResponseDto, SupplierDto, SupplierIdRequestDto,
    SupplierLedgerEntryDto, SupplierPaymentOutcomeDto, SupplierStatementDto,
    UpdatePurchaseOrderRequestDto, UpdateSupplierRequestDto,
};

fn to_supplier_dto(view: &baraka_application::ports::SupplierView) -> SupplierDto {
    SupplierDto {
        id: view.id.clone(),
        name: view.name.clone(),
        phone: view.phone.clone(),
        address: view.address.clone(),
        nif: view.nif.clone(),
        rc: view.rc.clone(),
        notes: view.notes.clone(),
        is_active: view.is_active,
        current_balance_centimes: view.current_balance_centimes.to_string(),
        origin_device_id: view.origin_device_id.clone(),
        created_at: view.created_at.clone(),
        updated_at: view.updated_at.clone(),
    }
}

fn parse_supplier_id(text: &str) -> Result<String, String> {
    parse_wire_id(text, "supplier_id")
        .map(|id| id.to_string())
        .map_err(|error| error_code(&error))
}

fn parse_purchase_lines(
    lines: &[PurchaseLineRequestDto],
) -> Result<Vec<PurchaseLineInput>, String> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        out.push(PurchaseLineInput {
            product_id: parse_wire_id(&line.product_id, "product_id")
                .map(ProductId::from)
                .map_err(|error| error_code(&error))?,
            unit_id: parse_wire_id(&line.unit_id, "unit_id")
                .map(UnitId::from)
                .map_err(|error| error_code(&error))?,
            quantity_micro: parse_wire_int(&line.quantity_micro, "quantity_micro")
                .map_err(|error| error_code(&error))?,
            unit_cost_centimes: parse_wire_int(&line.unit_cost_centimes, "unit_cost_centimes")
                .map_err(|error| error_code(&error))?,
        });
    }
    Ok(out)
}

fn to_purchase_order_dto(
    view: &baraka_application::ports::PurchaseDocumentView,
) -> PurchaseOrderDto {
    PurchaseOrderDto {
        id: view.id.clone(),
        number: view.number.clone(),
        doc_type: view.doc_type.clone(),
        status: view.status.clone(),
        supplier_id: view.supplier_id.clone(),
        warehouse_id: view.warehouse_id.clone(),
        source_document_id: view.source_document_id.clone(),
        total_centimes: view.total_centimes.to_string(),
        lines: view
            .lines
            .iter()
            .map(|line| PurchaseOrderLineDto {
                id: line.id.clone(),
                product_id: line.product_id.clone(),
                unit_id: line.unit_id.clone(),
                quantity_micro: line.quantity_micro.to_string(),
                quantity_base_micro: line.quantity_base_micro.to_string(),
                unit_cost_centimes: line.unit_cost_centimes.to_string(),
                line_total_centimes: line.line_total_centimes.to_string(),
            })
            .collect(),
        created_at: view.created_at.clone(),
        created_by: view.created_by.clone(),
    }
}

/// `suppliers.create` command: register a supplier (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_create(
    state: tauri::State<'_, AppState>,
    request: CreateSupplierRequestDto,
) -> Result<SupplierDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let view = CreateSupplierUseCase::new(state.suppliers.as_ref())
        .execute(
            &session,
            NewSupplier {
                name: request.name,
                phone: request.phone,
                address: request.address,
                nif: request.nif,
                rc: request.rc,
                notes: request.notes,
            },
            &state.terminal_id,
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_supplier_dto(&view))
}

/// `suppliers.update` command: sparse supplier update (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_update(
    state: tauri::State<'_, AppState>,
    request: UpdateSupplierRequestDto,
) -> Result<SupplierDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let view = UpdateSupplierUseCase::new(state.suppliers.as_ref())
        .execute(
            &session,
            &supplier_id,
            SupplierUpdate {
                name: request.name,
                phone: request.phone,
                address: request.address,
                nif: request.nif,
                rc: request.rc,
                notes: request.notes,
                is_active: request.is_active,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(to_supplier_dto(&view))
}

/// `suppliers_delete` command: dual-track safe deletion (ADR-092).
/// Hard-purges unreferenced suppliers; deactivates historically referenced
/// ones. Gerant/Admin only.
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_delete(
    state: tauri::State<'_, AppState>,
    request: DeleteEntityRequestDto,
) -> Result<DeleteEntityResultDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.id)?;
    let outcome = DeleteSupplierUseCase::new(state.suppliers.as_ref())
        .execute(&session, &supplier_id, &request.reason)
        .map_err(|error| error_code(&error))?;
    Ok(DeleteEntityResultDto {
        id: outcome.id,
        mode: outcome.mode.to_string(),
        message: outcome.message.to_string(),
    })
}

/// `suppliers.get` command: full supplier view.
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_get(
    state: tauri::State<'_, AppState>,
    request: SupplierIdRequestDto,
) -> Result<SupplierDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.id)?;
    let view = GetSupplierUseCase::new(state.suppliers.as_ref())
        .execute(&session, &supplier_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_supplier_dto(&view))
}

/// `suppliers.search` command: directory search (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_search(
    state: tauri::State<'_, AppState>,
    request: SearchSuppliersRequestDto,
) -> Result<SearchSuppliersResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let limit = request.limit.unwrap_or(20) as usize;
    let views = SearchSuppliersUseCase::new(state.suppliers.as_ref())
        .execute(&session, &request.query, limit)
        .map_err(|error| error_code(&error))?;
    Ok(SearchSuppliersResponseDto {
        suppliers: views.iter().map(to_supplier_dto).collect(),
    })
}

/// `suppliers_list` command: directory listing (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<SearchSuppliersResponseDto, String> {
    let session = require_session(&state, &session_token)?;
    let views = ListSuppliersUseCase::new(state.suppliers.as_ref())
        .execute(&session, true)
        .map_err(|error| error_code(&error))?;
    Ok(SearchSuppliersResponseDto {
        suppliers: views.iter().map(to_supplier_dto).collect(),
    })
}

/// `suppliers.getStatement` command: ledger rows with running balances.
#[tauri::command(rename_all = "snake_case")]
pub fn suppliers_get_statement(
    state: tauri::State<'_, AppState>,
    request: SupplierIdRequestDto,
) -> Result<SupplierStatementDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.id)?;
    let statement = GetSupplierStatementUseCase::new(state.suppliers.as_ref())
        .execute(&session, &supplier_id)
        .map_err(|error| error_code(&error))?;
    Ok(SupplierStatementDto {
        entries: statement
            .entries
            .iter()
            .map(|entry| SupplierLedgerEntryDto {
                id: entry.id.clone(),
                entry_type: entry.entry_type.clone(),
                amount_centimes: entry.amount_centimes.to_string(),
                reference_type: entry.reference_type.clone(),
                reference_id: entry.reference_id.clone(),
                description: entry.description.clone(),
                created_at: entry.created_at.clone(),
                created_by: entry.created_by.clone(),
                balance_after_centimes: entry.balance_after_centimes.to_string(),
            })
            .collect(),
        current_balance_centimes: statement.current_balance_centimes.to_string(),
    })
}

/// `purchases.createOrder` command: DRAFT BC, zero stock/ledger effect.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_create_order(
    state: tauri::State<'_, AppState>,
    request: CreatePurchaseOrderRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let view = CreatePurchaseOrderUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.updateOrder` command: replace the line set of a DRAFT BC.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_update_order(
    state: tauri::State<'_, AppState>,
    request: UpdatePurchaseOrderRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let document_id = parse_wire_id(&request.document_id, "document_id")
        .map(|id| id.to_string())
        .map_err(|error| error_code(&error))?;
    let view = UpdatePurchaseOrderUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        &document_id,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.receiveGoods` command: BR commit + PURCHASE_IN stock effect.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_receive_goods(
    state: tauri::State<'_, AppState>,
    request: ReceiveGoodsRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let source_bc_id = request
        .source_bc_id
        .as_deref()
        .map(|text| {
            parse_wire_id(text, "source_bc_id")
                .map(|id| id.to_string())
                .map_err(|error| error_code(&error))
        })
        .transpose()?;
    let view = ReceiveGoodsUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        source_bc_id,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.recordInvoice` command: FF commit, exactly one ledger DEBIT.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_record_invoice(
    state: tauri::State<'_, AppState>,
    request: RecordSupplierInvoiceRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let lines = parse_purchase_lines(&request.lines)?;
    let source_br_id = request
        .source_br_id
        .as_deref()
        .map(|text| {
            parse_wire_id(text, "source_br_id")
                .map(|id| id.to_string())
                .map_err(|error| error_code(&error))
        })
        .transpose()?;
    let view = RecordSupplierInvoiceUseCase::new(
        state.purchases.as_ref(),
        state.suppliers.as_ref(),
        state.warehouses.as_ref(),
    )
    .execute(
        &session,
        source_br_id,
        &supplier_id,
        parse_wire_id(&request.warehouse_id, "warehouse_id")
            .map(WarehouseId::from)
            .map_err(|error| error_code(&error))?,
        lines,
        parse_wire_int(&request.invoice_total_centimes, "invoice_total_centimes")
            .map_err(|error| error_code(&error))?,
        &state.terminal_id,
        &request.command_id,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}

/// `purchases.recordPayment` command: ledger CREDIT + optional drawer out.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_record_payment(
    state: tauri::State<'_, AppState>,
    request: RecordSupplierPaymentRequestDto,
) -> Result<SupplierPaymentOutcomeDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let supplier_id = parse_supplier_id(&request.supplier_id)?;
    let outcome =
        RecordSupplierPaymentUseCase::new(state.purchases.as_ref(), state.suppliers.as_ref())
            .execute(
                &session,
                &supplier_id,
                parse_wire_int(&request.amount_centimes, "amount_centimes")
                    .map_err(|error| error_code(&error))?,
                &request.payment_method,
                request.description,
                &state.terminal_id,
                &request.command_id,
            )
            .map_err(|error| error_code(&error))?;
    Ok(SupplierPaymentOutcomeDto {
        ledger_entry_id: outcome.ledger_entry_id,
        new_balance_centimes: outcome.new_balance_centimes.to_string(),
        cash_movement_id: outcome.cash_movement_id,
    })
}

/// `purchases.getDocument` command: load any BC/BR/FF with its lines.
#[tauri::command(rename_all = "snake_case")]
pub fn purchases_get_document(
    state: tauri::State<'_, AppState>,
    request: SupplierIdRequestDto,
) -> Result<PurchaseOrderDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let document_id = parse_wire_id(&request.id, "document_id")
        .map(|id| id.to_string())
        .map_err(|error| error_code(&error))?;
    let view = GetPurchaseDocumentUseCase::new(state.purchases.as_ref())
        .execute(&session, &document_id)
        .map_err(|error| error_code(&error))?;
    Ok(to_purchase_order_dto(&view))
}
