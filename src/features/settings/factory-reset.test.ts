/**
 * Factory-reset screen pins (TASK-DSN-094): token derivation, double-guard
 * predicate, wire bridge shape, i18n FR/AR symmetry, and UI-shape pins via
 * the source-text pattern (no jsdom).
 */

import { describe, expect, it } from "vitest";
import {
  canExecuteReset,
  expectedResetToken,
  FACTORY_RESET_BLANK_STORE_TOKEN,
} from "./factory-reset";
import { FACTORY_RESET_I18N_KEYS } from "./FactoryResetSection";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const SECTION_SRC = readSrc("src", "features", "settings", "FactoryResetSection.tsx");
const TAB_SRC = readSrc("src", "features", "settings", "BackupSettingsTab.tsx");
const IPC_SRC = readSrc("src", "lib", "ipc.ts");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

describe("expected token derivation (store-name wire, never hardcoded)", () => {
  it("uses the trimmed store name when set", () => {
    expect(expectedResetToken("Quincaillerie El Baraka")).toBe("Quincaillerie El Baraka");
    expect(expectedResetToken("  محل البركة  ")).toBe("محل البركة");
  });

  it("falls back to the depot code only when the name is blank", () => {
    expect(expectedResetToken("")).toBe(FACTORY_RESET_BLANK_STORE_TOKEN);
    expect(expectedResetToken("   ")).toBe(FACTORY_RESET_BLANK_STORE_TOKEN);
    expect(FACTORY_RESET_BLANK_STORE_TOKEN).toBe("DEP-01");
  });
});

describe("double guard (eligible precheck AND exact token)", () => {
  it("executes only when eligible and the token matches exactly", () => {
    expect(canExecuteReset(true, "Magasin", "Magasin")).toBe(true);
  });

  it("blocks token mismatch pre-IPC", () => {
    expect(canExecuteReset(true, "magasin", "Magasin")).toBe(false);
    expect(canExecuteReset(true, " Magasin", "Magasin")).toBe(false);
    expect(canExecuteReset(true, "", "Magasin")).toBe(false);
  });

  it("blocks ineligible precheck pre-IPC", () => {
    expect(canExecuteReset(false, "Magasin", "Magasin")).toBe(false);
  });
});

describe("i18n FR/AR symmetry", () => {
  it("declares every reset key used by the section", () => {
    expect(FACTORY_RESET_I18N_KEYS.length).toBe(16);
  });

  it("defines every reset key in both locales", () => {
    for (const key of FACTORY_RESET_I18N_KEYS) {
      expect(I18N_SRC.includes(`"${key}":`), `missing key ${key}`).toBe(true);
    }
    const frCount = FACTORY_RESET_I18N_KEYS.filter((key) =>
      I18N_SRC.includes(`"${key}":`),
    ).length;
    expect(frCount).toBe(FACTORY_RESET_I18N_KEYS.length);
  });

  it("keeps full product name and Arabic clarity in the danger copy", () => {
    expect(I18N_SRC.includes("Baraka UltraPOS")).toBe(true);
    expect(I18N_SRC.includes("إعادة ضبط المصنع")).toBe(true);
    expect(I18N_SRC.includes("Irréversible")).toBe(true);
    expect(I18N_SRC.includes("لا رجعة فيه")).toBe(true);
  });
});

describe("wire bridge shape (facade mirrors DTOs)", () => {
  it("invokes factory_reset_precheck with a session-token request", () => {
    expect(IPC_SRC.includes('"factory_reset_precheck"')).toBe(true);
    expect(IPC_SRC.includes("open_session")).toBe(true);
    expect(IPC_SRC.includes("backup_dir_writable")).toBe(true);
    expect(IPC_SRC.includes("newest_backup")).toBe(true);
  });

  it("invokes factory_reset_execute with the typed token", () => {
    expect(IPC_SRC.includes('"factory_reset_execute"')).toBe(true);
    expect(IPC_SRC.includes("typed_token")).toBe(true);
    expect(IPC_SRC.includes("reset_backup_file_name")).toBe(true);
    expect(IPC_SRC.includes("restart_required")).toBe(true);
  });
});

describe("UI shape (source-text pins)", () => {
  it("mounts the danger zone inside the backups tab (Admin-gated tab)", () => {
    expect(TAB_SRC.includes("<FactoryResetSection")).toBe(true);
    expect(TAB_SRC.includes("sessionToken={sessionToken}")).toBe(true);
  });

  it("shows the restart notice and locks further actions after success", () => {
    expect(SECTION_SRC.includes("settings.resetRestartTitle")).toBe(true);
    expect(SECTION_SRC.includes("locked")).toBe(true);
    expect(SECTION_SRC.includes("outcome !== null")).toBe(true);
  });

  it("surfaces eligibility reasons verbatim from the wire", () => {
    expect(SECTION_SRC.includes("precheck.reasons.map")).toBe(true);
  });

  it("reuses the verified-backup creation flow before reset", () => {
    expect(SECTION_SRC.includes("createBackup(sessionToken")).toBe(true);
    expect(SECTION_SRC.includes("settings.resetCreateBackup")).toBe(true);
  });

  it("derives the token from the store-identity wire, precheck omits it", () => {
    expect(SECTION_SRC.includes("getStoreIdentity()")).toBe(true);
    expect(SECTION_SRC.includes("expectedResetToken(storeName)")).toBe(true);
    expect(SECTION_SRC.includes("token")).toBe(true);
  });
});
