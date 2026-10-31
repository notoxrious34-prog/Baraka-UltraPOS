/**
 * BITMAP packer pins (BRK-101): bit-exactness + dimension guards.
 * Pure module — no canvas/DOM needed, runs in node vitest.
 */

import { describe, expect, it } from "vitest";
import { packBitmapRgba } from "./bitmap-pack";

function solid(width: number, height: number, r: number, g: number, b: number): Uint8ClampedArray {
  const rgba = new Uint8ClampedArray(width * height * 4);
  for (let p = 0; p < width * height; p += 1) {
    rgba[p * 4] = r;
    rgba[p * 4 + 1] = g;
    rgba[p * 4 + 2] = b;
    rgba[p * 4 + 3] = 255;
  }
  return rgba;
}

describe("pack-bit-exactness (MSB-first, dark = 1)", () => {
  it("packs an 8x2 checkerboard to exact bytes", () => {
    // Row 0: black,white alternating from black -> 0b10101010 = 0xAA.
    // Row 1: white,black alternating from white -> 0b01010101 = 0x55.
    const rgba = new Uint8ClampedArray(8 * 2 * 4);
    for (let col = 0; col < 8; col += 1) {
      const topBlack = col % 2 === 0;
      rgba[col * 4] = topBlack ? 0 : 255;
      rgba[col * 4 + 1] = topBlack ? 0 : 255;
      rgba[col * 4 + 2] = topBlack ? 0 : 255;
      rgba[col * 4 + 3] = 255;
      const bottomBlack = col % 2 === 1;
      const j = (8 + col) * 4;
      rgba[j] = bottomBlack ? 0 : 255;
      rgba[j + 1] = bottomBlack ? 0 : 255;
      rgba[j + 2] = bottomBlack ? 0 : 255;
      rgba[j + 3] = 255;
    }
    const frame = packBitmapRgba(rgba, 8, 2);
    expect(frame.widthBytes).toBe(1);
    expect(frame.heightRows).toBe(2);
    expect(Array.from(frame.packed)).toEqual([0xaa, 0x55]);
  });

  it("thresholds at mean 128 (127 dark, 128 white) with integer math", () => {
    // Mixed frames throughout: the blank guard rejects all-dark/all-white.
    const dark = solid(8, 1, 127, 127, 127);
    dark[0] = 255;
    dark[1] = 255;
    dark[2] = 255;
    expect(Array.from(packBitmapRgba(dark, 8, 1).packed)).toEqual([0x7f]);
    const white = solid(8, 1, 128, 128, 128);
    // 0x00 would trip the blank guard — mix one dark pixel in.
    white[0] = 0;
    white[1] = 0;
    white[2] = 0;
    expect(Array.from(packBitmapRgba(white, 8, 1).packed)).toEqual([0x80]);
  });

  it("ignores the alpha channel", () => {
    const rgba = solid(8, 1, 0, 0, 0);
    for (let p = 0; p < 8; p += 1) {
      rgba[p * 4 + 3] = 0;
    }
    rgba[0] = 255;
    rgba[1] = 255;
    rgba[2] = 255;
    expect(Array.from(packBitmapRgba(rgba, 8, 1).packed)).toEqual([0x7f]);
  });
});

describe("pack guards (refuse, never truncate)", () => {
  it("rejects non-positive dimensions", () => {
    expect(() => packBitmapRgba(new Uint8ClampedArray(0), 0, 1)).toThrow(
      "BITMAP_PACK_DIMENSIONS_MUST_BE_POSITIVE",
    );
  });

  it("rejects widths that are not byte-aligned", () => {
    expect(() => packBitmapRgba(new Uint8ClampedArray(12 * 4), 12, 1)).toThrow(
      "BITMAP_PACK_WIDTH_NOT_BYTE_ALIGNED",
    );
  });

  it("rejects length mismatches", () => {
    expect(() => packBitmapRgba(new Uint8ClampedArray(8), 8, 1)).toThrow(
      "BITMAP_PACK_LENGTH_MISMATCH",
    );
  });

  it("rejects all-white and all-black frames (blank guard)", () => {
    expect(() => packBitmapRgba(solid(8, 1, 255, 255, 255), 8, 1)).toThrow(
      "BITMAP_PACK_BLANK_IMAGE",
    );
    expect(() => packBitmapRgba(solid(8, 1, 0, 0, 0), 8, 1)).toThrow("BITMAP_PACK_BLANK_IMAGE");
  });
});
