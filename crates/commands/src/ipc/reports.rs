//! Report commands: sales aggregates, margins, credit, stock, activity.
use super::{error_code, require_session, AppState};
use baraka_application::use_cases::reports::{
    GetCashSessionsHistoryUseCase, GetCreditOutstandingReportUseCase, GetDailySalesSummaryUseCase,
    GetGrossMarginReportUseCase, GetLowStockReportUseCase, GetSalesByCategoryUseCase,
    GetSalesByProductUseCase, GetStockMovementHistoryUseCase, GetUserActivityLogUseCase,
};

use crate::dto::{
    CashSessionHistoryRowDto, CashSessionsHistoryRequestDto, CreditOutstandingRowDto,
    DailySalesSummaryDto, GrossMarginRowDto, LowStockRequestDto, LowStockRowDto,
    ReportDateRangeRequestDto, SalesByCategoryRowDto, SalesByProductRowDto,
    StockMovementHistoryRequestDto, StockMovementRowDto, UserActivityRequestDto,
    UserActivityRowDto,
};

fn to_daily_sales_dto(row: &baraka_application::ports::DailySalesRow) -> DailySalesSummaryDto {
    DailySalesSummaryDto {
        day: row.day.clone(),
        total_ttc_centimes: row.total_ttc_centimes.to_string(),
        total_ht_centimes: row.total_ht_centimes.to_string(),
        total_tva_centimes: row.total_tva_centimes.to_string(),
        document_count: row.document_count.to_string(),
        average_basket_centimes: row.average_basket_centimes.to_string(),
    }
}

/// `reports.dailySales` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_daily_sales(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<DailySalesSummaryDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetDailySalesSummaryUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows.iter().map(to_daily_sales_dto).collect())
}

/// `reports.salesByProduct` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_sales_by_product(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<SalesByProductRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetSalesByProductUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| SalesByProductRowDto {
            product_id: row.product_id.clone(),
            sku: row.sku.clone(),
            name_fr: row.name_fr.clone(),
            quantity_sold_base_micro: row.quantity_sold_base_micro.to_string(),
            revenue_ttc_centimes: row.revenue_ttc_centimes.to_string(),
            line_count: row.line_count.to_string(),
        })
        .collect())
}

/// `reports.salesByCategory` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_sales_by_category(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<SalesByCategoryRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetSalesByCategoryUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| SalesByCategoryRowDto {
            category_id: row.category_id.clone(),
            category_code: row.category_code.clone(),
            name_fr: row.name_fr.clone(),
            revenue_ttc_centimes: row.revenue_ttc_centimes.to_string(),
            line_count: row.line_count.to_string(),
        })
        .collect())
}

/// `reports.grossMargin` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_gross_margin(
    state: tauri::State<'_, AppState>,
    request: ReportDateRangeRequestDto,
) -> Result<Vec<GrossMarginRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetGrossMarginReportUseCase::new(state.reports.as_ref())
        .execute(&session, &request.date_from, &request.date_to)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| {
            // Display-only percentage in basis points, half away from zero;
            // "0" when revenue is zero (no division possible).
            let percent_bp = if row.revenue_ht_centimes == 0 {
                0i64
            } else {
                let numerator = i128::from(row.margin_centimes) * 10_000;
                let denominator = i128::from(row.revenue_ht_centimes);
                let rounded = if numerator >= 0 {
                    (numerator + denominator / 2) / denominator
                } else {
                    -((-numerator + denominator / 2) / denominator)
                };
                i64::try_from(rounded).unwrap_or(0)
            };
            GrossMarginRowDto {
                product_id: row.product_id.clone(),
                sku: row.sku.clone(),
                revenue_ht_centimes: row.revenue_ht_centimes.to_string(),
                cost_total_centimes: row.cost_total_centimes.to_string(),
                margin_centimes: row.margin_centimes.to_string(),
                margin_percent_bp: percent_bp.to_string(),
                has_untracked_cost: row.has_untracked_cost,
            }
        })
        .collect())
}

/// `reports.creditOutstanding` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_credit_outstanding(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<CreditOutstandingRowDto>, String> {
    let session = require_session(&state, &session_token)?;
    let rows = GetCreditOutstandingReportUseCase::new(state.reports.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| {
            // Display-only consumed-limit percentage in basis points;
            // None for UNLIMITED policies (no division possible).
            let used_bp = match (&row.credit_policy, row.credit_limit_centimes) {
                (policy, Some(limit)) if policy == "LIMITED" && limit > 0 => {
                    let numerator = i128::from(row.current_balance_centimes) * 10_000;
                    let rounded = (numerator + i128::from(limit) / 2) / i128::from(limit);
                    Some(i64::try_from(rounded).unwrap_or(0).to_string())
                }
                _ => None,
            };
            CreditOutstandingRowDto {
                customer_id: row.customer_id.clone(),
                name: row.name.clone(),
                phone: row.phone.clone(),
                credit_policy: row.credit_policy.clone(),
                credit_limit_centimes: row.credit_limit_centimes.map(|v| v.to_string()),
                current_balance_centimes: row.current_balance_centimes.to_string(),
                limit_used_bp: used_bp,
            }
        })
        .collect())
}

