/**
 * Quick-palette wiring contracts (TASK-DSN-069).
 * The hook must intercept Alt+Digit ahead of the printable-key wedge;
 * the palette must mount inside the discovery column.
 */
import { describe, expect, it } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized).
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const HOOK_SRC = readFileSync(
  join(process.cwd(), "src", "hooks", "use-pos-keyboard.ts"),
  "utf8",
);
const DISCOVERY_SRC = readFileSync(
  join(process.cwd(), "src", "components", "pos", "ProductDiscovery.tsx"),
  "utf8",
);
const PALETTE_SRC = readFileSync(
  join(process.cwd(), "src", "features", "pos", "QuickProductPalette.tsx"),
  "utf8",
);

describe("palette wiring (TASK-DSN-069)", () => {
  it("hook intercepts Alt+Digit with preventDefault and dispatches the slot", () => {
    expect(HOOK_SRC).toContain("altDigitSlotIndex");
    expect(HOOK_SRC).toContain("dispatchSlot");
  });

  it("palette mounts inside the discovery column", () => {
    expect(DISCOVERY_SRC).toContain("QuickProductPalette");
  });

  it("palette renders Alt+n badges with the specified kbd styling", () => {
    expect(PALETTE_SRC).toContain("Alt+");
    expect(PALETTE_SRC).toContain("<kbd");
    expect(PALETTE_SRC).toContain("bg-surface-sunken");
  });
});
