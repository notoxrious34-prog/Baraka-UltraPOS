/**
 * Transfer-picker instant-search migration contracts (TASK-DSN-067).
 * The manual Enter/button-only trigger must be replaced by the shared
 * debounced runner (175ms) with a stale-ticket guard.
 */
import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized).
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const SRC = readFileSync(
  join(process.cwd(), "src", "components", "pos", "modals", "WarehouseTransferModal.tsx"),
  "utf8",
);

describe("transfer picker instant search (TASK-DSN-067)", () => {
  it("drives the picker through the shared debounced runner", () => {
    expect(SRC).toContain("createDebouncedSearchRunner");
    expect(SRC).toContain("delayMs: 175");
  });

  it("guards async results against stale tickets", () => {
    expect(SRC).toContain("isLatest");
  });

  it("has no manual search button; Enter flushes the pending debounce", () => {
    expect(SRC).not.toContain("onClick={runPickerSearch}");
    expect(SRC).toContain("flush()");
  });
});
