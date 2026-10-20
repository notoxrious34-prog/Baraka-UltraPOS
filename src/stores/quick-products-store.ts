/**
 * Quick-products palette store (TASK-DSN-069).
 *
 * Eight configurable slots (Alt+1..Alt+8) holding full catalog products for
 * single-keystroke cart entry of barcode-less bulk items. Persistence is a
 * guarded `localStorage` key (precedent: theme store, starter-pack flag);
 * there is no generic settings IPC. Financial math stays in the cart store:
 * dispatch only calls `addProduct` (default-unit resolution included).
 *
 * DSN-076: palette enablement (`baraka-quick-products-enabled`, default
 * true), config-dialog control state, and an `is_active` invariant — only
 * active products may be assigned or dispatched; inactive attempts fail
 * safely without state or cart mutation.
 */

import { create } from "zustand";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import type { CatalogProduct } from "@/types/pos";

export const QUICK_PRODUCTS_STORAGE_KEY = "baraka-quick-products";
export const QUICK_PRODUCTS_ENABLED_KEY = "baraka-quick-products-enabled";

export const QUICK_SLOT_COUNT = 8;

interface QuickProductsState {
  slots: Array<CatalogProduct | null>;
  hydrated: boolean;
  /** Palette visibility + Alt+1..8 dispatch switch (persisted, default true). */
  enabled: boolean;
  /** Config-dialog visibility (session-only, never persisted). */
  isConfigOpen: boolean;
  lastFiredSlot: number | null;
  hydrate: (defaults: CatalogProduct[]) => void;
  setConfigOpen: (open: boolean) => void;
  toggleEnabled: () => void;
  assignSlot: (slotIndex: number, product: CatalogProduct) => void;
  clearSlot: (index: number) => void;
  /** Re-seed all slots from the provided active products (persisted). */
  resetDefaults: (defaults: CatalogProduct[]) => void;
  /** Add the slot's product to the cart; false when empty or inactive. */
  dispatchSlot: (slotIndex: number) => boolean;
  clearFired: () => void;
}

function emptySlots(): Array<CatalogProduct | null> {
  return Array.from({ length: QUICK_SLOT_COUNT }, () => null);
}

function isValidSlotProduct(value: unknown): value is CatalogProduct {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.product_id === "string" &&
    Array.isArray(candidate.units) &&
    candidate.units.length > 0
  );
}

function readStoredSlots(): Array<CatalogProduct | null> | null {
  try {
    if (typeof localStorage === "undefined") {
      return null;
    }
    const raw = localStorage.getItem(QUICK_PRODUCTS_STORAGE_KEY);
    if (raw === null) {
      return null;
    }
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed) || parsed.length !== QUICK_SLOT_COUNT) {
      return null;
    }
    return parsed.map((entry) => (isValidSlotProduct(entry) ? entry : null));
  } catch {
    return null;
  }
}

function readStoredEnabled(): boolean {
  try {
    if (typeof localStorage === "undefined") {
      return true;
    }
    return localStorage.getItem(QUICK_PRODUCTS_ENABLED_KEY) !== "false";
  } catch {
    return true;
  }
}

function persistEnabled(enabled: boolean): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(QUICK_PRODUCTS_ENABLED_KEY, enabled ? "true" : "false");
    }
  } catch {
    // Private-mode / quota failure: in-memory flag still applies this session.
  }
}

function persistSlots(slots: Array<CatalogProduct | null>): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(QUICK_PRODUCTS_STORAGE_KEY, JSON.stringify(slots));
    }
  } catch {
    // Private-mode / quota failure: in-memory slots still apply this session.
  }
}

function validIndex(slotIndex: number): boolean {
  return Number.isInteger(slotIndex) && slotIndex >= 0 && slotIndex < QUICK_SLOT_COUNT;
}

/** Seed helper: only structurally valid AND active products occupy slots. */
function seedFrom(defaults: CatalogProduct[]): Array<CatalogProduct | null> {
  const seeded = emptySlots();
  for (let i = 0; i < QUICK_SLOT_COUNT && i < defaults.length; i++) {
    const candidate = defaults[i];
    if (candidate !== undefined && isValidSlotProduct(candidate) && candidate.is_active === true) {
      seeded[i] = candidate;
    }
  }
  return seeded;
}

export const useQuickProductsStore = create<QuickProductsState>()((set, get) => ({
  slots: emptySlots(),
  hydrated: false,
  enabled: true,
  isConfigOpen: false,
  lastFiredSlot: null,
  hydrate: (defaults) => {
    if (get().hydrated) {
      return;
    }
    const enabled = readStoredEnabled();
    const stored = readStoredSlots();
    if (stored !== null) {
      set({ slots: stored, enabled, hydrated: true });
      return;
    }
    const seeded = seedFrom(defaults);
    persistSlots(seeded);
    set({ slots: seeded, enabled, hydrated: true });
  },
  setConfigOpen: (open) => {
    set({ isConfigOpen: open });
  },
  toggleEnabled: () => {
    const next = !get().enabled;
    persistEnabled(next);
    set({ enabled: next });
  },
  assignSlot: (slotIndex, product) => {
    if (!validIndex(slotIndex) || !isValidSlotProduct(product)) {
      return;
    }
    // Invariant: inactive products never occupy a slot.
    if (product.is_active !== true) {
      return;
    }
    const slots = get().slots.slice();
    slots[slotIndex] = product;
    persistSlots(slots);
    set({ slots });
  },
  clearSlot: (index) => {
    if (!validIndex(index)) {
      return;
    }
    const slots = get().slots.slice();
    slots[index] = null;
    persistSlots(slots);
    set({ slots });
  },
  resetDefaults: (defaults) => {
    const seeded = seedFrom(defaults);
    persistSlots(seeded);
    set({ slots: seeded, hydrated: true });
  },
  dispatchSlot: (slotIndex) => {
    if (!validIndex(slotIndex)) {
      return false;
    }
    const product = get().slots[slotIndex];
    if (product === null || product === undefined) {
      return false;
    }
    // Invariant: inactive products never reach the cart (stale stored slot).
    if (product.is_active !== true) {
      return false;
    }
    useCartStore.getState().addProduct(product);
    const locale = usePosUiStore.getState().locale;
    const label = locale === "ar-DZ" ? product.name_ar : product.name_fr;
    usePosUiStore.getState().setBanner(`Alt+${slotIndex + 1} · ${label}`);
    set({ lastFiredSlot: slotIndex });
    return true;
  },
  clearFired: () => {
    set({ lastFiredSlot: null });
  },
}));
