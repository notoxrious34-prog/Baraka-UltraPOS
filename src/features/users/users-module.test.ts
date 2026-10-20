/**
 * Users module tests (TASK-DSN-030): strict Admin gating, navigation
 * wiring, IPC binding shape, and pure helper behavior. Node-only harness
 * (no jsdom): source-text assertions + pure logic.
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
const MODULE_SRC = readSrc("src", "features", "users", "UsersModule.tsx");
const TABLE_SRC = readSrc("src", "features", "users", "UserListTable.tsx");
const FORM_SRC = readSrc("src", "features", "users", "UserFormModal.tsx");
const RESET_SRC = readSrc("src", "features", "users", "CredentialResetModal.tsx");
const HELPER_SRC = readSrc("src", "features", "users", "user-helpers.ts");

import {
  computeUserMetrics,
  roleBadgeClass,
  roleLabel,
  statusBadgeClass,
  userErrorMessage,
  validatePassword,
  validatePin,
  validateUsername,
} from "./user-helpers";
import type { UserInfo } from "@/lib/ipc";

const ROW = (overrides: Partial<UserInfo> = {}): UserInfo => ({
  id: "00000000-0000-0000-0000-000000000001",
  username: "admin",
  role: "ADMIN",
  is_active: true,
  has_pin: false,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  ...overrides,
});

describe("users nav gating (USER_MANAGE permission)", () => {
  it("extends AppView with users", () => {
    expect(VIEW_SRC).toContain('"users"');
  });

  it("renders the Users nav item behind the USER_MANAGE permission", () => {
    expect(APP_SRC).toContain('hasPermission("USER_MANAGE")');
    expect(APP_SRC).toContain("canManageUsers");
    expect(APP_SRC).not.toContain('role === "ADMIN" && entry("users"');
  });

  it("routes view users behind the permission gate with an unauthorized card", () => {
    expect(APP_SRC).toContain('view === "users"');
    expect(APP_SRC).toContain("users.unauthorized");
    expect(APP_SRC).toContain("<UsersModule />");
  });

  it("never mentions GERANT/CAISSIER/VENDEUR in the users gate", () => {
    const gateLines = APP_SRC.split("\n").filter(
      (line) => line.includes('"users"') || line.includes("UsersModule"),
    );
    for (const line of gateLines) {
      expect(line).not.toMatch(/GERANT|CAISSIER|VENDEUR/);
    }
  });
});

describe("users IPC bindings", () => {
  it("exports the four BRK-064 wrappers mapping to the backend commands", () => {
    for (const pair of [
      ["ipcListUsers", '"users_list"'],
      ["ipcCreateUser", '"users_create"'],
      ["ipcUpdateUser", '"users_update"'],
      ["ipcResetUserCredentials", '"users_reset_credentials"'],
    ]) {
      expect(IPC_SRC).toContain(pair[0]);
      expect(IPC_SRC).toContain(pair[1]);
    }
  });

  it("module dispatches list/create/update/reset through the injected bindings", () => {
    expect(MODULE_SRC).toContain("ipcListUsers(session.token)");
    expect(FORM_SRC).toContain("await createUser(sessionToken");
    expect(FORM_SRC).toContain("await updateUser(sessionToken");
    expect(RESET_SRC).toContain("await resetCredentials(sessionToken");
  });
});

describe("users i18n (FR/AR symmetric)", () => {
  it("declares nav.users and the users.* namespace", () => {
    for (const key of [
      '"nav.users"',
      '"users.title"',
      '"users.total"',
      '"users.admins"',
      '"users.newUser"',
      '"users.formCreate"',
      '"users.formEdit"',
      '"users.resetTitle"',
      '"users.unauthorized"',
    ]) {
      expect(I18N_SRC).toContain(key);
    }
  });

  it("carries the mandated Algerian business terminology", () => {
    for (const term of [
      "المستخدمين والصلاحيات",
      "مستخدم جديد",
      "اسم المستخدم",
      "كلمة المرور",
      "نشط",
      "معطل",
    ]) {
      expect(I18N_SRC).toContain(term);
    }
    // Role labels and conflict messages live in the pure helper (no i18n keys).
    for (const term of [
      "مدير عام",
      "مسيّر",
      "أمين صندوق",
      "بائع",
      "لا يمكن تعطيل أو خفض صلاحيات آخر مدير عام نشط",
      "اسم المستخدم مستعمل مسبقاً",
    ]) {
      expect(HELPER_SRC).toContain(term);
    }
  });
});

describe("role badges and labels", () => {
  it("covers every canonical role code with a semantic token palette", () => {
    expect(roleLabel("ADMIN", "fr-DZ")).toBe("Administrateur");
    expect(roleLabel("GERANT", "ar-DZ")).toBe("مسيّر");
    expect(roleBadgeClass("ADMIN")).toContain("surface-sunken");
    expect(roleBadgeClass("GERANT")).toContain("info-");
    expect(roleBadgeClass("CAISSIER")).toContain("warning-");
    expect(roleBadgeClass("VENDEUR")).toContain("success-");
  });

  it("localizes all four roles in FR and AR", () => {
    expect(roleLabel("GERANT", "fr-DZ")).toBe("Gérant");
    expect(roleLabel("CAISSIER", "fr-DZ")).toBe("Caissier");
    expect(roleLabel("VENDEUR", "fr-DZ")).toBe("Vendeur");
    expect(roleLabel("ADMIN", "ar-DZ")).toBe("مدير عام");
    expect(roleLabel("CAISSIER", "ar-DZ")).toBe("أمين صندوق");
    expect(roleLabel("VENDEUR", "ar-DZ")).toBe("بائع");
  });

  it("falls back to the raw code for unknown roles", () => {
    expect(roleLabel("SUPERADMIN", "fr-DZ")).toBe("SUPERADMIN");
    expect(roleBadgeClass("SUPERADMIN")).toContain("surface-sunken");
  });

  it("labels active/inactive status with status tokens", () => {
    expect(statusBadgeClass(true)).toContain("success-");
    // Inactive uses secondary-on-sunken (9.20:1); muted-on-sunken fails AA.
    expect(statusBadgeClass(false)).toContain("surface-sunken");
    expect(statusBadgeClass(false)).toContain("ink-secondary");
  });
});

describe("metric HUD calculations", () => {
  it("counts total, admins, cash-and-sales, and inactive accounts", () => {
    const users = [
      ROW({ username: "admin" }),
      ROW({ username: "gerant1", role: "GERANT" }),
      ROW({ username: "caisse1", role: "CAISSIER", has_pin: true }),
      ROW({ username: "vendeur1", role: "VENDEUR", is_active: false }),
    ];
    const metrics = computeUserMetrics(users);
    expect(metrics.total).toBe(4);
    expect(metrics.admins).toBe(1);
    expect(metrics.cashAndSales).toBe(2);
    expect(metrics.inactive).toBe(1);
  });

  it("renders four HUD cards without page scrollbars (inner table scrolls)", () => {
    expect(MODULE_SRC).toContain("users.total");
    expect(MODULE_SRC).toContain("users.admins");
    expect(MODULE_SRC).toContain("users.cashiers");
    expect(MODULE_SRC).toContain("users.inactive");
    expect(MODULE_SRC).toContain("overflow-hidden");
    expect(TABLE_SRC).toContain("flex-1 overflow-auto");
  });
});

describe("form validation (mirrors engine rules)", () => {
  it("accepts 3–30 ASCII letters/digits/underscore usernames", () => {
    expect(validateUsername("caisse1")).toBe(true);
    expect(validateUsername("ab")).toBe(false);
    expect(validateUsername("x".repeat(31))).toBe(false);
    expect(validateUsername("mélanie")).toBe(false);
  });

  it("requires a 6-char minimum password", () => {
    expect(validatePassword("secret1")).toBe(true);
    expect(validatePassword("12345")).toBe(false);
  });

  it("requires a 4–6 digit PIN", () => {
    expect(validatePin("1234")).toBe(true);
    expect(validatePin("123456")).toBe(true);
    expect(validatePin("123")).toBe(false);
    expect(validatePin("1234567")).toBe(false);
    expect(validatePin("12a4")).toBe(false);
  });
});

describe("credential reset modal", () => {
  it("masks password and PIN fields (ADR-079)", () => {
    expect((RESET_SRC.match(/type="password"/g) ?? []).length).toBeGreaterThanOrEqual(2);
    expect(RESET_SRC).toContain('inputMode="numeric"');
  });

  it("requires at least one credential before saving", () => {
    expect(RESET_SRC).toContain("password !== ");
    expect(RESET_SRC).toContain("users.resetHint");
  });
});

describe("create/edit modal", () => {
  it("masks secrets and trims the username", () => {
    expect((FORM_SRC.match(/type="password"/g) ?? []).length).toBeGreaterThanOrEqual(2);
    expect(FORM_SRC).toContain("username.trim()");
  });

  it("exposes the four canonical role options", () => {
    expect(FORM_SRC).toContain("roleOptions()");
  });
});

describe("backend conflict codes surface localized messages", () => {
  it("localizes LAST_ADMIN_PROTECTED in FR and AR", () => {
    expect(userErrorMessage("LAST_ADMIN_PROTECTED", "fr-DZ")).toContain("dernier administrateur");
    expect(userErrorMessage("LAST_ADMIN_PROTECTED", "ar-DZ")).toContain(
      "لا يمكن تعطيل أو خفض صلاحيات آخر مدير عام نشط",
    );
  });

  it("localizes USERNAME_TAKEN, INVALID_PIN, and WEAK_PASSWORD", () => {
    expect(userErrorMessage("USERNAME_TAKEN", "fr-DZ")).toContain("déjà utilisé");
    expect(userErrorMessage("USERNAME_TAKEN", "ar-DZ")).toContain("مستعمل مسبقاً");
    expect(userErrorMessage("INVALID_PIN", "fr-DZ")).toContain("4 à 6 chiffres");
    expect(userErrorMessage("WEAK_PASSWORD", "fr-DZ")).toContain("6 caractères");
  });

  it("falls back to the raw code for unknown errors", () => {
    expect(userErrorMessage("SOME_FUTURE_CODE", "fr-DZ")).toBe("SOME_FUTURE_CODE");
  });
});

describe("users permission gating (ADR-100 / DSN-071)", () => {
  it("gates the Users nav entry on hasPermission('USER_MANAGE')", () => {
    expect(APP_SRC).toContain('hasPermission("USER_MANAGE")');
    expect(APP_SRC).not.toContain('role === "ADMIN" && entry("users"');
  });

  it("routes the users view behind the granular check with the unauthorized fallback", () => {
    expect(APP_SRC).toContain("canManageUsers");
    expect(APP_SRC).toContain("users.unauthorized");
  });
});

describe("semantic design tokens (DSN-071)", () => {
  it("uses token classes and zero raw slate/gray in the Users module", () => {
    const sources: Array<[string, string]> = [
      ["UsersModule", MODULE_SRC],
      ["UserListTable", TABLE_SRC],
      ["UserFormModal", FORM_SRC],
      ["CredentialResetModal", RESET_SRC],
      ["user-helpers", HELPER_SRC],
    ];
    for (const [name, src] of sources) {
      expect(`${name}: ${src.includes("slate-")}`).toBe(`${name}: false`);
      expect(`${name}: ${src.includes("gray-")}`).toBe(`${name}: false`);
    }
    expect(MODULE_SRC).toContain("bg-surface-canvas");
    expect(TABLE_SRC).toContain("text-ink-");
    expect(FORM_SRC).toContain("border-line-default");
  });

  it("badges use status tokens with light/dark pairing", () => {
    expect(roleBadgeClass("ADMIN")).toContain("surface-sunken");
    expect(roleBadgeClass("GERANT")).toContain("info-");
    expect(roleBadgeClass("CAISSIER")).toContain("warning-");
    expect(roleBadgeClass("VENDEUR")).toContain("success-");
    expect(statusBadgeClass(true)).toContain("success-");
    expect(statusBadgeClass(false)).toContain("surface-sunken");
  });
});

describe("last-admin proactive protection (DSN-071)", () => {
  it("counts the sole active admin and disables its deactivation path with guidance", () => {
    expect(TABLE_SRC).toContain("activeAdminCount");
    expect(TABLE_SRC).toContain("users.lastAdminTitle");
    expect(TABLE_SRC).toContain("users.lastAdminBadge");
    expect(FORM_SRC).toContain("isProtected");
  });
});

describe("rtl bidi isolation (DSN-071)", () => {
  it("wraps the reset-modal username in a bidi guard", () => {
    expect(RESET_SRC).toContain("<bdi");
    expect(RESET_SRC).toContain('dir="ltr"');
  });
});

describe("directory table rendering contract", () => {
  it("isolates usernames and timestamps with bidi guards", () => {
    expect(TABLE_SRC).toContain('<bdi dir="ltr">');
  });

  it("shows PIN state, edit, and credential actions plus an empty state", () => {
    expect(TABLE_SRC).toContain("users.pinSet");
    expect(TABLE_SRC).toContain("users.edit");
    expect(TABLE_SRC).toContain("users.credentials");
    expect(TABLE_SRC).toContain("users.empty");
  });
});
