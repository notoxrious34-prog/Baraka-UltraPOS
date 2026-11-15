/**
 * Quantity-break ladder parity (BRK-121 / S1).
 * Mirrors `resolve_break_price` in `crates/domain/src/catalog/price_breaks.rs`:
 * greatest rung at or below the line quantity wins; a break may only lower
 * the tier price, never raise it. All values are canonical strings, compared
 * as BigInt — no floats, no Number().
 */

export interface PriceBreak {
  minQtyMicro: string;
  priceCentimes: string;
}

/** Winning rung price for `qtyMicro`, or `null` when no rung applies. Garbage-safe. */
export function resolveBreakPrice(breaks: PriceBreak[], qtyMicro: string): string | null {
  let qty: bigint;
  try {
    qty = BigInt(qtyMicro);
  } catch {
    return null;
  }
  if (qty <= 0n) {
    return null;
  }
  let winner: PriceBreak | null = null;
  for (const rung of breaks) {
    let min: bigint;
    let price: bigint;
    try {
      min = BigInt(rung.minQtyMicro);
      price = BigInt(rung.priceCentimes);
    } catch {
      continue;
    }
    if (min <= 0n || min > qty || price < 0n) {
      continue;
    }
    try {
      if (winner === null || min > BigInt(winner.minQtyMicro)) {
        winner = rung;
      }
    } catch {
      continue;
    }
  }
  return winner === null ? null : winner.priceCentimes;
}

/** `min(tierPrice, break)` — a corrupt high rung never overcharges. Garbage-safe. */
export function applyBreakPrice(
  tierPriceCentimes: string,
  breaks: PriceBreak[] | undefined,
  qtyMicro: string,
): string {
  if (breaks === undefined || breaks.length === 0) {
    return tierPriceCentimes;
  }
  const rung = resolveBreakPrice(breaks, qtyMicro);
  if (rung === null) {
    return tierPriceCentimes;
  }
  try {
    return BigInt(rung) < BigInt(tierPriceCentimes) ? rung : tierPriceCentimes;
  } catch {
    return tierPriceCentimes;
  }
}
