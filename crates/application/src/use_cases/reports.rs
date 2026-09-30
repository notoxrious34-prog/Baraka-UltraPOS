//! Read-only report use cases (TASK-BRK-012).
//!
//! Every command requires `ReportsView` (Gerant/Admin only) and validates
//! its date range (`date_from <= date_to`, ISO `YYYY-MM-DD`) before touching
//! the repository. No writes, no idempotency keys, no new tables.

use baraka_domain::errors::DomainError;
use baraka_domain::role::Permission;

use crate::errors::ApplicationError;
use crate::ports::{
    CashSessionHistoryRow, CreditOutstandingRow, DailySalesRow, GrossMarginRow, LowStockRow,
    ReportingRepository, SalesByCategoryRow, SalesByProductRow, SessionContext,
    StockMovementHistoryRow, UserActivityRow,
};

/// Default page size for paginated history reports.
pub const DEFAULT_PAGE_SIZE: usize = 50;
/// Maximum page size for paginated history reports.
///
/// TASK-BRK-012A mandates rejection (not clamping) above this bound:
/// `resolve_limit` maps 0 to the default and errors on anything larger.
pub const MAX_PAGE_SIZE: usize = 200;

/// Validate an ISO `YYYY-MM-DD` calendar date (lexicographic compare is
/// exact for this fixed-width format).
fn validate_day(value: &str, field: &'static str) -> Result<String, ApplicationError> {
    let trimmed = value.trim().to_string();
    if trimmed.len() != 10 {
        return Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString(format!("{field} must be YYYY-MM-DD")),
        ));
    }
    let bytes = trimmed.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || !trimmed[..4].bytes().all(|b| b.is_ascii_digit())
        || !trimmed[5..7].bytes().all(|b| b.is_ascii_digit())
        || !trimmed[8..10].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(ApplicationError::Validation(
            DomainError::InvalidCanonicalString(format!("{field} must be YYYY-MM-DD")),
        ));
    }
    Ok(trimmed)
}

/// Validate a closed date range; rejects `from > to` with InvalidDateRange.
fn validate_range(date_from: &str, date_to: &str) -> Result<(String, String), ApplicationError> {
    let from = validate_day(date_from, "date_from")?;
    let to = validate_day(date_to, "date_to")?;
    if from > to {
        return Err(ApplicationError::Validation(DomainError::InvalidDateRange));
    }
    Ok((from, to))
}

fn require_reports(session: &SessionContext) -> Result<(), ApplicationError> {
    if session.role.has_permission(Permission::ReportsView) {
        Ok(())
    } else {
        Err(ApplicationError::Unauthorized)
    }
}

/// Resolve a requested page size: 0 selects the default; anything above
/// MAX_PAGE_SIZE is rejected with ReportPaginationLimitExceeded.
fn resolve_limit(limit: usize) -> Result<usize, ApplicationError> {
    if limit == 0 {
        Ok(DEFAULT_PAGE_SIZE)
    } else if limit > MAX_PAGE_SIZE {
        Err(ApplicationError::Validation(
            DomainError::ReportPaginationLimitExceeded,
        ))
    } else {
        Ok(limit)
    }
}

/// Daily sales aggregates for committed sale documents in range.
pub struct GetDailySalesSummaryUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetDailySalesSummaryUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// Aggregate committed TICKET/FACTURE/BL documents by calendar day.
    pub fn execute(
        &self,
        session: &SessionContext,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<DailySalesRow>, ApplicationError> {
        require_reports(session)?;
        let (from, to) = validate_range(date_from, date_to)?;
        self.reports.daily_sales(&from, &to)
    }
}

/// Per-product sales aggregates in range, revenue descending.
pub struct GetSalesByProductUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetSalesByProductUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// Aggregate committed sale lines by product.
    pub fn execute(
        &self,
        session: &SessionContext,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<SalesByProductRow>, ApplicationError> {
        require_reports(session)?;
        let (from, to) = validate_range(date_from, date_to)?;
        self.reports.sales_by_product(&from, &to)
    }
}

