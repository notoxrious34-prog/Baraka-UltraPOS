/**
 * Reports bridge (TASK-BRK-012B).
 * Backend command mapping (sovereign contract):
 *   reports_daily_sales / reports_sales_by_product /
 *   reports_sales_by_category / reports_gross_margin /
 *   reports_credit_outstanding / reports_low_stock /
 *   reports_stock_movement_history / reports_cash_sessions_history /
 *   reports_user_activity - exact backend names.
 * All money crosses as centime strings; quantities as micro strings.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type {
  BrokenRun,
  CashSessionHistoryRow,
  CreditOutstandingRow,
  DailySalesRow,
  DeadStockLine,
  GrossMarginRow,
  LowStockRow,
  ModelMarginLine,
  ReplenishmentRow,
  ReportDateRange,
  SalesByCategoryRow,
  SalesByProductRow,
  SignalAging,
  SignalsFeed,
  SignalVelocity,
  StockMovementFilters,
  StockMovementRow,
  UnitMixRow,
  UserActivityFilters,
  UserActivityRow,
} from "@/types/reports";
import { call } from "./ipc-kernel";

interface DailySalesWire {
  day: string;
  total_ttc_centimes: string;
  total_ht_centimes: string;
  total_tva_centimes: string;
  document_count: string;
  average_basket_centimes: string;
}

interface SalesByProductWire {
  product_id: string;
  sku: string;
  name_fr: string;
  quantity_sold_base_micro: string;
  revenue_ttc_centimes: string;
  line_count: string;
}

interface SalesByCategoryWire {
  category_id: string | null;
  category_code: string | null;
  name_fr: string | null;
  revenue_ttc_centimes: string;
  line_count: string;
}

interface GrossMarginWire {
  product_id: string;
  sku: string;
  revenue_ht_centimes: string;
  cost_total_centimes: string;
  margin_centimes: string;
  margin_percent_bp: string;
  has_untracked_cost: boolean;
}

interface CreditOutstandingWire {
  customer_id: string;
  name: string;
  phone: string | null;
  credit_policy: string;
  credit_limit_centimes: string | null;
  current_balance_centimes: string;
  limit_used_bp: string | null;
}

interface StockMovementWire {
  id: string;
  movement_type: string;
  warehouse_id: string;
  product_id: string;
  quantity_base_micro: string;
  reference_type: string | null;
  reference_id: string | null;
  created_at: string;
  created_by: string;
}

interface CashSessionHistoryWire {
  id: string;
  terminal_id: string;
  status: string;
  opened_at: string;
  closed_at: string | null;
  opening_float_centimes: string;
  system_expected_centimes: string | null;
  actual_counted_centimes: string | null;
  variance_centimes: string | null;
}

interface UserActivityWire {
  sequence_no: string;
  timestamp: string;
  user_id: string;
  action: string;
  entity_type: string;
  entity_id: string;
  old_value_json: string | null;
  new_value_json: string | null;
}

interface LowStockWire {
  product_id: string;
  sku: string;
  name_fr: string;
  warehouse_id: string;
  warehouse_code: string;
  quantity_base_micro: string;
}

interface ReplenishmentWire {
  product_id: string;
  sku: string;
  name_fr: string;
  name_ar: string;
  on_hand_base_micro: string;
  sold_30d_base_micro: string;
  cover_days_micro: string;
}

interface UnitMixWire {
  unit_id: string;
  unit_name_fr: string;
  unit_name_ar: string;
  qty_base_micro: string;
  share_bp: string;
  sample_days: string;
  confidence_bp: string;
}

function rangeArgs(sessionToken: string, range: ReportDateRange): Record<string, unknown> {
  return {
    session_token: sessionToken,
    date_from: range.dateFrom,
    date_to: range.dateTo,
  };
}

/** Daily sales aggregates for committed documents in range. */
export async function ipcDailySales(
  sessionToken: string,
  range: ReportDateRange,
): Promise<DailySalesRow[]> {
  const wire = await call<DailySalesWire[]>("reports_daily_sales", {
    request: rangeArgs(sessionToken, range),
  });
  return wire.map((row) => ({
    day: row.day,
    totalTtcCentimes: row.total_ttc_centimes,
    totalHtCentimes: row.total_ht_centimes,
    totalTvaCentimes: row.total_tva_centimes,
    documentCount: row.document_count,
    averageBasketCentimes: row.average_basket_centimes,
  }));
}

/** Per-product sales aggregates in range, revenue descending. */
export async function ipcSalesByProduct(
  sessionToken: string,
  range: ReportDateRange,
): Promise<SalesByProductRow[]> {
  const wire = await call<SalesByProductWire[]>("reports_sales_by_product", {
    request: rangeArgs(sessionToken, range),
  });
  return wire.map((row) => ({
    productId: row.product_id,
    sku: row.sku,
    nameFr: row.name_fr,
    quantitySoldBaseMicro: row.quantity_sold_base_micro,
    revenueTtcCentimes: row.revenue_ttc_centimes,
    lineCount: row.line_count,
  }));
}

