/**
 * Privacy-shield price masking tests (TASK-BRK-127).
 *
 * While shielded, every selling price on the counter screen renders as
 * `•••` (unit price, line total, unit popover, last-price chip); the
 * payable total inside the checkout modal stays visible (cash collection
 * is an operational necessity) and the shield cannot flip mid-payment.
 * `CartTableView`/`UnitSelectPopover` render via `renderToStaticMarkup`
 * with explicit props; the toggle lock is pinned by source contract.
 */

// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CartTableView } from "@/components/pos/CartTable";
import { UnitSelectPopover } from "@/features/pos/UnitSelectPopover";
import { maskMoney } from "@/lib/money";
import type { CartLine, CartUnitSnapshot } from "@/types/pos";

const PIECE: CartUnitSnapshot = {
  unitId: "u-piece",
  productId: "p-sh",
  nameAr: "قطعة",
  nameFr: "Pièce",
  factorMicro: "10000",
  barcode: "PCS-009",
  salePriceCentimes: "25000",
  wholesalePriceCentimes: null,
  superGrosPriceCentimes: null,
  isBaseUnit: true,
  isDefaultSelling: true,
};

const LINE: CartLine = {
  key: "p-sh:u-piece",
  productId: "p-sh",
  sku: "SH-001",
  nameAr: "منتج",
  nameFr: "Produit",
  units: [PIECE],
  unitIndex: 0,
  quantityMicro: "20000",
  lineDiscountCentimes: "0",
  breaksByUnit: {},
};

function noop(): void {}

function viewProps(lines: CartLine[], shielded: boolean) {
  return {
    lines,
    selectedLineIndex: 0,
    locale: "fr-DZ" as const,
    customerId: null,
    customerTier: "retail" as const,
    margins: [],
    shielded,
    popoverKey: null,
    onSelectLine: noop,
    onTogglePopover: noop,
    onClosePopover: noop,
    onSelectUnit: noop,
    onStepQty: noop,
    onRemoveLine: noop,
  };
}

describe("maskMoney", () => {
  it("masks while shielded without parsing (garbage-safe)", () => {
    expect(maskMoney(true, "25000", "fr-DZ")).toBe("•••");
    expect(maskMoney(true, "not-a-number", "fr-DZ")).toBe("•••");
  });

  it("formats exactly while unshielded", () => {
    expect(maskMoney(false, "25000", "fr-DZ")).toBe("250,00 DZD");
  });
});

describe("CartTableView shield masking", () => {
  it("shows prices unshielded", () => {
    const html = renderToStaticMarkup(<CartTableView {...viewProps([LINE], false)} />);
    expect(html).toContain("250,00 DZD");
    expect(html).toContain("500,00 DZD");
    expect(html).not.toContain("•••");
  });

  it("masks unit price and line total while shielded", () => {
    const html = renderToStaticMarkup(<CartTableView {...viewProps([LINE], true)} />);
    expect(html).toContain("•••");
    expect(html).not.toContain("250,00 DZD");
    expect(html).not.toContain("500,00 DZD");
  });
});

describe("UnitSelectPopover shield masking", () => {
  it("masks unit prices while shielded", () => {
    const html = renderToStaticMarkup(
      <UnitSelectPopover
        isOpen={true}
        onClose={noop}
        units={[PIECE]}
        selectedUnitIndex={0}
        onSelectUnit={noop}
        locale="fr-DZ"
        shielded={true}
      />,
    );
    expect(html).toContain("•••");
    expect(html).not.toContain("250,00 DZD");
  });
});

describe("shield toggle lock (no flip during payment)", () => {
  it("pins the checkout lock in CartPanel source", () => {
    const source = readFileSync(
      "D:/Dev/Baraka UltraPOS/src/components/pos/CartPanel.tsx",
      "utf8",
    );
    expect(source).toContain('activeModal === "checkout"');
    expect(source).toContain("disabled={shieldLocked}");
  });

  it("pins breakdown masking with a visible payable TTC in CheckoutModal source", () => {
    const source = readFileSync(
      "D:/Dev/Baraka UltraPOS/src/components/pos/modals/CheckoutModal.tsx",
      "utf8",
    );
    expect(source).toContain("fmtBreakdown(preview?.subtotal_ht_centimes)");
    expect(source).toContain("fmt(preview?.total_ttc_centimes)");
  });
});
