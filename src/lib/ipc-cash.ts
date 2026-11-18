/**
 * Cash bridge (TASK-BRK-010).
 * Backend command mapping (sovereign contract):
 * - cash_open / cash_begin_closing / cash_cancel_closing /
 *   cash_confirm_closing / cash_get_active_session / cash_record_expense /
 *   cash_get_session_report — exact backend names.
 * Split from `ipc.ts` (Wave 3); byte-identical motion.
 */

import type { CashMovement, CashSession, CashSessionReport } from "@/types/cash";
import { call } from "./ipc-kernel";

interface CashSessionWire {
  id: string;
  warehouse_id: string;
  terminal_id: string;
  status: string;
  opened_by: string;
  opened_by_username: string;
  opened_at: string;
  opening_float_centimes: string;
  closed_by: string | null;
  closed_by_username: string | null;
  closed_at: string | null;
  actual_counted_centimes: string | null;
  system_expected_centimes: string | null;
  variance_centimes: string | null;
  closing_notes: string | null;
  revision: string;
  created_at: string;
  updated_at: string;
}

interface CashMovementWire {
  id: string;
  cash_session_id: string;
  movement_type: string;
  amount_centimes: string;
  reference_type: string | null;
  reference_id: string | null;
  description: string | null;
  created_at: string;
  created_by: string;
  created_by_username: string;
}

function toCashSession(wire: CashSessionWire): CashSession {
  return {
    id: wire.id,
    warehouseId: wire.warehouse_id,
    terminalId: wire.terminal_id,
    status: wire.status as CashSession["status"],
    openedBy: wire.opened_by,
    openedByUsername: wire.opened_by_username,
    openedAt: wire.opened_at,
    openingFloatCentimes: wire.opening_float_centimes,
    closedBy: wire.closed_by,
    closedByUsername: wire.closed_by_username,
    closedAt: wire.closed_at,
    actualCountedCentimes: wire.actual_counted_centimes,
    systemExpectedCentimes: wire.system_expected_centimes,
    varianceCentimes: wire.variance_centimes,
    closingNotes: wire.closing_notes,
    revision: wire.revision,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  };
}

function toCashMovement(wire: CashMovementWire): CashMovement {
  return {
    id: wire.id,
    cashSessionId: wire.cash_session_id,
    movementType: wire.movement_type,
    amountCentimes: wire.amount_centimes,
    referenceType: wire.reference_type,
    referenceId: wire.reference_id,
    description: wire.description,
    createdAt: wire.created_at,
    createdBy: wire.created_by,
    createdByUsername: wire.created_by_username,
  };
}

/** Open a cash session with an opening float (centime string). */
export async function ipcCashOpen(
  sessionToken: string,
  warehouseId: string,
  openingFloatCentimes: string,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_open", {
    request: {
      session_token: sessionToken,
      warehouse_id: warehouseId,
      opening_float_centimes: openingFloatCentimes,
      command_id: commandId,
    },
  });
  return toCashSession(wire);
}

/** Freeze the expected total (OPEN -> CLOSING). */
export async function ipcCashBeginClosing(
  sessionToken: string,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_begin_closing", {
    request: { session_token: sessionToken, command_id: commandId },
  });
  return toCashSession(wire);
}

/** Revert CLOSING -> OPEN. */
export async function ipcCashCancelClosing(
  sessionToken: string,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_cancel_closing", {
    request: { session_token: sessionToken, command_id: commandId },
  });
  return toCashSession(wire);
}

/** Count and seal (CLOSING -> CLOSED). */
export async function ipcCashConfirmClosing(
  sessionToken: string,
  sessionId: string,
  actualCountedCentimes: string,
  closingNotes: string | null,
  commandId: string,
): Promise<CashSession> {
  const wire = await call<CashSessionWire>("cash_confirm_closing", {
    request: {
      session_token: sessionToken,
      session_id: sessionId,
      actual_counted_centimes: actualCountedCentimes,
      closing_notes: closingNotes,
      command_id: commandId,
    },
  });
  return toCashSession(wire);
}

/** Active OPEN/CLOSING session, or null when none (token injected by the wrapper). */
export async function ipcCashGetActiveSession(): Promise<CashSession | null> {
  const wire = await call<CashSessionWire | null>("cash_get_active_session", {});
  return wire === null ? null : toCashSession(wire);
}

/** Record a drawer expense (positive centime string; stored negative). */
export async function ipcCashRecordExpense(
  sessionToken: string,
  amountCentimes: string,
  description: string,
  commandId: string,
): Promise<CashMovement> {
  const wire = await call<CashMovementWire>("cash_record_expense", {
    request: {
      session_token: sessionToken,
      amount_centimes: amountCentimes,
      description,
      command_id: commandId,
    },
  });
  return toCashMovement(wire);
}

/** Session report (interim X-Report or final Z-Report). */
export async function ipcCashGetSessionReport(
  sessionToken: string,
  sessionId: string,
): Promise<CashSessionReport> {
  interface ReportWire {
    session: CashSessionWire;
    movements: CashMovementWire[];
    totals_by_type: Array<{ movement_type: string; total_centimes: string; count: string }>;
    is_final: boolean;
  }
  const wire = await call<ReportWire>("cash_get_session_report", {
    request: { session_token: sessionToken, session_id: sessionId },
  });
  return {
    session: toCashSession(wire.session),
    movements: wire.movements.map(toCashMovement),
    totalsByType: wire.totals_by_type.map((total) => ({
      movementType: total.movement_type,
      totalCentimes: total.total_centimes,
      count: total.count,
    })),
    isFinal: wire.is_final,
  };
}
