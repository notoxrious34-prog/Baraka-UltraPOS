/**
 * Zero-dependency Code 128 (Subset B) vector barcode engine (TASK-DSN-039).
 *
 * Pure TypeScript: symbol encoding with Mod-103 checksum plus a
 * self-contained SVG renderer calibrated for 203 DPI direct-thermal
 * printing (8 dots/mm). No canvas, no DOM, no dependencies.
 */

/** Start-Code-B symbol value. */
export const CODE128_START_B = 104;
/** Stop symbol value. */
export const CODE128_STOP = 106;
/** Quiet-zone width on each side, in modules (spec minimum is 10). */
export const CODE128_QUIET_MODULES = 10;

/**
 * Element-width table: symbol value 0..105 maps to 6 bar/space widths
 * summing to 11 modules; the stop symbol (106) maps to 7 widths summing
 * to 13 modules (final double-width stop bar included).
 */
const PATTERNS: string[] = [
  "212222", // 0
  "222122", // 1
  "222221", // 2
  "121223", // 3
  "121322", // 4
  "131222", // 5
  "122213", // 6
  "122312", // 7
  "132212", // 8
  "221213", // 9
  "221312", // 10
  "231212", // 11
  "112232", // 12
  "122132", // 13
  "122231", // 14
  "113222", // 15
  "123122", // 16
  "123221", // 17
  "223211", // 18
  "221132", // 19
  "221231", // 20
  "213212", // 21
  "223112", // 22
  "312131", // 23
  "311222", // 24
  "321122", // 25
  "321221", // 26
  "312212", // 27
  "322112", // 28
  "322211", // 29
  "212123", // 30
  "212321", // 31
  "232121", // 32
  "111323", // 33
  "131123", // 34
  "131321", // 35
  "112313", // 36
  "132113", // 37
  "132311", // 38
  "211313", // 39
  "231113", // 40
  "231311", // 41
  "112133", // 42
  "112331", // 43
  "132131", // 44
  "113123", // 45
  "113321", // 46
  "133121", // 47
  "313121", // 48
  "211331", // 49
  "231131", // 50
  "213113", // 51
  "213311", // 52
  "213131", // 53
  "311123", // 54
  "311321", // 55
  "331121", // 56
  "312113", // 57
  "312311", // 58
  "332111", // 59
  "314111", // 60
  "221411", // 61
  "431111", // 62
  "111224", // 63
  "111422", // 64
  "121124", // 65
  "121421", // 66
  "141122", // 67
  "141221", // 68
  "112214", // 69
  "112412", // 70
  "122114", // 71
  "122411", // 72
  "142112", // 73
  "142211", // 74
  "241211", // 75
  "221114", // 76
  "413111", // 77
  "241112", // 78
  "134111", // 79
  "111242", // 80
  "121142", // 81
  "121241", // 82
  "114212", // 83
  "124112", // 84
  "124211", // 85
  "411212", // 86
  "421112", // 87
  "421211", // 88
  "212141", // 89
  "214121", // 90
  "412121", // 91
  "111143", // 92
  "111341", // 93
  "131141", // 94
  "114113", // 95
  "114311", // 96
  "411113", // 97
  "411311", // 98
  "113141", // 99
  "114131", // 100
  "311141", // 101
  "411131", // 102
  "211412", // 103 Start A
  "211214", // 104 Start B
  "211232", // 105 Start C
  "2331112", // 106 Stop
];

/** Total modules of the symbol stream for an n-character payload. */
export function code128ModuleCount(charCount: number): number {
  return 11 * (charCount + 2) + 13;
}

function assertEncodable(text: string): number[] {
  if (text.length === 0) {
    throw new Error("barcode128.empty");
  }
  const values: number[] = [];
  for (const char of text) {
    const code = char.codePointAt(0) ?? 0;
    if (code < 32 || code > 127) {
      throw new Error("barcode128.unsupportedChar");
    }
    values.push(code - 32);
  }
  return values;
}

/**
 * Encode text as a Code 128 Subset-B symbol stream:
 * `[START_B, v1..vn, checksum, STOP]` with
 * `checksum = (104 + Σ i·vi) mod 103`.
 */
export function encodeCode128B(text: string): number[] {
  const values = assertEncodable(text);
  let checksum = CODE128_START_B;
  for (let i = 0; i < values.length; i++) {
    const value = values[i];
    if (value === undefined) {
      throw new Error("barcode128.empty");
    }
    checksum += (i + 1) * value;
  }
  return [CODE128_START_B, ...values, checksum % 103, CODE128_STOP];
}

/** Pattern string for a symbol value (exposed for tests). */
export function code128Pattern(symbol: number): string {
  const pattern = PATTERNS[symbol];
  if (pattern === undefined) {
    throw new Error("barcode128.unknownSymbol");
  }
  return pattern;
}

export interface Code128SvgOptions {
  /** Pixels per module. Default 2 (≈0.25mm at 203 DPI). */
  moduleWidth?: number;
  /** Bar height in pixels. Default 60. */
  height?: number;
  /** Render human-readable text under the bars. Default true. */
  includeText?: boolean;
}

function escapeXml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/**
 * Render text as a self-contained Code 128 (Subset B) SVG string:
 * quiet zone + bars + optional human-readable caption.
 */
export function generateCode128Svg(text: string, options?: Code128SvgOptions): string {
  const symbols = encodeCode128B(text);
  const moduleWidth = options?.moduleWidth ?? 2;
  const height = options?.height ?? 60;
  const includeText = options?.includeText ?? true;
  if (!Number.isFinite(moduleWidth) || moduleWidth <= 0) {
    throw new Error("barcode128.invalidModuleWidth");
  }
  if (!Number.isFinite(height) || height <= 0) {
    throw new Error("barcode128.invalidHeight");
  }
  const captionHeight = includeText ? 16 : 0;
  const totalHeight = height + captionHeight + 4;
  const totalModules = code128ModuleCount(text.length) + 2 * CODE128_QUIET_MODULES;
  const totalWidth = totalModules * moduleWidth;

  let cursor = CODE128_QUIET_MODULES * moduleWidth;
  const bars: string[] = [];
  for (const symbol of symbols) {
    const pattern = code128Pattern(symbol);
    for (let element = 0; element < pattern.length; element++) {
      const widthModules = Number(pattern[element]);
      const widthPx = widthModules * moduleWidth;
      // Even element index = bar, odd = space.
      if (element % 2 === 0) {
        bars.push(`M${cursor} 2h${widthPx}v${height}H${cursor}z`);
      }
      cursor += widthPx;
    }
  }
  const caption = includeText
    ? `<text x="${totalWidth / 2}" y="${height + 14}" text-anchor="middle" font-family="monospace" font-size="12" fill="#000">${escapeXml(text)}</text>`
    : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="${totalHeight}" viewBox="0 0 ${totalWidth} ${totalHeight}" role="img" aria-label="${escapeXml(text)}"><rect width="${totalWidth}" height="${totalHeight}" fill="#fff"/><path d="${bars.join(" ")}" fill="#000"/>${caption}</svg>`;
}
