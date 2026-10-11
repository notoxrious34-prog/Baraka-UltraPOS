/**
 * Pure stock helpers (TASK-DSN-029): canonical movement-type grouping over
 * the seven engine codes (movement.rs), badge classes, and warehouse-code
 * validation. Quantities stay in micro strings — zero floats.
 */

/** Engine movement codes (crates/domain/src/inventory/movement.rs). */
export const MOVEMENT_CODES = [
  "SALE_OUT",
  "RETURN_IN",
  "PURCHASE_IN",
  "ADJUSTMENT_POSITIVE",
  "ADJUSTMENT_NEGATIVE",
  "TRANSFER_OUT",
  "TRANSFER_IN",
] as const;

/** Client-side history groups (spec §3): server filter stays null. */
export type MovementGroup = "ALL" | "IN" | "OUT" | "ADJUST" | "TRANSFER";

const IN_CODES: ReadonlySet<string> = new Set([
  "PURCHASE_IN",
  "RETURN_IN",
  "TRANSFER_IN",
  "ADJUSTMENT_POSITIVE",
]);
const OUT_CODES: ReadonlySet<string> = new Set([
  "SALE_OUT",
  "TRANSFER_OUT",
  "ADJUSTMENT_NEGATIVE",
]);
const ADJUST_CODES: ReadonlySet<string> = new Set([
  "ADJUSTMENT_POSITIVE",
  "ADJUSTMENT_NEGATIVE",
]);
const TRANSFER_CODES: ReadonlySet<string> = new Set(["TRANSFER_IN", "TRANSFER_OUT"]);

/** True when a movement code belongs to the selected history group. */
export function inMovementGroup(code: string, group: MovementGroup): boolean {
  switch (group) {
    case "ALL":
      return true;
    case "IN":
      return IN_CODES.has(code);
    case "OUT":
      return OUT_CODES.has(code);
    case "ADJUST":
      return ADJUST_CODES.has(code);
    case "TRANSFER":
      return TRANSFER_CODES.has(code);
  }
}

/** i18n group key for the history filter pills. */
export function movementGroupKey(
  group: MovementGroup,
): "stock.groupAll" | "stock.groupIn" | "stock.groupOut" | "stock.groupAdjust" | "stock.groupTransfer" {
  switch (group) {
    case "ALL":
      return "stock.groupAll";
    case "IN":
      return "stock.groupIn";
    case "OUT":
      return "stock.groupOut";
    case "ADJUST":
      return "stock.groupAdjust";
    case "TRANSFER":
      return "stock.groupTransfer";
  }
}

/** Semantic badge classes per movement code (light + dark). */
export function movementBadge(code: string): string {
  if (ADJUST_CODES.has(code)) {
    return "bg-amber-100 text-amber-900 dark:bg-amber-900/50 dark:text-amber-200";
  }
  if (TRANSFER_CODES.has(code)) {
    return "bg-sky-100 text-sky-800 dark:bg-sky-900/50 dark:text-sky-200";
  }
  if (IN_CODES.has(code)) {
    return "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/50 dark:text-emerald-200";
  }
  if (code === "SALE_OUT") {
    return "bg-slate-200 text-slate-800 dark:bg-slate-700 dark:text-slate-100";
  }
  if (OUT_CODES.has(code)) {
    return "bg-rose-100 text-rose-800 dark:bg-rose-900/50 dark:text-rose-200";
  }
  return "bg-slate-200 text-slate-700 dark:bg-slate-700 dark:text-slate-200";
}

/** Warehouse code rule: trimmed, 1..=20 chars (engine enforces the rest). */
export function isValidWarehouseCode(code: string): boolean {
  const trimmed = code.trim();
  return trimmed.length >= 1 && trimmed.length <= 20;
}

/** Warehouse name rule: trimmed, 1..=100 chars. */
export function isValidWarehouseName(name: string): boolean {
  const trimmed = name.trim();
  return trimmed.length >= 1 && trimmed.length <= 100;
}
