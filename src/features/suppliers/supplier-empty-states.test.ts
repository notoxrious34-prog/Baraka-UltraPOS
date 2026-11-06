/**
 * Suppliers empty-state discipline (TASK-DSN-054).
 *
 * Regression law: the no-match and error-state tests FAIL on the pre-fix
 * code (single undifferentiated sup.noResults line, raw error line without
 * retry) and pass after the fix. The final two tests pin the documented
 * contingency (no list-suppliers capability) and the untouched shared
 * behavior; they pass before and after.
 * Node-only harness (no DOM library): source-text contracts + i18n copy pins.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
// @ts-expect-error — see above.
import { readdirSync, statSync } from "node:fs";

declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const SCREEN_SRC = readSrc("src", "features", "suppliers", "SupplierListScreen.tsx");
const I18N_SRC = readSrc("src", "lib", "i18n.ts");
const SUPPLIERS_RS = readSrc("crates", "application", "src", "use_cases", "suppliers.rs");
const COMMANDS_RS = readSrc("crates", "commands", "src", "ipc", "suppliers.rs");
const STORE_SRC = readSrc("src", "stores", "supplier-store.ts");

function listSourceFiles(dir: string, out: string[]): void {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (entry === "node_modules" || entry === "target" || entry === ".git") {
      continue;
    }
    const st = statSync(full);
    if (st.isDirectory()) {
      listSourceFiles(full, out);
    } else if (/\.(ts|tsx)$/.test(entry) && !/\.test\.(ts|tsx)$/.test(entry)) {
      out.push(full);
    }
  }
}

describe("suppliers no-match state (TASK-DSN-054)", () => {
  it("distinguishes a non-empty query with zero matches from the empty-query case", () => {
    expect(SCREEN_SRC).toContain("sup.noMatchLead");
    // The empty-query branch must not reuse the no-match copy.
    expect(SCREEN_SRC).toContain('query.trim() !== ""');
  });

  it("no-match copy interpolates the literal query inside bidi-safe guillemets", () => {
    expect(SCREEN_SRC).toContain("«");
    expect(SCREEN_SRC).toContain("<bdi");
  });

  it("no-match state offers a clear-search secondary action that resets the query", () => {
    expect(SCREEN_SRC).toContain("sup.clearSearch");
    expect(SCREEN_SRC).toContain('setQuery("")');
  });

  it("i18n catalog carries the exact author-approved FR/AR no-match copy", () => {
    for (const entry of [
      '"sup.noMatchLead": "Aucun fournisseur ne correspond à"',
      '"sup.clearSearch": "Effacer la recherche"',
      '"sup.noMatchLead": "لا يوجد مورّد يطابق"',
      '"sup.clearSearch": "مسح البحث"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});

describe("suppliers fetch-error state (TASK-DSN-054)", () => {
  it("error panel renders the approved copy with a retry action", () => {
    expect(SCREEN_SRC).toContain("sup.loadError");
    expect(SCREEN_SRC).toContain("sup.retry");
  });

  it("i18n catalog carries the exact author-approved FR/AR error copy", () => {
    for (const entry of [
      '"sup.loadError": "Impossible de charger la liste des fournisseurs."',
      '"sup.retry": "Réessayer"',
      '"sup.loadError": "تعذّر تحميل قائمة الموردين."',
      '"sup.retry": "إعادة المحاولة"',
    ]) {
      expect(I18N_SRC).toContain(entry);
    }
  });
});

describe("suppliers default-fetch contingency (TASK-DSN-054; gap closed by TASK-BRK-057)", () => {
  it("documents the backend empty-query guard that blocks a default list", () => {
    expect(SUPPLIERS_RS).toContain("if trimmed.is_empty()");
    expect(SUPPLIERS_RS).toContain("return Ok(Vec::new());");
  });

  it("documents that the list-suppliers IPC capability now exists (TASK-BRK-057)", () => {
    expect(COMMANDS_RS).toContain("pub fn suppliers_search");
    expect(COMMANDS_RS).toContain("pub fn suppliers_list");
  });
});

describe("suppliers default list + teaching state (TASK-DSN-054-C2)", () => {
  it("(a) empty query + zero suppliers renders the teaching state, not the no-match panel", () => {
    expect(SCREEN_SRC).toContain("sup.emptyTitle");
    expect(SCREEN_SRC).toContain("sup.emptyBody");
    expect(SCREEN_SRC).toContain("sup.emptyCta");
    expect(SCREEN_SRC).toContain('query.trim() === ""');
    // The undifferentiated pre-fix paragraph is gone.
    expect(SCREEN_SRC).not.toContain("sup.noResults");
  });

  it("(b) the store fetches the default list via ipcListSuppliers on empty query", () => {
    expect(STORE_SRC).toContain("ipcListSuppliers");
    expect(STORE_SRC).toContain('query.trim() === ""');
  });

  it("(c) non-empty query + zero results still renders the pre-existing no-match panel", () => {
    expect(SCREEN_SRC).toContain("sup.noMatchLead");
    expect(SCREEN_SRC).toContain('query.trim() !== ""');
    expect(SCREEN_SRC).toContain("sup.clearSearch");
  });
});

describe("shared search behavior untouched (TASK-DSN-054 criterion 4)", () => {
  it("no fetchOnEmptyQuery-style option was added to any shared primitive", () => {
    const files: string[] = [];
    listSourceFiles(join(ROOT, "src"), files);
    const offenders = files.filter((file) => readFileSync(file, "utf8").includes("fetchOnEmptyQuery"));
    expect(offenders).toEqual([]);
  });
});
