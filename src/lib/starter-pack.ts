/**
 * Quincaillerie starter-pack lifecycle service (TASK-DSN-052, Mod 4).
 *
 * Pure CSV parsing delegates to the production import parser
 * (`parseCatalogCsv`: BOM/`;`/decimal handling, BigInt-safe integers —
 * zero floats). Installation provisions each item into the target depot
 * through injected create/adjust deps (sequential, contention-free);
 * uninstallation aggregates the dual-track `ipcDeleteProduct` modes.
 * The only side channel is a guarded `localStorage` installed flag.
 */

import { parseCatalogCsv } from "@/features/catalog/catalog-import-parser";
import type { CatalogProduct, Category } from "@/types/pos";
import type {
  CreateProductWire,
  DeleteEntityResult,
  StockAdjustPayload,
} from "@/lib/ipc";

/** Canonical pack size (the CSV ships exactly 300 rows). */
export const STARTER_PACK_SIZE = 300;

/** Progress chunk: UI yields between chunks to stay responsive. */
export const STARTER_INSTALL_CHUNK = 5;

/** Guarded `localStorage` installed flag. */
export const STARTER_PACK_FLAG = "baraka_starter_pack_installed";

/** Base selling unit for pack items (single piece, factor 1.0000). */
export const STARTER_UNIT_FR = "Pièce";
export const STARTER_UNIT_AR = "قطعة";
export const STARTER_FACTOR_MICRO = "10000";

/** Provisioning reason stamped on the initial INCREASE movement. */
export const STARTER_PROVISION_REASON = "Stock initial pack quincaillerie";

export interface StarterPackItem {
  code: string;
  nameFr: string;
  nameAr: string;
  categoryId: string | null;
  categoryName: string;
  unitNameFr: string;
  unitNameAr: string;
  purchasePriceCentimes: bigint;
  salePriceCentimes: bigint;
  initialStockMicro: bigint;
  barcode: string | null;
}

export interface StarterPackStatus {
  isInstalled: boolean;
  installedCount: number;
  totalPackCount: number;
}

export interface UninstallSummary {
  hardDeleted: number;
  deactivated: number;
  failed: number;
  totalProcessed: number;
}

export interface InstallOutcome {
  created: number;
  skipped: number;
}

/** Injected edges for installation (live IPC in the banner, mocks in tests). */
export interface InstallDeps {
  create: (token: string, wire: CreateProductWire) => Promise<CatalogProduct>;
  adjust: (payload: StockAdjustPayload) => Promise<unknown>;
}

/** Injected edge for uninstallation. */
export interface UninstallDeps {
  remove: (token: string, id: string, reason: string) => Promise<DeleteEntityResult>;
}

export type ProgressCallback = (current: number, total: number) => void;

/**
 * Parse the starter-pack CSV into install-ready items. Rows rejected by
 * the import parser are dropped (never thrown). Pure.
 */
export function parseStarterPackCsv(
  csvText: string,
  categoriesByCodeOrId?: Map<string, string>,
  categoryNames?: Map<string, { fr: string; ar: string }>,
): StarterPackItem[] {
  const { rows } = parseCatalogCsv(csvText, categoriesByCodeOrId);
  const items: StarterPackItem[] = [];
  for (const row of rows) {
    if (row.sku === undefined || row.sku === null || row.sku.trim() === "") {
      continue;
    }
    const names = categoryNames?.get(row.category_id ?? "");
    items.push({
      code: row.sku,
      nameFr: row.name_fr,
      nameAr: row.name_ar ?? row.name_fr,
      categoryId: row.category_id ?? null,
      categoryName: names?.fr ?? "",
      unitNameFr: row.base_unit_name_fr ?? STARTER_UNIT_FR,
      unitNameAr: row.base_unit_name_ar ?? STARTER_UNIT_AR,
      purchasePriceCentimes: BigInt(row.purchase_price_centimes ?? 0),
      salePriceCentimes: BigInt(row.sale_price_centimes),
      initialStockMicro: BigInt(row.initial_quantity_micro ?? 0),
      barcode:
        row.barcode === undefined || row.barcode === null || row.barcode === "" ? null : row.barcode,
    });
  }
  return items;
}

/** Build a category code/upper-code → id lookup from listed categories. Pure. */
export function buildCategoryLookup(categories: Category[]): Map<string, string> {
  const lookup = new Map<string, string>();
  for (const category of categories) {
    lookup.set(category.code, category.id);
    lookup.set(category.code.toUpperCase(), category.id);
  }
  return lookup;
}

/**
 * Dynamic installation detection: match pack codes against catalog SKUs.
 * Installed when at least one pack code is present. Pure.
 */
