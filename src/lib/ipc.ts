/**
 * Typed Tauri IPC bridge (ADR-004 wire contract).
 *
 * Backend command mapping (sovereign contract):
 * - auth_login / auth_logout / check_first_run_status /
 *   bootstrap_first_admin / sales_checkout / sales_preview_totals /
 *   sales_create_draft / warehouses_list / stock_get — exact backend names.
 * - catalog_search — the backend search command (the task text calls it
 *   `products_search`; the sovereign backend exposes `catalog_search` and
 *   Rust code is frozen, so this bridge targets the real name).
 */

import { invoke } from "@tauri-apps/api/core";
import { useSessionStore } from "@/stores/session-store";
import type {
  ActiveCustomer,
  CatalogProduct,
  Category,
  CustomerDetail,
  CustomerSummary,
  DocType,
  ImportProductRow,
  ImportProductsReport,
  Locale,
  PaymentMethodCode,
  SaleReceipt,
  SessionInfo,
  StatementEntry,
  WarehouseInfo,
} from "@/types/pos";

function bridgeError(): Error {
  const error = new Error("BRIDGE_UNAVAILABLE");
  return error;
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function call<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw bridgeError();
  }
  return invoke<T>(command, withSessionToken(command, args));
}

/**
 * Commands legitimately callable before any session exists. Every other
 * command receives the active session token automatically (single source of
 * truth: the session store — ADR-045 edge-centralization precedent).
 */
const PRE_AUTH_COMMANDS: readonly string[] = [
  "app_health",
  "auth_login",
  "check_first_run_status",
  "bootstrap_first_admin",
];

/**
 * Merge the store-owned session token into invoke args, overwriting any
 * caller-supplied value (single source of truth). Exactly ONE rule applies
 * project-wide: the key is always `session_token`, at the top level (bare
 * scalar parameters) and inside nested `request` DTO objects alike. This
 * holds because every #[tauri::command] in commands/src carries
 * rename_all = "snake_case" (uniform wire policy), while DTO struct fields
 * deserialize via serde with exact Rust field names (likewise snake_case).
 * Throws `IPC_NO_ACTIVE_SESSION` instead of invoking tokenless, so a missing
 * session fails fast and legibly at this edge rather than as a backend
 * arg-validation rejection downstream.
 */
function withSessionToken(
  command: string,
  args: Record<string, unknown>,
): Record<string, unknown> {
  if (PRE_AUTH_COMMANDS.includes(command)) {
    return args;
  }
  const token = useSessionStore.getState().getSessionToken();
  if (token === null) {
    throw new Error("IPC_NO_ACTIVE_SESSION");
  }
  const merged: Record<string, unknown> = { ...args, session_token: token };
  const nested = merged.request;
  if (typeof nested === "object" && nested !== null && !Array.isArray(nested)) {
    merged.request = { ...(nested as Record<string, unknown>), session_token: token };
  }
  return merged;
}

interface LoginWire {
  session_token: string;
  user_id: string;
  username: string;
  role: string;
  expires_at: string;
}

/** Authenticate; resolves a typed session or rejects with a wire code. */
export async function ipcLogin(username: string, password: string): Promise<SessionInfo> {
  const wire = await call<LoginWire>("auth_login", {
    request: { username, password },
  });
  return {
    token: wire.session_token,
    userId: wire.user_id,
    username: wire.username,
    role: wire.role,
    expiresAt: wire.expires_at,
  };
}

/** Revoke a session (idempotent server-side; token injected by the wrapper). */
export async function ipcLogout(): Promise<void> {
  await call<void>("auth_logout", { request: {} });
}

/** First-run probe: true while the users table is empty (no session needed). */
export async function ipcCheckFirstRunStatus(): Promise<boolean> {
  const wire = await call<{ requires_setup: boolean }>("check_first_run_status", {});
  return wire.requires_setup;
}

/**
 * One-time first Admin creation; resolves the stored id and name.
 * The caller then logs in through the standard login flow (no session
 * token is minted here).
 */
export async function ipcBootstrapFirstAdmin(
  username: string,
  password: string,
  commandId: string,
): Promise<{ userId: string; username: string }> {
  const wire = await call<{ user_id: string; username: string }>("bootstrap_first_admin", {
    request: { command_id: commandId, username, password },
  });
  return { userId: wire.user_id, username: wire.username };
}

/** Fetch one full product view by UUID (draft resume rehydration). */
export async function ipcGetProduct(
  sessionToken: string,
  productId: string,
): Promise<CatalogProduct> {
  return call<CatalogProduct>("catalog_get_product", {
    request: { session_token: sessionToken, product_id: productId },
  });
}

/** Dual-track deletion outcome (ADR-092): purge or soft-deactivation. */
export interface DeleteEntityResult {
  id: string;
  mode: "hard_deleted" | "deactivated";
  message: string;
}

/** Dual-track safe product deletion: purge when unreferenced, deactivate on history. */
export async function ipcDeleteProduct(
  sessionToken: string,
  productId: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("catalog_delete_product", {
    request: { session_token: sessionToken, id: productId },
  });
}

/** Prefix/multilingual catalog search (150ms debounce applied by caller). */
export async function ipcSearchCatalog(
  sessionToken: string,
  query: string,
  limit: number,
): Promise<CatalogProduct[]> {
  return call<CatalogProduct[]>("catalog_search", {
    request: { session_token: sessionToken, query, limit },
  });
}

export interface NewProductUnitWire {
  unit_name_ar: string;
  unit_name_fr: string;
  factor_micro: string;
  barcode: string | null;
  sale_price_centimes: string;
  purchase_price_centimes: string;
  wholesale_price_centimes: string | null;
  is_base_unit: boolean;
  is_default_selling: boolean;
}

export interface CreateProductWire {
  sku: string;
  barcode: string | null;
  name_ar: string;
  name_fr: string;
  category_id: string | null;
  is_kit: boolean;
  units: NewProductUnitWire[];
}

/** Create a product with its full unit set; resolves the stored product view. */
export async function ipcCreateProduct(
  sessionToken: string,
  input: CreateProductWire,
): Promise<CatalogProduct> {
  return call<CatalogProduct>("catalog_create_product", {
    request: { session_token: sessionToken, ...input },
  });
}

/** One unit upsert for product updates (mirrors UpdateProductUnitDto). */
export interface UpdateProductUnitPayload {
  unit_id: string | null;
  unit_name_ar: string;
  unit_name_fr: string;
  factor_micro: string;
  barcode: string | null;
  sale_price_centimes: string;
  purchase_price_centimes: string;
  wholesale_price_centimes: string | null;
  is_base_unit: boolean;
  is_default_selling: boolean;
}

/** Full-state product update payload (mirrors UpdateProductDto; SKU immutable). */
export interface UpdateProductPayload {
  session_token: string;
  product_id: string;
  name_ar: string;
  name_fr: string;
  category_id: string | null;
  is_active: boolean;
  units: UpdateProductUnitPayload[];
}

/**
 * Update a product (full-state payload, SKU immutable). The backend echoes
 * the fresh detail view; this wrapper resolves once committed. Rejects
 * with the backend wire code verbatim (UNAUTHORIZED, NOT_FOUND, …).
 */
export async function updateProduct(payload: UpdateProductPayload): Promise<void> {
  await call<unknown>("catalog_update_product", {
    request: { ...payload },
  });
}

/**
 * Category taxonomy (mirrors CategoryResponseDto list). Inactive rows are
 * hidden unless `includeInactive` is set. Rejects verbatim on bridge errors.
 */
export async function ipcListCategories(
  sessionToken: string,
  includeInactive?: boolean,
): Promise<Category[]> {
  return call<Category[]>("category_list", {
    session_token: sessionToken,
    include_inactive: includeInactive ?? false,
  });
}

/** Category creation payload (mirrors CreateCategoryRequestDto). */
export interface CreateCategoryPayload {
  session_token: string;
  code: string;
  name_fr: string;
  name_ar: string;
  parent_id?: string | null;
}

/**
 * Create a category (unique code, active by default). Resolves with the
 * stored view; rejects with `CATEGORY_CODE_DUPLICATE` on code collision.
 */
export async function ipcCreateCategory(
  sessionToken: string,
  payload: Omit<CreateCategoryPayload, "session_token">,
): Promise<Category> {
  return call<Category>("category_create", {
    request: { session_token: sessionToken, ...payload },
  });
}

/** Category update payload (mirrors UpdateCategoryRequestDto; code immutable). */
export interface UpdateCategoryPayload {
  session_token: string;
  category_id: string;
  name_fr: string;
  name_ar: string;
  parent_id?: string | null;
  is_active: boolean;
}

/** Update a category's names, parent, and active flag. */
export async function ipcUpdateCategory(
  sessionToken: string,
  payload: Omit<UpdateCategoryPayload, "session_token">,
): Promise<Category> {
  return call<Category>("category_update", {
    request: { session_token: sessionToken, ...payload },
  });
}

