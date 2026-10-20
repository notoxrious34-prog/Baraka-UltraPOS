/**
 * Customers action-level permission gating (TASK-DSN-073, Batch 2a).
 *
 * "Disabled with Guidance": unauthorized actions stay visible but disabled
 * with a localized `title` naming the missing habilitation — never silently
 * hidden. Rail visibility + route protection already migrated (DSN-072).
 *
 * Harness: node-only `renderToStaticMarkup` (repo convention — no DOM lib).
 * Only stable store accessors (`hasPermission`, `getSessionToken`, which close
 * over `get()`) stay live under SSR; value selectors freeze on initial state.
 * Permission sets mirror `crates/domain/src/role.rs:121-203`.
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { describe, expect, it, beforeEach } from "vitest";
declare const process: { cwd(): string };
import { renderToStaticMarkup } from "react-dom/server";
import { createElement } from "react";
import { CustomerListPane } from "@/features/customers/CustomerListPane";
import { CustomerDossierPane } from "@/features/customers/CustomerDossierPane";
import { CustomerFormModal } from "@/features/customers/CustomerFormModal";
import { RecordPaymentModal } from "@/components/pos/modals/RecordPaymentModal";
import { useSessionStore } from "@/stores/session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import { t } from "@/lib/i18n";
import type { CustomerDetail, SessionInfo } from "@/types/pos";

const ROOT = process.cwd();
const LIST_SRC: string = readFileSync(
  join(ROOT, "src", "features", "customers", "CustomerListPane.tsx"),
  "utf8",
);
const DOSSIER_SRC: string = readFileSync(
  join(ROOT, "src", "features", "customers", "CustomerDossierPane.tsx"),
  "utf8",
);
const FORM_SRC: string = readFileSync(
  join(ROOT, "src", "features", "customers", "CustomerFormModal.tsx"),
  "utf8",
);
const PAY_SRC: string = readFileSync(
  join(ROOT, "src", "components", "pos", "modals", "RecordPaymentModal.tsx"),
  "utf8",
);

/** Exact domain permission sets (role.rs:121-203), SCREAMING_SNAKE_CASE wire form. */
const VENDEUR = ["POS_SELL", "POS_PARK_RESUME", "VIEW_STOCK", "CREATE_SALE", "VIEW_CUSTOMERS"];
/**
 * Task-spec Vendeur session (DSN-073 §3.1): synthetic — carries CREATE_CUSTOMER
 * although the real Vendeur matrix (role.rs:121) does NOT hold CreateCustomer
 * (Caissier+ only, customers.rs:87,164). Enablement follows the permission,
 * so the real Vendeur correctly sees the button disabled (asserted below).
 */
const VENDEUR_SPEC = ["CREATE_SALE", "VIEW_STOCK", "CREATE_CUSTOMER", "VIEW_CUSTOMERS"];
const CAISSIER = [
  ...VENDEUR,
  "CASHIER_COLLECT_PAYMENT",
  "CASH_SESSION_OPEN",
  "CASH_SESSION_CLOSE",
  "CASH_EXPENSE_RECORD",
  "CASH_REPORT_VIEW",
  "CREATE_CUSTOMER",
  "RECORD_CREDIT_PAYMENT",
  "REPRINT_DOCUMENT",
];
const GERANT = [
  ...CAISSIER,
  "PRICE_OVERRIDE",
  "DISCOUNT_OVERRIDE",
  "NEGATIVE_STOCK_OVERRIDE",
  "CREDIT_LIMIT_OVERRIDE",
  "PROCESS_RETURN",
  "STOCK_ADJUSTMENT",
  "PURCHASE_MANAGE",
  "REPORTS_VIEW",
  "ADJUST_STOCK",
  "TRANSFER_STOCK",
  "MANAGE_CUSTOMER_CREDIT",
  "SUPPLIER_MANAGE",
  "PURCHASE_ORDER_CREATE",
  "PURCHASE_RECEIVE",
  "PURCHASE_INVOICE_RECORD",
  "SUPPLIER_PAYMENT_RECORD",
];

function seed(role: string, permissions: readonly string[]): void {
  const list = [...permissions];
  const session: SessionInfo = {
    token: "stub-token",
    userId: `u-${role.toLowerCase()}`,
    username: role.toLowerCase(),
    role,
    permissions: list,
    expiresAt: "2030-01-01T00:00:00",
  };
  useSessionStore.setState({ session, permissions: list });
  usePosUiStore.setState({ locale: "fr-DZ" });
}

