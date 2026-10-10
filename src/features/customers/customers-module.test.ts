/**
 * Customers & Credit module tests (TASK-DSN-027): symmetric i18n keys,
 * navigation wiring, IPC binding shape, and pure helper behavior.
 * Node-only harness (no jsdom): source-text assertions + pure logic.
 */

import { describe, expect, it } from "vitest";
import type { CustomerDetail } from "@/types/pos";
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
const MODULE_SRC = readSrc("src", "features", "customers", "CustomersModule.tsx");
const FORM_SRC = readSrc("src", "features", "customers", "CustomerFormModal.tsx");

describe("customers i18n keys", () => {
  it("declares nav.customers and the customers.* namespace in FR", () => {
    for (const entry of [
      '"nav.customers": "Clients & Crédit"',
      '"customers.title": "Clients & Crédit"',
      '"customers.filterDebtors": "Débiteurs"',
      '"customers.balanceDue": "Solde dû"',
      '"customers.creditLimit": "Plafond de crédit"',
      '"customers.availableCredit": "Crédit disponible"',
      '"customers.statement": "Relevé de compte"',
      '"customers.opSale": "Vente"',
      '"customers.opPayment": "Règlement"',
      '"customers.formNameError": "Nom requis (2 à 100 caractères)"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });

  it("declares nav.customers and the customers.* namespace in AR", () => {
    for (const entry of [
      '"nav.customers": "الزبائن والديون"',
      '"customers.balanceDue": "الرصيد المستحق"',
      '"customers.creditLimit": "سقف الائتمان"',
      '"customers.statement": "كشف الحساب"',
      '"customers.opPayment": "سند قبض"',
      '"customers.filterInactive": "غير نشطين"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});

describe("navigation and IPC wiring", () => {
  it("extends AppView with customers", () => {
    expect(VIEW_SRC).toContain('"customers"');
  });

  it("routes the customers view in App.tsx behind the manager gate", () => {
    expect(APP_SRC).toContain('entry("customers"');
    expect(APP_SRC).toContain('view === "customers" && manager');
    expect(APP_SRC).toContain("<CustomersModule />");
  });

  it("binds customers_update through the session-injecting bridge", () => {
    expect(IPC_SRC).toContain("export async function ipcUpdateCustomer");
    expect(IPC_SRC).toContain('call<CustomerDetail>("customers_update"');
    expect(IPC_SRC).toContain("credit_limit_centimes?: string | null");
  });
});

describe("module surface", () => {
  it("enriches directory rows and filters on details", () => {
    expect(MODULE_SRC).toContain("ipcListCustomers");
    expect(MODULE_SRC).toContain("ipcSearchCustomers");
    expect(MODULE_SRC).toContain("ipcGetCustomer");
    expect(MODULE_SRC).toContain("filterCustomers");
  });

  it("form modal validates and converts without floats", () => {
    expect(FORM_SRC).toContain("isValidCustomerName");
    expect(FORM_SRC).toContain("parseMoneyToCentimes");
    expect(FORM_SRC).not.toContain("Math.round");
    expect(FORM_SRC).not.toContain("parseFloat");
  });
});

describe("customer helpers", async () => {
  const helpers = await import("@/features/customers/customer-helpers");

  const detail = (over: Record<string, unknown>): CustomerDetail => ({
    id: "c1",
    name: "Client",
    phone: null,
    address: null,
    credit_policy: "LIMITED",
    credit_limit_centimes: "10000",
    current_balance_centimes: "0",
    is_active: true,
    ...over,
  });

  it("isDebtor is strictly positive and garbage-safe", () => {
    expect(helpers.isDebtor("1")).toBe(true);
    expect(helpers.isDebtor("0")).toBe(false);
    expect(helpers.isDebtor("-5")).toBe(false);
    expect(helpers.isDebtor("nope")).toBe(false);
  });

  it("filterCustomers isolates debtors and inactive rows", () => {
    const rows = [
      detail({ id: "a", current_balance_centimes: "250" }),
      detail({ id: "b", current_balance_centimes: "0" }),
      detail({ id: "c", current_balance_centimes: "0", is_active: false }),
    ];
    expect(helpers.filterCustomers(rows, "debtors").map((r) => r.id)).toEqual(["a"]);
    expect(helpers.filterCustomers(rows, "inactive").map((r) => r.id)).toEqual(["c"]);
    expect(helpers.filterCustomers(rows, "all")).toHaveLength(3);
  });

  it("availableCredit subtracts with floor, null for non-limited", () => {
    expect(helpers.availableCredit(detail({}))).toBe("10000");
    expect(
      helpers.availableCredit(detail({ credit_limit_centimes: "10000", current_balance_centimes: "250" })),
    ).toBe("9750");
    expect(
      helpers.availableCredit(detail({ credit_limit_centimes: "100", current_balance_centimes: "500" })),
    ).toBe("0");
    expect(helpers.availableCredit(detail({ credit_policy: "UNLIMITED" }))).toBeNull();
    expect(helpers.availableCredit(detail({ credit_policy: "DISABLED" }))).toBeNull();
  });

  it("isValidCustomerName enforces 2..100 trimmed chars", () => {
    expect(helpers.isValidCustomerName("AB")).toBe(true);
    expect(helpers.isValidCustomerName(" A ")).toBe(false);
    expect(helpers.isValidCustomerName("")).toBe(false);
    expect(helpers.isValidCustomerName("x".repeat(101))).toBe(false);
  });
});
