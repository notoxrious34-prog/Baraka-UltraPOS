/**
 * TSPL probe-mode selector pins (TASK-DSN-100): TEMPORARY-DIAGNOSTIC
 * segmented control in PrinterSettingsTab + ipc facade. Node-only harness:
 * source-text assertions (no jsdom), same pattern as printer-transport.test.ts.
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

const MODES = ["normal", "no_codepage", "ascii_only", "font1_only"] as const;
const LABEL_KEYS = [
  "settings.tsplProbeNormal",
  "settings.tsplProbeNoCodepage",
  "settings.tsplProbeAsciiOnly",
  "settings.tsplProbeFont1Only",
];
const NOTE_KEYS = [
  "settings.tsplProbeNoteNormal",
  "settings.tsplProbeNoteNoCodepage",
  "settings.tsplProbeNoteAsciiOnly",
  "settings.tsplProbeNoteFont1Only",
];

describe("probe selector block (TEMPORARY-DIAGNOSTIC badge + 4 modes)", () => {
  it("renders a badged, visually distinct group with one button per mode", () => {
    expect(TAB_SRC).toContain("TEMPORARY-DIAGNOSTIC");
    expect(TAB_SRC).toContain('aria-label={t(locale, "settings.tsplProbeTitle")}');
    expect(TAB_SRC).toContain('t(locale, "settings.tsplProbeBadge")');
    expect(TAB_SRC).toContain("border-dashed");
    // Buttons render from the label-key map (loop variable, not literals).
    expect(TAB_SRC).toContain("aria-pressed={probeMode === mode}");
    expect(TAB_SRC).toContain("(Object.keys(PROBE_LABEL_KEYS) as TsplProbeMode[]).map((mode)");
    for (const mode of MODES) {
      expect(TAB_SRC).toContain(`${mode}: "settings.tsplProbe`);
    }
  });

  it("carries a per-mode effect note and keys all four labels + notes", () => {
    for (const key of [...LABEL_KEYS, ...NOTE_KEYS]) {
      expect(TAB_SRC).toContain(`"${key}"`);
      expect(I18N_SRC).toContain(`"${key}"`);
    }
    expect(TAB_SRC).toContain("PROBE_NOTE_KEYS[probeMode]");
  });

  it("warns explicitly about Arabic in ascii_only mode (FR + AR)", () => {
    expect(TAB_SRC).toContain('probeMode === "ascii_only"');
    expect(TAB_SRC).toContain('t(locale, "settings.tsplProbeAsciiWarning")');
    expect(I18N_SRC).toContain("l’arabe");
    expect(I18N_SRC).toContain("العربية");
  });
});

describe("facade round-trip (read + validated write, invalid never sent)", () => {
  it("reads the stored mode through the existing settings IPC", () => {
    expect(IPC_SRC).toContain("tsplProbeMode: toTsplProbeMode(");
    expect(IPC_SRC).toContain("(wire as { tspl_probe_mode?: unknown }).tspl_probe_mode");
  });

  it("sends only the four valid spellings (typed union, no stringly hole)", () => {
    expect(IPC_SRC).toContain(
      "export type TsplProbeMode = 'normal' | 'no_codepage' | 'ascii_only' | 'font1_only'",
    );
    expect(IPC_SRC).toContain("tspl_probe_mode: tsplProbeMode ?? null");
    expect(IPC_SRC).toContain("toTsplProbeMode(wire.tspl_probe_mode)");
  });

  it("persists the mode through the tab save and restores it on load", () => {
    expect(TAB_SRC).toContain("setProbeMode(settings.tsplProbeMode)");
    const saveAt = TAB_SRC.indexOf("await ipcUpdatePrinterSettings(");
    expect(saveAt).toBeGreaterThan(-1);
    const callAt = TAB_SRC.indexOf("probeMode,", saveAt);
    expect(callAt).toBeGreaterThan(saveAt);
  });
});

describe("hygiene (no emoji, FR + AR coverage)", () => {
  it("uses no pictographs as UI icons", () => {
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

  it("defines every probe key in both locales", () => {
    const keys = [
      "settings.tsplProbeTitle",
      "settings.tsplProbeBadge",
      ...LABEL_KEYS,
      ...NOTE_KEYS,
      "settings.tsplProbeAsciiWarning",
    ];
    for (const key of keys) {
      const occurrences = I18N_SRC.split(`"${key}"`).length - 1;
      expect(occurrences).toBeGreaterThanOrEqual(2);
    }
  });
});