/**
 * Idempotently provision the Algerian hardware starter taxonomy. No-op
 * returning existing rows when any category exists.
 */
export async function ipcSeedStarterCategories(sessionToken: string): Promise<Category[]> {
  return call<Category[]>("category_seed_starter", {
    session_token: sessionToken,
  });
}

/**
 * Atomic bulk product import (mirrors ImportProductsResponseDto). Valid rows
 * commit inside one `BEGIN IMMEDIATE` transaction; per-row failures arrive in
 * the structured report. Rejects verbatim on bridge errors.
 */
export async function ipcImportProducts(
  token: string,
  rows: ImportProductRow[],
  warehouseId?: string,
  onDuplicateSku?: "skip" | "error",
): Promise<ImportProductsReport> {
  return call<ImportProductsReport>("catalog_import_products", {
    request: {
      session_token: token,
      rows,
      warehouse_id: warehouseId || null,
      on_duplicate_sku: onDuplicateSku || "skip",
    },
  });
}

/** Warehouse row (mirrors WarehouseDto; no is_default on the wire). */
export interface WarehouseItem {
  id: string;
  code: string;
  name: string;
  is_active: boolean;
}

/** Warehouse creation payload (mirrors CreateWarehouseRequestDto). */
export interface CreateWarehousePayload {
  session_token: string;
  code: string;
  name: string;
}

/** Single-line transfer payload (mirrors TransferStockRequestDto). */
export interface StockTransferPayload {
  session_token: string;
  source_warehouse_id: string;
  destination_warehouse_id: string;
  product_id: string;
  unit_id: string;
  quantity_micro: string;
  reason: string | null;
}

/** Transfer outcome (mirrors TransferStockResponseDto). */
export interface StockTransferResult {
  transfer_batch_id: string;
  out_movement_id: string;
  in_movement_id: string;
  source_new_quantity_base_micro: string;
  destination_new_quantity_base_micro: string;
}

/**
 * List active warehouses ordered by code. Rejects with the backend wire
 * code verbatim.
 */
export async function listWarehouses(sessionToken: string): Promise<WarehouseItem[]> {
  interface WarehousesWire {
    warehouses: WarehouseItem[];
  }
  const wire = await call<WarehousesWire>("warehouses_list", {
    session_token: sessionToken,
  });
  return wire.warehouses;
}

/**
 * Create a warehouse (ADMIN-only server-side via ManageWarehouses).
 * Resolves the stored row. Rejects verbatim (UNAUTHORIZED,
 * WAREHOUSE_CODE_EXISTS, …).
 */
export async function createWarehouse(payload: CreateWarehousePayload): Promise<WarehouseItem> {
  return call<WarehouseItem>("warehouses_create", {
    request: { ...payload },
  });
}

/**
 * Move stock as one atomic OUT/IN pair (Gerant/Admin via TransferStock).
 * Resolves movement ids and both new levels. Rejects verbatim
 * (TRANSFER_SAME_WAREHOUSE, WAREHOUSE_INACTIVE, STOCK_INSUFFICIENT, …).
 */
export async function transferStock(payload: StockTransferPayload): Promise<void> {
  await call<unknown>("stock_transfer", {
    request: { ...payload },
  });
}

/** List active warehouses (token injected by the wrapper). */
export async function ipcListWarehouses(): Promise<WarehouseInfo[]> {
  interface WarehousesWire {
    warehouses: Array<{ id: string; code: string; name: string; is_active: boolean }>;
  }
  const wire = await call<WarehousesWire>("warehouses_list", {});
  return wire.warehouses.map((warehouse) => ({
    id: warehouse.id,
    code: warehouse.code,
    name: warehouse.name,
    isActive: warehouse.is_active,
  }));
}

/** Create a warehouse (ADMIN only); resolves the stored view. */
export async function ipcCreateWarehouse(
  sessionToken: string,
  code: string,
  name: string,
): Promise<WarehouseInfo> {
  interface WarehouseWire {
    id: string;
    code: string;
    name: string;
    is_active: boolean;
  }
  const wire = await call<WarehouseWire>("warehouses_create", {
    request: {
      session_token: sessionToken,
      code: code.trim().toUpperCase(),
      name: name.trim(),
    },
  });
  return {
    id: wire.id,
    code: wire.code,
    name: wire.name,
    isActive: wire.is_active,
  };
}

/** Sparse warehouse update payload (ADR-092): rename and/or (de)activate. */
export interface WarehouseUpdateInput {
  name?: string;
  isActive?: boolean;
}

/** Sparse warehouse update (ADMIN only server-side; DEP-01 deactivation rejected). */
export async function ipcUpdateWarehouse(
  sessionToken: string,
  warehouseId: string,
  input: WarehouseUpdateInput,
): Promise<WarehouseInfo> {
  interface WarehouseWire {
    id: string;
    code: string;
    name: string;
    is_active: boolean;
  }
  const wire = await call<WarehouseWire>("warehouses_update", {
    request: {
      session_token: sessionToken,
      id: warehouseId,
      name: input.name,
      is_active: input.isActive,
    },
  });
  return {
    id: wire.id,
    code: wire.code,
    name: wire.name,
    isActive: wire.is_active,
  };
}

/** Dual-track safe warehouse deletion: purge when unreferenced, deactivate on history. */
export async function ipcDeleteWarehouse(
  sessionToken: string,
  warehouseId: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("warehouses_delete", {
    request: { session_token: sessionToken, id: warehouseId },
  });
}

/** Managed operator view (mirrors UserSummaryDto; never carries hashes). */
export interface UserInfo {
  id: string;
  username: string;
  role: "VENDEUR" | "CAISSIER" | "GERANT" | "ADMIN";
  is_active: boolean;
  has_pin: boolean;
  created_at: string;
  updated_at: string;
}

/** List every managed operator (Admin only). */
export async function ipcListUsers(sessionToken: string): Promise<UserInfo[]> {
  return call<UserInfo[]>("users_list", { session_token: sessionToken });
}

/** Create a validated operator; resolves the stored view (Admin only). */
export async function ipcCreateUser(
  sessionToken: string,
  payload: { username: string; password: string; pin?: string; role: string },
): Promise<UserInfo> {
  return call<UserInfo>("users_create", {
    request: {
      session_token: sessionToken,
      username: payload.username,
      password: payload.password,
      pin: payload.pin ?? null,
      role: payload.role,
    },
  });
}

/** Patch an operator's role/active flag; resolves the fresh view (Admin only). */
export async function ipcUpdateUser(
  sessionToken: string,
  payload: { user_id: string; role?: string; is_active?: boolean },
): Promise<UserInfo> {
  return call<UserInfo>("users_update", {
    request: {
      session_token: sessionToken,
      user_id: payload.user_id,
      role: payload.role ?? null,
      is_active: payload.is_active ?? null,
    },
  });
}

/** Replace an operator's password and/or PIN (Admin only). */
export async function ipcResetUserCredentials(
  sessionToken: string,
  payload: { user_id: string; password?: string; pin?: string },
): Promise<void> {
  await call<unknown>("users_reset_credentials", {
    request: {
      session_token: sessionToken,
      user_id: payload.user_id,
      password: payload.password ?? null,
      pin: payload.pin ?? null,
    },
  });
}

/** Read a stock level in base micros (`"0"` when absent). */
export async function ipcGetStock(
  sessionToken: string,
  productId: string,
  warehouseId: string,
): Promise<string> {
  interface StockWire {
    product_id: string;
    warehouse_id: string;
    base_unit_id: string;
    quantity_base_micro: string;
  }
  const wire = await call<StockWire>("stock_get", {
    request: {
      session_token: sessionToken,
      product_id: productId,
      warehouse_id: warehouseId,
    },
  });
  return wire.quantity_base_micro;
}

/** Manual stock correction payload (mirrors AdjustStockRequestDto). */
export interface StockAdjustPayload {
  session_token: string;
  product_id: string;
  warehouse_id: string;
  unit_id: string;
  /** Positive magnitude in selling-unit micros. */
  quantity_micro: string;
  /** Explicit direction; sign is never inferred. */
  direction: "INCREASE" | "DECREASE";
  /** Human reason (min 3 chars after trim, enforced server-side). */
  reason: string;
  /** Allow a negative resulting level (override permission required). */
  authorize_negative_stock: boolean;
}

/** Adjustment outcome (mirrors AdjustStockResponseDto). */
export interface StockAdjustResult {
  movement_id: string;
  new_quantity_base_micro: string;
}

/**
 * Apply one manual stock correction. Rejects with the backend wire code
 * verbatim (UNAUTHORIZED, INVALID_QUANTITY, ADJUSTMENT_REASON_REQUIRED,
 * STOCK_NEGATIVE_NOT_AUTHORIZED, STOCK_NEGATIVE_CONFIRMATION_REQUIRED, …).
 */
