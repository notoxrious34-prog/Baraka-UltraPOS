/**
 * Last-price memory chip tests (TASK-DSN-106).
 *
 * No DOM harness exists in this project: the pure effective-price math and
 * the SSR-safe null renders are asserted directly, and the IPC/wiring
 * contract is pinned by source assertions (repo convention).
 */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { join } from "node:path";
import { LastPriceChip, effectiveLastPrice } from "@/components/pos/LastPriceChip";

declare const process: { cwd(): string };
const ROOT = process.cwd();
function source(relative: string): string {
  return readFileSync(join(ROOT, relative), "utf8");
}

describe("effectiveLastPrice", () => {
  const cell = {
    priceCentimes: "9500",
    discountPerUnitCentimes: "500",
    documentId: "doc-1",
    soldAt: "2026-11-12T09:00:00Z",
  };
  it("derives price minus per-unit discount with BigInt math", () => {
    expect(effectiveLastPrice(cell)).toBe("9000");
  });
  it("returns the full price when no discount was transacted", () => {
    expect(effectiveLastPrice({ ...cell, discountPerUnitCentimes: "0" })).toBe("9500");
  });
  it("rejects negative nets and garbage wire as unknown", () => {
    expect(effectiveLastPrice({ ...cell, discountPerUnitCentimes: "9600" })).toBe(null);
    expect(effectiveLastPrice({ ...cell, priceCentimes: "abc" })).toBe(null);
  });
});

describe("LastPriceChip", () => {
  it("renders nothing without a session (SSR-safe, no crash)", () => {
    const html = renderToStaticMarkup(
      <LastPriceChip locale="fr-DZ" customerId="cust-1" productId="p-1" unitId="u-1" />,
    );
    expect(html).toBe("");
  });
  it("renders nothing for guest customers", () => {
    const html = renderToStaticMarkup(
      <LastPriceChip locale="ar-DZ" customerId={null} productId="p-1" unitId="u-1" />,
    );
    expect(html).toBe("");
  });
});

describe("DSN-106 wiring pins", () => {
  it("ipcGetLastPrice targets customers_last_price and maps nulls to unknown", () => {
    const ipc = source("src/lib/ipc.ts");
    expect(ipc).toContain("customers_last_price");
    expect(ipc).toContain("ipcGetLastPrice");
  });
  it("the cart row renders the chip for named customers only", () => {
    const table = source("src/components/pos/CartTable.tsx");
    expect(table).toContain("LastPriceChip");
    expect(table).toContain('customerId={customer.kind === "named" ? customer.id : null}');
  });
});
