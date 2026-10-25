/**
 * Honest test-print notice (TASK-DSN-093, AUD-042 §4a): split the settings
 * test banner by outcome class at a single construction site. No new IPC
 * fields — the deviceOutcome string already crosses the bridge.
 */

import type { Locale } from "@/types/pos";
import { t } from "@/lib/i18n";

export type TestPrintTone = "success" | "neutral" | "error";

export interface TestPrintNotice {
  tone: TestPrintTone;
  text: string;
}

export interface TestPrintOutcome {
  deviceOutcome: string;
  captureFileName: string;
}

/**
 * sent → success; capture-only → neutral guidance (no physical send
 * attempted); anything else → error carrying the outcome text.
 */
export function resolveTestPrintNotice(
  locale: Locale,
  outcome: TestPrintOutcome,
): TestPrintNotice {
  const sentLine = `${t(locale, "settings.testPrintSent")}: ${outcome.deviceOutcome} — ${outcome.captureFileName}`;
  if (outcome.deviceOutcome === "sent") {
    return { tone: "success", text: sentLine };
  }
  if (outcome.deviceOutcome === "capture-only") {
    return {
      tone: "neutral",
      text: `${sentLine} · ${t(locale, "settings.testPrintCaptureNote")}`,
    };
  }
  return { tone: "error", text: sentLine };
}

/** Tailwind tone classes for the notice paragraph (success / neutral / error). */
export function testPrintToneClass(tone: TestPrintTone): string {
  switch (tone) {
    case "success":
      return "bg-green-50 text-green-700 dark:bg-green-900/30 dark:text-green-200";
    case "neutral":
      return "bg-amber-50 text-amber-800 dark:bg-amber-900/30 dark:text-amber-200";
    case "error":
      return "bg-red-50 text-red-700 dark:bg-red-900/30 dark:text-red-200";
  }
}
