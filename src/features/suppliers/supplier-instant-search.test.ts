/**
 * Supplier instant-search tests (TASK-DSN-050).
 *
 * The supplier directory keeps its Enter/button immediate submission but
 * gains a ~150 ms debounced search on every keystroke, so the first typed
 * character refreshes results without any extra key. Node-only harness
 * (no DOM library): source-text contracts pinning the debounce wiring and
 * the preserved immediate paths.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const SCREEN_SRC = readFileSync(
  join(process.cwd(), "src", "features", "suppliers", "SupplierListScreen.tsx"),
  "utf8",
);

describe("SupplierListScreen instant search (source contract)", () => {
  it("debounces a store search ~150 ms after each query change", () => {
    expect(SCREEN_SRC).toContain("setTimeout(() => {");
    expect(SCREEN_SRC).toContain("}, 150);");
    expect(SCREEN_SRC).toContain("clearTimeout(timer);");
    expect(SCREEN_SRC).toContain("void search(token, query);");
  });

  it("re-arms the debounce whenever the query (or session) changes", () => {
    expect(SCREEN_SRC).toContain("[query, search, token]");
  });

  it("preserves immediate Enter submission and button submit", () => {
    expect(SCREEN_SRC).toContain('event.key === "Enter"');
    expect(SCREEN_SRC).toContain("onClick={runSearch}");
    // Clearing the input re-fires the same debounced effect with "".
    expect(SCREEN_SRC).toContain("onChange={(event) => {");
  });
});