export async function adjustStock(payload: StockAdjustPayload): Promise<StockAdjustResult> {
  return call<StockAdjustResult>("stock_adjust", {
    request: { ...payload },
  });
}

export interface CheckoutLineWire {
  product_id: string;
  unit_id: string;
  quantity_micro: string;
  unit_price_centimes: string | null;
  line_discount_centimes: string;
}

export interface CheckoutPaymentWire {
  payment_method: PaymentMethodCode;
  amount_centimes: string;
}

export interface CheckoutRequestWire {
  command_id: string;
  session_token: string;
  warehouse_id: string;
  customer_id: string | null;
  document_type: DocType;
  /** Explicit sale-time receipt language (no silent default). */
  document_language: "AR" | "FR";
  lines: CheckoutLineWire[];
  global_discount_centimes: string;
  payments: CheckoutPaymentWire[];
  /**
   * Explicit in-session sale-on-shortage confirmation (TASK-BRK-033).
   * Privileged roles (ADMIN/GERANT) only; the backend ignores it otherwise.
   */
  allow_negative_stock: boolean;
  notes: string | null;
}

/** Commit a sale atomically; rejects with a backend wire code on failure. */
export async function ipcCheckout(request: CheckoutRequestWire): Promise<SaleReceipt> {
  interface ReceiptWire {
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
  const wire = await call<ReceiptWire>("sales_checkout", { request });
  return {
    document_id: wire.document_id,
    document_number: wire.document_number,
    subtotal_ht_centimes: wire.subtotal_ht_centimes,
    tva_total_centimes: wire.tva_total_centimes,
    discount_total_centimes: wire.discount_total_centimes,
    timbre_centimes: wire.timbre_centimes,
    total_ttc_centimes: wire.total_ttc_centimes,
    paid_amount_centimes: wire.paid_amount_centimes,
    change_amount_centimes: wire.change_amount_centimes,
    print_job_id: wire.print_job_id,
  };
}

/** One leaf-level stock shortage (mirrors StockShortageWarningDto). */
export interface StockShortageWarningWire {
  product_id: string;
  product_name: string;
  sku: string;
  available_qty_micro: string;
  requested_qty_micro: string;
}

/** Margin Guard readout for one preview line (mirrors LineMarginDto). */
export interface LineMarginWire {
  line_index: number;
  margin_basis_points: number;
  is_below_cost: boolean;
}

/** Commit-identical totals preview (mirrors PreviewTotalsResponse). */
export interface PreviewTotalsWire {
  subtotal_ht_centimes: string;
  tva_total_centimes: string;
  global_discount_centimes: string;
  timbre_centimes: string;
  total_ttc_centimes: string;
  paid_amount_centimes: string;
  change_amount_centimes: string;
  remaining_due_centimes: string;
  /** Leaf-level shortages against the requested warehouse (advisory). */
  stock_warnings: StockShortageWarningWire[];
  /** Per-line Margin Guard readouts in cart order (advisory). */
  line_margins: LineMarginWire[];
}

/** Totals preview request body (same cart shape as checkout). */
export interface PreviewTotalsRequestWire {
  warehouse_id: string;
  document_type: DocType;
  lines: CheckoutLineWire[];
  global_discount_centimes: string;
  payments: CheckoutPaymentWire[];
  /** Sale customer id for tier pricing (`null` = retail guest). */
  customer_id: string | null;
}

/**
 * Read-only, commit-identical totals preview (TASK-BRK-031).
 * Prices the cart through the same Rust kernel as `sales_checkout`, so
 * the previewed totals can never diverge from the committed ones. The
 * checkout modal renders ONLY these values — zero client-side totals
 * arithmetic exists in the checkout flow.
 */
export async function ipcPreviewTotals(
  sessionToken: string,
  request: PreviewTotalsRequestWire,
): Promise<PreviewTotalsWire> {
  return call<PreviewTotalsWire>("sales_preview_totals", {
    request: { session_token: sessionToken, ...request },
  });
}

/** Park the current cart as an editable draft. Returns the draft id. */
export async function ipcCreateDraft(
  sessionToken: string,
  warehouseId: string,
  customer: ActiveCustomer,
  lines: CheckoutLineWire[],
  notes: string | null,
  documentLanguage: "AR" | "FR",
): Promise<string> {
  interface DraftWire {
    draft_id: string;
    created_at: string;
  }
  const wire = await call<DraftWire>("sales_create_draft", {
    request: {
      session_token: sessionToken,
      warehouse_id: warehouseId,
      customer_id: customer.kind === "named" ? customer.id : null,
      document_language: documentLanguage,
      lines,
      notes,
    },
  });
  return wire.draft_id;
}

/** Discard a parked draft (DRAFT -> CANCELLED). */
export async function ipcDiscardDraft(
  sessionToken: string,
  draftId: string,
): Promise<void> {
  return call("sales_discard_draft", {
    request: {
      session_token: sessionToken,
      draft_id: draftId,
    },
  });
}

export interface NewCustomerWire {
  name: string;
  phone: string | null;
  address: string | null;
  credit_policy: "DISABLED" | "LIMITED" | "UNLIMITED";
  credit_limit_centimes: string | null;
  customer_tier?: "retail" | "artisan" | "wholesale" | null;
}

export interface UpdateCustomerWire {
  customer_id: string;
  name?: string;
  phone?: string | null;
  address?: string | null;
  credit_policy?: "DISABLED" | "LIMITED" | "UNLIMITED";
  credit_limit_centimes?: string | null;
  customer_tier?: "retail" | "artisan" | "wholesale" | null;
  is_active?: boolean;
}

/** Sparse customer update; resolves the refreshed full view. */
export async function ipcUpdateCustomer(
  sessionToken: string,
  input: UpdateCustomerWire,
): Promise<CustomerDetail> {
  return call<CustomerDetail>("customers_update", {
    request: { session_token: sessionToken, ...input },
  });
}

/** Guarded dual-track customer deletion: rejects non-zero balances, deactivates on history. */
export async function ipcDeleteCustomer(
  sessionToken: string,
  customerId: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("customers_delete", {
    request: { session_token: sessionToken, id: customerId },
  });
}

/** Register a customer; resolves the stored id. */
export async function ipcCreateCustomer(
  sessionToken: string,
  input: NewCustomerWire,
): Promise<string> {
  const wire = await call<CustomerDetail>("customers_create", {
    request: { session_token: sessionToken, ...input },
  });
  return wire.id;
}

/** Search the customer directory. */
export async function ipcSearchCustomers(
  sessionToken: string,
  query: string,
  limit: number,
): Promise<CustomerSummary[]> {
  interface SearchWire {
    customers: CustomerSummary[];
  }
  const wire = await call<SearchWire>("customers_search", {
    request: { session_token: sessionToken, query, limit },
  });
  return wire.customers;
}

/** Fetch one full customer view. */
export async function ipcGetCustomer(
  sessionToken: string,
  customerId: string,
): Promise<CustomerDetail> {
  return call<CustomerDetail>("customers_get", {
    request: { session_token: sessionToken, customer_id: customerId },
  });
}

/** List customers (active only). */
export async function ipcListCustomers(sessionToken: string): Promise<CustomerSummary[]> {
  interface ListWire {
    customers: CustomerSummary[];
  }
  const wire = await call<ListWire>("customers_list", {
    session_token: sessionToken,
  });
  return wire.customers;
}

export interface CreditPaymentResult {
  ledger_entry_id: string;
  new_balance_centimes: string;
}

/** Record a customer credit payment; resolves the new balance. */
export async function ipcRecordCreditPayment(
  sessionToken: string,
  customerId: string,
  amountCentimes: string,
  reference: string | null,
  allowOverpayment: boolean,
): Promise<CreditPaymentResult> {
  return call<CreditPaymentResult>("credit_record_payment", {
    request: {
      session_token: sessionToken,
      customer_id: customerId,
      amount_centimes: amountCentimes,
      reference,
      allow_overpayment: allowOverpayment,
    },
  });
}

/** Fetch a customer statement in ledger order. */
export async function ipcGetStatement(
  sessionToken: string,
  customerId: string,
  fromDate: string | null,
  toDate: string | null,
): Promise<StatementEntry[]> {
  interface StatementWire {
    entries: StatementEntry[];
  }
  const wire = await call<StatementWire>("credit_get_statement", {
    request: {
      session_token: sessionToken,
      customer_id: customerId,
      from_date: fromDate,
      to_date: toDate,
    },
  });
  return wire.entries;
}

export interface PrintStatus {
  jobId: string;
  status: string;
  attemptCount: string;
}

/** Latest outbox state for a document (`NONE` when absent). */
export async function ipcGetPrintStatus(
  sessionToken: string,
  documentId: string,
): Promise<PrintStatus> {
  interface StatusWire {
    job_id: string;
    status: string;
    attempt_count: string;
  }
  const wire = await call<StatusWire>("documents_get_print_status", {
    session_token: sessionToken,
    document_id: documentId,
  });
  return {
    jobId: wire.job_id,
    status: wire.status,
    attemptCount: wire.attempt_count,
  };
}

/** Enqueue a watermarked DUPLICATA reprint; resolves the new job id. */
export async function ipcReprint(
  sessionToken: string,
  documentId: string,
): Promise<string> {
  interface ReprintWire {
    job_id: string;
    document_id: string;
  }
  const wire = await call<ReprintWire>("documents_reprint", {
    request: { session_token: sessionToken, document_id: documentId },
  });
  return wire.job_id;
}

/** DEVIS-to-BL conversion request (mirrors ConvertDevisToBlRequest). */
export interface ConvertDevisToBlWireRequest {
  session_token: string;
  command_id: string;
  source_document_id: string;
}

/** DEVIS-to-BL conversion response (mirrors ConvertDevisToBlResponse). */
export interface ConvertDevisToBlWireResponse {
  new_document_id: string;
  new_document_number: string;
  new_document_status: string;
  total_ttc_centimes: string;
  items_count: number;
  warnings: string[];
}

/**
 * Convert a FINALIZED DEVIS into a FINALIZED BL (stock deducted, source
 * sealed CONVERTED). Idempotent via command_id. Rejects with the backend
 * wire code verbatim (SOURCE_DOCUMENT_NOT_FOUND,
 * INVALID_CONVERSION_SOURCE_TYPE/STATUS, DOCUMENT_ALREADY_CONVERTED,
 * EMPTY_SOURCE_DOCUMENT, INSUFFICIENT_PAYMENT family, UNAUTHORIZED, …).
 */
export async function convertDevisToBl(
  request: ConvertDevisToBlWireRequest,
): Promise<ConvertDevisToBlWireResponse> {
  return call<ConvertDevisToBlWireResponse>("convert_devis_to_bl", {
    request: { ...request },
  });
}

/** BL-to-FACTURE closing tender line (mirrors PaymentLineDto). */
export interface ConvertBlToFactureTenderWire {
  payment_method: "CASH" | "CREDIT" | "CHEQUE" | "CARD";
  amount_centimes: string;
}

/** BL-to-FACTURE closing request (mirrors ConvertBlToFactureRequest). */
export interface ConvertBlToFactureWireRequest {
  session_token: string;
  command_id: string;
  source_document_id: string;
  document_language: "AR" | "FR";
  payments: ConvertBlToFactureTenderWire[];
}

/** BL-to-FACTURE closing response (mirrors ConvertBlToFactureResponse). */
export interface ConvertBlToFactureWireResponse {
  new_document_id: string;
  new_document_number: string;
  new_document_status: string;
  total_ttc_centimes: string;
  items_count: number;
  customer_new_balance_centimes: string | null;
}

/**
 * Close a FINALIZED BL into a FINALIZED FACTURE with exact tender
 * settlement (no change concept; zero stock writes). Idempotent via
 * command_id. Rejects with the backend wire code verbatim
 * (INSUFFICIENT_PAYMENT on tender mismatch, EMPTY_CART on empty tenders,
 * CREDIT_LIMIT_EXCEEDED, NO_OPEN_CASH_SESSION_FOR_TERMINAL,
 * DOCUMENT_ALREADY_CONVERTED, UNAUTHORIZED, …).
 */
export async function convertBlToFacture(
  request: ConvertBlToFactureWireRequest,
): Promise<ConvertBlToFactureWireResponse> {
  return call<ConvertBlToFactureWireResponse>("convert_bl_to_facture", {
    request: { ...request },
  });
}

/** Document search request (mirrors DocumentSearchRequest). */
export interface DocumentSearchWireRequest {
  session_token: string;
  document_type: string | null;
  status: string | null;
  query: string | null;
  limit: number | null;
}

/** Document summary row (mirrors DocumentSummaryDto). */
export interface DocumentSummaryWireDto {
  id: string;
  document_number: string;
  document_type: string;
  status: string;
  party_id: string | null;
  party_name: string | null;
  total_ttc_centimes: string;
  items_count: number;
  created_at: string;
}

/** Document search response (mirrors DocumentSearchResponse). */
export interface DocumentSearchWireResponse {
  documents: DocumentSummaryWireDto[];
}

/**
 * Search document headers by kind/status with partial human-number or
 * customer-name match (newest first, server-clamped limit). Rejects with
 * the backend wire code verbatim (UNAUTHORIZED, INVALID_INPUT, …).
 */
export async function searchDocuments(
  request: DocumentSearchWireRequest,
): Promise<DocumentSearchWireResponse> {
  return call<DocumentSearchWireResponse>("documents_search", {
    request: { ...request },
  });
}

/** Full document view (mirrors GetSaleResponse from sales_get). */
export interface DocumentDetailsWireResponse {
  document: {
    document_id: string;
    number: string;
    doc_type: string;
    status: string;
    warehouse_id: string;
    subtotal_ht_centimes: string;
    tva_total_centimes: string;
    total_ttc_centimes: string;
    paid_amount_centimes: string;
    change_amount_centimes: string;
    payment_status: string;
  };
  items: Array<{
    product_id: string;
    unit_id: string;
    quantity_micro: string;
    unit_price_centimes: string;
    line_tva_centimes: string;
    line_ttc_centimes: string;
  }>;
  payments: Array<{
    method_code: string;
    amount_centimes: string;
  }>;
  print_status: string;
}

/**
 * Fetch a full document view by UUID via the existing sales_get command.
 * Note: sales_get takes bare scalar params (no nested request object);
 * the session token is injected at the top level by the bridge edge.
 */
export async function getDocumentDetails(
  documentId: string,
): Promise<DocumentDetailsWireResponse> {
  return call<DocumentDetailsWireResponse>("sales_get", {
    document_id: documentId,
  });
}

/** One returnable source line (mirrors ReturnableLineDto). */
export interface ReturnableLine {
  source_item_id: string;
  product_id: string;
  product_name: string;
  sku: string;
  unit_id: string;
  unit_name: string;
  original_qty_micro: string;
  already_returned_qty_micro: string;
  remaining_qty_micro: string;
  unit_price_cents: string;
  net_unit_price_cents: string;
  tva_rate: string;
}

/** Returnable source summary (mirrors ReturnableDocumentSummaryDto). */
export interface ReturnableDocumentSummary {
  source_document_id: string;
  source_document_number: string;
  document_type: string;
  party_type: string | null;
  party_id: string | null;
  party_name: string | null;
  warehouse_id: string;
  total_ht_cents: string;
  total_ttc_cents: string;
  is_eligible_for_return: boolean;
  items: ReturnableLine[];
}

/**
 * Fetch the returnable state of one source document by UUID.
 * Note: like sales_get, this command takes bare scalar params.
 */
export async function getReturnableDocumentSummary(
  sourceDocumentId: string,
): Promise<ReturnableDocumentSummary> {
  return call<ReturnableDocumentSummary>("returns_get_source_summary", {
    source_document_id: sourceDocumentId,
  });
}

/** One returned line payload (mirrors ReturnItemPayloadDto). */
export interface ReturnItemInput {
  source_item_id: string;
  quantity_micro: string;
  reason: string | null;
}

/** Refund settlement selector (mirrors RefundDestinationDto). */
export type RefundDestination =
  | { type: "Cash"; payload: { cash_session_id: string } }
  | { type: "CustomerCredit"; payload: { customer_id: string } }
  | { type: "NoRefund" };

/** Return execution payload (mirrors ProcessReturnRequestDto). */
export interface ProcessReturnPayload {
  session_token: string;
  operator_id: string;
  source_document_id: string;
  terminal_id: string;
  items: ReturnItemInput[];
  refund: RefundDestination;
  idempotency_key: string;
  notes: string | null;
}

/** Return execution result (mirrors ProcessReturnResponseDto). */
export interface ProcessReturnResult {
  return_document_id: string;
  return_document_number: string;
  source_document_id: string;
  source_document_number: string;
  total_ht_cents: string;
  total_tva_cents: string;
  total_ttc_cents: string;
  refund_type: string;
  lines_count: number;
}

/**
 * Validate and atomically persist a merchandise return. Rejects with the
 * backend wire code verbatim (RETURN_QUANTITY_EXCEEDED,
 * CREDIT_CUSTOMER_REQUIRED, NO_OPEN_CASH_SESSION_FOR_TERMINAL,
 * UNAUTHORIZED, …).
 */
export async function processReturn(
  payload: ProcessReturnPayload,
): Promise<ProcessReturnResult> {
  return call<ProcessReturnResult>("returns_process", {
    request: { ...payload },
  });
}

/** One returnable source line, eligibility view (mirrors ReturnableItemDto). */
export interface ReturnableItem {
  source_item_id: string;
  product_id: string;
  product_name: string;
  barcode: string | null;
  unit_id: string;
  unit_name: string;
  unit_price_centimes: string;
  original_quantity_micro: string;
  already_returned_micro: string;
  remaining_allowable_micro: string;
  tva_rate: number;
  discount_centimes: string;
}

/** Return context for one source document (mirrors ReturnContextResponseDto). */
export interface ReturnContextResponse {
  source_document_id: string;
  document_number: string;
  document_type: string;
  party_id: string | null;
  party_name: string | null;
  warehouse_id: string;
  created_at: string;
  is_eligible: boolean;
  ineligibility_reason: string | null;
  items: ReturnableItem[];
}

/**
 * Fetch the return context of one source document by UUID (eligibility,
 * remaining ceilings, catalog labels). Like sales_get, this command takes
 * bare scalar params.
 */
export async function getDocumentReturnContext(
  sourceDocumentId: string,
): Promise<ReturnContextResponse> {
  return call<ReturnContextResponse>("documents_get_return_context", {
    source_document_id: sourceDocumentId,
  });
}

/** Simplified single-shot return payload (mirrors DocumentsProcessReturnRequestDto). */
export interface ProcessReturnRequest {
  session_token: string;
  source_document_id: string;
  refund_method: "CASH" | "CUSTOMER_CREDIT" | "NO_REFUND";
  items: Array<{
    source_item_id: string;
    quantity_micro: string;
  }>;
  notes: string | null;
}

/** Simplified single-shot return result (mirrors DocumentsProcessReturnResponseDto). */
export interface ProcessReturnResponse {
  return_document_id: string;
  return_document_number: string;
  total_refund_ttc_centimes: string;
  refund_method: string;
}

/**
 * Validate and atomically persist a merchandise return through the
 * simplified facade (operator/terminal/idempotency resolve at the edge).
 * Rejects with the dedicated return taxonomy verbatim
 * (RETURN_QUANTITY_EXCEEDED, INVALID_SOURCE_DOCUMENT_STATE,
 * NO_ACTIVE_CASH_SESSION, MISSING_CUSTOMER_FOR_CREDIT_REFUND,
 * UNAUTHORIZED, EMPTY_RETURN, …).
 */
export async function processSaleReturn(
  payload: ProcessReturnRequest,
): Promise<ProcessReturnResponse> {
  return call<ProcessReturnResponse>("documents_process_return", {
    request: { ...payload },
  });
}

export interface PrinterSettings {
  serialPort: string;
  serialBaud: number;
  paperWidthDots: number;
  paperWidthMm: number;
}

/** Read serial printer configuration. */
export async function ipcGetPrinterSettings(sessionToken: string): Promise<PrinterSettings> {
  interface SettingsWire {
    serial_port: string;
    serial_baud: number;
    paper_width_dots: number;
    paper_width_mm: number;
  }
  const wire = await call<SettingsWire>("printer_get_settings", {
    session_token: sessionToken,
  });
  return {
    serialPort: wire.serial_port,
    serialBaud: wire.serial_baud,
    paperWidthDots: wire.paper_width_dots,
    paperWidthMm: wire.paper_width_mm,
  };
}

/** Persist serial port + baud + paper width (ADMIN only server-side). */
export async function ipcUpdatePrinterSettings(
  sessionToken: string,
  serialPort: string,
  serialBaud: number,
  paperWidthMm: number,
): Promise<PrinterSettings> {
  const wire = await call<{
    serial_port: string;
    serial_baud: number;
    paper_width_dots: number;
    paper_width_mm: number;
  }>("printer_update_settings", {
    request: {
      session_token: sessionToken,
      serial_port: serialPort,
      serial_baud: serialBaud,
      paper_width_mm: paperWidthMm,
    },
  });
  return {
    serialPort: wire.serial_port,
    serialBaud: wire.serial_baud,
    paperWidthDots: wire.paper_width_dots,
    paperWidthMm: wire.paper_width_mm,
  };
}

/** Fiscal stamp settings view (ADR-092: `fiscal.timbre_enabled`, default true). */
export interface FiscalSettings {
  timbreEnabled: boolean;
}

/** Read the fiscal stamp toggle (any authenticated session). */
export async function ipcGetFiscalSettings(sessionToken: string): Promise<FiscalSettings> {
  const wire = await call<{ timbre_enabled: boolean }>("fiscal_settings_get", {
    session_token: sessionToken,
  });
  return { timbreEnabled: wire.timbre_enabled };
}

/** Persist the fiscal stamp toggle (ADMIN only server-side). */
export async function ipcUpdateFiscalSettings(
  sessionToken: string,
  input: FiscalSettings,
): Promise<FiscalSettings> {
  const wire = await call<{ timbre_enabled: boolean }>("fiscal_settings_update", {
    request: { session_token: sessionToken, timbre_enabled: input.timbreEnabled },
  });
  return { timbreEnabled: wire.timbre_enabled };
}

/** Render + transmit a bilingual test page (ADMIN only server-side). */
export async function ipcTestPrint(sessionToken: string): Promise<PrintStatus> {
  const wire = await call<{
    job_id: string;
    status: string;
    attempt_count: string;
  }>("printer_test_print", { session_token: sessionToken });
  return {
    jobId: wire.job_id,
    status: wire.status,
    attemptCount: wire.attempt_count,
  };
}

// Store identity + print diagnostics bridge (TASK-BRK-056) -----------------
// Backend command mapping (sovereign contract):
// - store_identity_get / store_identity_save / printer_test_print —
//   exact backend names. Rust DTOs are canonical: snake_case wire fields,
//   device_kind wire values "physical" | "capture", device_outcome
//   "capture-only" | "sent" | classified device-error code.

/** Store identity: all string fields, empty when unset. */
export interface StoreIdentity {
  storeName: string;
  storeAddress: string;
  storePhone: string;
  nif: string;
  nis: string;
  rc: string;
  ai: string;
  ticketFooterMessage: string;
}

/** Diagnostics device kind (saved setting, selects the test-print path). */
export type DeviceKind = "physical" | "capture";

/** Test-print receipt language, fixed at call time. */
export type TestPrintLanguage = "ar" | "fr";

export interface StoreIdentityState {
  identity: StoreIdentity;
  deviceKind: DeviceKind;
}

/** One monospace preview line (shared layout blueprint, pre-raster). */
export interface PreviewLine {
  kind: string;
  centered: boolean;
  sizePx: number;
  text: string;
}

/** Structural verdict over the captured artifact. */
export interface DecodeSummary {
  bands: number;
  feedLines: number;
  unknownSpans: number;
}

export interface TestPrintResult {
  preview: PreviewLine[];
  decodeSummary: DecodeSummary;
  captureFileName: string;
  deviceOutcome: string;
}

interface StoreIdentityWire {
  store_name: string;
  store_address: string;
  store_phone: string;
  nif: string;
  nis: string;
  rc: string;
  ai: string;
  ticket_footer_message: string;
}

function toIdentity(wire: StoreIdentityWire): StoreIdentity {
  return {
    storeName: wire.store_name,
    storeAddress: wire.store_address,
    storePhone: wire.store_phone,
    nif: wire.nif,
    nis: wire.nis,
    rc: wire.rc,
    ai: wire.ai,
    ticketFooterMessage: wire.ticket_footer_message,
  };
}

function fromIdentity(identity: StoreIdentity): StoreIdentityWire {
  return {
    store_name: identity.storeName,
    store_address: identity.storeAddress,
    store_phone: identity.storePhone,
    nif: identity.nif,
    nis: identity.nis,
    rc: identity.rc,
    ai: identity.ai,
    ticket_footer_message: identity.ticketFooterMessage,
  };
}

/** Read identity + device kind (session-gated server-side). */
export async function getStoreIdentity(): Promise<StoreIdentityState> {
  const wire = await call<{
    identity: StoreIdentityWire;
    device_kind: string;
  }>("store_identity_get", {});
  return {
    identity: toIdentity(wire.identity),
    deviceKind: wire.device_kind === "capture" ? "capture" : "physical",
  };
}

/** Persist identity + device kind (SystemSettingsManage server-side). */
export async function saveStoreIdentity(
  identity: StoreIdentity,
  deviceKind: DeviceKind,
): Promise<StoreIdentityState> {
  const wire = await call<{
    identity: StoreIdentityWire;
    device_kind: string;
  }>("store_identity_save", {
    request: { identity: fromIdentity(identity), device_kind: deviceKind },
  });
  return {
    identity: toIdentity(wire.identity),
    deviceKind: wire.device_kind === "capture" ? "capture" : "physical",
  };
}

/**
 * Synchronous diagnostics run (SystemSettingsManage server-side).
 * Kind is NOT a parameter: the backend uses the SAVED device_kind.
 */
export async function printingTestPrint(language: TestPrintLanguage): Promise<TestPrintResult> {
  const wire = await call<{
    preview: Array<{ kind: string; centered: boolean; size_px: number; text: string }>;
    decode_summary: { bands: number; feed_lines: number; unknown_spans: number };
    capture_file_name: string;
    device_outcome: string;
  }>("printer_test_print", { request: { language } });
  return {
    preview: wire.preview.map((line) => ({
      kind: line.kind,
      centered: line.centered,
      sizePx: line.size_px,
      text: line.text,
    })),
    decodeSummary: {
      bands: wire.decode_summary.bands,
      feedLines: wire.decode_summary.feed_lines,
      unknownSpans: wire.decode_summary.unknown_spans,
    },
    captureFileName: wire.capture_file_name,
    deviceOutcome: wire.device_outcome,
  };
}

export type { Locale };

// Cash session bridge (TASK-BRK-010) ------------------------------------
// Backend command mapping (sovereign contract):
// - cash_open / cash_begin_closing / cash_cancel_closing /
//   cash_confirm_closing / cash_get_active_session / cash_record_expense /
//   cash_get_session_report — exact backend names.

import type { CashMovement, CashSession, CashSessionReport } from "@/types/cash";

interface CashSessionWire {
  id: string;
  warehouse_id: string;
  terminal_id: string;
  status: string;
  opened_by: string;
  opened_by_username: string;
  opened_at: string;
  opening_float_centimes: string;
  closed_by: string | null;
  closed_by_username: string | null;
  closed_at: string | null;
  actual_counted_centimes: string | null;
  system_expected_centimes: string | null;
  variance_centimes: string | null;
  closing_notes: string | null;
  revision: string;
  created_at: string;
  updated_at: string;
}

interface CashMovementWire {
  id: string;
  cash_session_id: string;
  movement_type: string;
  amount_centimes: string;
  reference_type: string | null;
  reference_id: string | null;
  description: string | null;
  created_at: string;
  created_by: string;
  created_by_username: string;
}

function toCashSession(wire: CashSessionWire): CashSession {
  return {
    id: wire.id,
    warehouseId: wire.warehouse_id,
    terminalId: wire.terminal_id,
    status: wire.status as CashSession["status"],
    openedBy: wire.opened_by,
    openedByUsername: wire.opened_by_username,
    openedAt: wire.opened_at,
    openingFloatCentimes: wire.opening_float_centimes,
    closedBy: wire.closed_by,
    closedByUsername: wire.closed_by_username,
    closedAt: wire.closed_at,
    actualCountedCentimes: wire.actual_counted_centimes,
    systemExpectedCentimes: wire.system_expected_centimes,
    varianceCentimes: wire.variance_centimes,
    closingNotes: wire.closing_notes,
    revision: wire.revision,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

function toCashMovement(wire: CashMovementWire): CashMovement {
  return {
    id: wire.id,
    cashSessionId: wire.cash_session_id,
    movementType: wire.movement_type,
    amountCentimes: wire.amount_centimes,
    referenceType: wire.reference_type,
    referenceId: wire.reference_id,
    description: wire.description,
    createdAt: wire.created_at,
    createdBy: wire.created_by,
    createdByUsername: wire.created_by_username,
  };
}

/** Open a cash session with an opening float (centime string). */
export async function ipcCashOpen(
  sessionToken: string,
  warehouseId: string,
  openingFloatCentimes: string,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_open", {
    request: {
      session_token: sessionToken,
      warehouse_id: warehouseId,
      opening_float_centimes: openingFloatCentimes,
      command_id: commandId,
    },
  });
  return toCashSession(wire);
}

/** Freeze the expected total (OPEN -> CLOSING). */
export async function ipcCashBeginClosing(
  sessionToken: string,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_begin_closing", {
    request: { session_token: sessionToken, command_id: commandId },
  });
  return toCashSession(wire);
}

