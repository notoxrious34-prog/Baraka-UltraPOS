/**
 * Pure settings helpers: byte formatting, identity/fiscal validation, and
 * trigger badge styling. Numbers and codes render inside `<bdi dir="ltr">`
 * at call sites.
 */

export const BAUD_OPTIONS: readonly number[] = [9600, 19200, 38400, 57600, 115200];

/** Human byte size: B, KB (1 decimal), or MB (1 decimal). */
/**
 * Wire-enabling (BRK-094): the list wire carries `number | null`
 * (`None` = legacy row). Unknown renders as an em dash — never 0.
 * DSN-095 owns the final human-size pass (o/Ko/Mo).
 */
export function formatBytes(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes) || bytes <= 0) {
    return "—";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
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