/** Per-category sales aggregates in range, revenue descending. */
export async function ipcSalesByCategory(
  sessionToken: string,
  range: ReportDateRange,
): Promise<SalesByCategoryRow[]> {
  const wire = await call<SalesByCategoryWire[]>("reports_sales_by_category", {
    request: rangeArgs(sessionToken, range),
  });
  return wire.map((row) => ({
    categoryId: row.category_id,
    categoryCode: row.category_code,
    nameFr: row.name_fr,
    revenueTtcCentimes: row.revenue_ttc_centimes,
    lineCount: row.line_count,
  }));
}

/** Per-product gross margin in range. */
export async function ipcGrossMargin(
  sessionToken: string,
  range: ReportDateRange,
): Promise<GrossMarginRow[]> {
  const wire = await call<GrossMarginWire[]>("reports_gross_margin", {
    request: rangeArgs(sessionToken, range),
  });
  return wire.map((row) => ({
    productId: row.product_id,
    sku: row.sku,
    revenueHtCentimes: row.revenue_ht_centimes,
    costTotalCentimes: row.cost_total_centimes,
    marginCentimes: row.margin_centimes,
    marginPercentBp: row.margin_percent_bp,
    hasUntrackedCost: row.has_untracked_cost,
  }));
}

/** Outstanding customer credit balances. */
export async function ipcCreditOutstanding(
  sessionToken: string,
): Promise<CreditOutstandingRow[]> {
  const wire = await call<CreditOutstandingWire[]>("reports_credit_outstanding", {
    session_token: sessionToken,
  });
  return wire.map((row) => ({
    customerId: row.customer_id,
    name: row.name,
    phone: row.phone,
    creditPolicy: row.credit_policy,
    creditLimitCentimes: row.credit_limit_centimes,
    currentBalanceCentimes: row.current_balance_centimes,
    limitUsedBp: row.limit_used_bp,
  }));
}

/** Depleted stock levels (ADR-037: quantity <= 0). */
export async function ipcLowStock(
  sessionToken: string,
  warehouseId: string | null,
): Promise<LowStockRow[]> {
  const wire = await call<LowStockWire[]>("reports_low_stock", {
    request: { session_token: sessionToken, warehouse_id: warehouseId },
  });
  return wire.map((row) => ({
    productId: row.product_id,
    sku: row.sku,
    nameFr: row.name_fr,
    warehouseId: row.warehouse_id,
    warehouseCode: row.warehouse_code,
    quantityBaseMicro: row.quantity_base_micro,
  }));
}

/** Replenishment radar: trailing-30d movers, most urgent first (S3). */
export async function ipcReplenishmentRadar(
  sessionToken: string,
  limit: number,
): Promise<ReplenishmentRow[]> {
  const wire = await call<ReplenishmentWire[]>("reports_replenishment_radar", {
    request: { session_token: sessionToken, limit },
  });
  return wire.map((row) => ({
    productId: row.product_id,
    sku: row.sku,
    nameFr: row.name_fr,
    nameAr: row.name_ar,
    onHandBaseMicro: row.on_hand_base_micro,
    sold30dBaseMicro: row.sold_30d_base_micro,
    coverDaysMicro: row.cover_days_micro,
  }));
}

/** Unit-mix: one product's window sales split by selling unit (S4). */
export async function ipcUnitSalesMix(
  sessionToken: string,
  productId: string,
): Promise<UnitMixRow[]> {
  const wire = await call<UnitMixWire[]>("reports_unit_sales_mix", {
    request: { session_token: sessionToken, product_id: productId, window_days: null },
  });
  return wire.map((row) => ({
    unitId: row.unit_id,
    unitNameFr: row.unit_name_fr,
    unitNameAr: row.unit_name_ar,
    qtyBaseMicro: row.qty_base_micro,
    shareBp: row.share_bp,
    sampleDays: row.sample_days,
    confidenceBp: row.confidence_bp,
  }));
}

/** Filtered, paginated stock-movement history (newest first). */
export async function ipcStockMovementHistory(
  sessionToken: string,
  filters: StockMovementFilters,
): Promise<StockMovementRow[]> {
  const wire = await call<StockMovementWire[]>("reports_stock_movement_history", {
    request: {
      session_token: sessionToken,
      warehouse_id: filters.warehouseId,
      product_id: filters.productId,
      movement_type: filters.movementType,
      date_from: filters.dateFrom,
      date_to: filters.dateTo,
      limit: Number(filters.limit),
      offset: Number(filters.offset),
    },
  });
  return wire.map((row) => ({
    id: row.id,
    movementType: row.movement_type,
    warehouseId: row.warehouse_id,
    productId: row.product_id,
    quantityBaseMicro: row.quantity_base_micro,
    referenceType: row.reference_type,
    referenceId: row.reference_id,
    createdAt: row.created_at,
    createdBy: row.created_by,
  }));
}

