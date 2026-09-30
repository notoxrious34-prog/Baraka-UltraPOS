/**
 * Unit tests for the canonical string/BigInt converters in money.ts.
 * Zero-float law: these tests pin exact decimal-string -> wire-string
 * behavior (and rejection of malformed input) used by the catalog form.
 */

import { describe, expect, it } from "vitest";
import { parseMoneyToCentimes, parseQtyToMicro } from "@/lib/money";

describe("parseMoneyToCentimes", () => {
  it("converts plain integers to centimes", () => {
    expect(parseMoneyToCentimes("1200")).toBe("120000");
    expect(parseMoneyToCentimes("0")).toBe("0");
  });

  it("converts one- and two-digit fractions", () => {
    expect(parseMoneyToCentimes("150.50")).toBe("15050");
    expect(parseMoneyToCentimes("99.99")).toBe("9999");
    expect(parseMoneyToCentimes("150.5")).toBe("15050");
  });

  it("accepts comma decimals and surrounding whitespace", () => {
    expect(parseMoneyToCentimes("150,50")).toBe("15050");
    expect(parseMoneyToCentimes("  12.5  ")).toBe("1250");
  });

  it("rejects more than two fractional digits", () => {
    expect(() => parseMoneyToCentimes("10.555")).toThrow();
    expect(() => parseMoneyToCentimes("10.5.5")).toThrow();
  });

  it("rejects empty and non-numeric input", () => {
    expect(() => parseMoneyToCentimes("")).toThrow();
    expect(() => parseMoneyToCentimes("abc")).toThrow();
    expect(() => parseMoneyToCentimes("12a")).toThrow();
  });
});

describe("parseQtyToMicro", () => {
  it("converts integers to micros", () => {
    expect(parseQtyToMicro("1")).toBe("10000");
    expect(parseQtyToMicro("12")).toBe("120000");
  });

  it("converts up to four fractional digits", () => {
    expect(parseQtyToMicro("1.5")).toBe("15000");
    expect(parseQtyToMicro("2.0001")).toBe("20001");
    expect(parseQtyToMicro("0.25")).toBe("2500");
  });

  it("accepts comma decimals and surrounding whitespace", () => {
    expect(parseQtyToMicro("1,5")).toBe("15000");
    expect(parseQtyToMicro("  3  ")).toBe("30000");
  });

  it("rejects more than four fractional digits", () => {
    expect(() => parseQtyToMicro("1.00001")).toThrow();
  });

  it("rejects empty and non-numeric input", () => {
    expect(() => parseQtyToMicro("")).toThrow();
    expect(() => parseQtyToMicro("box")).toThrow();
  });
});
