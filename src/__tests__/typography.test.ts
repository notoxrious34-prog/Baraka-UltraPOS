/**
 * Typography regression test (TASK-DSN-024): local OFL font bundling.
 *
 * Verifies the 10 bundled font files exist with valid sizes, the two new
 * OFL license files are present, and src/index.css wires @font-face rules
 * with font-display: swap for Inter, Cairo, and JetBrains Mono.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { existsSync, readFileSync, statSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

// Minimal ambient typing for the single Node global used below.
// Type-only; zero runtime impact.
declare const process: { cwd(): string };

const ROOT = process.cwd();
const FONTS_DIR = join(ROOT, "public", "fonts");
const CSS_PATH = join(ROOT, "src", "index.css");

const EXPECTED_FONTS = [
  "Cairo-Regular.ttf",
  "Cairo-SemiBold.ttf",
  "Cairo-Bold.ttf",
  "Inter-Regular.woff2",
  "Inter-Medium.woff2",
  "Inter-SemiBold.woff2",
  "Inter-Bold.woff2",
  "JetBrainsMono-Regular.woff2",
  "JetBrainsMono-Medium.woff2",
  "JetBrainsMono-Bold.woff2",
];

const EXPECTED_LICENSES = [
  join(ROOT, "assets", "fonts", "LICENSE-Cairo.txt"),
  join(ROOT, "assets", "fonts", "LICENSE-JetBrainsMono.txt"),
];

describe("bundled font files", () => {
  it("ships exactly the 10 specified font files, each exceeding 10,000 bytes", () => {
    for (const name of EXPECTED_FONTS) {
      const path = join(FONTS_DIR, name);
      expect(existsSync(path), `missing: ${name}`).toBe(true);
      const size = statSync(path).size;
      expect(size, `too small: ${name} (${size}B)`).toBeGreaterThan(10000);
    }
  });

  it("ships non-empty OFL license files for Cairo and JetBrains Mono", () => {
    for (const path of EXPECTED_LICENSES) {
      expect(existsSync(path), `missing: ${path}`).toBe(true);
      const content = readFileSync(path, "utf8");
      expect(content.length, `empty: ${path}`).toBeGreaterThan(0);
      expect(content).toContain("SIL OPEN FONT LICENSE");
    }
  });
});

describe("index.css font wiring", () => {
  const css = readFileSync(CSS_PATH, "utf8");

  it("declares @font-face for Inter, Cairo, and JetBrains Mono", () => {
    expect(css).toContain("@font-face");
    expect(css).toContain("font-family: 'Inter'");
    expect(css).toContain("font-family: 'Cairo'");
    expect(css).toContain("font-family: 'JetBrains Mono'");
  });

  it("uses font-display: swap on every @font-face rule", () => {
    const blocks = css.match(/@font-face\s*{[^}]*}/g) ?? [];
    expect(blocks.length).toBeGreaterThanOrEqual(10);
    for (const block of blocks) {
      expect(block).toContain("font-display: swap");
    }
  });

  it("references the bundled /fonts/ URLs", () => {
    for (const name of EXPECTED_FONTS) {
      expect(css).toContain(`/fonts/${name}`);
    }
  });
});