/** Revert CLOSING -> OPEN. */
export async function ipcCashCancelClosing(
  sessionToken: string,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_cancel_closing", {
    request: { session_token: sessionToken, command_id: commandId },
  });
  return toCashSession(wire);
}

/** Count and seal (CLOSING -> CLOSED). */
export async function ipcCashConfirmClosing(
  sessionToken: string,
  sessionId: string,
  actualCountedCentimes: string,
  closingNotes: string | null,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_confirm_closing", {
    request: {
      session_token: sessionToken,
      session_id: sessionId,
      actual_counted_centimes: actualCountedCentimes,
      closing_notes: closingNotes,
      command_id: commandId,
    },
  });
  return toCashSession(wire);
}

/** Active OPEN/CLOSING session, or null when none (token injected by the wrapper). */
export async function ipcCashGetActiveSession(): Promise<CashSession | null> {
  const wire = await call<CashSessionWire | null>("cash_get_active_session", {});
  return wire === null ? null : toCashSession(wire);
}

/** Record a drawer expense (positive centime string; stored negative). */
export async function ipcCashRecordExpense(
  sessionToken: string,
  amountCentimes: string,
  description: string,
  commandId: string,
): Promise<CashMovement> {
  const wire = await call<CashMovementWire>("cash_record_expense", {
    request: {
      session_token: sessionToken,
      amount_centimes: amountCentimes,
      description,
      command_id: commandId,
    },
  });
  return toCashMovement(wire);
}

