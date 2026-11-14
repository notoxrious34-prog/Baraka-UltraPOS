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

/**
 * Risk band for one dossier (BRK-110, mirrors the domain classifier):
 * `OVER_LIMIT` / `NEAR_LIMIT` escalate on LIMITED policies only;
 * anything else (including garbage strings) is `OK` — the banner
 * never fires on unparseable figures.
 */
export function assessDetailRisk(detail: CustomerDetail): "OVER_LIMIT" | "NEAR_LIMIT" | "OK" {
  if (detail.credit_policy !== "LIMITED" || detail.credit_limit_centimes === null) {
    return "OK";
  }
  try {
    const limit = BigInt(detail.credit_limit_centimes);
    const balance = BigInt(detail.current_balance_centimes);
    if (balance <= 0n) {
      return "OK";
    }
    if (balance > limit) {
      return "OVER_LIMIT";
    }
    if (balance * 100n >= limit * 80n) {
      return "NEAR_LIMIT";
    }
    return "OK";
  } catch {
    return "OK";
  }
}

/** Validate a display name per engine rules (trimmed, 2..=100 chars). */
export function isValidCustomerName(name: string): boolean {
  const trimmed = name.trim();
  return trimmed.length >= 2 && trimmed.length <= 100;
}
