/**
 * Integer-only display helpers shared by every report screen (ADR-039).
 *
 * Basis points and micro quantities render through BigInt arithmetic —
 * no parseFloat, no Number() on financial magnitudes.
 */

/** `"1234"` bp → `"12.34%"` (sign-preserving, two decimals). */
export function formatBasisPoints(bp: string): string {
  const value = BigInt(bp);
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const whole = (abs / 100n).toString();
  const frac = (abs % 100n).toString().padStart(2, "0");
  return `${negative ? "-" : ""}${whole}.${frac}%`;
}

/** True for canonical signed integer strings (backend wire shape). */
export function isIntString(value: string): boolean {
  return /^-?\d+$/.test(value);
}
