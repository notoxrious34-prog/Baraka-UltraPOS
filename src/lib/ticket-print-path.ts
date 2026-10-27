/**
 * Shared spooler gate (TASK-DSN-098): one rule for the drawer and the
 * post-sale reprint area. Spooler + thermal document -> silent TSPL
 * (never a dialog). Anything else keeps the frozen paths: graphique
 * dialog printing, engine DUPLICATA reprint, or A4 untouched.
 */

/** Silent-or-legacy ticket path decision. */
export type TicketPrintPath =
  | "silent-tspl"
  | "graphique"
  | "engine-reprint"
  | "existing";

export interface TicketPrintGate {
  transport: string;
  renderMode: string;
  documentType: string;
}

/** A4-eligible types keep their sheet path in every transport arm. */
export function isA4Document(documentType: string): boolean {
  return documentType === "BL" || documentType === "FACTURE";
}

export function resolveTicketPrintPath(gate: TicketPrintGate): TicketPrintPath {
  if (isA4Document(gate.documentType)) {
    return "existing";
  }
  if (gate.transport === "spooler") {
    return "silent-tspl";
  }
  if (gate.renderMode === "graphique") {
    return "graphique";
  }
  return "engine-reprint";
}
