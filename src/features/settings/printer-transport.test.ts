/**
 * Printer transport selection pins (TASK-DSN-091): USB-spooler vs serial
 * COM UI. Node-only harness: source-text assertions (no jsdom), same
 * pattern as settings-module.test.ts.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const TAB_SRC = readSrc("src", "features", "settings", "PrinterSettingsTab.tsx");
const IPC_SRC = readSrc("src", "lib", "ipc.ts");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

const NEW_KEYS = [
  "settings.printerTransportTitle",
  "settings.printerTransportSerial",
  "settings.printerTransportSpooler",
  "settings.printerQueueName",
  "settings.printerQueuePlaceholder",
  "settings.printerQueueHelp",
  "settings.printerQueueRequired",
];

describe("transport segmented control (icon+text, both arms)", () => {
  it("renders a labelled group with one icon+text button per arm", () => {
    expect(TAB_SRC).toContain('role="group"');
    expect(TAB_SRC).toContain('aria-label={t(locale, "settings.printerTransportTitle")}');
    expect(TAB_SRC).toContain('aria-pressed={transport === "serial"}');
    expect(TAB_SRC).toContain('aria-pressed={transport === "spooler"}');
    expect(TAB_SRC).toContain("settings.printerTransportSerial");
    expect(TAB_SRC).toContain("settings.printerTransportSpooler");
  });

  it("reuses the icons.tsx set (no new icons, no emoji)", () => {
    expect(TAB_SRC).toContain(
      'import { PrinterIcon, DocumentsIcon, SettingsIcon } from "@/components/icons"',
    );
    expect(TAB_SRC).toContain("<SettingsIcon");
    expect(TAB_SRC).toContain("<PrinterIcon");
    expect(TAB_SRC).toContain("<DocumentsIcon");
    // No pictographs as UI icons (charter §4): code-point scan instead of
    // a combined-class regex (lint no-misleading-character-class).
    let pictograph = false;
    for (const ch of TAB_SRC) {
      const cp = ch.codePointAt(0) ?? 0;
      if ((cp >= 0x1f300 && cp <= 0x1faff) || cp === 0xfe0f) {
        pictograph = true;
        break;
      }
    }
    expect(pictograph).toBe(false);
  });
});

describe("conditional fields (COM iff serial, queue iff spooler)", () => {
  it("gates COM fields on the serial arm and the queue field on spooler", () => {
    expect(TAB_SRC).toContain('{transport === "serial" ? (');
    expect(TAB_SRC).toContain("settings.printerQueueName");
    expect(TAB_SRC).toContain('placeholder={t(locale, "settings.printerQueuePlaceholder")}');
    expect(TAB_SRC).toContain("settings.printerQueueHelp");
  });

  it("keeps paper width visible on both arms", () => {
    const branchAt = TAB_SRC.indexOf('{transport === "serial" ? (');
    const paperAt = TAB_SRC.indexOf("settings.paperWidth", branchAt);
    const guideAt = TAB_SRC.indexOf("printerComGuideTitle", branchAt);
    // TASK-PRT-05 layout: connection card (branch + guide), then the ticket
    // card hosting paper width — the guide now precedes paper width.
    expect(paperAt).toBeGreaterThan(branchAt);
    expect(paperAt).toBeGreaterThan(guideAt);
  });

  it("isolates the queue name LTR and never clears the hidden arm", () => {
    const queueBlock = TAB_SRC.slice(
      TAB_SRC.indexOf("settings.printerQueueName"),
      TAB_SRC.indexOf("settings.printerQueueHelp"),
    );
    expect(queueBlock).toContain('<bdi dir="ltr"');
    // Switching arms only flips state; no setter blanks the other field.
    expect(TAB_SRC).not.toContain('setSerialPort("")');
    expect(TAB_SRC).not.toContain('setQueueName("")');
  });
});

describe("save validation + payload", () => {
  it("blocks the spooler save on an empty queue with an inline message (no IPC)", () => {
    const saveAt = TAB_SRC.indexOf("async function handleSave");
    const guardAt = TAB_SRC.indexOf('trimmedQueue === ""', saveAt);
    const callAt = TAB_SRC.indexOf("await ipcUpdatePrinterSettings(", saveAt);
    expect(guardAt).toBeGreaterThan(saveAt);
    expect(callAt).toBeGreaterThan(guardAt);
    expect(
      TAB_SRC.slice(guardAt, callAt),
    ).toContain('settings.printerQueueRequired');
  });

  it("sends transport + trimmed queue through the facade", () => {
    expect(TAB_SRC).toContain("transport,\n        trimmedQueue,");
    expect(IPC_SRC).toContain("queue_name: queueName,");
    expect(IPC_SRC).toContain("transport,");
    expect(IPC_SRC).toContain("transport: wire.transport === 'spooler' ? 'spooler' : 'serial'");
    expect(IPC_SRC).toContain("queueName: typeof wire.queue_name === 'string'");
  });
});

describe("i18n (FR/AR symmetric, stale denial gone)", () => {
  it("declares every new key in both locales", () => {
    for (const key of NEW_KEYS) {
      expect(I18N_SRC).toContain(`"${key}"`);
    }
    // Each key twice (FR + AR blocks).
    for (const key of NEW_KEYS) {
      const first = I18N_SRC.indexOf(`"${key}"`);
      expect(I18N_SRC.indexOf(`"${key}"`, first + 1)).toBeGreaterThan(first);
    }
  });

  it("no longer claims USB-direct is unsupported", () => {
    expect(I18N_SRC).not.toContain("n'est pas prise en charge");
    expect(I18N_SRC).not.toContain("غير مدعومة");
  });
});
