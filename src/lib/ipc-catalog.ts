/**
 * Catalog bridge: products, quantity-break ladders, taxonomy, bulk import.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type {
  CatalogProduct,
  Category,
  ImportProductRow,
  ImportProductsReport,
  PriceBreakRung,
} from "@/types/pos";
import { call, type DeleteEntityResult } from "./ipc-kernel";

/** Fetch one full product view by UUID (draft resume rehydration). */
export async function ipcGetProduct(
  sessionToken: string,
  productId: string,
): Promise<CatalogProduct> {
  return call<CatalogProduct>("catalog_get_product", {
    request: { session_token: sessionToken, product_id: productId },
  });
}

/** Dual-track safe product deletion: purge when unreferenced, deactivate on history. */
export async function ipcDeleteProduct(
  sessionToken: string,
  productId: string,
  reason: string,
): Promise<DeleteEntityResult> {
  return call<DeleteEntityResult>("catalog_delete_product", {
    request: { session_token: sessionToken, id: productId, reason },
  });
}

/** Database-level catalog aggregates for the management hub KPIs. */
export interface CatalogMetricsDto {
  total_products: number;
  in_stock: number;
  out_of_stock: number;
  inactive: number;
}

/** Prefix/multilingual catalog search (150ms debounce applied by caller). */
export async function ipcSearchCatalog(
  sessionToken: string,
  query: string,
  limit = 20,
  includeInactive = false,
): Promise<CatalogProduct[]> {
  return call<CatalogProduct[]>("catalog_search", {
    request: { session_token: sessionToken, query, limit, include_inactive: includeInactive },
  });
}

/** One quantity-break rung as carried on the wire (BRK-121). */
export interface PriceBreakWire {
  min_qty_micro: string;
  price_centimes: string;
}

/** Quantity-break ladder for one product x unit, ascending by rung. */
export async function ipcListPriceBreaks(
  sessionToken: string,
  productId: string,
  unitId: string,
): Promise<PriceBreakWire[]> {
  return call<PriceBreakWire[]>("catalog_list_price_breaks", {
    request: { session_token: sessionToken, product_id: productId, unit_id: unitId },
  });
}

/** Atomically replace the ladder (empty clears; BRK-121b). Managers only. */
export async function ipcReplacePriceBreaks(
  sessionToken: string,
  productId: string,
  unitId: string,
  rungs: PriceBreakRung[],
): Promise<void> {
  await call<unknown>("catalog_replace_price_breaks", {
    request: {
      session_token: sessionToken,
      product_id: productId,
      unit_id: unitId,
      rungs: rungs.map((rung) => ({
        min_qty_micro: rung.minQtyMicro,
        price_centimes: rung.priceCentimes,
      })),
    },
  });
}

/** Database-level catalog aggregates (total / in-stock / out-of-stock / inactive). */
export async function ipcGetCatalogMetrics(sessionToken: string): Promise<CatalogMetricsDto> {
  return call<CatalogMetricsDto>("catalog_get_metrics", {
    session_token: sessionToken,
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
  /** Bulk price; omit to leave unchanged (`null` clears, BRK-117). */
  super_gros_price_centimes?: string | null;
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
  /** Bulk price; omit to leave unchanged (`null` clears, BRK-117). */
  super_gros_price_centimes?: string | null;
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
  onDuplicateSku?: "skip" | "error" | "update",
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

/** One per-cell override at matrix generation (mirrors VariantCellOverrideDto). */
export interface VariantCellOverrideWire {
  axis1: string;
  axis2: string;
  /** Replacement sale price in centimes, canonical string (`None` = inherit). */
  sale_price_centimes: string | null;
  barcode: string | null;
}

/** Generation input (mirrors GenerateVariantMatrixDto). Money as strings. */
export interface GenerateVariantMatrixWire {
  name_ar: string;
  name_fr: string;
  axis1_name: string;
  axis2_name: string;
  axis1_values: string[];
  axis2_values: string[];
  base_sku: string;
  category_id: string | null;
  template_product_id: string;
  overrides: VariantCellOverrideWire[];
}

/** Generation result: the model id plus every minted SKU (BRK-114). */
export interface GenerateVariantMatrixResult {
  model_id: string;
  cell_count: number;
  skus: string[];
}

/**
 * Atomically fan a template product out into one product per
 * (`axis1`, `axis2`) cell. Rejects verbatim on bridge errors.
 */
export async function ipcGenerateVariantMatrix(
  sessionToken: string,
  input: GenerateVariantMatrixWire,
): Promise<GenerateVariantMatrixResult> {
  return call<GenerateVariantMatrixResult>("catalog_generate_variant_matrix", {
    request: { session_token: sessionToken, ...input },
  });
}

/** One 2D-grid row (mirrors VariantMatrixViewRowDto). Micros/prices as strings. */
export interface VariantMatrixRowWire {
  axis1_value: string;
  axis2_value: string;
  product_id: string;
  sku: string;
  name_ar: string;
  barcode: string | null;
  on_hand_micro: string;
  sale_price_centimes: string | null;
  default_unit: MatrixGridUnitWire | null;
}

/** Default-selling unit riding a 2D-grid row (DSN-108). One row = one cart line. */
export interface MatrixGridUnitWire {
  unit_id: string;
  unit_name_ar: string;
  unit_name_fr: string;
  factor_micro: string;
  unit_barcode: string | null;
  wholesale_price_centimes: string | null;
  super_gros_price_centimes: string | null;
  is_base_unit: boolean;
}

/** Read a model's 2D grid in display order (DSN-108/109 feed). */
export async function ipcMatrixView(
  sessionToken: string,
  modelId: string,
): Promise<VariantMatrixRowWire[]> {
  return call<VariantMatrixRowWire[]>("catalog_matrix_view", {
    request: { session_token: sessionToken, model_id: modelId },
  });
}

/** Variant model descriptor for the POS grid picker (DSN-108). */
export interface VariantModelWire {
  model_id: string;
  name_ar: string;
  name_fr: string;
  axis1_name: string;
  axis2_name: string;
}

/** List variant models for the POS grid picker. */
export async function ipcMatrixModels(
  sessionToken: string,
): Promise<VariantModelWire[]> {
  return call<VariantModelWire[]>("catalog_matrix_models", {
    request: { session_token: sessionToken },
  });
}

/** One série line: a first-axis value plus pieces per single série (BRK-115). */
export interface SerieLineWire {
  axis1_value: string;
  pieces: number;
}

/** Série composition input (mirrors ComposeSerieDto). Micros as strings. */
export interface ComposeSerieWire {
  kit_id: string;
  model_id: string;
  axis2_value: string;
  lines: SerieLineWire[];
  warehouse_id: string | null;
  serie_count: number;
}

/** One reported shortfall (mirrors SerieShortfallDto). */
export interface SerieShortfallWire {
  product_id: string;
  sku: string;
  required_micro: string;
  on_hand_micro: string;
}

/** Série composition result: persisted kit plus the availability report (BRK-115). */
export interface ComposeSerieResult {
  kit_id: string;
  component_count: number;
  total_pieces: number;
  shortfalls: SerieShortfallWire[];
}

/**
 * Resolve a size run into a kit and report availability tolerantly.
 * Rejects verbatim on bridge errors.
 */
export async function ipcComposeSerie(
  sessionToken: string,
  input: ComposeSerieWire,
): Promise<ComposeSerieResult> {
  return call<ComposeSerieResult>("catalog_compose_serie", {
    request: { session_token: sessionToken, ...input },
  });
}
