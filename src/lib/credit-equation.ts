/**
 * Wholesale credit-equation helpers (TASK-DSN-103, ADR-111/112 print rule).
 *
 * The engine freezes `Ancien + Montant − Versement = Nouveau` per finalized
 * document (migration 0020). Two wire facts shape this module:
 *
 * 1. `paid_amount_centimes` counts EVERY tender, the CREDIT leg included —
 *    so the printed `Versement` is the non-credit portion
 *    `Montant − |delta|`, never the raw paid column.
 * 2. A `null` snapshot part means ABSENT (cash-only or legacy row): surfaces
 *    render the equation block absent, never zero.
 *
 * Pure BigInt over int-strings — no `Number`, no floats, no rounding. Every
 * function returns `null` on malformed input instead of printing a lie.
 */

/** Centime-string int test (engine `to_string()` spelling, optional sign). */
function isIntString(value: string): boolean {
  return /^[+-]?\d+$/.test(value.trim());
}

function toBigInt(value: string): bigint | null {
  if (!isIntString(value)) return null;
  try {
    return BigInt(value.trim());
  } catch {
    return null;
  }
}

/** Frozen equation figures for one document, all display-ready strings. */
export interface CreditEquation {
  ancien: string;
  montant: string;
  versement: string;
  nouveau: string;
}

/**
 * Build the printable equation from the details-wire document, or `null`
 * when the snapshot is absent (cash-only / legacy) or malformed.
 */
export function creditEquationFromDocument(doc: {
  total_ttc_centimes: string;
  credit_solde_before_centimes: string | null;
  credit_solde_after_centimes: string | null;
  credit_delta_centimes: string | null;
}): CreditEquation | null {
  const { total_ttc_centimes, credit_solde_before_centimes: before, credit_solde_after_centimes: after, credit_delta_centimes: delta } = doc;
  if (before === null || after === null || delta === null) return null;
  const montant = toBigInt(total_ttc_centimes);
  const ancien = toBigInt(before);
  const nouveau = toBigInt(after);
  const movement = toBigInt(delta);
  if (montant === null || ancien === null || nouveau === null || movement === null) return null;
  const magnitude = movement < 0n ? -movement : movement;
  const versement = montant - magnitude;
  // Prove the printed form before handing it to a surface. Charge
  // (delta ≥ 0): Ancien + Montant − Versement = Nouveau. AVOIR
  // (delta < 0): Ancien − Montant + Versement = Nouveau.
  const holds =
    movement >= 0n
      ? ancien + montant - versement === nouveau
      : ancien - montant + versement === nouveau;
  if (!holds) return null;
  return {
    ancien: ancien.toString(),
    montant: montant.toString(),
    versement: versement.toString(),
    nouveau: nouveau.toString(),
  };
}