export function checkStarterPackStatus(
  items: StarterPackItem[],
  catalogSkus: ReadonlySet<string> | Array<{ sku: string }>,
): StarterPackStatus {
  const has = (code: string): boolean =>
    Array.isArray(catalogSkus)
      ? catalogSkus.some((row) => row.sku === code)
      : catalogSkus.has(code);
  let installedCount = 0;
  for (const item of items) {
    if (has(item.code)) {
      installedCount += 1;
    }
  }
  return { isInstalled: installedCount > 0, installedCount, totalPackCount: items.length };
}

function toCreateWire(item: StarterPackItem): CreateProductWire {
  return {
    sku: item.code,
    barcode: item.barcode,
    name_ar: item.nameAr,
    name_fr: item.nameFr,
    category_id: item.categoryId,
    is_kit: false,
    units: [
      {
        unit_name_ar: item.unitNameAr,
        unit_name_fr: item.unitNameFr,
        factor_micro: STARTER_FACTOR_MICRO,
        barcode: item.barcode,
        sale_price_centimes: item.salePriceCentimes.toString(),
        purchase_price_centimes: item.purchasePriceCentimes.toString(),
        wholesale_price_centimes: null,
        is_base_unit: true,
        is_default_selling: true,
      },
    ],
  };
}

/** Yield to the UI event loop between chunks (keeps progress fluid). */
function yieldToUi(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

/**
 * Install pack items sequentially into the target depot. SKUs already in
 * `existingSkus` (or failing mid-flight) count as skipped — the batch
 * never aborts. Progress fires per item.
 */
export async function installStarterPack(
  token: string,
  items: StarterPackItem[],
  targetWarehouseId: string,
  existingSkus: ReadonlySet<string>,
  deps: InstallDeps,
  onProgress?: ProgressCallback,
): Promise<InstallOutcome> {
  let created = 0;
  let skipped = 0;
  for (let index = 0; index < items.length; index += 1) {
    const item = items[index];
    if (item === undefined) {
      continue;
    }
    if (existingSkus.has(item.code)) {
      skipped += 1;
    } else {
      try {
        const product = await deps.create(token, toCreateWire(item));
        if (item.initialStockMicro > 0n && targetWarehouseId !== "") {
          const base =
            product.units.find((unit) => unit.is_base_unit) ?? product.units[0];
          if (base !== undefined) {
            const payload: StockAdjustPayload = {
              session_token: token,
              product_id: product.product_id,
              warehouse_id: targetWarehouseId,
              unit_id: base.unit_id,
              quantity_micro: item.initialStockMicro.toString(),
              direction: "INCREASE",
              reason: STARTER_PROVISION_REASON,
              authorize_negative_stock: false,
            };
            await deps.adjust(payload);
          }
        }
        created += 1;
      } catch {
        skipped += 1;
      }
    }
    onProgress?.(index + 1, items.length);
    if ((index + 1) % STARTER_INSTALL_CHUNK === 0) {
      await yieldToUi();
    }
  }
  writePackFlag(true);
  return { created, skipped };
}

/**
 * Uninstall pack items through the dual-track engine. Matches catalog
 * rows by SKU, deletes each, and aggregates backend modes. Progress
 * fires per processed row.
 */
export async function uninstallStarterPack(
  token: string,
  packCodes: ReadonlySet<string>,
  catalogProducts: Array<{ id: string; sku: string }>,
  deps: UninstallDeps,
  onProgress?: ProgressCallback,
  reason: string = "starter-pack uninstall",
): Promise<UninstallSummary> {
  const targets = catalogProducts.filter((row) => packCodes.has(row.sku));
  const summary: UninstallSummary = {
    hardDeleted: 0,
    deactivated: 0,
    failed: 0,
    totalProcessed: targets.length,
  };
  for (let index = 0; index < targets.length; index += 1) {
    const target = targets[index];
    if (target === undefined) {
      continue;
    }
    try {
      const result = await deps.remove(token, target.id, reason);
      if (result.mode === "hard_deleted") {
        summary.hardDeleted += 1;
      } else {
        summary.deactivated += 1;
      }
    } catch {
      summary.failed += 1;
    }
    onProgress?.(index + 1, targets.length);
    if ((index + 1) % STARTER_INSTALL_CHUNK === 0) {
      await yieldToUi();
    }
  }
  writePackFlag(false);
  return summary;
}

/** Read the installed flag (absent storage reads as unset). Never throws. */
export function readPackFlag(): boolean {
  try {
    if (typeof localStorage === "undefined") {
      return false;
    }
    return localStorage.getItem(STARTER_PACK_FLAG) === "true";
  } catch {
    return false;
  }
}

/** Persist the installed flag. Never throws. */
export function writePackFlag(installed: boolean): void {
  try {
    if (typeof localStorage === "undefined") {
      return;
    }
    localStorage.setItem(STARTER_PACK_FLAG, installed ? "true" : "false");
  } catch {
    // Storage unavailable (private mode, SSR): detection stays dynamic.
  }
}
