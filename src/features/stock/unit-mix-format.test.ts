import { describe, expect, it } from "vitest";

import { formatBpPercent } from "@/features/stock/unit-mix-format";

describe("formatBpPercent", () => {
  it("formats hand-computed shares", () => {
    expect(formatBpPercent("7058")).toBe("70.58%");
    expect(formatBpPercent("2941")).toBe("29.41%");
    expect(formatBpPercent("666")).toBe("6.66%");
  });

  it("trims trailing zeros and handles zero", () => {
    expect(formatBpPercent("7000")).toBe("70%");
    expect(formatBpPercent("0")).toBe("0%");
    expect(formatBpPercent("10000")).toBe("100%");
  });

  it("rejects non-canonical input", () => {
    expect(() => formatBpPercent("70.58")).toThrow();
    expect(() => formatBpPercent("")).toThrow();
    expect(() => formatBpPercent("abc")).toThrow();
  });
});