/** Session report (interim X-Report or final Z-Report). */
export async function ipcCashGetSessionReport(
  sessionToken: string,
  sessionId: string,
): Promise<CashSessionReport> {
  interface ReportWire {
    session: CashSessionWire;
    movements: CashMovementWire[];
    totals_by_type: Array<{ movement_type: string; total_centimes: string; count: string }>;
    is_final: boolean;
  }
  const wire = await call<ReportWire>("cash_get_session_report", {
    request: { session_token: sessionToken, session_id: sessionId },
  });
  return {
    session: toCashSession(wire.session),
    movements: wire.movements.map(toCashMovement),
    totalsByType: wire.totals_by_type.map((total) => ({
      movementType: total.movement_type,
      totalCentimes: total.total_centimes,
      count: total.count,
    })),
    isFinal: wire.is_final,
  };
}

// ---------------------------------------------------------------------
// Supplier & purchase bridge (TASK-BRK-011).
// Backend command mapping (sovereign contract):
//   suppliers_create / suppliers_update / suppliers_get / suppliers_search /
//   suppliers_get_statement / purchases_create_order / purchases_update_order /
//   purchases_receive_goods / purchases_record_invoice /
//   purchases_record_payment / purchases_get_document - exact backend names.
// All money crosses as centime strings; all quantities as micro strings.

