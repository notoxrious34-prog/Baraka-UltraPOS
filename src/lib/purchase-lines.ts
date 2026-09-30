/**
 * Shared purchase-line helpers (TASK-BRK-011). All money is centime
 * strings; all quantities are micro strings; BigInt only, never `number`.
 */

import type { PurchaseLineInput } from "@/types/supplier";
import { mulQtyPrice, parseMoneyToCentimes, parseQtyToMicro } from "@/lib/money";

/** One editable purchase line row (raw user text until submit/total). */
export interface PurchaseRow {
  productId: string;
  unitId: string;
  label: string;
  qtyText: string;
  costText: string;
}

/** Line total in centimes (display only; the backend commits truth). */
export function rowTotalCentimes(row: PurchaseRow): string | null {
  try {
    return mulQtyPrice(parseQtyToMicro(row.qtyText), parseMoneyToCentimes(row.costText));
  } catch {
    return null;
  }
}

/** Sum of row totals in centimes, or null when any row is unparsable. */
export function linesTotalCentimes(rows: PurchaseRow[]): string | null {
  if (rows.length === 0) {
    return null;
  }
  try {
    let total = 0n;
    for (const row of rows) {
      total += BigInt(
        mulQtyPrice(parseQtyToMicro(row.qtyText), parseMoneyToCentimes(row.costText)),
      );
    }
    return total.toString();
  } catch {
    return null;
  }
}

/** Wire-shaped line inputs, or null when any row is unparsable/empty. */
export function toLineInputs(rows: PurchaseRow[]): PurchaseLineInput[] | null {
  if (rows.length === 0) {
    return null;
  }
  try {
    return rows.map((row) => ({
      productId: row.productId,
      unitId: row.unitId,
      quantityMicro: parseQtyToMicro(row.qtyText),
      unitCostCentimes: parseMoneyToCentimes(row.costText),
    }));
  } catch {
    return null;
  }
}
