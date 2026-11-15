import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { toRows } from "@/features/catalog/PriceBreaksEditor";
import { ProductEditModalView, formFromProduct } from "@/features/catalog/ProductEditModal";
import type { CatalogProduct } from "@/types/pos";

function product(): CatalogProduct {
  return {
    product_id: "p-brk",
    sku: "BRK-1",
    barcode: null,
    name_ar: "قطعة",
    name_fr: "Piece",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [
      {
        unit_id: "u-base",
        product_id: "p-brk",
        unit_name_ar: "قطعة",
        unit_name_fr: "Piece",
        factor_micro: "10000",
        barcode: null,
        sale_price_centimes: "1000",
        purchase_price_centimes: "800",
        wholesale_price_centimes: null,
        super_gros_price_centimes: null,
        is_base_unit: true,
        is_default_selling: true,
      },
    ],
    kit_items: [],
  };
}

describe("PriceBreaksEditor (BRK-121b)", () => {
  it("maps wire rungs to display rows", () => {
    expect(
      toRows([
        { min_qty_micro: "50000", price_centimes: "450" },
        { min_qty_micro: "oops", price_centimes: "nope" },
      ]),
    ).toEqual([
      { qty: "5", price: "4.5" },
      { qty: "", price: "0" },
    ]);
  });

  it("renders the breaks section when an editor node is provided", () => {
    const item = product();
    const html = renderToStaticMarkup(
      <ProductEditModalView
        product={item}
        form={formFromProduct(item)}
        locale="fr-DZ"
        attempted={false}
        busy={false}
        submitError={null}
        cardRef={{ current: null }}
        onPatch={() => undefined}
        onClose={() => undefined}
        onSubmit={() => undefined}
        breaksEditor={<div data-testid="breaks-stub" />}
      />,
    );
    expect(html).toContain("Paliers de quantité");
    expect(html).toContain("breaks-stub");
  });

  it("hides the breaks section without an editor node", () => {
    const item = product();
    const html = renderToStaticMarkup(
      <ProductEditModalView
        product={item}
        form={formFromProduct(item)}
        locale="ar-DZ"
        attempted={false}
        busy={false}
        submitError={null}
        cardRef={{ current: null }}
        onPatch={() => undefined}
        onClose={() => undefined}
        onSubmit={() => undefined}
      />,
    );
    expect(html).not.toContain("breaks-stub");
    expect(html).toContain("تعديل المنتج");
  });
});
