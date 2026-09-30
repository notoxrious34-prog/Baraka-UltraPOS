/**
 * Zero-float monetary and quantity helpers (ADR-003 / ADR-004).
 *
 * Wire values are decimal integer strings: centimes for money, micros
 * (scale 10^4) for quantities. All arithmetic uses BigInt. Cart preview
 * totals computed here are display-only; the sales_checkout response is
 * the sole source of truth for committed financial data.
 */

const CENTIMES_RE = /^-?\d+$/;
const GROUP = " ";

/** True for canonical integer strings (`"125000"`, `"-50"`, `"0"`). */
export function isIntString(value: string): boolean {
  return CENTIMES_RE.test(value);
}

function parseBigIntStrict(value: string, field: string): bigint {
  const trimmed = value.trim();
  if (!isIntString(trimmed)) {
    throw new Error(`Invalid ${field}: ${value}`);
  }
  return BigInt(trimmed);
}

function groupThousands(digits: string): string {
  if (digits.length <= 3) {
    return digits;
  }
  const parts: string[] = [];
  let rest = digits;
  while (rest.length > 3) {
    parts.unshift(rest.slice(-3));
    rest = rest.slice(0, -3);
  }
  parts.unshift(rest);
  return parts.join(GROUP);
}

/**
 * Format centimes string -> `"1 250,00 DZD"` (fr-DZ) or `"1 250,00 د.ج"` (ar-DZ).
 * Throws on non-canonical input.
 */
export function formatMoney(centimesStr: string, locale: "ar-DZ" | "fr-DZ"): string {
  const value = parseBigIntStrict(centimesStr, "centimes");
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const whole = (abs / 100n).toString();
  const frac = (abs % 100n).toString().padStart(2, "0");
  const sign = negative ? "-" : "";
  const suffix = locale === "ar-DZ" ? "د.ج" : "DZD";
  return `${sign}${groupThousands(whole)},${frac} ${suffix}`;
}

/**
 * Format micro string -> trimmed decimal (`"25000"` -> `"2.5"`, `"10000"` -> `"1"`).
 * Throws on non-canonical input.
 */
export function formatQty(microStr: string): string {
  const value = parseBigIntStrict(microStr, "micros");
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const whole = (abs / 10_000n).toString();
  const frac = (abs % 10_000n).toString().padStart(4, "0").replace(/0+$/, "");
  const sign = negative ? "-" : "";
  return frac.length === 0 ? `${sign}${whole}` : `${sign}${whole}.${frac}`;
}

/**
 * Human decimal input (`"12"`, `"12.5"`, `"12,5"`) -> micro string (`"125000"`).
 * At most 4 fractional digits; throws otherwise.
 */
export function parseQtyToMicro(input: string): string {
  const trimmed = input.trim().replace(",", ".");
  const match = /^(-)?(\d+)(?:\.(\d{1,4}))?$/.exec(trimmed);
  if (match === null) {
    throw new Error(`Invalid quantity: ${input}`);
  }
  const sign = match[1] === "-" ? -1n : 1n;
  const whole = BigInt(match[2] ?? "0");
  const frac = (match[3] ?? "").padEnd(4, "0");
  return (sign * (whole * 10_000n + BigInt(frac))).toString();
}

/**
 * Human monetary input (`"150"`, `"150.5"`, `"150,50"`) -> centimes (`"15050"`).
 * At most 2 fractional digits; throws otherwise.
 */
export function parseMoneyToCentimes(input: string): string {
  const trimmed = input.trim().replace(",", ".");
  const match = /^(-)?(\d+)(?:\.(\d{1,2}))?$/.exec(trimmed);
  if (match === null) {
    throw new Error(`Invalid amount: ${input}`);
  }
  const sign = match[1] === "-" ? -1n : 1n;
  const whole = BigInt(match[2] ?? "0");
  const frac = (match[3] ?? "").padEnd(2, "0");
  return (sign * (whole * 100n + BigInt(frac))).toString();
}

/** Canonical `"0"` for empty/blank numeric inputs. */
export function emptyToZero(value: string): string {
  return value.trim() === "" ? "0" : value.trim();
}

/** BigInt addition over centime/micro strings (throws on bad input). */
export function addBig(a: string, b: string): string {
  return (parseBigIntStrict(a, "operand") + parseBigIntStrict(b, "operand")).toString();
}

/** BigInt subtraction over centime/micro strings (throws on bad input). */
export function subBig(a: string, b: string): string {
  return (parseBigIntStrict(a, "operand") - parseBigIntStrict(b, "operand")).toString();
}

/** Clamp a signed integer string at zero (never negative). */
export function maxZero(value: string): string {
  const parsed = parseBigIntStrict(value, "operand");
  return (parsed < 0n ? 0n : parsed).toString();
}

/**
 * Line total preview: `round_half_away(qtyMicro * priceCentimes / 10000)`.
 * Preview-only for non-negative magnitudes; backend commits truth.
 */
export function mulQtyPrice(qtyMicro: string, priceCentimes: string): string {
  const qty = parseBigIntStrict(qtyMicro, "quantity");
  const price = parseBigIntStrict(priceCentimes, "price");
  const numerator = qty * price;
  const rounded = numerator >= 0n ? (numerator + 5_000n) / 10_000n : -((-numerator + 5_000n) / 10_000n);
  return rounded.toString();
}

/**
 * Credit usage in basis points (0..10000+ scale) without floats:
 * `balance * 10000 / limit`. Throws on bad input or non-positive limit.
 */
export function creditUsageBp(balanceCentimes: string, limitCentimes: string): bigint {
  const balance = parseBigIntStrict(balanceCentimes, "balance");
  const limit = parseBigIntStrict(limitCentimes, "limit");
  if (limit <= 0n) {
    throw new Error("Credit limit must be positive");
  }
  return (balance * 10_000n) / limit;
}

/** UUID v4 for idempotency command ids (WebCrypto, no dependency). */
export function newCommandId(): string {
  return crypto.randomUUID();
}
