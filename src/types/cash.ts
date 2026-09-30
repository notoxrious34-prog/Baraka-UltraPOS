/**
 * Cash session wire types (TASK-BRK-010).
 *
 * Money travels as integer-centime strings; ids are UUID strings.
 * The `number` type never carries money here.
 */

/** Cash session status (`OPEN`, `CLOSING`, `CLOSED`). */
export type CashSessionStatus = "OPEN" | "CLOSING" | "CLOSED";

/** Stored cash session view (mirrors CashSessionDto). */
export interface CashSession {
  id: string;
  warehouseId: string;
  terminalId: string;
  status: CashSessionStatus;
  openedBy: string;
  openedByUsername: string;
  openedAt: string;
  openingFloatCentimes: string;
  closedBy: string | null;
  closedByUsername: string | null;
  closedAt: string | null;
  actualCountedCentimes: string | null;
  systemExpectedCentimes: string | null;
  varianceCentimes: string | null;
  closingNotes: string | null;
  revision: string;
  createdAt: string;
  updatedAt: string;
}

/** Stored cash movement view (mirrors CashMovementDto). */
export interface CashMovement {
  id: string;
  cashSessionId: string;
  movementType: string;
  amountCentimes: string;
  referenceType: string | null;
  referenceId: string | null;
  description: string | null;
  createdAt: string;
  createdBy: string;
  createdByUsername: string;
}

/** Totals grouped by movement type. */
export interface CashMovementTotal {
  movementType: string;
  totalCentimes: string;
  count: string;
}

/** Session report (mirrors CashSessionReportDto). */
export interface CashSessionReport {
  session: CashSession;
  movements: CashMovement[];
  totalsByType: CashMovementTotal[];
  isFinal: boolean;
}