import type {
  PurchaseDocumentView,
  PurchaseLineInput,
  SupplierFormInput,
  SupplierLedgerEntry,
  SupplierPaymentOutcome,
  SupplierStatement,
  SupplierUpdateInput,
  SupplierView,
} from "@/types/supplier";

interface SupplierWire {
  id: string;
  name: string;
  phone: string | null;
  address: string | null;
  nif: string | null;
  rc: string | null;
  notes: string | null;
  is_active: boolean;
  current_balance_centimes: string;
  origin_device_id: string;
  created_at: string;
  updated_at: string;
}

interface PurchaseLineWire {
  id: string;
  product_id: string;
  unit_id: string;
  quantity_micro: string;
  quantity_base_micro: string;
  unit_cost_centimes: string;
  line_total_centimes: string;
}

interface PurchaseDocumentWire {
  id: string;
  number: string;
  doc_type: string;
  status: string;
  supplier_id: string;
  warehouse_id: string;
  source_document_id: string | null;
  total_centimes: string;
  lines: PurchaseLineWire[];
  created_at: string;
  created_by: string;
}

interface SupplierLedgerEntryWire {
  id: string;
  entry_type: string;
  amount_centimes: string;
  reference_type: string;
  reference_id: string;
  description: string | null;
  created_at: string;
  created_by: string;
  balance_after_centimes: string;
}

interface SupplierStatementWire {
  entries: SupplierLedgerEntryWire[];
  current_balance_centimes: string;
}

interface SupplierPaymentWire {
  ledger_entry_id: string;
  new_balance_centimes: string;
  cash_movement_id: string | null;
}

