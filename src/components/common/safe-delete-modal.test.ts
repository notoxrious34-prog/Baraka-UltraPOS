/**
 * Safe deletion UI tests (TASK-DSN-051, ADR-092): reusable modal contract,
 * bilingual copy, and per-entity IPC bindings.
 * Node-only harness (no jsdom): source-text assertions.
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
const MODAL_SRC = readSrc("src", "components", "common", "SafeDeleteModal.tsx");
const CATALOG_SRC = readSrc("src", "features", "catalog", "CatalogModule.tsx");
const EDIT_SRC = readSrc("src", "features", "catalog", "ProductEditModal.tsx");
const CUSTOMERS_SRC = readSrc("src", "features", "customers", "CustomersModule.tsx");
const DOSSIER_SRC = readSrc("src", "features", "customers", "CustomerDossierPane.tsx");
const SUPPLIERS_SRC = readSrc("src", "features", "suppliers", "SupplierListScreen.tsx");
const DEPOTS_SRC = readSrc("src", "components", "pos", "modals", "WarehouseTransferModal.tsx");
const DEPOT_STORE_SRC = readSrc("src", "stores", "warehouse-transfer-store.ts");

describe("SafeDeleteModal contract", () => {
  it("exposes dialog semantics with Escape dismissal and a Tab focus trap", () => {
    expect(MODAL_SRC).toContain('role="dialog"');
    expect(MODAL_SRC).toContain('aria-modal="true"');
    expect(MODAL_SRC).toContain('event.key === "Escape"');
    expect(MODAL_SRC).toContain('event.key !== "Tab"');
    expect(MODAL_SRC).toContain("button:not([disabled])");
  });

  it("hard-disables confirm while blocked or busy and shows the blocked card", () => {
    expect(MODAL_SRC).toContain("disabled={busy || blocked}");
    expect(MODAL_SRC).toContain('role="alert"');
    expect(MODAL_SRC).toContain('"del.blockedTitle"');
    expect(MODAL_SRC).toContain("disabled:cursor-not-allowed");
  });

  it("uses a danger-tinted confirm action with a 1.75px-stroke SVG icon", () => {
    expect(MODAL_SRC).toContain('"del.confirm"');
    expect(MODAL_SRC).toContain("bg-[#C62828]");
    expect(MODAL_SRC).toContain('strokeWidth="1.75"');
    expect(MODAL_SRC).not.toContain("🔒");
  });

  it("reports the backend mode through onSuccess and surfaces IPC errors inline", () => {
    expect(MODAL_SRC).toContain("onSuccess: (result: DeleteEntityResult) => void");
    expect(MODAL_SRC).toContain("translateBackendError(error, locale)");
  });
});

describe("deletion bilingual copy (FR/AR)", () => {
  it("declares the del.* namespace with the dual-track explanation", () => {
    for (const entry of [
      '"del.confirm": "Confirmer la suppression"',
      '"del.confirm": "تأكيد الحذف"',
      '"del.deleted": "Élément supprimé définitivement"',
      '"del.deleted": "تم حذف العنصر نهائياً"',
      '"del.archived": "Élément archivé et désactivé"',
      '"del.archived": "تمت أرشفة العنصر وتعطيله"',
      '"del.productTitle": "Supprimer le produit"',
      '"del.productTitle": "حذف المنتج"',
      '"del.customerTitle": "Supprimer le client"',
      '"del.customerTitle": "حذف العميل"',
      '"del.supplierTitle": "Supprimer le fournisseur"',
      '"del.supplierTitle": "حذف المورّد"',
      '"del.warehouseTitle": "Supprimer le dépôt"',
      '"del.warehouseTitle": "حذف المستودع"',
      '"del.blockedBalance"',
      '"del.blockedDepot"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});

describe("catalog deletion binding", () => {
  it("adds a row delete action and a mode-aware confirmation flow", () => {
    expect(CATALOG_SRC).toContain("ipcDeleteProduct");
    expect(CATALOG_SRC).toContain("SafeDeleteModal");
    expect(CATALOG_SRC).toContain('titleKey="del.productTitle"');
    expect(CATALOG_SRC).toContain('"del.deleted"');
    expect(CATALOG_SRC).toContain('"del.archived"');
  });

  it("exposes deletion from the edit modal footer", () => {
    expect(EDIT_SRC).toContain("onDelete");
    expect(EDIT_SRC).toContain('"del.delete"');
  });
});

describe("customer deletion binding", () => {
  it("blocks open balances client-side and clears selection on success", () => {
    expect(CUSTOMERS_SRC).toContain("ipcDeleteCustomer");
    expect(CUSTOMERS_SRC).toContain("hasOpenBalance");
    expect(CUSTOMERS_SRC).toContain("blockedBalanceReason");
    expect(CUSTOMERS_SRC).toContain('titleKey="del.customerTitle"');
    expect(CUSTOMERS_SRC).toContain("isBlocked={blocked}");
    expect(DOSSIER_SRC).toContain("onDelete");
  });
});

describe("supplier deletion binding", () => {
  it("adds a row delete action with list refresh on success", () => {
    expect(SUPPLIERS_SRC).toContain("ipcDeleteSupplier");
    expect(SUPPLIERS_SRC).toContain("SafeDeleteModal");
    expect(SUPPLIERS_SRC).toContain('titleKey="del.supplierTitle"');
  });
});

describe("depot management binding", () => {
  it("locks DEP-01 and wires update/delete through the transfer store", () => {
    expect(DEPOTS_SRC).toContain("PRINCIPAL_DEPOT_CODE");
    expect(DEPOTS_SRC).toContain('"DEP-01"');
    expect(DEPOTS_SRC).toContain('"wh.locked"');
    expect(DEPOTS_SRC).toContain('titleKey="del.warehouseTitle"');
    expect(DEPOTS_SRC).toContain("ManageDepotsTab");
    expect(DEPOT_STORE_SRC).toContain("submitUpdateWarehouse");
    expect(DEPOT_STORE_SRC).toContain("submitDeleteWarehouse");
    expect(DEPOT_STORE_SRC).toContain("ipcUpdateWarehouse");
    expect(DEPOT_STORE_SRC).toContain("ipcDeleteWarehouse");
  });
});

describe("DSN-082 reason + typed-gate (FAIL-before: unimplemented)", () => {
  it("collects a mandatory reason with a live counter and a 3-char minimum", () => {
    expect(MODAL_SRC).toContain('"del.reasonLabel"');
    expect(MODAL_SRC).toContain('"del.reasonHint"');
    expect(MODAL_SRC).toContain("MIN_REASON_LENGTH");
    expect(MODAL_SRC).toContain("reasonTooShort");
  });

  it("gates confirm on an exact case-sensitive typed match of the entity code", () => {
    expect(MODAL_SRC).toContain('"del.typeToConfirm"');
    expect(MODAL_SRC).toContain("typedInput === expectedToken");
    expect(MODAL_SRC).not.toContain("toLowerCase");
    expect(MODAL_SRC).not.toContain("toUpperCase");
  });

  it("passes the collected reason into onConfirm and every caller forwards it", () => {
    expect(MODAL_SRC).toContain("onConfirm: (reason: string) => Promise<DeleteEntityResult>");
    expect(CATALOG_SRC).toContain("onConfirm={(reason: string) =>");
    expect(CUSTOMERS_SRC).toContain("onConfirm={(reason: string) =>");
    expect(SUPPLIERS_SRC).toContain("onConfirm={(reason: string) =>");
    expect(DEPOTS_SRC).toContain("onConfirm={(reason: string) =>");
    expect(DEPOT_STORE_SRC).toContain("submitDeleteWarehouse: async (id, reason");
  });

  it("threads reason through all four IPC delete wrappers", () => {
    const IPC_SRC = readSrc("src", "lib", "ipc.ts");
    for (const fn of [
      "ipcDeleteProduct",
      "ipcDeleteCustomer",
      "ipcDeleteSupplier",
      "ipcDeleteWarehouse",
    ]) {
      expect(IPC_SRC).toContain(fn);
    }
    expect(IPC_SRC).toContain("reason: string");
    expect(IPC_SRC).toContain("reason },");
  });

  it("maps engine delete codes to guidance in both locales", () => {
    expect(I18N_SRC).toContain("CUSTOMER_BALANCE_NONZERO");
    expect(I18N_SRC).toContain("SUPPLIER_BALANCE_NONZERO");
    expect(I18N_SRC).toContain("WAREHOUSE_HAS_STOCK");
    expect(I18N_SRC).toContain("DEPOT_PRINCIPAL_IMMUTABLE");
    expect(I18N_SRC).toContain('"del.reasonTooShort"');
  });

  it("announces modal errors assertively and traps Tab across textarea inputs", () => {
    const alerts = MODAL_SRC.match(/role="alert"/g) ?? [];
    expect(alerts.length).toBeGreaterThanOrEqual(2);
    expect(MODAL_SRC).toContain("textarea:not([disabled])");
  });

  it("never renders a delete affordance for the locked principal depot", () => {
    expect(DEPOTS_SRC).toContain("{!locked");
    expect(DEPOTS_SRC).toContain("isBlocked={deleting.code === PRINCIPAL_DEPOT_CODE}");
  });
});
