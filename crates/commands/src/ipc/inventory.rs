//! Inventory commands: warehouses, stock levels, adjustments, transfers.
use super::{
    adjust_error_code, error_code, parse_wire_id, parse_wire_int, require_session, AppState,
};
use baraka_application::errors::ApplicationError;
use baraka_application::use_cases::deletion::{DeleteWarehouseUseCase, UpdateWarehouseUseCase};
use baraka_application::use_cases::inventory::{
    AdjustStockInput, AdjustStockUseCase, CreateWarehouseUseCase, GetStockLevelUseCase,
    ListWarehousesUseCase, StockDirection, TransferStockInput, TransferStockUseCase,
    WarehouseStockPresenceUseCase,
};
use baraka_domain::catalog::{ProductId, UnitId};
use baraka_domain::errors::DomainError;
use baraka_domain::inventory::WarehouseId;

use crate::dto::{
    AdjustStockRequestDto, AdjustStockResponseDto, CreateWarehouseRequestDto,
    DeleteEntityRequestDto, DeleteEntityResultDto, GetStockLevelRequestDto,
    GetStockLevelResponseDto, ListWarehousesResponseDto, TransferStockRequestDto,
    TransferStockResponseDto, UpdateWarehouseRequestDto, WarehouseDto, WarehouseStockPresenceDto,
    WarehouseStockPresenceResponseDto,
};

/// `warehouses_create` command (`warehouses.create`): ADMIN-only creation.
#[tauri::command(rename_all = "snake_case")]
pub fn warehouses_create(
    state: tauri::State<'_, AppState>,
    request: CreateWarehouseRequestDto,
) -> Result<WarehouseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = CreateWarehouseUseCase::new(state.warehouses.as_ref())
        .execute(&session, &request.code, &request.name, &state.terminal_id)
        .map_err(|error| error_code(&error))?;
    let warehouse = state
        .warehouses
        .get_by_id(id)
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| "INTERNAL_ERROR".to_string())?;
    Ok(WarehouseDto {
        id: warehouse.id.to_string(),
        code: warehouse.code.as_str().to_string(),
        name: warehouse.name.clone(),
        is_active: warehouse.is_active,
    })
}

/// `warehouses_list` command (`warehouses.list`): any authenticated session.
#[tauri::command(rename_all = "snake_case")]
pub fn warehouses_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<ListWarehousesResponseDto, String> {
    let session = require_session(&state, &session_token)?;
    let warehouses = ListWarehousesUseCase::new(state.warehouses.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(ListWarehousesResponseDto {
        warehouses: warehouses
            .into_iter()
            .map(|warehouse| WarehouseDto {
                id: warehouse.id.to_string(),
                code: warehouse.code.as_str().to_string(),
                name: warehouse.name.clone(),
                is_active: warehouse.is_active,
            })
            .collect(),
    })
}

/// `warehouses_update` command: sparse rename and/or (de)activation
/// (ADR-092, ADMIN only via `ManageWarehouses`; DEP-01 deactivation
/// rejected upstream).
#[tauri::command(rename_all = "snake_case")]
pub fn warehouses_update(
    state: tauri::State<'_, AppState>,
    request: UpdateWarehouseRequestDto,
) -> Result<WarehouseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.id, "id").map_err(|error| error_code(&error))?;
    UpdateWarehouseUseCase::new(state.warehouses.as_ref())
        .execute(
            &session,
            WarehouseId::from(id),
            request.name,
            request.is_active,
        )
        .map_err(|error| error_code(&error))?;
    let warehouse = state
        .warehouses
        .get_by_id(WarehouseId::from(id))
        .map_err(|error| error_code(&error))?
        .ok_or_else(|| "INTERNAL_ERROR".to_string())?;
    Ok(WarehouseDto {
        id: warehouse.id.to_string(),
        code: warehouse.code.as_str().to_string(),
        name: warehouse.name.clone(),
        is_active: warehouse.is_active,
    })
}

/// `warehouses_delete` command: dual-track safe deletion (ADR-092).
/// DEP-01 immutable; non-zero stock blocks; history deactivates.
#[tauri::command(rename_all = "snake_case")]
pub fn warehouses_delete(
    state: tauri::State<'_, AppState>,
    request: DeleteEntityRequestDto,
) -> Result<DeleteEntityResultDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let id = parse_wire_id(&request.id, "id").map_err(|error| error_code(&error))?;
    let outcome = DeleteWarehouseUseCase::new(state.warehouses.as_ref())
        .execute(&session, WarehouseId::from(id), &request.reason)
        .map_err(|error| error_code(&error))?;
    Ok(DeleteEntityResultDto {
        id: outcome.id,
        mode: outcome.mode.to_string(),
        message: outcome.message.to_string(),
    })
}

