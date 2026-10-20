/**
 * Settings module tests (TASK-DSN-031): manager gating, tab wiring, IPC
 * binding shape, and pure helper behavior. Node-only harness (no jsdom):
 * source-text assertions + pure logic.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
// Type-only; zero runtime impact.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const I18N_SRC = readSrc("src", "lib", "i18n.ts");
const IPC_SRC = readSrc("src", "lib", "ipc.ts");
const APP_SRC = readSrc("src", "App.tsx");
const VIEW_SRC = readSrc("src", "stores", "app-view-store.ts");
const MODULE_SRC = readSrc("src", "features", "settings", "SettingsModule.tsx");
const IDENTITY_SRC = readSrc("src", "features", "settings", "StoreIdentityTab.tsx");
const PRINTER_SRC = readSrc("src", "features", "settings", "PrinterSettingsTab.tsx");
const BACKUP_SRC = readSrc("src", "features", "settings", "BackupSettingsTab.tsx");

import {
  checksumPrefix,
  formatBytes,
  triggerBadgeClass,
  validateFiscalField,
  validatePhone,
  validateStoreName,
} from "./settings-helpers";

describe("settings nav gating (manager)", () => {
  it("extends AppView with settings", () => {
    expect(VIEW_SRC).toContain('"settings"');
  });

  it("renders the Settings nav item for managers (no ADMIN-only gate)", () => {
    expect(APP_SRC).toContain('entry("settings"');
    const navBlock = APP_SRC.slice(APP_SRC.indexOf("function NavRail"), APP_SRC.indexOf("</nav>"));
    expect(navBlock).toContain('entry("settings"');
    expect(navBlock).not.toMatch(/ADMIN.*entry\("settings"|settings.*ADMIN/);
  });

  it("routes view settings behind the permission gate", () => {
    expect(APP_SRC).toContain("canAccessSettings");
    expect(APP_SRC).toContain('hasPermission("SYSTEM_SETTINGS_MANAGE")');
    expect(APP_SRC).toContain("<SettingsModule />");
  });
});

describe("settings tabs", () => {
  it("declares identity, printer, and backups tabs", () => {
    expect(MODULE_SRC).toContain("settings.tabIdentity");
    expect(MODULE_SRC).toContain("settings.tabPrinter");
    expect(MODULE_SRC).toContain("settings.tabBackups");
    expect(MODULE_SRC).toContain("<StoreIdentityTab");
    expect(MODULE_SRC).toContain("<PrinterSettingsTab");
    expect(MODULE_SRC).toContain("<BackupSettingsTab");
  });

  it("keeps the container scrollbar-free with independently scrolling panes", () => {
    expect(MODULE_SRC).toContain("overflow-hidden");
    expect(IDENTITY_SRC).toContain("overflow-y-auto");
    expect(PRINTER_SRC).toContain("overflow-y-auto");
    expect(BACKUP_SRC).toContain("flex-1 overflow-auto");
  });

  it("gates restricted tabs by write permission with a lock badge, never a role check", () => {
    expect(BACKUP_SRC).toContain("settings.backupRestricted");
    expect(MODULE_SRC).toContain("managePermissions");
    expect(MODULE_SRC).toContain("canWriteTab");
    expect(MODULE_SRC).not.toContain("lockedForNonAdmin");
    expect(MODULE_SRC).not.toContain('session?.role === "ADMIN"');
    expect(MODULE_SRC).not.toContain("session.role");
  });

  it("maps each tab to its engine write permission", () => {
    expect(MODULE_SRC).toContain('managePermissions: ["SYSTEM_SETTINGS_MANAGE"]');
    expect(MODULE_SRC).toContain('managePermissions: ["STOCK_ADJUSTMENT", "SYSTEM_SETTINGS_MANAGE"]');
    expect(MODULE_SRC).toContain('managePermissions: ["MANAGE_PRINTER_SETTINGS"]');
    expect(MODULE_SRC).toContain("managePermissions: []");
  });
});

describe("store identity bindings", () => {
  it("loads via getStoreIdentity and saves via saveStoreIdentity", () => {
    expect(IDENTITY_SRC).toContain("getStoreIdentity()");
    expect(IDENTITY_SRC).toContain("await saveStoreIdentity(identity, deviceKind)");
    expect(IPC_SRC).toContain('"store_identity_get"');
    expect(IPC_SRC).toContain('"store_identity_save"');
  });

  it("renders the four fiscal identifiers with LTR isolation", () => {
    for (const key of ["settings.rc", "settings.nif", "settings.nis", "settings.ai"]) {
      expect(IDENTITY_SRC).toContain(key);
    }
    expect(IDENTITY_SRC).toContain('<bdi dir="ltr"');
  });

  it("validates the commercial name (2–100 chars)", () => {
    expect(validateStoreName("Quincaillerie Baraka")).toBe(true);
    expect(validateStoreName("A")).toBe(false);
    expect(validateStoreName("  ")).toBe(false);
  });

  it("validates fiscal fields (empty ok, 2–30 RC/NIF/NIS/AI shapes)", () => {
    expect(validateFiscalField("")).toBe(true);
    expect(validateFiscalField("16/00-1234567 B 23")).toBe(true);
    expect(validateFiscalField("X")).toBe(false);
    expect(validateFiscalField("12@#")).toBe(false);
  });

  it("validates phone numbers (empty ok, 6–20 dial shapes)", () => {
    expect(validatePhone("")).toBe(true);
    expect(validatePhone("+213 555 12 34 56")).toBe(true);
    expect(validatePhone("123")).toBe(false);
    expect(validatePhone("abc-def")).toBe(false);
  });
});

describe("printer bindings", () => {
  it("loads, updates, and test-prints through the ipc.ts wrappers", () => {
    expect(PRINTER_SRC).toContain("ipcGetPrinterSettings(sessionToken)");
    expect(PRINTER_SRC).toContain(
      "await ipcUpdatePrinterSettings(",
    );
    expect(PRINTER_SRC).toContain("paperWidthMm,");
    expect(PRINTER_SRC).toContain('handleTestPrint("fr")');
    expect(PRINTER_SRC).toContain('handleTestPrint("ar")');
    expect(IPC_SRC).toContain('"printer_get_settings"');
    expect(IPC_SRC).toContain('"printer_update_settings"');
    expect(IPC_SRC).toContain('"printer_test_print"');
  });

  it("offers baud options and isolates the serial port LTR", () => {
    expect(PRINTER_SRC).toContain("BAUD_OPTIONS");
    expect(PRINTER_SRC).toContain('<bdi dir="ltr"');
  });

  it("exposes a bilingual 58/80mm paper width selector persisted via IPC", () => {
    expect(PRINTER_SRC).toContain("settings.paperWidth80");
    expect(PRINTER_SRC).toContain("settings.paperWidth58");
    expect(PRINTER_SRC).toContain("setPaperWidthMm(Number(event.target.value))");
    expect(PRINTER_SRC).toContain("settings.paperWidthMm");
    expect(IPC_SRC).toContain("paper_width_mm");
    expect(IPC_SRC).toContain("paperWidthMm: wire.paper_width_mm");
  });
});

describe("backup bindings", () => {
  it("creates MANUAL snapshots and lists history through the ipc.ts wrappers", () => {
    expect(BACKUP_SRC).toContain('await createBackup(sessionToken, "MANUAL")');
    expect(BACKUP_SRC).toContain("await listBackups(sessionToken)");
    expect(IPC_SRC).toContain('"backups_create"');
    expect(IPC_SRC).toContain('"backups_list"');
  });

  it("renders file/date/type/size/status columns with bidi guards", () => {
    for (const key of [
      "settings.colFile",
      "settings.colDate",
      "settings.colType",
      "settings.colSize",
      "settings.colStatus",
    ]) {
      expect(BACKUP_SRC).toContain(key);
    }
    expect(BACKUP_SRC).toContain('<bdi dir="ltr">');
    expect(BACKUP_SRC).toContain("settings.backupEmpty");
  });

  it("formats byte sizes (B / KB / MB)", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
    expect(formatBytes(-1)).toBe("0 B");
  });

  it("badges MANUAL blue and other triggers green", () => {
    expect(triggerBadgeClass("MANUAL")).toContain("blue");
    expect(triggerBadgeClass("SHIFT_CLOSE")).toContain("green");
  });

  it("shortens checksums to a 12-char prefix", () => {
    expect(checksumPrefix("abcdef1234567890")).toBe("abcdef123456…");
    expect(checksumPrefix("abc")).toBe("abc");
  });
});

describe("settings i18n (FR/AR symmetric)", () => {
  it("declares nav.settings and the settings.* namespace", () => {
    for (const key of [
      '"nav.settings"',
      '"settings.title"',
      '"settings.tabIdentity"',
      '"settings.tabPrinter"',
      '"settings.tabBackups"',
      '"settings.save"',
      '"settings.saved"',
      '"settings.createBackup"',
      '"settings.backupRestricted"',
    ]) {
      expect(I18N_SRC).toContain(key);
    }
  });

  it("carries the mandated Algerian commercial terminology", () => {
    for (const term of [
      "الإعدادات وضبط النظام",
      "هوية المتجر والفواتير",
      "الطباعة والملحقات",
      "النسخ الاحتياطي والأمان",
      "اسم المحل التجاري",
      "الهاتف",
      "العنوان",
      "السجل التجاري",
      "رقم التعريف الجبائي",
      "رقم التعريف الإحصائي",
      "المادة الضريبية",
      "تذييل الوصل",
      "اختبار الطباعة",
      "إنشاء نسخة احتياطية الآن",
      "حجم النسخة",
      "تم حفظ الإعدادات بنجاح",
    ]) {
      expect(I18N_SRC).toContain(term);
    }
  });
});

describe("settings write gating with guidance (TASK-DSN-079)", () => {
  const FISCAL_SRC: string = readSrc("src", "features", "settings", "FiscalSettingsTab.tsx");

  it("gates the fiscal switch by SYSTEM_SETTINGS_MANAGE with guidance, no isAdmin prop", () => {
    expect(FISCAL_SRC).toContain('hasPermission("SYSTEM_SETTINGS_MANAGE")');
    expect(FISCAL_SRC).toContain("disabled={saving || !canWrite}");
    expect(FISCAL_SRC).toContain("settings.fiscalWriteReservedGuidance");
    expect(FISCAL_SRC).not.toContain("isAdmin");
  });

  it("gates backup creation by SYSTEM_SETTINGS_MANAGE with a disabled button, no hide", () => {
    expect(BACKUP_SRC).toContain('hasPermission("SYSTEM_SETTINGS_MANAGE")');
    expect(BACKUP_SRC).toContain("settings.backupCreateReservedGuidance");
    expect(BACKUP_SRC).toContain("disabled");
    expect(BACKUP_SRC).not.toContain("isAdmin");
    expect(BACKUP_SRC).toContain("settings.backupRestricted");
  });

  it("declares all five guidance keys in both locales", () => {
    for (const key of [
      "settings.identityWriteReservedGuidance",
      "settings.catalogManageReservedGuidance",
      "settings.printerManageReservedGuidance",
      "settings.fiscalWriteReservedGuidance",
      "settings.backupCreateReservedGuidance",
    ]) {
      expect(I18N_SRC).toContain(`"${key}"`);
    }
    expect(I18N_SRC).toContain('"settings.fiscalWriteReservedGuidance": "Réservé — Gestion système."');
    expect(I18N_SRC).toContain('"settings.backupCreateReservedGuidance": "مقيّد — إدارة النظام."');
  });
});
