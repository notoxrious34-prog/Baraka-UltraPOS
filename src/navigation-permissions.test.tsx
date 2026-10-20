/**
 * Navigation shell permission gating (TASK-DSN-072, Batch 1).
 *
 * Rail visibility + view routing are decided exclusively by
 * `hasPermission(...)` via the pure `canAccessView` helper. Permission sets
 * below mirror the Rust domain matrix (`crates/domain/src/role.rs:121-203`).
 * Rail DOM is asserted through `renderToStaticMarkup` (node-only harness, no
 * DOM library — repo convention); routing decisions through `canAccessView`
 * for all 4 roles x 9 views; wiring through source-text pins.
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { describe, expect, it, beforeEach } from "vitest";
declare const process: { cwd(): string };
import { renderToStaticMarkup } from "react-dom/server";
import { createElement } from "react";
import { AppShell, NavRail, canAccessView } from "@/App";
import { useSessionStore } from "@/stores/session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { useAppViewStore, type AppView } from "@/stores/app-view-store";
import { t } from "@/lib/i18n";
import type { SessionInfo } from "@/types/pos";

const ROOT = process.cwd();
const APP_SRC: string = readFileSync(join(ROOT, "src", "App.tsx"), "utf8");

/** Exact domain permission sets (role.rs:121-203), SCREAMING_SNAKE_CASE wire form. */
const PERMS = {
  VENDEUR: ["POS_SELL", "POS_PARK_RESUME", "VIEW_STOCK", "CREATE_SALE", "VIEW_CUSTOMERS"],
  CAISSIER: [
    "POS_SELL",
    "POS_PARK_RESUME",
    "CASHIER_COLLECT_PAYMENT",
    "CASH_SESSION_OPEN",
    "CASH_SESSION_CLOSE",
    "CASH_EXPENSE_RECORD",
    "CASH_REPORT_VIEW",
    "VIEW_STOCK",
    "CREATE_SALE",
    "VIEW_CUSTOMERS",
    "CREATE_CUSTOMER",
    "RECORD_CREDIT_PAYMENT",
    "REPRINT_DOCUMENT",
  ],
  GERANT: [
    "POS_SELL",
    "POS_PARK_RESUME",
    "CASHIER_COLLECT_PAYMENT",
    "CASH_SESSION_OPEN",
    "CASH_SESSION_CLOSE",
    "CASH_EXPENSE_RECORD",
    "CASH_REPORT_VIEW",
    "PRICE_OVERRIDE",
    "DISCOUNT_OVERRIDE",
    "NEGATIVE_STOCK_OVERRIDE",
    "CREDIT_LIMIT_OVERRIDE",
    "PROCESS_RETURN",
    "STOCK_ADJUSTMENT",
    "PURCHASE_MANAGE",
    "REPORTS_VIEW",
    "VIEW_STOCK",
    "ADJUST_STOCK",
    "TRANSFER_STOCK",
    "CREATE_SALE",
    "VIEW_CUSTOMERS",
    "CREATE_CUSTOMER",
    "MANAGE_CUSTOMER_CREDIT",
    "RECORD_CREDIT_PAYMENT",
    "REPRINT_DOCUMENT",
    "SUPPLIER_MANAGE",
    "PURCHASE_ORDER_CREATE",
    "PURCHASE_RECEIVE",
    "PURCHASE_INVOICE_RECORD",
    "SUPPLIER_PAYMENT_RECORD",
  ],
  ADMIN: [
    "POS_SELL",
    "POS_PARK_RESUME",
    "CASHIER_COLLECT_PAYMENT",
    "CASH_SESSION_OPEN",
    "CASH_SESSION_CLOSE",
    "CASH_EXPENSE_RECORD",
    "CASH_REPORT_VIEW",
    "PRICE_OVERRIDE",
    "DISCOUNT_OVERRIDE",
    "NEGATIVE_STOCK_OVERRIDE",
    "CREDIT_LIMIT_OVERRIDE",
    "PROCESS_RETURN",
    "STOCK_ADJUSTMENT",
    "PURCHASE_MANAGE",
    "REPORTS_VIEW",
    "USER_MANAGE",
    "SYSTEM_SETTINGS_MANAGE",
    "BACKUP_RESTORE_MANAGE",
    "VIEW_STOCK",
    "ADJUST_STOCK",
    "TRANSFER_STOCK",
    "MANAGE_WAREHOUSES",
    "CREATE_SALE",
    "VIEW_CUSTOMERS",
    "CREATE_CUSTOMER",
    "MANAGE_CUSTOMER_CREDIT",
    "RECORD_CREDIT_PAYMENT",
    "REPRINT_DOCUMENT",
    "MANAGE_PRINTER_SETTINGS",
    "SUPPLIER_MANAGE",
    "PURCHASE_ORDER_CREATE",
    "PURCHASE_RECEIVE",
    "PURCHASE_INVOICE_RECORD",
    "SUPPLIER_PAYMENT_RECORD",
  ],
} as const;

