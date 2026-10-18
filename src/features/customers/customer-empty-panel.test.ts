/**
 * Customers empty-selection teaching panel (TASK-DSN-054).
 *
 * Regression law: this file FAILS on the pre-fix code (minimal selectHint
 * paragraph, no CTA, no shared create handler) and passes after the fix.
 * Node-only harness (no DOM library): source-text contracts + i18n copy pins.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const PANE_SRC = readSrc("src", "features", "customers", "CustomerDossierPane.tsx");
const MODULE_SRC = readSrc("src", "features", "customers", "CustomersModule.tsx");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");

describe("customers empty-selection teaching panel (TASK-DSN-054)", () => {
  it("dossier pane renders a teaching panel (not a bare hint) when detail is null", () => {
    expect(PANE_SRC).toContain("customers.emptyTitle");
    expect(PANE_SRC).toContain("customers.emptyBody");
  });

  it("teaching panel CTA reuses the create-customer flow via an onNew prop", () => {
    expect(PANE_SRC).toContain("customers.emptyCta");
    expect(PANE_SRC).toContain("onNew");
    expect(PANE_SRC).toContain("props.onNew");
  });

  it("teaching panel carries a fitting icon (inline SVG, 1.75px stroke convention)", () => {
    expect(PANE_SRC).toContain("<svg");
    expect(PANE_SRC).toContain('strokeWidth="1.75"');
    expect(PANE_SRC).not.toContain("emoji");
  });

  it("module wires ONE shared create handler to both the list pane and the dossier pane", () => {
    expect(MODULE_SRC).toContain("openCreate");
    expect(MODULE_SRC).toContain("CustomerDossierPane");
    // The dossier pane receives the same handler as the list pane's onNew.
    const dossierBlock = MODULE_SRC.slice(MODULE_SRC.indexOf("<CustomerDossierPane"));
    expect(dossierBlock).toContain("onNew={openCreate}");
  });

  it("an unresolvable selection falls back to the teaching panel, never a blank pane", () => {
    expect(MODULE_SRC).toContain("?? null");
    expect(PANE_SRC).toContain("detail === null");
  });

  it("i18n catalog carries the exact author-approved FR copy", () => {
    for (const entry of [
      '"customers.emptyTitle": "Sélectionnez un client"',
      '"customers.emptyBody": "Choisissez un client dans la liste pour afficher son compte, son historique de crédit et son solde."',
      '"customers.emptyCta": "Nouveau client"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });

  it("i18n catalog carries the exact author-approved AR copy", () => {
    for (const entry of [
      '"customers.emptyTitle": "اختر زبونًا"',
      '"customers.emptyBody": "اختر زبونًا من القائمة لعرض كشف حسابه وسجل الكريدي ورصيده."',
      '"customers.emptyCta": "زبون جديد"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});