function toSupplier(wire: SupplierWire): SupplierView {
  return {
    id: wire.id,
    name: wire.name,
    phone: wire.phone,
    address: wire.address,
    nif: wire.nif,
    rc: wire.rc,
    notes: wire.notes,
    isActive: wire.is_active,
    currentBalanceCentimes: wire.current_balance_centimes,
    originDeviceId: wire.origin_device_id,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

function toPurchaseLine(wire: PurchaseLineWire) {
  return {
    id: wire.id,
    productId: wire.product_id,
    unitId: wire.unit_id,
    quantityMicro: wire.quantity_micro,
    quantityBaseMicro: wire.quantity_base_micro,
    unitCostCentimes: wire.unit_cost_centimes,
    lineTotalCentimes: wire.line_total_centimes,
  };
}

function toPurchaseDocument(wire: PurchaseDocumentWire): PurchaseDocumentView {
  return {
    id: wire.id,
    number: wire.number,
    docType: wire.doc_type,
    status: wire.status,
    supplierId: wire.supplier_id,
    warehouseId: wire.warehouse_id,
    sourceDocumentId: wire.source_document_id,
    totalCentimes: wire.total_centimes,
    lines: wire.lines.map(toPurchaseLine),
    createdAt: wire.created_at,
    createdBy: wire.created_by,
  };
}

function toLedgerEntry(wire: SupplierLedgerEntryWire): SupplierLedgerEntry {
  return {
    id: wire.id,
    entryType: wire.entry_type,
    amountCentimes: wire.amount_centimes,
    referenceType: wire.reference_type,
    referenceId: wire.reference_id,
    description: wire.description,
    createdAt: wire.created_at,
    createdBy: wire.created_by,
    balanceAfterCentimes: wire.balance_after_centimes,
  };
}

function purchaseLineWire(line: PurchaseLineInput) {
  return {
    product_id: line.productId,
    unit_id: line.unitId,
    quantity_micro: line.quantityMicro,
    unit_cost_centimes: line.unitCostCentimes,
  };
}

/** Register a supplier (Gerant/Admin). */
export async function ipcCreateSupplier(
  sessionToken: string,
  input: SupplierFormInput,
): Promise<SupplierView> {
  const wire = await call<SupplierWire>("suppliers_create", {
    request: {
      session_token: sessionToken,
      name: input.name,
      phone: input.phone,
      address: input.address,
      nif: input.nif,
      rc: input.rc,
      notes: input.notes,
    },
  });
  return toSupplier(wire);
}

/** Sparse supplier update (`null` clears a field; omitted keeps it). */
export async function ipcUpdateSupplier(
  sessionToken: string,
  supplierId: string,
  input: SupplierUpdateInput,
): Promise<SupplierView> {
  const wire = await call<SupplierWire>("suppliers_update", {
    request: {
      session_token: sessionToken,
      supplier_id: supplierId,
      name: input.name,
      phone: input.phone,
      address: input.address,
      nif: input.nif,
      rc: input.rc,
      notes: input.notes,
      is_active: input.isActive,
    },
  });
  return toSupplier(wire);
}

/** Full supplier view by id. */
export async function ipcGetSupplier(
  sessionToken: string,
  supplierId: string,
): Promise<SupplierView> {
  const wire = await call<SupplierWire>("suppliers_get", {
    request: { session_token: sessionToken, id: supplierId },
  });
  return toSupplier(wire);
}

/** Dual-track safe supplier deletion: purge when unreferenced, deactivate on history. */
export async function ipcDeleteSupplier(
  sessionToken: string,
  supplierId: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("suppliers_delete", {
    request: { session_token: sessionToken, id: supplierId },
  });
}

/** Supplier directory search (blank query returns no rows). */
export async function ipcSearchSuppliers(
  sessionToken: string,
  query: string,
  limit: number,
): Promise<SupplierView[]> {
  interface SearchWire {
    suppliers: SupplierWire[];
  }
  const wire = await call<SearchWire>("suppliers_search", {
    request: { session_token: sessionToken, query, limit },
  });
  return wire.suppliers.map(toSupplier);
}

/** Supplier statement with server-side running balances. */
export async function ipcGetSupplierStatement(
  sessionToken: string,
  supplierId: string,
): Promise<SupplierStatement> {
  const wire = await call<SupplierStatementWire>("suppliers_get_statement", {
    request: { session_token: sessionToken, id: supplierId },
  });
  return {
    entries: wire.entries.map(toLedgerEntry),
    currentBalanceCentimes: wire.current_balance_centimes,
  };
}

/** Create a DRAFT purchase order (BC). */
export async function ipcCreatePurchaseOrder(
  sessionToken: string,
  commandId: string,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_create_order", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
    },
  });
  return toPurchaseDocument(wire);
}

/** Replace the line set of a DRAFT purchase order. */
export async function ipcUpdatePurchaseOrder(
  sessionToken: string,
  commandId: string,
  documentId: string,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_update_order", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      document_id: documentId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
    },
  });
  return toPurchaseDocument(wire);
}

/** Receive goods (BR); optional source BC id, null for standalone. */
export async function ipcReceiveGoods(
  sessionToken: string,
  commandId: string,
  sourceBcId: string | null,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_receive_goods", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      source_bc_id: sourceBcId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
    },
  });
  return toPurchaseDocument(wire);
}

/** Record a supplier invoice (FF); optional source BR (ADR-029). */
export async function ipcRecordSupplierInvoice(
  sessionToken: string,
  commandId: string,
  sourceBrId: string | null,
  supplierId: string,
  warehouseId: string,
  lines: PurchaseLineInput[],
  invoiceTotalCentimes: string,
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_record_invoice", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      source_br_id: sourceBrId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      lines: lines.map(purchaseLineWire),
      invoice_total_centimes: invoiceTotalCentimes,
    },
  });
  return toPurchaseDocument(wire);
}

/** Record a supplier payment (CASH or OTHER, ADR-030). */
export async function ipcRecordSupplierPayment(
  sessionToken: string,
  commandId: string,
  supplierId: string,
  amountCentimes: string,
  paymentMethod: "CASH" | "OTHER",
  description: string | null,
): Promise<SupplierPaymentOutcome> {
  const wire = await call<SupplierPaymentWire>("purchases_record_payment", {
    request: {
      session_token: sessionToken,
      command_id: commandId,
      supplier_id: supplierId,
      amount_centimes: amountCentimes,
      payment_method: paymentMethod,
      description,
    },
  });
  return {
    ledgerEntryId: wire.ledger_entry_id,
    newBalanceCentimes: wire.new_balance_centimes,
    cashMovementId: wire.cash_movement_id,
  };
}

/** Load any purchase document (BC/BR/FF) by id. */
export async function ipcGetPurchaseDocument(
  sessionToken: string,
  documentId: string,
): Promise<PurchaseDocumentView> {
  const wire = await call<PurchaseDocumentWire>("purchases_get_document", {
    request: { session_token: sessionToken, id: documentId },
  });
  return toPurchaseDocument(wire);
}

// ---------------------------------------------------------------------
// Reports bridge (TASK-BRK-012B).
// Backend command mapping (sovereign contract):
//   reports_daily_sales / reports_sales_by_product /
//   reports_sales_by_category / reports_gross_margin /
//   reports_credit_outstanding / reports_low_stock /
//   reports_stock_movement_history / reports_cash_sessions_history /
//   reports_user_activity - exact backend names.
// All money crosses as centime strings; quantities as micro strings.

import type {
  CashSessionHistoryRow,
  CreditOutstandingRow,
  DailySalesRow,
  GrossMarginRow,
  LowStockRow,
  ReportDateRange,
  SalesByCategoryRow,
  SalesByProductRow,
  StockMovementFilters,
  StockMovementRow,
  UserActivityFilters,
  UserActivityRow,
} from "@/types/reports";

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

/** Backup trigger selector (mirrors the Rust trigger codes). */
export type BackupTriggerType = "MANUAL" | "SHIFT_CLOSE" | "SCHEDULED";

/** Backup metadata view (mirrors BackupMetadataDto). */
export interface BackupMetadata {
  id: string;
  fileName: string;
  fileSizeBytes: number;
  sha256Checksum: string;
  triggerType: string;
  status: string;
  schemaVersion: number;
  errorMessage: string | null;
  createdAt: string;
}

interface BackupMetadataWire {
  id: string;
  file_name: string;
  file_size_bytes: number;
  sha256_checksum: string;
  trigger_type: string;
  status: string;
  schema_version: number;
  error_message: string | null;
  created_at: string;
}

function toBackupMetadata(wire: BackupMetadataWire): BackupMetadata {
  return {
    id: wire.id,
    fileName: wire.file_name,
    fileSizeBytes: wire.file_size_bytes,
    sha256Checksum: wire.sha256_checksum,
    triggerType: wire.trigger_type,
    status: wire.status,
    schemaVersion: wire.schema_version,
    errorMessage: wire.error_message,
    createdAt: wire.created_at,
  };
}

/**
 * Capture a verified online backup snapshot (Admin-only server-side).
 * Rejects with the backend wire code verbatim (UNAUTHORIZED,
 * BACKUP_VERIFICATION_FAILED, …).
 */
export async function createBackup(
  sessionToken: string,
  triggerType?: BackupTriggerType,
): Promise<BackupMetadata> {
  const wire = await call<BackupMetadataWire>("backups_create", {
    request: {
      session_token: sessionToken,
      trigger_type: triggerType ?? "MANUAL",
    },
  });
  return toBackupMetadata(wire);
}

/**
 * List recent backup audit rows, newest first (Admin-only server-side).
 */