/** Cash-session history in range (newest first). */
export async function ipcCashSessionsHistory(
  sessionToken: string,
  range: ReportDateRange,
  terminalId: string | null,
): Promise<CashSessionHistoryRow[]> {
  const wire = await call<CashSessionHistoryWire[]>("reports_cash_sessions_history", {
    request: { ...rangeArgs(sessionToken, range), terminal_id: terminalId },
  });
  return wire.map((row) => ({
    id: row.id,
    terminalId: row.terminal_id,
    status: row.status,
    openedAt: row.opened_at,
    closedAt: row.closed_at,
    openingFloatCentimes: row.opening_float_centimes,
    systemExpectedCentimes: row.system_expected_centimes,
    actualCountedCentimes: row.actual_counted_centimes,
    varianceCentimes: row.variance_centimes,
  }));
}

/** Filtered, paginated audit-log rows (newest first). */
export async function ipcUserActivity(
  sessionToken: string,
  filters: UserActivityFilters,
): Promise<UserActivityRow[]> {
  const wire = await call<UserActivityWire[]>("reports_user_activity", {
    request: {
      session_token: sessionToken,
      user_id: filters.userId,
      action: filters.action,
      date_from: filters.dateFrom,
      date_to: filters.dateTo,
      limit: Number(filters.limit),
      offset: Number(filters.offset),
    },
  });
  return wire.map((row) => ({
    sequenceNo: row.sequence_no,
    timestamp: row.timestamp,
    userId: row.user_id,
    action: row.action,
    entityType: row.entity_type,
    entityId: row.entity_id,
    oldValueJson: row.old_value_json,
    newValueJson: row.new_value_json,
  }));
}

interface BrokenCellWire {
  axis1_value: string;
  axis2_value: string;
  sibling_units_sold: string;
}

interface BrokenRunWire {
  model_id: string;
  cells: BrokenCellWire[];
}

interface DeadStockWire {
  product_id: string;
  qty_on_hand_micro: string;
  days_since_last_movement: string;
}

interface ModelMarginWire {
  model_id: string;
  revenue_centimes: string;
  cogs_centimes: string;
  margin_bps: string | null;
}

interface AgingBucketsWire {
  bucket_0_30_centimes: string;
  bucket_31_60_centimes: string;
  bucket_61_90_centimes: string;
  bucket_90_plus_centimes: string;
}

interface VelocityWire {
  units_per_day: string;
  is_slow: boolean;
  trend_bps: string | null;
}

interface SignalsFeedWire {
  profile: string;
  broken_runs: BrokenRunWire[];
  dead_stock: DeadStockWire[];
  velocity: VelocityWire | null;
  model_margins: ModelMarginWire[];
  aging: AgingBucketsWire;
}

/** Signal-cockpit feed: five signals in one read-only computation (BRK-116). */
export async function ipcSignals(
  sessionToken: string,
  windowDays?: number,
): Promise<SignalsFeed> {
  const wire = await call<SignalsFeedWire>("reports_signals", {
    request: {
      session_token: sessionToken,
      window_days: windowDays ?? null,
    },
  });
  const toRun = (run: BrokenRunWire): BrokenRun => ({
    modelId: run.model_id,
    cells: run.cells.map((cell) => ({
      axis1Value: cell.axis1_value,
      axis2Value: cell.axis2_value,
      siblingUnitsSold: cell.sibling_units_sold,
    })),
  });
  const toDead = (line: DeadStockWire): DeadStockLine => ({
    productId: line.product_id,
    qtyOnHandMicro: line.qty_on_hand_micro,
    daysSinceLastMovement: line.days_since_last_movement,
  });
  const toMargin = (margin: ModelMarginWire): ModelMarginLine => ({
    modelId: margin.model_id,
    revenueCentimes: margin.revenue_centimes,
    cogsCentimes: margin.cogs_centimes,
    marginBps: margin.margin_bps,
  });
  const toVelocity = (velocity: VelocityWire): SignalVelocity => ({
    unitsPerDay: velocity.units_per_day,
    isSlow: velocity.is_slow,
    trendBps: velocity.trend_bps,
  });
  const toAging = (aging: AgingBucketsWire): SignalAging => ({
    bucket030Centimes: aging.bucket_0_30_centimes,
    bucket3160Centimes: aging.bucket_31_60_centimes,
    bucket6190Centimes: aging.bucket_61_90_centimes,
    bucket90PlusCentimes: aging.bucket_90_plus_centimes,
  });
  return {
    profile: wire.profile,
    brokenRuns: wire.broken_runs.map(toRun),
    deadStock: wire.dead_stock.map(toDead),
    velocity: wire.velocity === null ? null : toVelocity(wire.velocity),
    modelMargins: wire.model_margins.map(toMargin),
    aging: toAging(wire.aging),
  };
}
