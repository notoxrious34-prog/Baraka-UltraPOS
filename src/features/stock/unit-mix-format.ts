/**
 * Basis-point percent formatting for the S4 unit-mix (pure, BigInt only).
 * `"7058"` -> `"70.58%"`, `"666"` -> `"6.66%"`, `"0"` -> `"0%"`.
 * Throws on non-canonical input — same contract as money.ts parsers.
 */
export function formatBpPercent(bp: string): string {
  if (!/^-?\d+$/.test(bp)) {
    throw new Error(`Invalid basis points: ${bp}`);
  }
  const value = BigInt(bp);
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const whole = (abs / 100n).toString();
  const frac = (abs % 100n).toString().padStart(2, "0").replace(/0+$/, "");
  const sign = negative ? "-" : "";
  return frac.length === 0 ? `${sign}${whole}%` : `${sign}${whole}.${frac}%`;
}
