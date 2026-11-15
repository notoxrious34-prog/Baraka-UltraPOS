/**
 * Screen-list instant formatting (DSN-107 / F-B8): medium date + short time
 * in the POS locale. Unparseable input falls back to the raw string —
 * never a blank cell. (Print surfaces keep the backend `*_display` wire.)
 */

import type { Locale } from "@/types/pos";

function bcp47(locale: Locale): string {
  return locale === "ar-DZ" ? "ar-DZ" : "fr-DZ";
}

export function formatInstant(iso: string, locale: Locale): string {
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
