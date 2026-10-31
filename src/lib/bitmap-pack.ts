/**
 * 1-bit TSPL BITMAP packer (BRK-101, pure — no canvas here).
 *
 * RGBA → 1-bit rows, MSB-first, left-to-right (TSC BITMAP data order):
 * dark pixel = 1, white pixel = 0. Darkness test is integer-only
 * (`r+g+b < 384`, i.e. mean < 128) — no floats anywhere.
 *
 * Refuses, never truncates: every rejection is a `BITMAP_PACK_*` code.
 */

export interface PackedBitmap {
  /** Frame width in BYTES (dots = widthBytes x 8). */
  widthBytes: number;
  /** Frame height in DOTS (rows). */
  heightRows: number;
  /** Row-major packed rows, length == widthBytes x heightRows. */
  packed: Uint8Array;
}

/** Pack an RGBA buffer (length == width x height x 4) into 1-bit rows. */
export function packBitmapRgba(
  rgba: Uint8ClampedArray,
  width: number,
  height: number,
): PackedBitmap {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) {
    throw new Error("BITMAP_PACK_DIMENSIONS_MUST_BE_POSITIVE");
  }
  if (width % 8 !== 0) {
    throw new Error("BITMAP_PACK_WIDTH_NOT_BYTE_ALIGNED");
  }
  if (rgba.length !== width * height * 4) {
    throw new Error("BITMAP_PACK_LENGTH_MISMATCH");
  }
  const widthBytes = width / 8;
  const packed = new Uint8Array(widthBytes * height);
  for (let row = 0; row < height; row += 1) {
    for (let col = 0; col < width; col += 1) {
      const i = (row * width + col) * 4;
      const r: number = rgba[i] ?? 0;
      const g: number = rgba[i + 1] ?? 0;
      const b: number = rgba[i + 2] ?? 0;
      if (r + g + b < 384) {
        const byteIndex = row * widthBytes + (col >> 3);
        const current: number = packed[byteIndex] ?? 0;
        packed[byteIndex] = current | (0x80 >> (col & 7));
      }
    }
  }
  if (packed.every((byte) => byte === 0x00) || packed.every((byte) => byte === 0xff)) {
    throw new Error("BITMAP_PACK_BLANK_IMAGE");
  }
  return { widthBytes, heightRows: height, packed };
}
