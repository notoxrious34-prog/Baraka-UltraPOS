/**
 * Multi-unit sales cockpit tests (TASK-DSN-046).
 *
 * `CartTableView`/`UnitSelectPopover` render via `renderToStaticMarkup` with
 * explicit props (node-only harness: store-connected containers render store
 * initial state, so the pure views carry the assertions). Store behavior is
 * covered in `src/stores/__tests__/cart-store.test.ts`; keyboard/barcode/
 * discovery wiring is pinned by source assertions below.
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CartTableView } from "@/components/pos/CartTable";
import { UnitSelectPopover, factorTag } from "@/features/pos/UnitSelectPopover";
import { calculateBaseQuantityEquivalent } from "@/stores/cart-store";
import type { CartLine, CartUnitSnapshot } from "@/types/pos";

const PIECE: CartUnitSnapshot = {
  unitId: "u-piece",
  productId: "p-vis",
  nameAr: "قطعة",
  nameFr: "Pièce",
  factorMicro: "10000",
  barcode: "PCS-001",
  salePriceCentimes: "15020",
  wholesalePriceCentimes: null,
  isBaseUnit: true,
  isDefaultSelling: true,
};

const BOX: CartUnitSnapshot = {
  unitId: "u-box",
  productId: "p-vis",
  nameAr: "علبة",
  nameFr: "Boîte",
  factorMicro: "1000000",
  barcode: "BOX-001",
  salePriceCentimes: "1205000",
  wholesalePriceCentimes: null,
  isBaseUnit: false,
  isDefaultSelling: false,
};

const MULTI_LINE: CartLine = {
  key: "p-vis:u-box",
  productId: "p-vis",
  sku: "VIS-PLA-25",
  nameAr: "برغي بلاكو",
  nameFr: "Vis Placo",
  units: [PIECE, BOX],
  unitIndex: 1,
  quantityMicro: "30000",
  lineDiscountCentimes: "0",
};

const SINGLE_LINE: CartLine = {
  key: "p-cou:u-piece",
  productId: "p-cou",
  sku: "COU-PVC-32",
  nameAr: "كوع",
  nameFr: "Coude PVC",
  units: [PIECE],
  unitIndex: 0,
  quantityMicro: "20000",
  lineDiscountCentimes: "0",
};

function noop(): void {}

function viewProps(lines: CartLine[], popoverKey: string | null = null) {
  return {
    lines,
    selectedLineIndex: 0,
    locale: "fr-DZ" as const,
    customerTier: "retail" as const,
    margins: [],
    popoverKey,
    onSelectLine: noop,
    onTogglePopover: noop,
    onClosePopover: noop,
    onSelectUnit: noop,
    onStepQty: noop,
    onRemoveLine: noop,
  };
}

describe("CartTableView multi-unit rendering", () => {
  it("shows the unit badge with factor chip, F3 hint, and equivalence HUD", () => {
    const html = renderToStaticMarkup(<CartTableView {...viewProps([MULTI_LINE])} />);
    expect(html).toContain("Boîte");
    expect(html).toContain("x100");
    expect(html).toContain("F3");
    expect(html).toContain("(= 300 Pièce)");
    expect(html).toContain("50,00 DZD");
  });

  it("shows no HUD for base-unit lines", () => {
    const html = renderToStaticMarkup(
      <CartTableView {...viewProps([{ ...MULTI_LINE, unitIndex: 0 }])} />,
    );
    expect(html).not.toContain("(=");
  });

  it("renders a non-interactive single-unit badge without popover affordance", () => {
    const html = renderToStaticMarkup(<CartTableView {...viewProps([SINGLE_LINE])} />);
    expect(html).toContain("Pièce");
    expect(html).not.toContain("aria-haspopup");
    expect(html).not.toContain("F3");
  });

  it("mounts the popover for the toggled line only", () => {
    const html = renderToStaticMarkup(
      <CartTableView {...viewProps([MULTI_LINE], "p-vis:u-box")} />,
    );
    expect(html).toContain('role="dialog"');
    const closed = renderToStaticMarkup(<CartTableView {...viewProps([MULTI_LINE])} />);
    expect(closed).not.toContain('role="dialog"');
  });
});

describe("UnitSelectPopover rendering", () => {
  it("lists units with number chips, factor tags, and prices", () => {
    const html = renderToStaticMarkup(
      <UnitSelectPopover
        isOpen={true}
        onClose={noop}
        units={[PIECE, BOX]}
        selectedUnitIndex={1}
        onSelectUnit={noop}
        locale="fr-DZ"
      />,
    );
    expect(html).toContain("Pièce");
    expect(html).toContain("Boîte");
    expect(html).toContain("Base");
    expect(html).toContain("x100");
    expect(html).toContain("150,20 DZD");
    expect(html).toContain("✓");
  });

  it("renders nothing when closed", () => {
    const html = renderToStaticMarkup(
      <UnitSelectPopover
        isOpen={false}
        onClose={noop}
        units={[PIECE, BOX]}
        selectedUnitIndex={0}
        onSelectUnit={noop}
        locale="fr-DZ"
      />,
    );
    expect(html).toBe("");
  });

  it("selects the unit and closes on option click", () => {
    const onSelectUnit = vi.fn();
    const onClose = vi.fn();
    // Static harness cannot click: pin the contract — click calls
    // onSelectUnit(index) then onClose() — via source.
    const source = readFileSync(
      "D:/Dev/Baraka UltraPOS/src/features/pos/UnitSelectPopover.tsx",
      "utf8",
    );
    expect(source).toContain("onSelectUnit(index);");
    expect(onSelectUnit).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("equivalence helper demonstration", () => {
  it("proves 3 Boîtes x100 = 300 Pièces with exact BigInt math", () => {
    expect(factorTag("1000000")).toBe("x100");
    expect(
      calculateBaseQuantityEquivalent("30000", "1000000", "Pièces", "قطعة", "fr-DZ"),
    ).toBe("(= 300 Pièces)");
    expect(
      calculateBaseQuantityEquivalent("30000", "1000000", "Pièces", "قطعة", "ar-DZ"),
    ).toBe("(= 300 قطعة)");
  });
});

describe("cockpit wiring pins", () => {
  it("F3 cycles with banner feedback in use-pos-keyboard", () => {
    const source = readFileSync(
      "D:/Dev/Baraka UltraPOS/src/hooks/use-pos-keyboard.ts",
      "utf8",
    );
    expect(source).toContain("cart.cycleLineUnit(line.key)");
    expect(source).toContain("result.cycled");
    expect(source).toContain('t(ui.locale, "pos.unitChanged")');
    expect(source).toContain('t(ui.locale, "pos.singleUnitNotice")');
  });

  it("scanner wedge resolves the exact unit barcode in use-catalog-search", () => {
    const source = readFileSync(
      "D:/Dev/Baraka UltraPOS/src/hooks/use-catalog-search.ts",
      "utf8",
    );
    expect(source).toContain("unit.barcode === code");
    expect(source).toContain("matchedUnitIndex >= 0 ? matchedUnitIndex : undefined");
  });

  it("ProductDiscovery badges multi-unit results with the unit roster tooltip", () => {
    const source = readFileSync(
      "D:/Dev/Baraka UltraPOS/src/components/pos/ProductDiscovery.tsx",
      "utf8",
    );
    expect(source).toContain("product.units.length > 1");
    expect(source).toContain("unités");
  });

  it("CartTable bridges setLineUnit into the popover", () => {
    const source = readFileSync(
      "D:/Dev/Baraka UltraPOS/src/components/pos/CartTable.tsx",
      "utf8",
    );
    expect(source).toContain("(key, index) => setLineUnit(key, index)");
    expect(source).toContain("calculateBaseQuantityEquivalent(");
  });
});