const GUIDANCE = {
  create: t("fr-DZ", "customers.createCustomerReservedGuidance"),
  payment: t("fr-DZ", "customers.paymentReservedGuidance"),
  credit: t("fr-DZ", "customers.creditLimitReservedGuidance"),
};

const LIMITED_DETAIL: CustomerDetail = {
  id: "c-1",
  name: "Quincaillerie Test",
  phone: "0550000000",
  address: null,
  credit_policy: "LIMITED",
  credit_limit_centimes: "50000",
  customer_tier: "retail",
  current_balance_centimes: "12000",
  is_active: true,
};

function listHtml(): string {
  return renderToStaticMarkup(
    createElement(CustomerListPane, {
      locale: "fr-DZ",
      query: "",
      onQuery: () => undefined,
      filter: "all",
      onFilter: () => undefined,
      rows: [],
      selectedId: null,
      onSelect: () => undefined,
      onNew: () => undefined,
      loading: false,
    }),
  );
}

describe("CustomerListPane creation gate", () => {
  beforeEach(() => {
    seed("VENDEUR", VENDEUR);
  });

  it("spec Vendeur session (holds CREATE_CUSTOMER) sees an enabled Nouveau Client", () => {
    seed("VENDEUR", VENDEUR_SPEC);
    const html = listHtml();
    expect(html).toContain(t("fr-DZ", "customers.newCustomer"));
    expect(html).not.toContain(GUIDANCE.create);
  });

  it("real Vendeur matrix (no CreateCustomer) is disabled with guidance", () => {
    seed("VENDEUR", VENDEUR);
    const html = listHtml();
    expect(html).toContain("disabled");
    expect(html).toContain(`title="${GUIDANCE.create}"`);
  });

  it("a session without CREATE_CUSTOMER gets a disabled button with guidance", () => {
    seed("NONE", []);
    const html = listHtml();
    expect(html).toContain("disabled");
    expect(html).toContain(`title="${GUIDANCE.create}"`);
  });
});

describe("CustomerDossierPane empty-state creation gate", () => {
  it("spec Vendeur session sees an enabled CTA; unauthorized sees disabled with guidance", () => {
    seed("VENDEUR", VENDEUR_SPEC);
    const open = renderToStaticMarkup(
      createElement(CustomerDossierPane, {
        locale: "fr-DZ",
        token: "stub-token",
        detail: null,
        onEdit: () => undefined,
        onDelete: () => undefined,
        onNew: () => undefined,
      }),
    );
    expect(open).not.toContain(GUIDANCE.create);

    seed("NONE", []);
    const closed = renderToStaticMarkup(
      createElement(CustomerDossierPane, {
        locale: "fr-DZ",
        token: "stub-token",
        detail: null,
        onEdit: () => undefined,
        onDelete: () => undefined,
        onNew: () => undefined,
      }),
    );
    expect(closed).toContain("disabled");
    expect(closed).toContain(`title="${GUIDANCE.create}"`);
  });
});

describe("CustomerFormModal credit-limit gate", () => {
  function formHtml(): string {
    return renderToStaticMarkup(
      createElement(CustomerFormModal, {
        locale: "fr-DZ",
        token: "stub-token",
        mode: { kind: "edit", detail: LIMITED_DETAIL },
        onClose: () => undefined,
        onSaved: () => undefined,
      }),
    );
  }

  it("Vendeur (no MANAGE_CUSTOMER_CREDIT) gets a disabled limit with helper text", () => {
    seed("VENDEUR", VENDEUR);
    const html = formHtml();
    expect(html).toContain(`title="${GUIDANCE.credit}"`);
    expect(html).toContain(GUIDANCE.credit);
    expect(html).toContain("disabled");
  });

  it("Caissier (no MANAGE_CUSTOMER_CREDIT) is likewise read-only on the limit", () => {
    seed("CAISSIER", CAISSIER);
    const html = formHtml();
    expect(html).toContain(`title="${GUIDANCE.credit}"`);
    expect(html).toContain("disabled");
  });

  it("Gerant (holds MANAGE_CUSTOMER_CREDIT) edits the limit with no guidance", () => {
    seed("GERANT", GERANT);
    const html = formHtml();
    expect(html).not.toContain(GUIDANCE.credit);
  });
});

