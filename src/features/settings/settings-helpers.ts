/**
 * Pure settings helpers: byte formatting, identity/fiscal validation, and
 * trigger badge styling. Numbers and codes render inside `<bdi dir="ltr">`
 * at call sites.
 */

import type { Locale } from "@/types/pos";

export const BAUD_OPTIONS: readonly number[] = [9600, 19200, 38400, 57600, 115200];

/** Canonical BCP 47 tag for a POS locale (both are DZ-rooted). */
function bcp47(locale: Locale): string {
  return locale === "ar-DZ" ? "ar-DZ" : "fr-DZ";
}

/**
 * Human file size, locale-aware (DSN-095): French uses o/Ko/Mo/Go with
 * fr-DZ grouping; Arabic uses ar-DZ numerals with ب/ك.ب/م.ب/غ.ب units.
 * Unknown (legacy NULL), non-finite, zero, or negative renders as an em
 * dash — never 0. Call sites wrap the result in `<bdi>` (LTR isolation).
 */
export function formatBytes(bytes: number | null, locale: Locale = "fr-DZ"): string {
  if (bytes === null || !Number.isFinite(bytes) || bytes <= 0) {
    return "—";
  }
  const units = locale === "ar-DZ" ? ["ب", "ك.ب", "م.ب", "غ.ب"] : ["o", "Ko", "Mo", "Go"];
  let value = bytes;
  let unit = units[0] ?? (locale === "ar-DZ" ? "ب" : "o");
  for (const next of units.slice(1)) {
    if (value < 1024) {
      break;
    }
    value /= 1024;
    unit = next;
  }
  const numeral = new Intl.NumberFormat(bcp47(locale), {
    maximumFractionDigits: value < 10 && value !== Math.floor(value) ? 1 : 0,
  }).format(value);
  return `${numeral} ${unit}`;
}

/**
 * Locale backup timestamp (DSN-095): medium date + short time in the POS
 * locale (fr-DZ / ar-DZ). Unparseable input falls back to the raw string —
 * never a blank cell.
 */
export function formatBackupDate(iso: string, locale: Locale): string {
  const time = Date.parse(iso);
  if (Number.isNaN(time)) {
    return iso;
  }
  try {
    return new Intl.DateTimeFormat(bcp47(locale), {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(time));
  } catch {
    return iso;
  }
}

/** Commercial name rule: 2–100 chars after trimming. */
export function validateStoreName(value: string): boolean {
  const trimmed = value.trim();
  return trimmed.length >= 2 && trimmed.length <= 100;
}

/**
 * Fiscal identifier rule: empty (unset) or 2–30 chars of digits, letters,
 * spaces, dashes, and slashes (Algerian RC/NIF/NIS/AI surface shapes).
 */
export function validateFiscalField(value: string): boolean {
  const trimmed = value.trim();
  if (trimmed === "") {
    return true;
  }
  if (trimmed.length < 2 || trimmed.length > 30) {
    return false;
  }
  return /^[0-9A-Za-z \-/]+$/.test(trimmed);
}

/** Phone rule: empty (unset) or 6–20 chars of digits, spaces, +, -, (). */
export function validatePhone(value: string): boolean {
  const trimmed = value.trim();
  if (trimmed === "") {
    return true;
  }
  if (trimmed.length < 6 || trimmed.length > 20) {
    return false;
  }
  return /^[0-9+()\-.\s]+$/.test(trimmed);
}

/** Badge palette for backup trigger codes (MANUAL blue, others gray/green). */
export function triggerBadgeClass(triggerType: string): string {
  if (triggerType === "MANUAL") {
    return "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-200";
  }
  return "bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-200";
}

/** Short SHA-256 display prefix (first 12 hex chars + ellipsis). */
export function checksumPrefix(checksum: string): string {
  if (checksum.length <= 12) {
    return checksum;
  }
  return `${checksum.slice(0, 12)}…`;
}