export async function listBackups(sessionToken: string): Promise<BackupMetadata[]> {
  const wire = await call<BackupMetadataWire[]>("backups_list", {
    session_token: sessionToken,
  });
  return wire.map(toBackupMetadata);
}

/** Backup verification outcome (mirrors VerifyBackupResponseDto). */
export interface VerifyBackupResponse {
  backupId: string;
  isValid: boolean;
  sha256Checksum: string;
  status: string;
  message: string | null;
}

/**
 * Re-verify one logged snapshot on disk (Admin-only server-side).
 * Transport-level failures reject; content failures surface as
 * BACKUP_* wire codes (BACKUP_MISSING, BACKUP_CHECKSUM_MISMATCH,
 * BACKUP_CORRUPTED).
 */
export async function verifyBackup(
  sessionToken: string,
  backupId: string,
): Promise<VerifyBackupResponse> {
  interface VerifyBackupWire {
    backup_id: string;
    is_valid: boolean;
    sha256_checksum: string;
    status: string;
    message: string | null;
  }
  const wire = await call<VerifyBackupWire>("backups_verify", {
    request: { session_token: sessionToken, backup_id: backupId },
  });
  return {
    backupId: wire.backup_id,
    isValid: wire.is_valid,
    sha256Checksum: wire.sha256_checksum,
    status: wire.status,
    message: wire.message,
  };
}

/** One of the eight restore verification gates (mirrors PreRestoreChecksDto). */
export interface PreRestoreChecks {
  fileExists: boolean;
  sha256Matches: boolean;
  integrityOk: boolean;
  foreignKeysOk: boolean;
  auditChainOk: boolean;
  customerLedgerOk: boolean;
  supplierLedgerOk: boolean;
  stockLevelsOk: boolean;
}

/** Non-blocking restore advisory (mirrors RestoreWarningDto). */
export type RestoreWarning =
  | { type: "open_cash_session_in_backup"; openedAt: string }
  | { type: "older_schema_will_migrate"; fromVersion: number; toVersion: number }
  | { type: "newer_activity_will_be_quarantined"; documentCount: number };

/** Restore ineligibility reason (mirrors RestoreIneligibleReasonDto). */
export type RestoreIneligibleReason =
  | { type: "backup_file_missing" }
  | { type: "hash_mismatch" }
  | { type: "integrity_check_failed" }
  | { type: "foreign_key_violation" }
  | { type: "audit_chain_broken" }
  | { type: "customer_ledger_mismatch"; mismatchCount: number }
  | { type: "supplier_ledger_mismatch"; mismatchCount: number }
  | { type: "stock_level_mismatch"; mismatchCount: number }
  | { type: "schema_newer_than_app"; backupVersion: number; appVersion: number };

/** Backup content summary (mirrors BackupContentSummaryDto). */
export interface BackupContentSummary {
  products: number;
  customers: number;
  documentsTotal: number;
  lastDocumentAt: string | null;
}

/** Full read-only precheck report (mirrors PreRestoreReportDto). */
export interface PreRestoreReport {
  backupId: string;
  fileName: string;
  fileSizeBytes: number;
  createdAt: string;
  schemaVersionBackup: number;
  schemaVersionApp: number;
  checks: PreRestoreChecks;
  contentSummary: BackupContentSummary;
  warnings: RestoreWarning[];
  eligible: boolean;
  ineligibleReasons: RestoreIneligibleReason[];
}

/** Verified restore outcome (mirrors RestoreOutcomeDto). */
export interface RestoreOutcome {
  restoredBackupId: string;
  quarantinedTo: string;
  restoredAt: string;
  restartRequired: boolean;
}

type RestoreWarningWire =
  | { type: "open_cash_session_in_backup"; opened_at: string }
  | { type: "older_schema_will_migrate"; from_version: number; to_version: number }
  | { type: "newer_activity_will_be_quarantined"; document_count: number };

type RestoreIneligibleReasonWire =
  | { type: "backup_file_missing" }
  | { type: "hash_mismatch" }
  | { type: "integrity_check_failed" }
  | { type: "foreign_key_violation" }
  | { type: "audit_chain_broken" }
  | { type: "customer_ledger_mismatch"; mismatch_count: number }
  | { type: "supplier_ledger_mismatch"; mismatch_count: number }
  | { type: "stock_level_mismatch"; mismatch_count: number }
  | { type: "schema_newer_than_app"; backup_version: number; app_version: number };

interface PreRestoreReportWire {
  backup_id: string;
  file_name: string;
  file_size_bytes: number;
  created_at: string;
  schema_version_backup: number;
  schema_version_app: number;
  checks: {
    file_exists: boolean;
    sha256_matches: boolean;
    integrity_ok: boolean;
    foreign_keys_ok: boolean;
    audit_chain_ok: boolean;
    customer_ledger_ok: boolean;
    supplier_ledger_ok: boolean;
    stock_levels_ok: boolean;
  };
  content_summary: {
    products: number;
    customers: number;
    documents_total: number;
    last_document_at: string | null;
  };
  warnings: RestoreWarningWire[];
  eligible: boolean;
  ineligible_reasons: RestoreIneligibleReasonWire[];
}

function toRestoreWarning(wire: RestoreWarningWire): RestoreWarning {
  switch (wire.type) {
    case "open_cash_session_in_backup":
      return { type: wire.type, openedAt: wire.opened_at };
    case "older_schema_will_migrate":
      return { type: wire.type, fromVersion: wire.from_version, toVersion: wire.to_version };
    case "newer_activity_will_be_quarantined":
      return { type: wire.type, documentCount: wire.document_count };
  }
}

function toRestoreIneligibleReason(wire: RestoreIneligibleReasonWire): RestoreIneligibleReason {
  switch (wire.type) {
    case "backup_file_missing":
    case "hash_mismatch":
    case "integrity_check_failed":
    case "foreign_key_violation":
    case "audit_chain_broken":
      return { type: wire.type };
    case "customer_ledger_mismatch":
    case "supplier_ledger_mismatch":
    case "stock_level_mismatch":
      return { type: wire.type, mismatchCount: wire.mismatch_count };
    case "schema_newer_than_app":
      return { type: wire.type, backupVersion: wire.backup_version, appVersion: wire.app_version };
  }
}

function toPreRestoreReport(wire: PreRestoreReportWire): PreRestoreReport {
  return {
    backupId: wire.backup_id,
    fileName: wire.file_name,
    fileSizeBytes: wire.file_size_bytes,
    createdAt: wire.created_at,
    schemaVersionBackup: wire.schema_version_backup,
    schemaVersionApp: wire.schema_version_app,
    checks: {
      fileExists: wire.checks.file_exists,
      sha256Matches: wire.checks.sha256_matches,
      integrityOk: wire.checks.integrity_ok,
      foreignKeysOk: wire.checks.foreign_keys_ok,
      auditChainOk: wire.checks.audit_chain_ok,
      customerLedgerOk: wire.checks.customer_ledger_ok,
      supplierLedgerOk: wire.checks.supplier_ledger_ok,
      stockLevelsOk: wire.checks.stock_levels_ok,
    },
    contentSummary: {
      products: wire.content_summary.products,
      customers: wire.content_summary.customers,
      documentsTotal: wire.content_summary.documents_total,
      lastDocumentAt: wire.content_summary.last_document_at,
    },
    warnings: wire.warnings.map(toRestoreWarning),
    eligible: wire.eligible,
    ineligibleReasons: wire.ineligible_reasons.map(toRestoreIneligibleReason),
  };
}

/**
 * Full read-only verification report for one logged backup
 * (Admin-only server-side). Rejects with the backend wire code verbatim
 * (UNAUTHORIZED, BACKUP_NOT_FOUND, BACKUP_MISSING, …).
 */
export async function precheckRestoreBackup(
  sessionToken: string,
  backupId: string,
): Promise<PreRestoreReport> {
  const wire = await call<PreRestoreReportWire>("backups_restore_precheck", {
    request: { session_token: sessionToken, backup_id: backupId },
  });
  return toPreRestoreReport(wire);
}

/**
 * Verified destructive restore with quarantine and rollback safety
 * (Admin-only server-side). The backend re-verifies everything
 * internally; transport failures reject with the wire code verbatim
 * (BACKUP_CHECKSUM_MISMATCH, BACKUP_CORRUPTED, RESTORE_SWAP_FAILED, …).
 */
export async function restoreBackup(
  sessionToken: string,
  backupId: string,
): Promise<RestoreOutcome> {
  interface RestoreOutcomeWire {
    restored_backup_id: string;
    quarantined_to: string;
    restored_at: string;
    restart_required: boolean;
  }
  const wire = await call<RestoreOutcomeWire>("backups_restore", {
    request: { session_token: sessionToken, backup_id: backupId },
  });
  return {
    restoredBackupId: wire.restored_backup_id,
    quarantinedTo: wire.quarantined_to,
    restoredAt: wire.restored_at,
    restartRequired: wire.restart_required,
  };
}
