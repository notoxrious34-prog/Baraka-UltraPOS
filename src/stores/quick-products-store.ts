/**
 * Quick-products palette store (TASK-DSN-069).
 *
 * Eight configurable slots (Alt+1..Alt+8) holding full catalog products for
 * single-keystroke cart entry of barcode-less bulk items. Persistence is a
 * guarded `localStorage` key (precedent: theme store, starter-pack flag);
 * there is no generic settings IPC. Financial math stays in the cart store:
 * dispatch only calls `addProduct` (default-unit resolution included).
 */

import { create } from "zustand";
import { useCartStore } from "@/stores/cart-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import type { CatalogProduct } from "@/types/pos";

export const QUICK_PRODUCTS_STORAGE_KEY = "baraka-quick-products";

export const QUICK_SLOT_COUNT = 8;

interface QuickProductsState {
  slots: Array<CatalogProduct | null>;
  hydrated: boolean;
  lastFiredSlot: number | null;
  hydrate: (defaults: CatalogProduct[]) => void;
  assignSlot: (slotIndex: number, product: CatalogProduct) => void;
  clearSlot: (index: number) => void;
  /** Add the slot's product to the cart; false when the slot is empty. */
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

export const useQuickProductsStore = create<QuickProductsState>()((set, get) => ({
  slots: emptySlots(),
  hydrated: false,
  lastFiredSlot: null,
  hydrate: (defaults) => {
    if (get().hydrated) {
      return;
    }
    const stored = readStoredSlots();
    if (stored !== null) {
      set({ slots: stored, hydrated: true });
      return;
    }
    const seeded = emptySlots();
    for (let i = 0; i < QUICK_SLOT_COUNT && i < defaults.length; i++) {
      const candidate = defaults[i];
      if (candidate !== undefined && isValidSlotProduct(candidate)) {
        seeded[i] = candidate;
      }
    }
    persistSlots(seeded);
    set({ slots: seeded, hydrated: true });
  },
  assignSlot: (slotIndex, product) => {
    if (!validIndex(slotIndex) || !isValidSlotProduct(product)) {
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
  dispatchSlot: (slotIndex) => {
    if (!validIndex(slotIndex)) {
      return false;
    }
    const product = get().slots[slotIndex];
    if (product === null || product === undefined) {
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