function seedSession(role: keyof typeof PERMS): void {
  const permissions = [...PERMS[role]];
  const session: SessionInfo = {
    token: "stub-token",
    userId: `u-${role.toLowerCase()}`,
    username: role.toLowerCase(),
    role,
    permissions,
    expiresAt: "2030-01-01T00:00:00",
  };
  useSessionStore.setState({ session, permissions });
}

function railHtml(role: keyof typeof PERMS): string {
  seedSession(role);
  usePosUiStore.setState({ locale: "fr-DZ" });
  return renderToStaticMarkup(
    createElement(NavRail, { view: "pos", onSelect: () => undefined }),
  );
}

function appHtml(role: keyof typeof PERMS, view: AppView): string {
  seedSession(role);
  usePosUiStore.setState({ locale: "fr-DZ" });
  return renderToStaticMarkup(
    createElement(AppShell, { view, onSelectView: () => undefined }),
  );
}

const NAV_LABELS: Record<AppView, string> = {
  pos: t("fr-DZ", "nav.pos"),
  catalog: t("fr-DZ", "nav.catalog"),
  stock: t("fr-DZ", "nav.stock"),
  customers: t("fr-DZ", "nav.customers"),
  documents: t("fr-DZ", "nav.documents"),
  suppliers: t("fr-DZ", "nav.suppliers"),
  reports: t("fr-DZ", "nav.reports"),
  settings: t("fr-DZ", "nav.settings"),
  users: t("fr-DZ", "nav.users"),
};
const NAV_LABEL = (view: AppView): string => NAV_LABELS[view].replace(/&/g, "&amp;");

describe("navigation permission matrix (canAccessView)", () => {
  it("grants Vendeur pos/catalog/stock/customers/documents and nothing else", () => {
    const p = [...PERMS.VENDEUR];
    for (const view of ["pos", "catalog", "stock", "customers", "documents"] as const) {
      expect(canAccessView(view, p)).toBe(true);
    }
    for (const view of ["suppliers", "reports", "settings", "users"] as const) {
      expect(canAccessView(view, p)).toBe(false);
    }
  });

  it("grants Caissier pos/catalog/stock/customers/documents/reports and nothing else", () => {
    const p = [...PERMS.CAISSIER];
    for (const view of ["pos", "catalog", "stock", "customers", "documents", "reports"] as const) {
      expect(canAccessView(view, p)).toBe(true);
    }
    for (const view of ["suppliers", "settings", "users"] as const) {
      expect(canAccessView(view, p)).toBe(false);
    }
  });

  it("grants Gerant everything except users", () => {
    const p = [...PERMS.GERANT];
    for (const view of ["pos", "catalog", "stock", "customers", "documents", "suppliers", "reports", "settings"] as const) {
      expect(canAccessView(view, p)).toBe(true);
    }
    expect(canAccessView("users", p)).toBe(false);
  });

  it("grants Admin all 9 views", () => {
    const p = [...PERMS.ADMIN];
    for (const view of ["pos", "catalog", "stock", "customers", "documents", "suppliers", "reports", "settings", "users"] as const) {
      expect(canAccessView(view, p)).toBe(true);
    }
  });
});

