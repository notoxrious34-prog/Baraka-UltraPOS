/**
 * Reports cockpit wire types (TASK-BRK-012B).
 *
 * Every interface mirrors its backend DTO 1:1. Money travels as canonical
 * decimal-string centimes, quantities as canonical decimal-string micros.
 * The `number` type never carries money or quantities here.
 */

/** One daily sales aggregate row (mirrors DailySalesSummaryDto). */
export interface DailySalesRow {
  day: string;
  totalTtcCentimes: string;
  totalHtCentimes: string;
  totalTvaCentimes: string;
  documentCount: string;
  averageBasketCentimes: string;
}

/** One per-product sales aggregate row (mirrors SalesByProductRowDto). */
export interface SalesByProductRow {
  productId: string;
  sku: string;
  nameFr: string;
  quantitySoldBaseMicro: string;
  revenueTtcCentimes: string;
  lineCount: string;
}

/** One per-category sales aggregate row (mirrors SalesByCategoryRowDto). */
export interface SalesByCategoryRow {
  categoryId: string | null;
  categoryCode: string | null;
  nameFr: string | null;
  revenueTtcCentimes: string;
  lineCount: string;
}

/** One per-product gross-margin row (mirrors GrossMarginRowDto). */
export interface GrossMarginRow {
  productId: string;
  sku: string;
  revenueHtCentimes: string;
  costTotalCentimes: string;
  marginCentimes: string;
  /** Margin percentage in basis points (÷ 100 for display). */
  marginPercentBp: string;
  /** True when a contributing line had zero unit cost. */
  hasUntrackedCost: boolean;
}

/** One credit-outstanding row (mirrors CreditOutstandingRowDto). */
export interface CreditOutstandingRow {
  customerId: string;
  name: string;
  phone: string | null;
  creditPolicy: string;
  /** Limit in centimes (`null` for UNLIMITED). */
  creditLimitCentimes: string | null;
  currentBalanceCentimes: string;
  /** Limit consumed in basis points (`null` for UNLIMITED). */
  limitUsedBp: string | null;
}

/** One stock-movement history row (mirrors StockMovementRowDto). */
export interface StockMovementRow {
  id: string;
  movementType: string;
  warehouseId: string;
  productId: string;
  /** Signed delta in base micros. */
  quantityBaseMicro: string;
  referenceType: string | null;
  referenceId: string | null;
  createdAt: string;
  createdBy: string;
}

/** One cash-session history row (mirrors CashSessionHistoryRowDto). */
export interface CashSessionHistoryRow {
  id: string;
  terminalId: string;
  status: string;
  openedAt: string;
  closedAt: string | null;
  openingFloatCentimes: string;
  systemExpectedCentimes: string | null;
  actualCountedCentimes: string | null;
  varianceCentimes: string | null;
}

/** One user-activity row (mirrors UserActivityRowDto). */
export interface UserActivityRow {
  sequenceNo: string;
  timestamp: string;
  userId: string;
  action: string;
  entityType: string;
  entityId: string;
  oldValueJson: string | null;
  newValueJson: string | null;
}

/** One low-stock row (mirrors LowStockRowDto, ADR-037: qty <= 0). */
export interface LowStockRow {
  productId: string;
  sku: string;
  nameFr: string;
  warehouseId: string;
  warehouseCode: string;
  quantityBaseMicro: string;
}

/** Shared date-range filter shared by every ranged report screen. */
export interface ReportDateRange {
  dateFrom: string;
  dateTo: string;
}

/** Filters for the stock-movement history screen. */
export interface StockMovementFilters extends ReportDateRange {
  warehouseId: string | null;
  productId: string | null;
  movementType: string | null;
  limit: string;
  offset: string;
}

/** Filters for the user-activity screen. */
export interface UserActivityFilters extends ReportDateRange {
  userId: string | null;
  action: string | null;
  limit: string;
  offset: string;
}
