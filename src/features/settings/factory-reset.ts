/**
 * Factory-reset screen helpers (TASK-DSN-094).
 *
 * Pure, testable rules behind the danger-zone section: the expected
 * confirmation token derives from the store-identity wire (the precheck
 * deliberately omits it — BRK-091 §5), and the destructive call stays
 * disabled unless the precheck is eligible AND the typed token matches.
 * The engine triple-guards; these guards only prevent misfires in the UI.
 */

/**
 * Fallback token when the store name is blank. Mirrors the engine
 * `expected_reset_token` fallback (`sqlite_backup_manager.rs`) — the
 * principal depot code the seed migration guarantees. Documented mirror,
 * not a hardcoded secret: the engine re-validates unconditionally.
 */
export const FACTORY_RESET_BLANK_STORE_TOKEN = "DEP-01";

/** Expected confirmation token for a store name (blank → depot fallback). */
export function expectedResetToken(storeName: string): string {
  const trimmed = storeName.trim();
  return trimmed === "" ? FACTORY_RESET_BLANK_STORE_TOKEN : trimmed;
}

/**
 * Double guard: no destructive call without an eligible precheck AND an
 * exact token match (SafeDeleteModal `typedOk` convention, no trimming —
 * the operator must reproduce the token exactly).
 */
export function canExecuteReset(
  eligible: boolean,
  typedToken: string,
  expected: string,
): boolean {
  return eligible && typedToken !== "" && typedToken === expected;
}
