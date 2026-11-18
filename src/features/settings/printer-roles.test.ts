/**
 * Dual-printer role routing pins (TASK-BRK-113): counter Receipt role vs
 * dedicated 40x20 Label role. Node-only harness: source-text assertions
 * (no jsdom), same pattern as printer-transport.test.ts.
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
const IPC_SRC = [
  "ipc.ts",
  "ipc-kernel.ts",
  "ipc-auth.ts",
  "ipc-catalog.ts",
  "ipc-inventory.ts",
  "ipc-users.ts",
  "ipc-sales.ts",
  "ipc-customers.ts",
  "ipc-printing.ts",
  "ipc-cash.ts",
  "ipc-suppliers.ts",
  "ipc-reports.ts",
  "ipc-backups.ts",
].map((file) => readSrc("src", "lib", file)).join("\n");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

const NEW_KEYS = [
  "settings.sectionRoles",
  "settings.sectionRolesDesc",
  "settings.roleReceipt",
  "settings.roleReceiptDesc",
  "settings.roleLabel",
  "settings.roleLabelDesc",
  "settings.rolePrimaryQueue",
  "settings.roleFallbackQueue",
  "settings.roleFallbackHelp",
  "settings.roleQueueRequired",
];

describe("roles section (receipt + label groups, own save)", () => {
  it("renders two role groups behind one roles section with its own save", () => {
    expect(TAB_SRC).toContain('title={t(locale, "settings.sectionRoles")}');
    expect(TAB_SRC).toContain('title={t(locale, "settings.roleReceipt")}');
    expect(TAB_SRC).toContain('title={t(locale, "settings.roleLabel")}');
    expect(TAB_SRC).toContain("void handleSaveRoles();");
    expect(TAB_SRC).toContain("async function handleSaveRoles");
  });

  it("toggles each role arm independently and gates queues on spooler", () => {
    expect(TAB_SRC).toContain('arm={receiptTransport}');
    expect(TAB_SRC).toContain('arm={labelTransport}');
    expect(TAB_SRC).toContain('aria-pressed={arm === "serial"}');
    expect(TAB_SRC).toContain('aria-pressed={arm === "spooler"}');
    expect(TAB_SRC).toContain('{arm === "spooler" ? (');
    expect(TAB_SRC).toContain('settings.rolePrimaryQueue');
    expect(TAB_SRC).toContain('settings.roleFallbackQueue');
    expect(TAB_SRC).toContain('settings.roleFallbackHelp');
  });

  it("blocks a spooler role with an empty primary queue (no IPC)", () => {
    const saveAt = TAB_SRC.indexOf("async function handleSaveRoles");
    const guardAt = TAB_SRC.indexOf('primaryLabel === ""', saveAt);
    const callAt = TAB_SRC.indexOf("await ipcUpdatePrinterRoleSettings(", saveAt);
    expect(guardAt).toBeGreaterThan(saveAt);
    expect(callAt).toBeGreaterThan(guardAt);
    expect(TAB_SRC.slice(guardAt, callAt)).toContain("settings.roleQueueRequired");
  });
});

describe("role facade (six keys, null-keeps, full parse)", () => {
  it("sends the six role keys through printer_update_settings", () => {
    expect(IPC_SRC).toContain("export async function ipcUpdatePrinterRoleSettings(");
    expect(IPC_SRC).toContain("receipt_transport: patch.receiptTransport ?? null,");
    expect(IPC_SRC).toContain("receipt_queue_name: patch.receiptQueueName ?? null,");
    expect(IPC_SRC).toContain(
      "receipt_queue_name_fallback: patch.receiptQueueNameFallback ?? null,",
    );
    expect(IPC_SRC).toContain("label_transport: patch.labelTransport ?? null,");
    expect(IPC_SRC).toContain("label_queue_name: patch.labelQueueName ?? null,");
    expect(IPC_SRC).toContain("label_queue_name_fallback: patch.labelQueueNameFallback ?? null,");
  });

  it("parses all six role fields (receipt defaults serial, label spooler)", () => {
    expect(IPC_SRC).toContain("receiptTransport: toRoleTransport(");
    expect(IPC_SRC).toContain("labelTransport: toRoleTransport(");
    expect(IPC_SRC).toContain("receiptQueueName: toQueueName(");
    expect(IPC_SRC).toContain("labelQueueName: toQueueName(");
    expect(IPC_SRC).toContain("receiptQueueNameFallback: toQueueName(");
    expect(IPC_SRC).toContain("labelQueueNameFallback: toQueueName(");
  });
});

describe("roles i18n (FR/AR symmetric)", () => {
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
});
