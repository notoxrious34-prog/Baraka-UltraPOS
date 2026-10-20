/**
 * Documents module tests (TASK-DSN-028): symmetric i18n keys, navigation
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
const MODULE_SRC = readSrc("src", "features", "documents", "DocumentsModule.tsx");
const TABLE_SRC = readSrc("src", "features", "documents", "DocumentTable.tsx");
const DRAWER_SRC = readSrc("src", "features", "documents", "DocumentDetailDrawer.tsx");
const FILTER_SRC = readSrc("src", "features", "documents", "DocumentFilterBar.tsx");

describe("documents i18n keys", () => {
  it("declares nav.documents and the documents.* namespace in FR", () => {
    for (const entry of [
      '"nav.documents": "Documents & Factures"',
      '"documents.title": "Documents & Factures"',
      '"documents.typeBl": "Bons de livraison"',
      '"documents.typeAvoir": "Avoirs"',
      '"documents.statusFinalized": "Finalisés"',
      '"documents.statusDraft": "Brouillons"',
      '"documents.walkIn": "Client comptant"',
      '"documents.reprint": "Réimprimer"',
      '"documents.reprintOk": "Ordre d\'impression envoyé"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });

  it("declares nav.documents and the documents.* namespace in AR", () => {
    for (const entry of [
      '"nav.documents": "الوثائق والفواتير"',
      '"documents.noResults": "لا توجد وثائق مطابقة"',
      '"documents.typeTicket": "وصول بيع"',
      '"documents.typeFacture": "فواتير"',
      '"documents.reprint": "إعادة طباعة"',
      '"documents.reprintOk": "تم إرسال أمر الطباعة بنجاح"',
      '"documents.reprintFail": "فشل إعادة الطباعة"',
      '"documents.walkIn": "زبون عابر"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});

describe("navigation and IPC wiring", () => {
  it("extends AppView with documents", () => {
    expect(VIEW_SRC).toContain('"documents"');
  });

  it("routes the documents view in App.tsx behind the permission gate", () => {
    expect(APP_SRC).toContain('entry("documents"');
    expect(APP_SRC).toContain("canAccessDocuments");
    expect(APP_SRC).toContain('hasPermission("CREATE_SALE")');
    expect(APP_SRC).toContain("<DocumentsModule />");
  });

  it("reuses the verified searchDocuments and ipcReprint bindings", () => {
    expect(IPC_SRC).toContain("export async function searchDocuments");
    expect(IPC_SRC).toContain('call<DocumentSearchWireResponse>("documents_search"');
    expect(IPC_SRC).toContain("export async function ipcReprint");
    expect(IPC_SRC).toContain('call<ReprintWire>("documents_reprint"');
    expect(DRAWER_SRC).toContain("ipcReprint(props.token, document.id)");
  });
});

describe("module surface", () => {
  it("filter bar owns type pills, status selector, and refresh", () => {
    expect(FILTER_SRC).toContain("DOCUMENT_TYPES");
    expect(FILTER_SRC).toContain("DOCUMENT_STATUSES");
    expect(FILTER_SRC).toContain("onRefresh");
    expect(FILTER_SRC).toContain('placeholder={t(locale, "documents.searchPlaceholder")}');
  });

  it("module passes token and null-narrows filters server-side", () => {
    expect(MODULE_SRC).toContain("session_token: token");
    expect(MODULE_SRC).toContain("document_type: docType");
    expect(MODULE_SRC).toContain("status: status");
  });

  it("table formats amounts with formatMoney, bdi isolation, and tabular figures", () => {
    expect(TABLE_SRC).toContain("formatMoney(");
    expect(TABLE_SRC).toContain('<bdi dir="ltr"');
    expect(TABLE_SRC).toContain("tabular-nums");
    expect(TABLE_SRC).toContain("total_ttc_centimes");
    expect(TABLE_SRC).toContain("min-h-0 flex-1 overflow-auto");
    expect(TABLE_SRC).toContain('t(locale, "documents.noResults")');
  });

  it("drawer shows metadata, financial card, and reprint lifecycle", () => {
    expect(DRAWER_SRC).toContain('t(locale, "documents.reprint")');
    expect(DRAWER_SRC).toContain("reprintOk");
    expect(DRAWER_SRC).toContain("reprintFail");
    expect(DRAWER_SRC).toContain("Escape");
  });
});

describe("document helpers", async () => {
  const helpers = await import("@/features/documents/document-helpers");

  it("resolves a distinct label key per document type", () => {
    expect(helpers.documentTypeKey("TICKET")).toBe("documents.typeTicket");
    expect(helpers.documentTypeKey("BL")).toBe("documents.typeBl");
    expect(helpers.documentTypeKey("FACTURE")).toBe("documents.typeFacture");
    expect(helpers.documentTypeKey("DEVIS")).toBe("documents.typeDevis");
    expect(helpers.documentTypeKey("AVOIR")).toBe("documents.typeAvoir");
  });

  it("resolves a distinct label key per status", () => {
    expect(helpers.documentStatusKey("FINALIZED")).toBe("documents.statusFinalized");
    expect(helpers.documentStatusKey("CONVERTED")).toBe("documents.statusConverted");
    expect(helpers.documentStatusKey("DRAFT")).toBe("documents.statusDraft");
    expect(helpers.documentStatusKey("CANCELLED")).toBe("documents.statusCancelled");
  });

  it("assigns distinct badge classes per type and status", () => {
    const typeBadges = new Set(
      ["TICKET", "BL", "FACTURE", "DEVIS", "AVOIR"].map((kind) => helpers.documentTypeBadge(kind)),
    );
    expect(typeBadges.size).toBe(5);
    const statusBadges = new Set(
      ["FINALIZED", "CONVERTED", "DRAFT", "CANCELLED"].map((state) =>
        helpers.documentStatusBadge(state),
      ),
    );
    expect(statusBadges.size).toBe(4);
  });
});

describe("module root gate (TASK-DSN-078)", () => {
  // NOTE: the module root subscribes to the zustand session store, whose
  // seeded state is invisible to renderToStaticMarkup (SSR server snapshot).
  // Per repo convention, container wiring is pinned via source-text
  // assertions (see catalog-hub.test.tsx header, DSN-077).
  it("authorizes the root view via CREATE_SALE, not a manager role check", () => {
    expect(MODULE_SRC).toContain('hasPermission("CREATE_SALE")');
    expect(MODULE_SRC).toContain("useSessionStore((state) => state.hasPermission)");
    expect(MODULE_SRC).not.toContain("isManagerRole");
    expect(MODULE_SRC).not.toContain("session.role");
  });

  it("renders the access-denied fallback when unauthorized, never blank", () => {
    expect(MODULE_SRC).toContain("if (!authorized || token === null)");
    expect(MODULE_SRC).toContain('translateBackendError("UNAUTHORIZED"');
  });

  it("keeps the journal surface (filter bar, table, drawer) past the gate", () => {
    expect(MODULE_SRC).toContain("<DocumentFilterBar");
    expect(MODULE_SRC).toContain("<DocumentTable");
    expect(MODULE_SRC).toContain("<DocumentDetailDrawer");
  });
});