/// `reports.stockMovementHistory` command (Gerant/Admin, paginated).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_stock_movement_history(
    state: tauri::State<'_, AppState>,
    request: StockMovementHistoryRequestDto,
) -> Result<Vec<StockMovementRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetStockMovementHistoryUseCase::new(state.reports.as_ref())
        .execute(
            &session,
            request.warehouse_id.as_deref(),
            request.product_id.as_deref(),
            request.movement_type.as_deref(),
            &request.date_from,
            &request.date_to,
            request.limit.unwrap_or(50) as usize,
            request.offset.unwrap_or(0) as usize,
        )
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| StockMovementRowDto {
            id: row.id.clone(),
            movement_type: row.movement_type.clone(),
            warehouse_id: row.warehouse_id.clone(),
            product_id: row.product_id.clone(),
            quantity_base_micro: row.quantity_base_micro.to_string(),
            reference_type: row.reference_type.clone(),
            reference_id: row.reference_id.clone(),
            created_at: row.created_at.clone(),
            created_by: row.created_by.clone(),
        })
        .collect())
}

/// `reports.cashSessionsHistory` command (Gerant/Admin).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_cash_sessions_history(
    state: tauri::State<'_, AppState>,
    request: CashSessionsHistoryRequestDto,
) -> Result<Vec<CashSessionHistoryRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetCashSessionsHistoryUseCase::new(state.reports.as_ref())
        .execute(
            &session,
            request.terminal_id.as_deref(),
            &request.date_from,
            &request.date_to,
        )
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| CashSessionHistoryRowDto {
            id: row.id.clone(),
            terminal_id: row.terminal_id.clone(),
            status: row.status.clone(),
            opened_at: row.opened_at.clone(),
            closed_at: row.closed_at.clone(),
            opening_float_centimes: row.opening_float_centimes.to_string(),
            system_expected_centimes: row.system_expected_centimes.map(|v| v.to_string()),
            actual_counted_centimes: row.actual_counted_centimes.map(|v| v.to_string()),
            variance_centimes: row.variance_centimes.map(|v| v.to_string()),
        })
        .collect())
}

/// `reports.lowStock` command (Gerant/Admin, ADR-037 depletion signal).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_low_stock(
    state: tauri::State<'_, AppState>,
    request: LowStockRequestDto,
) -> Result<Vec<LowStockRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetLowStockReportUseCase::new(state.reports.as_ref())
        .execute(&session, request.warehouse_id.as_deref())
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| LowStockRowDto {
            product_id: row.product_id.clone(),
            sku: row.sku.clone(),
            name_fr: row.name_fr.clone(),
            warehouse_id: row.warehouse_id.clone(),
            warehouse_code: row.warehouse_code.clone(),
            quantity_base_micro: row.quantity_base_micro.to_string(),
        })
        .collect())
}

/// `reports.userActivity` command (Gerant/Admin, paginated, generic payload).
#[tauri::command(rename_all = "snake_case")]
pub fn reports_user_activity(
    state: tauri::State<'_, AppState>,
    request: UserActivityRequestDto,
) -> Result<Vec<UserActivityRowDto>, String> {
    let session = require_session(&state, &request.session_token)?;
    let rows = GetUserActivityLogUseCase::new(state.reports.as_ref())
        .execute(
            &session,
            request.user_id.as_deref(),
            request.action.as_deref(),
            &request.date_from,
            &request.date_to,
            request.limit.unwrap_or(50) as usize,
            request.offset.unwrap_or(0) as usize,
        )
        .map_err(|error| error_code(&error))?;
    Ok(rows
        .iter()
        .map(|row| UserActivityRowDto {
            sequence_no: row.sequence_no.to_string(),
            timestamp: row.timestamp.clone(),
            user_id: row.user_id.clone(),
            action: row.action.clone(),
            entity_type: row.entity_type.clone(),
            entity_id: row.entity_id.clone(),
            old_value_json: row.old_value_json.clone(),
            new_value_json: row.new_value_json.clone(),
        })
        .collect())
}
