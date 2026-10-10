/**
 * Pure customer-credit helpers (TASK-DSN-027): BigInt balance predicates,
 * directory filtering, and available-credit arithmetic. Zero floats;
 * all money stays in canonical centime strings.
 */

import type { CustomerDetail } from "@/types/pos";

export type CustomerFilter = "all" | "debtors" | "inactive";

/** True when the ledger balance is strictly positive (safe false on garbage). */
export function isDebtor(balanceCentimes: string): boolean {
  try {
    return BigInt(balanceCentimes) > 0n;
  } catch {
    return false;
  }
}

/** Partition enriched directory rows by the active filter pill. */
export function filterCustomers(
  rows: CustomerDetail[],
  filter: CustomerFilter,
): CustomerDetail[] {
  if (filter === "debtors") {
    return rows.filter((row) => isDebtor(row.current_balance_centimes));
  }
  if (filter === "inactive") {
    return rows.filter((row) => !row.is_active);
  }
  return rows;
}

/**
 * Remaining credit for LIMITED policies (`limit - balance`, floored at 0).
 * Null for DISABLED/UNLIMITED (not applicable).
 */
export function availableCredit(detail: CustomerDetail): string | null {
  if (detail.credit_policy !== "LIMITED" || detail.credit_limit_centimes === null) {
    return null;
  }
  try {
    const remaining =
      BigInt(detail.credit_limit_centimes) - BigInt(detail.current_balance_centimes);
    return (remaining < 0n ? 0n : remaining).toString();
  } catch {
    return null;
  }
}

/** Validate a display name per engine rules (trimmed, 2..=100 chars). */
export function isValidCustomerName(name: string): boolean {
  const trimmed = name.trim();
  return trimmed.length >= 2 && trimmed.length <= 100;
}
