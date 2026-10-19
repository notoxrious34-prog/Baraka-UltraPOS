/**
 * Quick-products palette store regression (TASK-DSN-069).
 * Slots persist to `baraka-quick-products`; dispatch reuses the cart's
 * `addProduct` (default-unit resolution, no financial logic here).
 * Node environment: localStorage is faked per test.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CatalogProduct } from "@/types/pos";
import { altDigitSlotIndex } from "@/hooks/use-pos-keyboard";

function product(id: string, name: string): CatalogProduct {
  return {
    product_id: id,
    sku: `SKU-${id}`,
    barcode: null,
    name_ar: name,
    name_fr: name,
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [
      {
        unit_id: `u-${id}`,
        product_id: id,
        unit_name_ar: "p",
        unit_name_fr: "p",
        factor_micro: "10000",
        barcode: null,
        is_base_unit: true,
        sale_price_centimes: "100",
        purchase_price_centimes: "50",
        wholesale_price_centimes: null,
        is_default_selling: true,
      },
    ],
    kit_items: [],
  };
}

function installStorage(initial: string | null): Map<string, string> {
  const values = new Map<string, string>();
  if (initial !== null) {
    values.set("baraka-quick-products", initial);
  }
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.localStorage = {
    getItem: (key: string): string | null => values.get(key) ?? null,
    setItem: (key: string, value: string): void => {
      values.set(key, String(value));
    },
    removeItem: (key: string): void => {
      values.delete(key);
    },
  };
  return values;
}

async function freshStore() {
  vi.resetModules();
  return import("@/stores/quick-products-store");
}

describe("quick-products store (TASK-DSN-069)", () => {
  beforeEach(() => {
    vi.resetModules();
    const globals = globalThis as unknown as Record<string, unknown>;
    delete globals.localStorage;
  });

  it("hydrates 8 empty slots when storage is absent", async () => {
    installStorage(null);
    const { useQuickProductsStore } = await freshStore();
    useQuickProductsStore.getState().hydrate([]);
    expect(useQuickProductsStore.getState().slots).toHaveLength(8);
    expect(useQuickProductsStore.getState().slots.every((s) => s === null)).toBe(true);
  });

  it("seeds defaults from the first catalog products on first run", async () => {
    installStorage(null);
    const { useQuickProductsStore } = await freshStore();
    const defaults = [product("p1", "Ciment"), product("p2", "Sable")];
    useQuickProductsStore.getState().hydrate(defaults);
    const slots = useQuickProductsStore.getState().slots;
    expect(slots[0]?.product_id).toBe("p1");
    expect(slots[1]?.product_id).toBe("p2");
    expect(slots[2]).toBe(null);
  });

  it("assign/clear round-trips through localStorage", async () => {
    const values = installStorage(null);
    const { useQuickProductsStore } = await freshStore();
    useQuickProductsStore.getState().hydrate([]);
    useQuickProductsStore.getState().assignSlot(3, product("p9", "Clou"));
    expect(useQuickProductsStore.getState().slots[3]?.product_id).toBe("p9");
    const stored = JSON.parse(values.get("baraka-quick-products") ?? "[]") as unknown[];
    expect(stored).toHaveLength(8);
    expect((stored[3] as { product_id: string }).product_id).toBe("p9");
    useQuickProductsStore.getState().clearSlot(3);
    expect(useQuickProductsStore.getState().slots[3]).toBe(null);
  });

  it("restores slots from localStorage on hydrate", async () => {
    const seed = JSON.stringify([
      product("p5", "Gravier"),
      null,
      null,
      null,
      null,
      null,
      null,
      null,
    ]);
    installStorage(seed);
    const { useQuickProductsStore } = await freshStore();
    useQuickProductsStore.getState().hydrate([]);
    expect(useQuickProductsStore.getState().slots[0]?.product_id).toBe("p5");
  });

  it("dispatchSlot adds the assigned product to the cart", async () => {
    installStorage(null);
    const { useQuickProductsStore } = await freshStore();
    const { useCartStore } = await import("@/stores/cart-store");
    useCartStore.getState().clearCart();
    useQuickProductsStore.getState().hydrate([product("p1", "Ciment")]);
    const fired = useQuickProductsStore.getState().dispatchSlot(0);
    expect(fired).toBe(true);
    expect(useCartStore.getState().lines).toHaveLength(1);
    expect(useQuickProductsStore.getState().dispatchSlot(7)).toBe(false);
    expect(useCartStore.getState().lines).toHaveLength(1);
    useCartStore.getState().clearCart();
  });
});

describe("Alt+Digit slot mapping (TASK-DSN-069)", () => {
  it("maps Digit1..8 / Numpad1..8 to slots 0..7, rejects the rest", () => {
    const base = { altKey: true, ctrlKey: false, metaKey: false };
    expect(altDigitSlotIndex({ ...base, code: "Digit1" })).toBe(0);
    expect(altDigitSlotIndex({ ...base, code: "Digit8" })).toBe(7);
    expect(altDigitSlotIndex({ ...base, code: "Numpad3" })).toBe(2);
    expect(altDigitSlotIndex({ ...base, code: "Digit9" })).toBe(null);
    expect(altDigitSlotIndex({ ...base, code: "Digit0" })).toBe(null);
    expect(altDigitSlotIndex({ ...base, code: "KeyA" })).toBe(null);
    expect(altDigitSlotIndex({ altKey: false, ctrlKey: false, metaKey: false, code: "Digit1" })).toBe(null);
    expect(altDigitSlotIndex({ altKey: true, ctrlKey: true, metaKey: false, code: "Digit1" })).toBe(null);
  });
});
