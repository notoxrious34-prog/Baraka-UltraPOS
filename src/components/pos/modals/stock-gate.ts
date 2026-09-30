/**
 * Pure stock-shortage confirmation gate (TASK-BRK-033).
 *
 * Framework-free so the checkout modal and its Vitest suite share one
 * decision core: non-privileged roles are hard-blocked whenever shortage
 * warnings exist; privileged roles (ADMIN/GERANT) must explicitly toggle
 * the in-session confirmation before the modal may dispatch. No money,
 * no totals, no I/O here — only the role × warnings × toggle decision.
 */

export type StockGateRole = string;

/** Standing grant holders for sale-on-shortage (mirrors the backend permission). */
export function isPrivilegedStockRole(role: StockGateRole): boolean {
  return role === "ADMIN" || role === "GERANT";
}

export interface StockGateInput {
  /** Backend session role code (`ADMIN`, `GERANT`, `VENDEUR`, `CAISSIER`). */
  role: StockGateRole;
  /** Number of leaf-level shortage warnings on the current preview. */
  warningCount: number;
  /** Operator's explicit in-session confirmation toggle. */
  allowNegativeStock: boolean;
}

export interface StockGateResult {
  /** True when at least one shortage warning exists. */
  hasShortage: boolean;
  /** True for ADMIN/GERANT sessions. */
  isPrivileged: boolean;
  /** True when confirmation must refuse to dispatch. */
  blocked: boolean;
  /** True when the explicit checkbox must be shown. */
  needsConfirmation: boolean;
}

/** Evaluate the shortage gate for the current modal state. */
export function evaluateStockGate(input: StockGateInput): StockGateResult {
  const hasShortage = input.warningCount > 0;
  const isPrivileged = isPrivilegedStockRole(input.role);
  const needsConfirmation = hasShortage && isPrivileged;
  const blocked = hasShortage && (!isPrivileged || !input.allowNegativeStock);
  return { hasShortage, isPrivileged, blocked, needsConfirmation };
}
