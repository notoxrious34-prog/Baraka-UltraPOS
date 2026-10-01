/**
 * First-run default language guard (TASK-DSN-001, ADR-069).
 *
 * ADR-069: French is the first-run default; the store carries the default
 * and there is no persistence layer (no zustand/middleware persist, no
 * localStorage in prod code), so every launch starts from this value.
 * These tests pin the default and the explicit-choice transitions at the
 * store boundary. The App.tsx document mapping (locale -> dir/lang) is a
 * plain inline ternary over this same value and is not rendered here:
 * no DOM harness (@testing-library/jsdom) is installed and adding one
 * would be a new dependency.
 */

import { beforeEach, describe, expect, it } from "vitest";
import { usePosUiStore } from "@/stores/pos-ui-store";

// Captured at module import, before any test (or beforeEach) can mutate the
// singleton: this is the true creator default from create<PosUiState>().
const initialLocale = usePosUiStore.getState().locale;

beforeEach(() => {
  // Reset to the creator default before each test; the first test asserts
  // what that creator default is on a pristine module registry.
  usePosUiStore.setState({ locale: "fr-DZ" });
});

describe("pos ui store language default (ADR-069)", () => {
  it("defaults to French (fr-DZ) — fails on the old ar-DZ default", () => {
    // NOTE: beforeEach already restored fr-DZ, so this alone cannot catch a
    // regressed creator default. The creator default is pinned by reading the
    // initial state captured at import time below.
    expect(initialLocale).toBe("fr-DZ");
    expect(usePosUiStore.getState().locale).toBe("fr-DZ");
  });

  it("keeps an explicit Arabic choice in-session", () => {
    usePosUiStore.getState().setLocale("ar-DZ");
    expect(usePosUiStore.getState().locale).toBe("ar-DZ");
  });

  it("toggleLocale flips between fr-DZ and ar-DZ", () => {
    const store = usePosUiStore.getState();
    store.setLocale("fr-DZ");
    usePosUiStore.getState().toggleLocale();
    expect(usePosUiStore.getState().locale).toBe("ar-DZ");
    usePosUiStore.getState().toggleLocale();
    expect(usePosUiStore.getState().locale).toBe("fr-DZ");
  });
});
