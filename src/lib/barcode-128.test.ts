/**
 * Unit tests for the Code 128 Subset-B engine (TASK-DSN-039).
 *
 * Checksum vectors below are hand-computed from the spec formula
 * checksum = (104 + Σ i·vi) mod 103 with vi = charCode − 32:
 * - "A":      (104 + 1·33) mod 103 = 137 mod 103 = 34
 * - "AB":     (104 + 33 + 2·34) mod 103 = 205 mod 103 = 102
 * - "P001":   (104 + 48 + 2·16 + 3·16 + 4·17) mod 103 = 300 mod 103 = 94
 * - "BARAKA": (104 + 34+66+150+132+215+198) mod 103 = 899 mod 103 = 75
 */

import { describe, expect, it } from "vitest";
import {
  CODE128_QUIET_MODULES,
  code128ModuleCount,
  code128Pattern,
  encodeCode128B,
  fitModuleWidth,
  generateCode128Svg,
  labelBarcodeBudgetPx,
} from "@/lib/barcode-128";

describe("encodeCode128B", () => {
  it("frames the stream with Start-B and Stop around data values", () => {
    expect(encodeCode128B("A")).toEqual([104, 33, 34, 106]);
    expect(encodeCode128B("AB")).toEqual([104, 33, 34, 102, 106]);
  });

  it("computes the Mod-103 checksum on known vectors", () => {
    expect(encodeCode128B("P001")).toEqual([104, 48, 16, 16, 17, 94, 106]);
    expect(encodeCode128B("12345678")).toEqual([
      104, 17, 18, 19, 20, 21, 22, 23, 24, 60, 106,
    ]);
    expect(encodeCode128B("BARAKA")).toEqual([104, 34, 33, 50, 33, 43, 33, 75, 106]);
  });

  it("rejects empty strings and characters outside ASCII 32..127", () => {
    expect(() => encodeCode128B("")).toThrowError();
    expect(() => encodeCode128B("caf\u00e9")).toThrowError("barcode128.unsupportedChar");
    expect(() => encodeCode128B("line\nbreak")).toThrowError("barcode128.unsupportedChar");
    expect(() => generateCode128Svg("")).toThrowError();
  });
});

describe("code128Pattern", () => {
  it("sums to 11 modules for data/start symbols and 13 for stop", () => {
    const sum = (pattern: string): number =>
      [...pattern].reduce((total, digit) => total + Number(digit), 0);
    for (let symbol = 0; symbol <= 105; symbol++) {
      expect(sum(code128Pattern(symbol))).toBe(11);
    }
    expect(sum(code128Pattern(106))).toBe(13);
    expect(() => code128Pattern(107)).toThrowError("barcode128.unknownSymbol");
  });
});

describe("generateCode128Svg", () => {
  it("emits a self-contained svg with bar path and caption", () => {
    const svg = generateCode128Svg("P001");
    expect(svg).toContain("<svg");
    expect(svg).toContain("</svg>");
    expect(svg).toContain('<path d="');
    expect(svg).toContain("P001");
    // Width = (11·(4+2)+13 + 2·10 quiet) · 2px modules.
    const expectedWidth = (code128ModuleCount(4) + 2 * CODE128_QUIET_MODULES) * 2;
    expect(svg).toContain(`width="${expectedWidth}"`);
  });

  it("omits the caption when includeText is false", () => {
    const svg = generateCode128Svg("AB", { includeText: false });
    expect(svg).toContain("<svg");
    expect(svg).not.toContain("<text");
  });

  it("rejects non-positive geometry", () => {
    expect(() => generateCode128Svg("AB", { moduleWidth: 0 })).toThrowError();
    expect(() => generateCode128Svg("AB", { height: -5 })).toThrowError();
  });
});

describe("labelBarcodeBudgetPx", () => {
  it("publishes the printable CSS-px budget per preset", () => {
    expect(labelBarcodeBudgetPx("50x30")).toBe(174);
    expect(labelBarcodeBudgetPx("40x25")).toBe(136);
    expect(labelBarcodeBudgetPx("100x50")).toBe(362);
    expect(labelBarcodeBudgetPx("39x20")).toBe(132);
  });
});

describe("fitModuleWidth", () => {
  it("fits a 13-char code into the 39x20 budget without clipping", () => {
    const mw = fitModuleWidth(13, labelBarcodeBudgetPx("39x20"), 1);
    expect(mw).toBeLessThanOrEqual(1);
    expect(mw).toBeGreaterThan(0);
    expect((code128ModuleCount(13) + 2 * CODE128_QUIET_MODULES) * mw).toBeLessThanOrEqual(132);
  });

  it("keeps the preferred width for short payloads", () => {
    expect(fitModuleWidth(5, labelBarcodeBudgetPx("39x20"), 1)).toBe(1);
  });

  it("never grows beyond the preferred width", () => {
    expect(fitModuleWidth(2, 10000, 1)).toBe(1);
  });

  it("falls back to preferred on degenerate input", () => {
    expect(fitModuleWidth(3, 0, 2)).toBe(2);
  });
});