describe("RecordPaymentModal permission gate", () => {
  function payHtml(): string {
    return renderToStaticMarkup(
      createElement(RecordPaymentModal, {
        customerId: "c-1",
        customerName: "Quincaillerie Test",
        currentBalanceCentimes: "12000",
        isOpen: true,
        onClose: () => undefined,
        onSuccess: () => undefined,
      }),
    );
  }

  it("Vendeur (no RECORD_CREDIT_PAYMENT) sees disabled inputs with guidance", () => {
    seed("VENDEUR", VENDEUR);
    const html = payHtml();
    expect(html).toContain("disabled");
    expect(html).toContain(t("fr-DZ", "payment.vendeurRestricted"));
    expect(html).toContain(`title="${GUIDANCE.payment}"`);
  });

  it("Caissier (holds RECORD_CREDIT_PAYMENT) records with no guidance", () => {
    seed("CAISSIER", CAISSIER);
    const html = payHtml();
    expect(html).not.toContain(t("fr-DZ", "payment.vendeurRestricted"));
    expect(html).not.toContain(GUIDANCE.payment);
  });

  it("overpayment checkbox needs MANAGE_CUSTOMER_CREDIT even for Caissier", () => {
    seed("CAISSIER", CAISSIER);
    const html = payHtml();
    expect(html).toContain(t("fr-DZ", "payment.overpaymentRestricted"));
    seed("GERANT", GERANT);
    const managed = payHtml();
    expect(managed).not.toContain(t("fr-DZ", "payment.overpaymentRestricted"));
  });
});

describe("customers action wiring (source pins)", () => {
  it("contains zero raw role-string checks in the four components", () => {
    for (const src of [LIST_SRC, DOSSIER_SRC, FORM_SRC, PAY_SRC]) {
      expect(src).not.toMatch(/role\s*===/);
    }
    expect(FORM_SRC).not.toContain("isManager");
    expect(PAY_SRC).not.toContain("isVendeur");
    expect(PAY_SRC).not.toContain("canUseOverpayment");
  });

  it("gates actions through hasPermission with the domain codes", () => {
    expect(LIST_SRC).toContain('hasPermission("CREATE_CUSTOMER")');
    expect(DOSSIER_SRC).toContain('hasPermission("CREATE_CUSTOMER")');
    expect(FORM_SRC).toContain('hasPermission("MANAGE_CUSTOMER_CREDIT")');
    expect(PAY_SRC).toContain('hasPermission("RECORD_CREDIT_PAYMENT")');
    expect(PAY_SRC).toContain('hasPermission("MANAGE_CUSTOMER_CREDIT")');
  });

  it("keeps money and phone numbers bidi-isolated", () => {
    expect(LIST_SRC).toContain("<bdi");
    expect(DOSSIER_SRC).toContain('<bdi dir="ltr">');
    expect(FORM_SRC).toContain('dir="ltr"');
  });
});

describe("module root gate (TASK-DSN-077)", () => {
  const MODULE_SRC: string = readFileSync(
    join(ROOT, "src", "features", "customers", "CustomersModule.tsx"),
    "utf8",
  );

  it("authorizes the root view via VIEW_CUSTOMERS, not a manager role check", () => {
    expect(MODULE_SRC).toContain('hasPermission("VIEW_CUSTOMERS")');
    expect(MODULE_SRC).not.toContain("isManagerRole");
  });

  // NOTE: module roots subscribe to the zustand session store, whose
  // seeded state is invisible to renderToStaticMarkup (SSR server
  // snapshot). Per repo convention (catalog-hub.test.tsx header),
  // container wiring is pinned via source-text assertions; store-driven
  // rendering is covered by the prop-level view tests below and in DSN-073.
  it("keeps the UNAUTHORIZED fallback for sessions without VIEW_CUSTOMERS", () => {
    expect(MODULE_SRC).toContain('translateBackendError("UNAUTHORIZED"');
    expect(MODULE_SRC).toContain("if (!authorized || token === null)");
  });

  it("still renders the directory chrome (title + panes) past the gate", () => {
    expect(MODULE_SRC).toContain('"customers.title"');
    expect(MODULE_SRC).toContain("<CustomerListPane");
    expect(MODULE_SRC).toContain("<CustomerDossierPane");
  });
});
