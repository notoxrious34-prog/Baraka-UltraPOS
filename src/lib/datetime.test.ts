import { describe, expect, it } from "vitest";
import { formatInstant } from "./datetime";

describe("formatInstant", () => {
  it("formats a valid ISO instant in both locales", () => {
    const fr = formatInstant("2026-11-15T13:07:50.538Z", "fr-DZ");
    const ar = formatInstant("2026-11-15T13:07:50.538Z", "ar-DZ");
    expect(fr).not.toBe("2026-11-15T13:07:50.538Z");
    expect(ar).not.toBe("2026-11-15T13:07:50.538Z");
    expect(fr.length).toBeGreaterThan(0);
    expect(ar.length).toBeGreaterThan(0);
  });

  it("passes unparseable input through untouched", () => {
    expect(formatInstant("not-a-date", "fr-DZ")).toBe("not-a-date");
    expect(formatInstant("", "ar-DZ")).toBe("");
  });
});
