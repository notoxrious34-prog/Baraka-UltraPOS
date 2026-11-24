/**
 * Two-till sync bridge (TASK-BRK-124).
 * Backend command mapping (sovereign contract):
 *   sync_export_package / sync_import_package - exact backend names.
 * The package is opaque canonical text (BARAKA-SYNC-1); the frontend
 * copies it verbatim between tills via a USB file round-trip.
 * Counts cross as numbers; renumber rows map origin->local numbers.
 */

import { call } from "./ipc-kernel";

/** One applied renumber (origin till number -> importer gapless number). */
export interface SyncRenumber {
  originNumber: string;
  newNumber: string;
}

/** One importer-wins stock line (negative forced, needs radar watch). */
export interface SyncStockWarning {
  sku: string;
  newNumber: string;
}

/** Atomic import outcome (all-or-nothing, fail-closed). */
export interface SyncImportOutcome {
  importedDocs: number;
  renumbered: SyncRenumber[];
  negativeStockWarnings: SyncStockWarning[];
}

interface SyncRenumberWire {
  origin_number: string;
  new_number: string;
}

interface SyncStockWarningWire {
  sku: string;
  new_number: string;
}

interface SyncExportWire {
  package_text: string;
  exported_docs: number;
}

interface SyncImportWire {
  imported_docs: number;
  renumbered: SyncRenumberWire[];
  negative_stock_warnings: SyncStockWarningWire[];
}

/** Assemble the canonical v1 package of locally created sale docs. */
export async function ipcSyncExport(
  sessionToken: string,
  sinceCreatedAt: string,
  limit?: number,
): Promise<{ packageText: string; exportedDocs: number }> {
  const wire = await call<SyncExportWire>("sync_export_package", {
    request: {
      session_token: sessionToken,
      since_created_at: sinceCreatedAt,
      limit: limit ?? null,
    },
  });
  return { packageText: wire.package_text, exportedDocs: wire.exported_docs };
}

/** Merge a verified package atomically (fail-closed on any identity gap). */
export async function ipcSyncImport(
  sessionToken: string,
  packageText: string,
  warehouseId: string,
): Promise<SyncImportOutcome> {
  const wire = await call<SyncImportWire>("sync_import_package", {
    request: {
      session_token: sessionToken,
      package_text: packageText,
      warehouse_id: warehouseId,
    },
  });
  return {
    importedDocs: wire.imported_docs,
    renumbered: wire.renumbered.map((row) => ({
      originNumber: row.origin_number,
      newNumber: row.new_number,
    })),
    negativeStockWarnings: wire.negative_stock_warnings.map((line) => ({
      sku: line.sku,
      newNumber: line.new_number,
    })),
  };
}
