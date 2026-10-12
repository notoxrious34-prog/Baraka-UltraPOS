/**
 * Wedge interceptor tests (TASK-DSN-040).
 *
 * The DOM/event wiring is exercised through the pure `isPrintableKey` gate
 * (the decision that the wedge listens for) plus source-text contracts that
 * pin the focus-redirect behavior. No DOM library is installed (node-only
 * harness), matching the repo convention.
 */

import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { isPrintableKey } from "@/hooks/use-pos-keyboard";

declare const process: { cwd(): string };

const HOOK_SRC = readFileSync(join(process.cwd(), "src", "hooks", "use-pos-keyboard.ts"), "utf8");

function key(overrides?: Partial<{ key: string; ctrlKey: boolean; altKey: boolean; metaKey: boolean }>): {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
} {
  return { key: "a", ctrlKey: false, altKey: false, metaKey: false, ...overrides };
}

describe("isPrintableKey (wedge gate)", () => {
  it("flags a bare single printable character", () => {
    expect(isPrintableKey(key())).toBe(true);
    expect(isPrintableKey(key({ key: "4" }))).toBe(true);
    expect(isPrintableKey(key({ key: " " }))).toBe(true);
  });

  it("never intercepts modifier combos (Ctrl / Alt / Meta)", () => {
    expect(isPrintableKey(key({ ctrlKey: true }))).toBe(false);
    expect(isPrintableKey(key({ altKey: true }))).toBe(false);
    expect(isPrintableKey(key({ metaKey: true }))).toBe(false);
    expect(isPrintableKey(key({ key: "k", ctrlKey: true }))).toBe(false);
  });

  it("never intercepts sovereign / navigation keys (F-keys, Esc, Enter, arrows)", () => {
    for (const k of ["F1", "F2", "F12", "Escape", "Enter", "Delete", "Backspace", "ArrowUp"]) {
      expect(isPrintableKey(key({ key: k }))).toBe(false);
    }
  });
});

describe("wedge source contracts", () => {
  it("redirects a printable key into the search input and forwards it", () => {
    expect(HOOK_SRC).toContain('isPrintableKey(event)');
    expect(HOOK_SRC).toContain('getElementById(SEARCH_INPUT_ID)');
    expect(HOOK_SRC).toContain('targetInput.dispatchEvent(new Event("input", { bubbles: true }))');
    expect(HOOK_SRC).toContain('targetInput.setSelectionRange(length, length)');
  });

  it("is gated by the modal-open and in-input guards (no background capture)", () => {
    // The redirect must sit below both guards.
    const redirectIndex = HOOK_SRC.indexOf('isPrintableKey(event)');
    const inInputIndex = HOOK_SRC.indexOf("if (inInput)");
    const modalIndex = HOOK_SRC.indexOf("if (modalOpen)");
    expect(redirectIndex).toBeGreaterThan(inInputIndex);
    expect(redirectIndex).toBeGreaterThan(modalIndex);
  });
});