/// Per-category sales aggregates in range, revenue descending.
pub struct GetSalesByCategoryUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetSalesByCategoryUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// Aggregate committed sale lines by category (`None` = uncategorized).
    pub fn execute(
        &self,
        session: &SessionContext,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<SalesByCategoryRow>, ApplicationError> {
        require_reports(session)?;
        let (from, to) = validate_range(date_from, date_to)?;
        self.reports.sales_by_category(&from, &to)
    }
}

/// Per-product gross margin in range.
pub struct GetGrossMarginReportUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetGrossMarginReportUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// Revenue HT minus selling-unit cost per product (no exclusions:
    /// cost_price_centimes is NOT NULL on every committed row).
    pub fn execute(
        &self,
        session: &SessionContext,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<GrossMarginRow>, ApplicationError> {
        require_reports(session)?;
        let (from, to) = validate_range(date_from, date_to)?;
        self.reports.gross_margin(&from, &to)
    }
}

/// Outstanding customer credit balances.
pub struct GetCreditOutstandingReportUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetCreditOutstandingReportUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// Non-DISABLED customers with positive balance, balance descending.
    /// Drill down per row via the existing customer statement command.
    pub fn execute(
        &self,
        session: &SessionContext,
    ) -> Result<Vec<CreditOutstandingRow>, ApplicationError> {
        require_reports(session)?;
        self.reports.credit_outstanding()
    }
}

/// Filtered, paginated stock-movement history (newest first).
pub struct GetStockMovementHistoryUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetStockMovementHistoryUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// List movements matching the optional filters in range.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        warehouse_id: Option<&str>,
        product_id: Option<&str>,
        movement_type: Option<&str>,
        date_from: &str,
        date_to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StockMovementHistoryRow>, ApplicationError> {
        require_reports(session)?;
        let (from, to) = validate_range(date_from, date_to)?;
        self.reports.stock_movement_history(
            warehouse_id.map(str::trim).filter(|v| !v.is_empty()),
            product_id.map(str::trim).filter(|v| !v.is_empty()),
            movement_type.map(str::trim).filter(|v| !v.is_empty()),
            &from,
            &to,
            resolve_limit(limit)?,
            offset,
        )
    }
}

/// Low-stock (stockout/depletion) signal per warehouse (ADR-037).
pub struct GetLowStockReportUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetLowStockReportUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// Products with quantity_base_micro <= 0, most depleted first.
    pub fn execute(
        &self,
        session: &SessionContext,
        warehouse_id: Option<&str>,
    ) -> Result<Vec<LowStockRow>, ApplicationError> {
        require_reports(session)?;
        self.reports
            .low_stock(warehouse_id.map(str::trim).filter(|v| !v.is_empty()))
    }
}

/// Cash-session history in range (newest first).
pub struct GetCashSessionsHistoryUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetCashSessionsHistoryUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// List sessions in range; drill down per row via GetSessionReport.
    pub fn execute(
        &self,
        session: &SessionContext,
        terminal_id: Option<&str>,
        date_from: &str,
        date_to: &str,
    ) -> Result<Vec<CashSessionHistoryRow>, ApplicationError> {
        require_reports(session)?;
        let (from, to) = validate_range(date_from, date_to)?;
        self.reports.cash_sessions_history(
            terminal_id.map(str::trim).filter(|v| !v.is_empty()),
            &from,
            &to,
        )
    }
}

/// Filtered, paginated audit-log rows (newest first, generic payload).
pub struct GetUserActivityLogUseCase<'a, R>
where
    R: ?Sized,
{
    reports: &'a R,
}

impl<'a, R> GetUserActivityLogUseCase<'a, R>
where
    R: ReportingRepository + ?Sized,
{
    /// Wire the use case to its port.
    pub fn new(reports: &'a R) -> Self {
        Self { reports }
    }

    /// List audit entries matching the optional filters in range.
    #[allow(clippy::too_many_arguments)]
    pub fn execute(
        &self,
        session: &SessionContext,
        user_id: Option<&str>,
        action: Option<&str>,
        date_from: &str,
        date_to: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<UserActivityRow>, ApplicationError> {
        require_reports(session)?;
        let (from, to) = validate_range(date_from, date_to)?;
        self.reports.user_activity(
            user_id.map(str::trim).filter(|v| !v.is_empty()),
            action.map(str::trim).filter(|v| !v.is_empty()),
            &from,
            &to,
            resolve_limit(limit)?,
            offset,
        )
    }
}
