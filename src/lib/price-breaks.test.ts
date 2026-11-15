import { describe, expect, it } from "vitest";
import { applyBreakPrice, resolveBreakPrice } from "@/lib/price-breaks";

const LADDER = [
  { minQtyMicro: "100000", priceCentimes: "500" },
  { minQtyMicro: "10000", priceCentimes: "550" },
  { minQtyMicro: "50000", priceCentimes: "520" },
];

describe("price-break ladder parity (BRK-121)", () => {
  it("returns null below the first rung", () => {
    expect(resolveBreakPrice(LADDER, "9999")).toBeNull();
  });

  it("applies the rung boundary exactly", () => {
    expect(resolveBreakPrice(LADDER, "10000")).toBe("550");
  });

  it("picks the greatest reached rung regardless of order", () => {
    expect(resolveBreakPrice(LADDER, "60000")).toBe("520");
    expect(resolveBreakPrice(LADDER, "250000")).toBe("500");
  });

  it("ignores garbage without throwing", () => {
    expect(resolveBreakPrice(LADDER, "abc")).toBeNull();
    expect(resolveBreakPrice(LADDER, "0")).toBeNull();
    expect(resolveBreakPrice([{ minQtyMicro: "xx", priceCentimes: "1" }], "99999")).toBeNull();
  });

  it("never raises the tier price", () => {
    expect(applyBreakPrice("400", LADDER, "250000")).toBe("400");
    expect(applyBreakPrice("600", LADDER, "250000")).toBe("500");
    expect(applyBreakPrice("600", [], "250000")).toBe("600");
    expect(applyBreakPrice("600", undefined, "250000")).toBe("600");
    expect(applyBreakPrice("oops", LADDER, "250000")).toBe("oops");
  });
});
