/**
 * Sovereign POS Cockpit — strict UI and wire types.
 *
 * Monetary wire values are `string` integer centimes, quantities are
 * `string` integer micros. The `number` type never carries money here.
 */

export type Locale = "ar-DZ" | "fr-DZ";

export type DocType = "TICKET" | "FACTURE" | "BL";

export type PaymentMethodCode = "CASH" | "CREDIT" | "CHEQUE" | "CARD";

export type ActiveModal =
  | "none"
  | "checkout"
  | "customer"
  | "conversion"
  | "return"
  | "edit-product"
  | "warehouse-transfer"
  | "stock-adjust"
  | "backup"
  | "identity"
  | "payment"
  | "discount"
  | "quantity"
  | "cash-open"
  | "cash-close"
  | "cash-expense"
  | "parked";

/** One purchasable packaging snapshot carried inside a cart line. */
export interface CartUnitSnapshot {
  unitId: string;
  productId: string;
  nameAr: string;
  nameFr: string;
  factorMicro: string;
  barcode: string | null;
  salePriceCentimes: string;
  isBaseUnit: boolean;
  isDefaultSelling: boolean;
}

/** One active-sale cart line (preview only; backend commits truth). */
export interface CartLine {
  key: string;
  productId: string;
  sku: string;
  nameAr: string;
  nameFr: string;
  units: CartUnitSnapshot[];
  unitIndex: number;
  quantityMicro: string;
  lineDiscountCentimes: string;
}

/** Guest (anonymous cash) customer. */
export interface GuestCustomer {
  kind: "guest";
}

/**
 * Named customer selected in the F2 modal.
 * Profile fields come from the live `customers_get` backend view;
 * the backend remains the authoritative limit enforcer.
 */
export interface NamedCustomer {
  kind: "named";
  id: string;
  name: string;
  policy: "LIMITED" | "UNLIMITED" | "DISABLED";
  limitCentimes: string | null;
  balanceCentimes: string | null;
  source: "backend";
}

export type ActiveCustomer = GuestCustomer | NamedCustomer;

/** Customer list row (mirrors CustomerSummaryDto). */
export interface CustomerSummary {
  id: string;
  name: string;
  phone: string | null;
  credit_policy: "LIMITED" | "UNLIMITED" | "DISABLED";
}

/** Full customer view (mirrors CustomerDetailDto). */
export interface CustomerDetail {
  id: string;
  name: string;
  phone: string | null;
  address: string | null;
  credit_policy: "LIMITED" | "UNLIMITED" | "DISABLED";
  credit_limit_centimes: string | null;
  current_balance_centimes: string;
  is_active: boolean;
}

/** Statement line (mirrors StatementEntryDto). */
export interface StatementEntry {
  entry_type: string;
  amount_centimes: string;
  reference_type: string | null;
  reference_id: string | null;
  created_at: string;
  balance_after_centimes: string;
}

/** Authenticated session (mirrors LoginResponseDto). */
export interface SessionInfo {
  token: string;
  userId: string;
  username: string;
  role: string;
  expiresAt: string;
}

/** Active warehouse (mirrors WarehouseDto). */
export interface WarehouseInfo {
  id: string;
  code: string;
  name: string;
  isActive: boolean;
}

/** Catalog product view (mirrors ProductResponseDto). */
export interface CatalogProduct {
  product_id: string;
  sku: string;
  barcode: string | null;
  name_ar: string;
  name_fr: string;
  category_id: string | null;
  is_kit: boolean;
  is_active: boolean;
  units: CatalogUnit[];
  kit_items: Array<{
    component_id: string;
    unit_id: string;
    quantity_micro: string;
  }>;
}

/** Catalog unit view (mirrors ProductUnitDto). */
export interface CatalogUnit {
  unit_id: string;
  product_id: string;
  unit_name_ar: string;
  unit_name_fr: string;
  factor_micro: string;
  barcode: string | null;
  sale_price_centimes: string;
  purchase_price_centimes: string;
  is_base_unit: boolean;
  is_default_selling: boolean;
}

/** Checkout tender split (all centime strings). */
export interface TenderSplit {
  cash: string;
  credit: string;
  cheque: string;
  card: string;
}

/** Committed sale receipt (mirrors CheckoutSaleResponse). */
export interface SaleReceipt {
  document_id: string;
  document_number: string;
  subtotal_ht_centimes: string;
  tva_total_centimes: string;
  discount_total_centimes: string;
  timbre_centimes: string;
  total_ttc_centimes: string;
  paid_amount_centimes: string;
  change_amount_centimes: string;
  print_job_id: string;
}
