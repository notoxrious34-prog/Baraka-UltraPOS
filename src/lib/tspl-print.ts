/**
 * Silent TSPL facade (TASK-DSN-098): validated bridges to the BRK-097
 * engine commands. Copies are enforced exactly once, here
 * (`validateTsplCopies`); the engine PRINTs n — no double multiplication.
 * Failures surface as verbatim engine codes (translateBackendError maps
 * the TSPL_* codes to themselves — no new prose).
 */

import {
  ipcPrintTsplLabel as bridgeLabel,
  ipcPrintTsplTicket as bridgeTicket,
} from "@/lib/ipc";

/** Copies floor/ceiling (engine TSPL_MIN/MAX_COPIES parity). */
export const TSPL_COPIES_MIN = 1;
export const TSPL_COPIES_MAX = 99;

/** Validate a copies count (integer 1..99); throws the verbatim engine code otherwise. */
export function validateTsplCopies(raw: unknown): number {
  if (typeof raw !== "number" || !Number.isInteger(raw)) {
    throw new Error("TSPL_COPIES_OUT_OF_RANGE");
  }
  if (raw < TSPL_COPIES_MIN || raw > TSPL_COPIES_MAX) {
    throw new Error("TSPL_COPIES_OUT_OF_RANGE");
  }
  return raw;
}

/**
 * Clamp a UI copies value into 1..99 (DSN-102: garbage in the modal must
 * never reach the throwing validator — the user gets a sane value, not
 * an error card). Non-finite input floors to the minimum.
 */
export function clampTsplCopies(raw: unknown): number {
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return TSPL_COPIES_MIN;
  }
  const floored = Math.floor(raw);
  if (floored < TSPL_COPIES_MIN) {
    return TSPL_COPIES_MIN;
  }
  if (floored > TSPL_COPIES_MAX) {
    return TSPL_COPIES_MAX;
  }
  return floored;
}

export interface TsplLabelInput {
  name: string;
  priceCentimes: string;
  barcode: string;
  copies: number;
}

/** Silent 40x20 label send; resolves the spooled byte count. */
export function ipcPrintTsplLabel(
  sessionToken: string,
  input: TsplLabelInput,
): Promise<number> {
  const copies = validateTsplCopies(input.copies);
  return bridgeLabel(sessionToken, { ...input, copies });
}

/** Silent 80mm ticket send; resolves the spooled byte count. */
export function ipcPrintTsplTicket(
  sessionToken: string,
  documentId: string,
): Promise<number> {
  return bridgeTicket(sessionToken, documentId);
}