describe("navigation rail visibility", () => {
  beforeEach(() => {
    useAppViewStore.setState({ view: "pos" });
  });

  it("Vendeur sees pos/catalog/stock/customers/documents entries only", () => {
    const html = railHtml("VENDEUR");
    for (const view of ["pos", "catalog", "stock", "customers", "documents"] as const) {
      expect(html).toContain(NAV_LABEL(view));
    }
    for (const view of ["suppliers", "reports", "settings", "users"] as const) {
      expect(html).not.toContain(NAV_LABEL(view));
    }
  });

  it("Caissier additionally sees the reports entry", () => {
    const html = railHtml("CAISSIER");
    expect(html).toContain(NAV_LABEL("reports"));
    for (const view of ["suppliers", "settings", "users"] as const) {
      expect(html).not.toContain(NAV_LABEL(view));
    }
  });

  it("Gerant sees everything except users", () => {
    const html = railHtml("GERANT");
    for (const view of ["pos", "catalog", "stock", "customers", "documents", "suppliers", "reports", "settings"] as const) {
      expect(html).toContain(NAV_LABEL(view));
    }
    expect(html).not.toContain(NAV_LABEL("users"));
  });

  it("Admin sees all 9 rail entries", () => {
    const html = railHtml("ADMIN");
    for (const view of ["pos", "catalog", "stock", "customers", "documents", "suppliers", "reports", "settings", "users"] as const) {
      expect(html).toContain(NAV_LABEL(view));
    }
  });
});

describe("view routing fallbacks", () => {
  it("renders the access-denied fallback for a denied view", () => {
    const html = appHtml("VENDEUR", "suppliers");
    expect(html).toContain(t("fr-DZ", "common.accessDenied"));
  });

  it("renders no fallback for a granted view", () => {
    const html = appHtml("VENDEUR", "catalog");
    expect(html).not.toContain(t("fr-DZ", "common.accessDenied"));
  });

  it("keeps the users unauthorized card behind USER_MANAGE", () => {
    const denied = appHtml("GERANT", "users");
    expect(denied).toContain(t("fr-DZ", "users.unauthorized"));
    const granted = appHtml("ADMIN", "users");
    expect(granted).not.toContain(t("fr-DZ", "common.accessDenied"));
  });
});

describe("App.tsx session reactivity (AUD-031)", () => {
  it("subscribes to the session value so login/logout re-render immediately", () => {
    // getSessionToken/hasPermission are stable closures: selecting them alone
    // never re-renders. The session identity subscription is the reactive pin.
    expect(APP_SRC).toContain("useSessionStore((state) => state.session)");
  });
});

describe("App.tsx permission wiring (source pins)", () => {
  it("contains zero raw role-string checks", () => {
    expect(APP_SRC).not.toMatch(/role\s*===/);
    expect(APP_SRC).not.toContain("isManagerRole");
    expect(APP_SRC).not.toContain("isAdmin");
  });

  it("gates every view through hasPermission", () => {
    for (const code of [
      "CREATE_SALE",
      "VIEW_STOCK",
      "VIEW_CUSTOMERS",
      "SUPPLIER_MANAGE",
      "REPORTS_VIEW",
      "CASH_REPORT_VIEW",
      "SYSTEM_SETTINGS_MANAGE",
      "STOCK_ADJUSTMENT",
      "USER_MANAGE",
    ]) {
      expect(APP_SRC).toContain(`hasPermission("${code}")`);
    }
  });
});