/// `warehouses_stock_presence` command (BRK-095): read-only non-zero
/// level counts for every active depot, code-ordered. Same
/// `ManageWarehouses` gate as the depot deletion path (enforced in the
/// use case); performs zero writes. The delete pre-check wire.
#[tauri::command(rename_all = "snake_case")]
pub fn warehouses_stock_presence(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<WarehouseStockPresenceResponseDto, String> {
    let session = require_session(&state, &session_token)?;
    let presence = WarehouseStockPresenceUseCase::new(state.warehouses.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(WarehouseStockPresenceResponseDto {
        presence: presence
            .into_iter()
            .map(|row| WarehouseStockPresenceDto {
                warehouse_id: row.warehouse_id.to_string(),
                nonzero_levels: row.nonzero_levels,
            })
            .collect(),
    })
}

/// `stock_get` command (`stock.get`): level read, 0 when absent.
#[tauri::command(rename_all = "snake_case")]
pub fn stock_get(
    state: tauri::State<'_, AppState>,
    request: GetStockLevelRequestDto,
) -> Result<GetStockLevelResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let product_id = parse_wire_id(&request.product_id, "product_id")
        .map(ProductId::from)
        .map_err(|error| error_code(&error))?;
    let warehouse_id = parse_wire_id(&request.warehouse_id, "warehouse_id")
        .map(WarehouseId::from)
        .map_err(|error| error_code(&error))?;
    let base_unit_id = state
        .catalog
        .find_by_id(product_id)
        .map_err(|error| error_code(&error))?
        .and_then(|(_, units)| {
            units
                .iter()
                .find(|unit| unit.is_base_unit)
                .map(|unit| unit.id.to_string())
        })
        .unwrap_or_default();
    let quantity = GetStockLevelUseCase::new(state.stock.as_ref())
        .execute(&session, product_id, warehouse_id)
        .map_err(|error| error_code(&error))?;
    Ok(GetStockLevelResponseDto {
        product_id: product_id.to_string(),
        warehouse_id: warehouse_id.to_string(),
        base_unit_id,
        quantity_base_micro: quantity.to_string(),
    })
}

/// Parse an explicit adjustment direction.
fn parse_direction(text: &str) -> Result<StockDirection, ApplicationError> {
    match text.trim() {
        "INCREASE" => Ok(StockDirection::Increase),
        "DECREASE" => Ok(StockDirection::Decrease),
        _ => Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString(
                "direction must be INCREASE or DECREASE".to_string(),
            ),
        )),
    }
}

/// `stock_adjust` command (`stock.adjust`): RBAC-gated manual correction.
#[tauri::command(rename_all = "snake_case")]
pub fn stock_adjust(
    state: tauri::State<'_, AppState>,
    request: AdjustStockRequestDto,
) -> Result<AdjustStockResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let outcome = AdjustStockUseCase::new(state.stock.as_ref())
        .execute(
            &session,
            AdjustStockInput {
                product_id: parse_wire_id(&request.product_id, "product_id")
                    .map(ProductId::from)
                    .map_err(|error| error_code(&error))?,
                warehouse_id: parse_wire_id(&request.warehouse_id, "warehouse_id")
                    .map(WarehouseId::from)
                    .map_err(|error| error_code(&error))?,
                unit_id: parse_wire_id(&request.unit_id, "unit_id")
                    .map(UnitId::from)
                    .map_err(|error| error_code(&error))?,
                quantity_micro: parse_wire_int(&request.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
                direction: parse_direction(&request.direction)
                    .map_err(|error| error_code(&error))?,
                reason: request.reason,
                authorize_negative_stock: request.authorize_negative_stock,
            },
        )
        .map_err(|error| adjust_error_code(&error))?;
    Ok(AdjustStockResponseDto {
        movement_id: outcome.movement_id,
        new_quantity_base_micro: outcome.new_quantity_base_micro.to_string(),
    })
}

/// `stock_transfer` command (`stock.transfer`): atomic OUT/IN pair.
#[tauri::command(rename_all = "snake_case")]
pub fn stock_transfer(
    state: tauri::State<'_, AppState>,
    request: TransferStockRequestDto,
) -> Result<TransferStockResponseDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let result = TransferStockUseCase::new(state.stock.as_ref(), state.warehouses.as_ref())
        .execute(
            &session,
            TransferStockInput {
                source_warehouse_id: parse_wire_id(
                    &request.source_warehouse_id,
                    "source_warehouse_id",
                )
                .map(WarehouseId::from)
                .map_err(|error| error_code(&error))?,
                destination_warehouse_id: parse_wire_id(
                    &request.destination_warehouse_id,
                    "destination_warehouse_id",
                )
                .map(WarehouseId::from)
                .map_err(|error| error_code(&error))?,
                product_id: parse_wire_id(&request.product_id, "product_id")
                    .map(ProductId::from)
                    .map_err(|error| error_code(&error))?,
                unit_id: parse_wire_id(&request.unit_id, "unit_id")
                    .map(UnitId::from)
                    .map_err(|error| error_code(&error))?,
                quantity_micro: parse_wire_int(&request.quantity_micro, "quantity_micro")
                    .map_err(|error| error_code(&error))?,
                reason: request.reason,
            },
        )
        .map_err(|error| error_code(&error))?;
    Ok(TransferStockResponseDto {
        transfer_batch_id: result.transfer_batch_id,
        out_movement_id: result.out_movement_id,
        in_movement_id: result.in_movement_id,
        source_new_quantity_base_micro: result.source_new_quantity_base_micro.to_string(),
        destination_new_quantity_base_micro: result.destination_new_quantity_base_micro.to_string(),
    })
}
