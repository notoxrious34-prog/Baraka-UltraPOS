/**
 * Purchase-order search instant migration contracts (TASK-DSN-067).
 */
import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized).
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const SRC = readFileSync(
  join(process.cwd(), "src", "features", "purchases", "PurchaseOrderFormScreen.tsx"),
  "utf8",
);

describe("purchase search instant (TASK-DSN-067)", () => {
  it("drives product search through the shared debounced runner", () => {
    expect(SRC).toContain("createDebouncedSearchRunner");
    expect(SRC).toContain("delayMs: 175");
  });

  it("guards async results against stale tickets", () => {
    expect(SRC).toContain("isLatest");
  });

  it("has no manual search trigger; Enter flushes, Escape resets", () => {
    expect(SRC).not.toContain("void runProductSearch()");
    expect(SRC).toContain("flush()");
    expect(SRC).toContain('"Escape"');
  });
});
