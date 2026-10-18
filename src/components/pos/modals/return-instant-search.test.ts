/**
 * Return-lookup instant migration contracts (TASK-DSN-067).
 * 200ms debounce, minimum-length gate, ticket guard; Enter keeps its direct
 * immediate path (barcode-safe, bypasses the gate); Escape closes the modal
 * per the global contract.
 */
import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized).
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const MODAL_SRC = readFileSync(
  join(process.cwd(), "src", "components", "pos", "modals", "ReturnModal.tsx"),
  "utf8",
);
const STORE_SRC = readFileSync(
  join(process.cwd(), "src", "stores", "return-modal-store.ts"),
  "utf8",
);

describe("return lookup instant (TASK-DSN-067)", () => {
  it("debounces the lookup at 200ms with a minimum-length gate", () => {
    expect(MODAL_SRC).toContain("createDebouncedSearchRunner");
    expect(MODAL_SRC).toContain("delayMs: 200");
    expect(MODAL_SRC).toContain("length >= 2");
  });

  it("Enter keeps a direct immediate lookup path (barcode-safe)", () => {
    expect(MODAL_SRC).toContain("void s.searchAndLoadDocument(s.searchQuery)");
  });

  it("the store guards overlapping lookups with a monotonic ticket", () => {
    expect(STORE_SRC).toContain("searchTicket");
  });

  it("has no manual search button", () => {
    expect(MODAL_SRC).not.toContain("returns.search\")}");
  });
});
