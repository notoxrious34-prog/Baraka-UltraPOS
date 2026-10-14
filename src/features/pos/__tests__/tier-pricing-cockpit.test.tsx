/**
 * Pillar 4 tier-pricing cockpit tests (TASK-DSN-047).
 *
 * `CustomerTierBadge` labels, cart wholesale-price derivation
 * (`lineUnitPriceCentimes` / `isLineTierPriced`), Margin Guard cart pills,
 * and the AboutTab brand showcase — all via `renderToStaticMarkup`
 * (node-only harness, zero floats).
 */

import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CustomerTierBadge } from "@/components/pos/CustomerTierBadge";
import { CartTableView } from "@/components/pos/CartTable";
import { AboutTab } from "@/features/settings/AboutTab";
import { isLineTierPriced, lineUnitPriceCentimes } from "@/stores/cart-store";
import type { CartLine } from "@/types/pos";

function noop(): void {}

function tierLine(wholesale: string | null): CartLine {
  return {
    key: "p-vis:u-piece",
    productId: "p-vis",
    sku: "VIS-8X40",
    nameAr: "برغي",
    nameFr: "Vis 8x40",
    units: [
      {
        unitId: "u-piece",
        productId: "p-vis",
        nameAr: "قطعة",
        nameFr: "Pièce",
        factorMicro: "10000",
        barcode: null,
        salePriceCentimes: "10000",
        wholesalePriceCentimes: wholesale,
        isBaseUnit: true,
        isDefaultSelling: true,
      },
    ],
    unitIndex: 0,
    quantityMicro: "10000",
    lineDiscountCentimes: "0",
  };
}

describe("CustomerTierBadge", () => {
  it("renders retail/artisan/wholesale labels in French", () => {
    expect(renderToStaticMarkup(<CustomerTierBadge tier="retail" locale="fr-DZ" />)).toContain(
      "Détail",
    );
    expect(renderToStaticMarkup(<CustomerTierBadge tier="artisan" locale="fr-DZ" />)).toContain(
      "Artisan",
    );
    expect(renderToStaticMarkup(<CustomerTierBadge tier="wholesale" locale="fr-DZ" />)).toContain(
      "Gros",
    );
  });

  it("renders retail/artisan/wholesale labels in Arabic", () => {
    expect(renderToStaticMarkup(<CustomerTierBadge tier="retail" locale="ar-DZ" />)).toContain(
      "تجزئة",
    );
    expect(renderToStaticMarkup(<CustomerTierBadge tier="artisan" locale="ar-DZ" />)).toContain(
      "حرفي",
    );
    expect(renderToStaticMarkup(<CustomerTierBadge tier="wholesale" locale="ar-DZ" />)).toContain(
      "جملة",
    );
  });

  it("uses distinct color tokens per tier", () => {
    const retail = renderToStaticMarkup(<CustomerTierBadge tier="retail" locale="fr-DZ" />);
    const artisan = renderToStaticMarkup(<CustomerTierBadge tier="artisan" locale="fr-DZ" />);
    const wholesale = renderToStaticMarkup(<CustomerTierBadge tier="wholesale" locale="fr-DZ" />);
    expect(retail).toContain("bg-slate-100");
    expect(artisan).toContain("bg-amber-50");
    expect(wholesale).toContain("bg-indigo-50");
  });
});

describe("cart tier-price derivation", () => {
  it("charges the sale price for retail customers", () => {
    const line = tierLine("8500");
    expect(lineUnitPriceCentimes(line, "retail")).toBe("10000");
    expect(isLineTierPriced(line, "retail")).toBe(false);
  });

  it("charges the wholesale price for artisan and wholesale customers", () => {
    const line = tierLine("8500");
    expect(lineUnitPriceCentimes(line, "artisan")).toBe("8500");
    expect(lineUnitPriceCentimes(line, "wholesale")).toBe("8500");
    expect(isLineTierPriced(line, "artisan")).toBe(true);
    expect(isLineTierPriced(line, "wholesale")).toBe(true);
  });

  it("falls back to the sale price when no wholesale price is set", () => {
    const line = tierLine(null);
    expect(lineUnitPriceCentimes(line, "artisan")).toBe("10000");
    expect(isLineTierPriced(line, "artisan")).toBe(false);
  });
});

describe("CartTableView tier and Margin Guard pills", () => {
  function viewProps(
    lines: CartLine[],
    customerTier: "retail" | "artisan" | "wholesale" = "retail",
    margins: Array<{ line_index: number; margin_basis_points: number; is_below_cost: boolean }> = [],
  ) {
    return {
      lines,
      selectedLineIndex: 0,
      locale: "fr-DZ" as const,
      customerTier,
      margins,
      popoverKey: null,
      onSelectLine: noop,
      onTogglePopover: noop,
      onClosePopover: noop,
      onSelectUnit: noop,
      onStepQty: noop,
      onRemoveLine: noop,
    };
  }

  it("shows the PRO pill and tier price for artisan customers", () => {
    const html = renderToStaticMarkup(<CartTableView {...viewProps([tierLine("8500")], "artisan")} />);
    expect(html).toContain("PRO");
    expect(html).toContain("85,00 DZD");
  });

  it("shows the GROS pill for wholesale customers", () => {
    const html = renderToStaticMarkup(
      <CartTableView {...viewProps([tierLine("8500")], "wholesale")} />,
    );
    expect(html).toContain("GROS");
  });

  it("shows no tier pill for retail customers", () => {
    const html = renderToStaticMarkup(<CartTableView {...viewProps([tierLine("8500")])} />);
    expect(html).not.toContain("PRO");
    expect(html).not.toContain("GROS");
  });

  it("pulses the below-cost warning with the formatted margin", () => {
    const html = renderToStaticMarkup(
      <CartTableView
        {...viewProps([tierLine("8500")], "artisan", [
          { line_index: 0, margin_basis_points: -2500, is_below_cost: true },
        ])}
      />,
    );
    expect(html).toContain("Vente à perte");
    expect(html).toContain("animate-pulse");
    expect(html).toContain("-25%");
  });

  it("shows no warning for healthy margins", () => {
    const html = renderToStaticMarkup(
      <CartTableView
        {...viewProps([tierLine("8500")], "artisan", [
          { line_index: 0, margin_basis_points: 1764, is_below_cost: false },
        ])}
      />,
    );
    expect(html).not.toContain("Vente à perte");
  });
});

describe("AboutTab brand showcase", () => {
  it("renders the official logo and version 1.0.0", () => {
    const html = renderToStaticMarkup(<AboutTab locale="fr-DZ" />);
    expect(html).toContain("baraka-logo.png");
    expect(html).toContain("1.0.0");
    expect(html).toContain("Baraka UltraPOS");
  });
});
