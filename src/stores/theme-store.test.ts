/**
 * Global theme store regression (TASK-DSN-065).
 *
 * The old Topbar pattern (local `useState(false)` + unmount) stripped the
 * document-root `dark` class whenever POS remounted, and nothing persisted
 * across restarts. These tests pin the centralized store contract:
 * localStorage persistence, document-root sync, and survival across
 * (re-)initialization cycles that simulate view transitions and reloads —
 * plus source contracts proving Topbar no longer owns theme state.
 * Node environment: localStorage/document are faked per test.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";

declare const process: { cwd(): string };

const TOPBAR_SRC = readFileSync(
  join(process.cwd(), "src", "components", "pos", "Topbar.tsx"),
  "utf8",
);

interface FakeDom {
  classes: Set<string>;
  values: Map<string, string>;
}

function installFakes(stored: string | null): FakeDom {
  const fakes: FakeDom = { classes: new Set<string>(), values: new Map<string, string>() };
  if (stored !== null) {
    fakes.values.set("baraka-theme", stored);
  }
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.localStorage = {
    getItem: (key: string): string | null => fakes.values.get(key) ?? null,
    setItem: (key: string, value: string): void => {
      fakes.values.set(key, String(value));
    },
    removeItem: (key: string): void => {
      fakes.values.delete(key);
    },
  };
  globals.document = {
    documentElement: {
      classList: {
        toggle: (token: string, force?: boolean): boolean => {
          const want = force ?? !fakes.classes.has(token);
          if (want) {
            fakes.classes.add(token);
          } else {
            fakes.classes.delete(token);
          }
          return want;
        },
        contains: (token: string): boolean => fakes.classes.has(token),
      },
    },
  };
  return fakes;
}

async function freshStore(): Promise<typeof import("@/stores/theme-store")> {
  vi.resetModules();
  return import("@/stores/theme-store");
}

describe("theme store persistence (TASK-DSN-065)", () => {
  beforeEach(() => {
    vi.resetModules();
    const globals = globalThis as unknown as Record<string, unknown>;
    delete globals.localStorage;
    delete globals.document;
  });

  it("initializes dark from localStorage", async () => {
    installFakes("dark");
    const { useThemeStore } = await freshStore();
    useThemeStore.getState().initTheme();
    expect(useThemeStore.getState().theme).toBe("dark");
    expect(useThemeStore.getState().isDark).toBe(true);
  });

  it("defaults to light without a stored preference", async () => {
    installFakes(null);
    const { useThemeStore } = await freshStore();
    useThemeStore.getState().initTheme();
    expect(useThemeStore.getState().theme).toBe("light");
    expect(useThemeStore.getState().isDark).toBe(false);
  });

  it("toggle flips state, persists, and syncs the document root", async () => {
    const fakes = installFakes(null);
    const { useThemeStore } = await freshStore();
    useThemeStore.getState().initTheme();
    useThemeStore.getState().toggleTheme();
    expect(useThemeStore.getState().isDark).toBe(true);
    expect(fakes.values.get("baraka-theme")).toBe("dark");
    expect(fakes.classes.has("dark")).toBe(true);
    useThemeStore.getState().toggleTheme();
    expect(useThemeStore.getState().isDark).toBe(false);
    expect(fakes.values.get("baraka-theme")).toBe("light");
    expect(fakes.classes.has("dark")).toBe(false);
  });

  it("survives re-initialization (view transition / reload) without reset", async () => {
    const fakes = installFakes(null);
    const { useThemeStore } = await freshStore();
    useThemeStore.getState().initTheme();
    useThemeStore.getState().setTheme("dark");
    // Simulate POS unmount -> Stock mount -> POS remount: App re-runs init.
    useThemeStore.getState().initTheme();
    expect(useThemeStore.getState().isDark).toBe(true);
    expect(fakes.classes.has("dark")).toBe(true);
    expect(fakes.values.get("baraka-theme")).toBe("dark");
  });
});

describe("Topbar theme ownership (TASK-DSN-065 source contracts)", () => {
  it("no longer owns view-local dark state or touches the document root", () => {
    expect(TOPBAR_SRC).not.toContain("useDarkMode");
    expect(TOPBAR_SRC).not.toContain("document.documentElement");
  });

  it("consumes the global store toggle", () => {
    expect(TOPBAR_SRC).toContain("useThemeStore");
    expect(TOPBAR_SRC).toContain("toggleTheme");
  });
});
