/**
 * Family-2 contrast token migration contracts (TASK-DSN-068, pre-verified in AUD-024).
 * Precedent: supplier-empty-states.test.ts from 6b06f93 (node-only source-text
 * contracts). Targets 1-4 take `text-ink-muted`; the DocumentTable thead takes
 * `text-ink-secondary` (ink-muted would still fail light AA on slate-100).
 * Adjacent out-of-scope `text-slate-500` instances (ProductDiscovery hints,
 * DocumentTable body/empty states) are intentionally left untouched.
 */
import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized).
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const ROOT = process.cwd();
const readSrc = (...parts: string[]): string => readFileSync(join(ROOT, ...parts), "utf8");

const DISCOVERY_SRC = readSrc("src", "components", "pos", "ProductDiscovery.tsx");
const TOTALS_SRC = readSrc("src", "components", "pos", "TotalsFooter.tsx");
const PARKED_SRC = readSrc("src", "components", "pos", "ParkedSalesModal.tsx");
const DISCOUNT_SRC = readSrc("src", "components", "pos", "modals", "DiscountModal.tsx");
const DOCTABLE_SRC = readSrc("src", "features", "documents", "DocumentTable.tsx");

describe("Family-2 contrast tokens (TASK-DSN-068)", () => {
  it("POS empty catalogue state uses text-ink-muted", () => {
    expect(DISCOVERY_SRC).toContain("p-8 text-center text-sm text-ink-muted");
    expect(DISCOVERY_SRC).not.toContain("p-8 text-center text-sm text-slate-500");
  });

  it("TotalsFooter sub-labels use text-ink-muted", () => {
    expect(TOTALS_SRC).toContain("text-ink-muted");
    expect(TOTALS_SRC).not.toContain("text-slate-500");
  });

  it("ParkedSalesModal metadata uses text-ink-muted", () => {
    expect(PARKED_SRC).toContain("text-ink-muted");
    expect(PARKED_SRC).not.toContain("text-slate-500");
  });

  it("DiscountModal labels use text-ink-muted", () => {
    expect(DISCOUNT_SRC).toContain("text-ink-muted");
    expect(DISCOUNT_SRC).not.toContain("text-slate-500");
  });

  it("DocumentTable thead uses text-ink-secondary", () => {
    expect(DOCTABLE_SRC).toContain("text-start text-ink-secondary");
    expect(DOCTABLE_SRC).not.toContain("text-start text-slate-500");
  });
});
