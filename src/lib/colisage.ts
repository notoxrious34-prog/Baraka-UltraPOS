/**
 * Read-time Colisage display helper (TASK-DSN-104, mirrors BRK-109).
 *
 * Pure display math over wire/store strings — BigInt only, no `Number`,
 * no floats. The domain `breakdown` stays the reference implementation;
 * this file only decides what a human eye sees:
 * - `null` = no breakdown worth printing (single rung, fraction under the
 *   smallest rung, invalid strings, or a ladder with fewer than 2 rungs).
 *   Callers render the base quantity they already show — never a zero.
 */

export interface ColisageLadderRung {
  factorMicro: string;
  nameFr: string;
  nameAr: string;
}

export interface ColisageShare {
  nameFr: string;
  nameAr: string;
  /** Whole-rung count, canonical integer string. */
  count: string;
}

function parsePositiveInt(value: string): bigint | null {
  if (!/^[0-9]+$/.test(value)) {
    return null;
  }
  try {
    return BigInt(value);
  } catch {
    return null;
  }
}

/**
 * Greedy largest-first breakdown of a selling-unit quantity into whole
 * packaging rungs. Returns `null` when nothing informative prints.
 */
export function colisageShares(
  quantityMicro: string,
  factorMicro: string,
  units: ColisageLadderRung[],
): ColisageShare[] | null {
  const qty = parsePositiveInt(quantityMicro);
  const factor = parsePositiveInt(factorMicro);
  if (qty === null || factor === null || factor === 0n) {
    return null;
  }
  // Selling quantity → base micros (same edge as cart-store preview).
  const base = (qty * factor) / 10000n;

  const ladder = new Map<bigint, ColisageLadderRung>();
  for (const rung of units) {
    const width = parsePositiveInt(rung.factorMicro);
    if (width === null || width <= 0n) {
      return null;
    }
    if (!ladder.has(width)) {
      ladder.set(width, rung);
    }
  }
  if (ladder.size < 2 || base === 0n) {
    return null;
  }
  const widths = [...ladder.keys()].sort((a, b) => (a > b ? -1 : a < b ? 1 : 0));
  let remaining = base;
  const shares: ColisageShare[] = [];
  for (const width of widths) {
    const count = remaining / width;
    remaining -= count * width;
    if (count > 0n) {
      const rung = ladder.get(width);
      if (rung === undefined) {
        return null;
      }
      shares.push({ nameFr: rung.nameFr, nameAr: rung.nameAr, count: count.toString() });
    }
  }
  // Fraction under the smallest rung, or a single-rung expression the row
  // already shows (e.g. `150 × Pièce`) — collapse, never print noise.
  if (remaining !== 0n || shares.length < 2) {
    return null;
  }
  return shares;
}

/** `12 × Douzaine + 1 × Demi-Douzaine` in the row locale. */
export function formatColisage(
  shares: ColisageShare[],
  locale: "ar-DZ" | "fr-DZ",
): string {
  return shares
    .map((share) => `${share.count} × ${locale === "ar-DZ" ? share.nameAr : share.nameFr}`)
    .join(" + ");
}

/** Wire-format share (document details wire, BRK-109 unit names). */
export interface WireColisageShare {
  unit_name_fr: string | null;
  unit_name_ar: string | null;
  count: string | number;
}

/**
 * Format document-wire shares for a ticket line. `null` = legacy row
 * (predates BRK-109) or an empty breakdown — the caller prints nothing.
 */
export function formatWireColisage(
  shares: WireColisageShare[] | null | undefined,
  fr: boolean,
): string | null {
  if (shares === undefined || shares === null || shares.length === 0) {
    return null;
  }
  const parts: string[] = [];
  for (const share of shares) {
    const name = fr
      ? (share.unit_name_fr ?? share.unit_name_ar)
      : (share.unit_name_ar ?? share.unit_name_fr);
    if (name === null || name === undefined || name === "") {
      return null;
    }
    parts.push(`${String(share.count)} × ${name}`);
  }
  return parts.length === 0 ? null : parts.join(" + ");
}
