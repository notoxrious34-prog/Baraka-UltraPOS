/**
 * Master Catalog Hub tests (TASK-DSN-041).
 *
 * Pure helpers (margin math, depot resolution, provisioning orchestration)
 * are unit-tested directly; the presentational hub renders via
 * `renderToStaticMarkup` (node-only harness, no DOM library — same pattern
 * as the label-modal view tests); container wiring (discovery load, 150ms
 * debounce, adjust payload) is pinned via source-text assertions, per repo
 * convention.
 */

import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import {
  baseUnitOf,
  CatalogHubView,
  computeKpis,
  computeMargin,
  filterProducts,
  formatMarginPct,
  marginOfBigint,
  nextSort,
  sortProducts,
  stockOfBigint,
  type CatalogKpis,
} from "@/features/catalog/CatalogModule";
import {
  buildModalWire,
  createProductWithProvisioning,
  emptyModalForm,
  isPositiveMicro,
  parseInitialStockToMicro,
  pickDepotId,
  PRINCIPAL_DEPOT_CODE,
  PROVISION_REASON,
  validateModalForm,
  type ModalFormData,
} from "@/features/catalog/ProductCreateModal";
import { t } from "@/lib/i18n";
import type { CatalogProduct } from "@/types/pos";
import type { CreateProductWire } from "@/lib/ipc";

declare const process: { cwd(): string };

function unit(overrides: Partial<CatalogProduct["units"][number]>) {
  return {
    unit_id: "u-1",
    product_id: "p-1",
    unit_name_ar: "قطعة",
    unit_name_fr: "PCS",
    factor_micro: "10000",
    barcode: null,
    sale_price_centimes: "15050",
    purchase_price_centimes: "12000",
    wholesale_price_centimes: null,
    is_base_unit: true,
    is_default_selling: true,
    ...overrides,
  };
}

function product(overrides: Partial<CatalogProduct>): CatalogProduct {
  return {
    product_id: "p-1",
    sku: "VIS-8X40",
    barcode: "6111250000013",
    name_ar: "برغي",
    name_fr: "Vis 8x40",
    category_id: null,
    is_kit: false,
    is_active: true,
    units: [unit({})],
    kit_items: [],
    ...overrides,
  };
}

function validModal(): ModalFormData {
  return {
    ...emptyModalForm(),
    sku: "VIS-8X40",
    barcode: "6111250000013",
    nameAr: "برغي",
    nameFr: "Vis 8x40",
    preset: "PCS",
    salePrice: "150.50",
    costPrice: "120",
    initialStock: "10",
    printAfter: true,
  };
}

describe("computeMargin", () => {
  it("derives amount and truncated percent with integer BigInt math", () => {
    // 15050 - 12000 = 3050; 3050*10000/12000 = 2541 (truncated) -> 25.41.
    expect(computeMargin("15050", "12000")).toEqual({ amountCentimes: "3050", pct: 25.41 });
  });

  it("reports negative margins", () => {
    // 10000 - 12000 = -2000; -2000*10000/12000 = -1666 (truncated) -> -16.66.
    expect(computeMargin("10000", "12000")).toEqual({ amountCentimes: "-2000", pct: -16.66 });
  });

  it("reports zero margin without division artifacts", () => {
    expect(computeMargin("12000", "12000")).toEqual({ amountCentimes: "0", pct: 0 });
  });

  it("returns null percent when purchase is zero", () => {
    expect(computeMargin("15050", "0")).toEqual({ amountCentimes: "15050", pct: null });
  });

  it("treats blank strings as zero", () => {
    expect(computeMargin("", "")).toEqual({ amountCentimes: "0", pct: null });
  });
});

describe("formatMarginPct", () => {
  it("signs positive values and leaves others bare", () => {
    expect(formatMarginPct(25.5)).toBe("+25.5%");
    expect(formatMarginPct(-10)).toBe("-10%");
    expect(formatMarginPct(0)).toBe("0%");
  });
});

describe("baseUnitOf", () => {
  it("prefers the base unit and falls back to the first row", () => {
    const extra = unit({ unit_id: "u-9", is_base_unit: false, unit_name_fr: "BT" });
    expect(baseUnitOf(product({ units: [extra, unit({})] }))?.unit_id).toBe("u-1");
    expect(baseUnitOf(product({ units: [extra] }))?.unit_id).toBe("u-9");
    expect(baseUnitOf(product({ units: [] }))).toBeNull();
  });
});

describe("validateModalForm", () => {
  it("accepts a complete valid form", () => {
    expect(validateModalForm(validModal())).toEqual({});
  });

  it("requires SKU and the French designation", () => {
    const errors = validateModalForm({ ...validModal(), sku: "  ", nameFr: "" });
    expect(errors.sku).toBe("cat.required");
    expect(errors.nameFr).toBe("cat.required");
  });

  it("keeps the Arabic designation optional but length-guarded", () => {
    expect(validateModalForm({ ...validModal(), nameAr: "" })).toEqual({});
    expect(validateModalForm({ ...validModal(), nameAr: "x".repeat(101) }).nameAr).toBe(
      "cat.required",
    );
  });

  it("rejects CUSTOM units without a French name", () => {
    const errors = validateModalForm({ ...validModal(), preset: "CUSTOM", customFr: "" });
    expect(errors.customFr).toBe("cat.required");
  });

  it("rejects negative prices and malformed stock", () => {
    const errors = validateModalForm({ ...validModal(), salePrice: "-5", initialStock: "abc" });
    expect(errors.salePrice).toBe("cat.invalidPrice");
    expect(errors.initialStock).toBe("cat.invalidPrice");
  });

  it("rejects negative initial stock", () => {
    expect(validateModalForm({ ...validModal(), initialStock: "-3" }).initialStock).toBe(
      "cat.invalidPrice",
    );
  });
});

describe("buildModalWire", () => {
  it("emits exact backend key names for the single base unit", () => {
    expect(buildModalWire(validModal())).toEqual({
      sku: "VIS-8X40",
      barcode: "6111250000013",
      name_ar: "برغي",
      name_fr: "Vis 8x40",
      category_id: null,
      is_kit: false,
      units: [
        {
          unit_name_ar: "قطعة",
          unit_name_fr: "PCS",
          factor_micro: "10000",
          barcode: null,
          sale_price_centimes: "15050",
          purchase_price_centimes: "12000",
          wholesale_price_centimes: null,
          is_base_unit: true,
          is_default_selling: true,
        },
      ],
    });
  });

  it("falls back to French names and nulls blank barcodes", () => {
    const wire = buildModalWire({ ...validModal(), nameAr: "  ", barcode: "  " });
    expect(wire.name_ar).toBe("Vis 8x40");
    expect(wire.barcode).toBeNull();
  });

  it("maps CUSTOM preset names through", () => {
    const wire = buildModalWire({
      ...validModal(),
      preset: "CUSTOM",
      customFr: "Rouleau",
      customAr: "لفة",
    });
    expect(wire.units[0]?.unit_name_fr).toBe("Rouleau");
    expect(wire.units[0]?.unit_name_ar).toBe("لفة");
  });

  it("throws the first message key on invalid input", () => {
    expect(() => buildModalWire({ ...validModal(), sku: "" })).toThrow("cat.required");
  });
});

describe("parseInitialStockToMicro / isPositiveMicro", () => {
  it("maps blank to zero and decimals to micros", () => {
    expect(parseInitialStockToMicro("")).toBe("0");
    expect(parseInitialStockToMicro("   ")).toBe("0");
    expect(parseInitialStockToMicro("10")).toBe("100000");
    expect(parseInitialStockToMicro("2.5")).toBe("25000");
  });

  it("rejects negative and malformed input", () => {
    expect(() => parseInitialStockToMicro("-3")).toThrow();
    expect(() => parseInitialStockToMicro("abc")).toThrow();
  });

  it("detects strictly positive micros without throwing", () => {
    expect(isPositiveMicro("100000")).toBe(true);
    expect(isPositiveMicro("0")).toBe(false);
    expect(isPositiveMicro("-5")).toBe(false);
    expect(isPositiveMicro("nope")).toBe(false);
  });
});

describe("pickDepotId", () => {
  const rows = [
    { id: "w-9", code: "SEC-02", name: "Secondaire", is_active: true },
    { id: "w-1", code: "DEP-01", name: "Principal", is_active: true },
  ];

  it("prefers the principal depot code", () => {
    expect(PRINCIPAL_DEPOT_CODE).toBe("DEP-01");
    expect(pickDepotId(rows)).toBe("w-1");
  });

  it("falls back to the first active warehouse, then the first row", () => {
    expect(pickDepotId([rows[0]!])).toBe("w-9");
    const inactive = { id: "w-0", code: "X", name: "X", is_active: false };
    expect(pickDepotId([inactive])).toBe("w-0");
  });

  it("returns null for an empty list", () => {
    expect(pickDepotId([])).toBeNull();
  });
});

describe("createProductWithProvisioning", () => {
  const created = product({});
  const wire: CreateProductWire = {
    sku: "VIS-8X40",
    barcode: null,
    name_ar: "برغي",
    name_fr: "Vis 8x40",
    category_id: null,
    is_kit: false,
    units: [],
  };

  it("provisions with an INCREASE movement on the base unit", async () => {
    const create = vi.fn().mockResolvedValue(created);
    const adjust = vi.fn().mockResolvedValue({ movement_id: "m-1", new_quantity_base_micro: "100000" });
    const outcome = await createProductWithProvisioning("tok-1", wire, "100000", "depot-uuid", {
      create,
      adjust,
    });
    expect(outcome).toEqual({ product: created, provisionError: null });
    expect(create).toHaveBeenCalledWith("tok-1", wire);
    expect(adjust).toHaveBeenCalledTimes(1);
    expect(adjust).toHaveBeenCalledWith({
      session_token: "tok-1",
      product_id: "p-1",
      warehouse_id: "depot-uuid",
      unit_id: "u-1",
      quantity_micro: "100000",
      direction: "INCREASE",
      reason: PROVISION_REASON,
      authorize_negative_stock: false,
    });
    expect(PROVISION_REASON).toBe("Stock initial à la création");
  });

  it("skips the movement when stock is zero or no depot resolved", async () => {
    const create = vi.fn().mockResolvedValue(created);
    const adjust = vi.fn();
    expect((await createProductWithProvisioning("t", wire, "0", "d", { create, adjust })).provisionError).toBeNull();
    expect((await createProductWithProvisioning("t", wire, "100000", null, { create, adjust })).provisionError).toBeNull();
    expect(adjust).not.toHaveBeenCalled();
  });

  it("keeps the created product and reports the wire code on stock failure", async () => {
    const create = vi.fn().mockResolvedValue(created);
    const adjust = vi.fn().mockRejectedValue("STOCK_NEGATIVE_NOT_AUTHORIZED");
    const outcome = await createProductWithProvisioning("t", wire, "50000", "d", {
      create,
      adjust,
    });
    expect(outcome).toEqual({ product: created, provisionError: "STOCK_NEGATIVE_NOT_AUTHORIZED" });
  });

  it("falls back to the first unit and flags a unit-less product", async () => {
    const create = vi.fn().mockResolvedValue(product({ units: [unit({ unit_id: "u-9", is_base_unit: false })] }));
    const adjust = vi.fn().mockResolvedValue({});
    const outcome = await createProductWithProvisioning("t", wire, "10000", "d", { create, adjust });
    expect(outcome.provisionError).toBeNull();
    expect(adjust.mock.calls[0]?.[0]).toMatchObject({ unit_id: "u-9" });
    const unitless = vi.fn().mockResolvedValue(product({ units: [] }));
    const failed = await createProductWithProvisioning("t", wire, "10000", "d", {
      create: unitless,
      adjust,
    });
    expect(failed.provisionError).toBe("NO_BASE_UNIT");
  });
});

describe("CatalogHubView markup", () => {
  const noop = (): void => undefined;
  const noopEdit = (): void => undefined;
  const noopFilter = (): void => undefined;
  const noopSort = (): void => undefined;
  const testKpis: CatalogKpis = { total: 2, inStock: 1, outOfStock: 0, inactive: 1 };
  const rows = [
    product({}),
    product({
      product_id: "p-2",
      sku: "CLOU-5",
      barcode: null,
      name_ar: "مسمار",
      name_fr: "Clou",
      is_active: false,
      units: [unit({ unit_id: "u-2", product_id: "p-2", sale_price_centimes: "10000", purchase_price_centimes: "12000" })],
    }),
  ];

  it("renders SKUs, bilingual names, prices, margins, stock and statuses", () => {
    const html = renderToStaticMarkup(
      <CatalogHubView
        locale="fr-DZ"
        products={rows}
        stockLevels={{ "p-1": "150000", "p-2": "0" }}
        loading={false}
        error={null}
        query=""
        onQuery={noop}
        onClear={noop}
        onRefresh={noop}
        onNew={noop}
        onPrintLabel={noop}
        onEdit={noopEdit}
        onDelete={noopEdit}
        onToggleActive={noopEdit}
        togglingId={null}
        filter="all"
        kpis={testKpis}
        sort={null}
        onFilter={noopFilter}
        onSort={noopSort}
      />,
    );
    expect(html).toContain("VIS-8X40");
    expect(html).toContain("Vis 8x40");
    expect(html).toContain("برغي");
    expect(html).toContain("150,50 DZD");
    expect(html).toContain("120,00 DZD");
    expect(html).toContain("30,50 DZD");
    expect(html).toContain("+25.41%");
    expect(html).toContain("-16.66%");
    expect(html).toContain("15 PCS");
    expect(html).toContain("0 - Rupture");
    expect(html).toContain("Actif");
    expect(html).toContain("Inactif");
    expect(html).toContain("🏷️");
    expect(html).toContain("2 produits");
  });

  it("shows the empty state and pending stock placeholders", () => {
    const html = renderToStaticMarkup(
      <CatalogHubView
        locale="ar-DZ"
        products={[]}
        stockLevels={{}}
        loading={false}
        error={null}
        query=""
        onQuery={noop}
        onClear={noop}
        onRefresh={noop}
        onNew={noop}
        onPrintLabel={noop}
        onEdit={noopEdit}
        onDelete={noopEdit}
        onToggleActive={noopEdit}
        togglingId={null}
        filter="all"
        kpis={testKpis}
        sort={null}
        onFilter={noopFilter}
        onSort={noopSort}
      />,
    );
    expect(html).toContain("لا توجد منتجات");
    expect(html).toContain("كتالوج المنتجات");
    const pending = renderToStaticMarkup(
      <CatalogHubView
        locale="fr-DZ"
        products={[product({})]}
        stockLevels={{}}
        loading={false}
        error={null}
        query="vis"
        onQuery={noop}
        onClear={noop}
        onRefresh={noop}
        onNew={noop}
        onPrintLabel={noop}
        onEdit={noopEdit}
        onDelete={noopEdit}
        onToggleActive={noopEdit}
        togglingId={null}
        filter="all"
        kpis={testKpis}
        sort={null}
        onFilter={noopFilter}
        onSort={noopSort}
      />,
    );
    expect(pending).toContain("…");
  });

  it("renders zero-purchase margins as an em dash", () => {
    const html = renderToStaticMarkup(
      <CatalogHubView
        locale="fr-DZ"
        products={[
          product({ units: [unit({ sale_price_centimes: "5000", purchase_price_centimes: "0" })] }),
        ]}
        stockLevels={{ "p-1": "10000" }}
        loading={false}
        error={null}
        query=""
        onQuery={noop}
        onClear={noop}
        onRefresh={noop}
        onNew={noop}
        onPrintLabel={noop}
        onEdit={noopEdit}
        onDelete={noopEdit}
        onToggleActive={noopEdit}
        togglingId={null}
        filter="all"
        kpis={testKpis}
        sort={null}
        onFilter={noopFilter}
        onSort={noopSort}
      />,
    );
    expect(html).toContain("—");
  });
});

describe("hub i18n keys", () => {
  it("declares the hub strings in FR and AR", () => {
    expect(t("fr-DZ", "cat.masterTitle")).toBe("Catalogue des Produits");
    expect(t("ar-DZ", "cat.masterTitle")).toBe("كتالوج المنتجات");
    expect(t("fr-DZ", "cat.newProduct")).toBe("+ Nouveau Produit");
    expect(t("ar-DZ", "cat.newProduct")).toBe("+ منتج جديد");
    expect(t("fr-DZ", "cat.outOfStock")).toBe("Rupture");
    expect(t("ar-DZ", "cat.outOfStock")).toBe("نفاد");
    expect(t("fr-DZ", "cat.noProducts")).toBe("Aucun produit trouvé");
    expect(t("ar-DZ", "cat.noProducts")).toBe("لا توجد منتجات");
    expect(t("fr-DZ", "cat.colMargin")).toBe("Marge");
    expect(t("ar-DZ", "cat.colMargin")).toBe("الهامش");
    expect(t("fr-DZ", "cat.stockInitial")).toBe("Stock initial");
    expect(t("ar-DZ", "cat.stockInitial")).toBe("المخزون الأولي");
  });
});

describe("container wiring (source pins)", () => {
  const root = process.cwd();
  const hubSrc = readFileSync(join(root, "src", "features", "catalog", "CatalogModule.tsx"), "utf8") as string;
  const modalSrc = readFileSync(
    join(root, "src", "features", "catalog", "ProductCreateModal.tsx"),
    "utf8",
  ) as string;

  it("loads discovery rows on mount with a blank query", () => {
    expect(hubSrc).toContain('void loadCatalog(activeToken, "", alive)');
  });

  it("debounces search at 150ms and searches with limit 500 including inactive", () => {
    expect(hubSrc).toContain("SEARCH_DEBOUNCE_MS = 150");
    expect(hubSrc).toContain("SEARCH_LIMIT = 500");
    expect(hubSrc).toContain("ipcSearchCatalog(tok, term, SEARCH_LIMIT, true)");
    expect(hubSrc).toContain("void loadCatalog(activeToken, query.trim(), alive)");
  });

  it("fetches real database metrics alongside rows for the KPI cards", () => {
    expect(hubSrc).toContain("ipcGetCatalogMetrics(tok)");
    expect(hubSrc).toContain("setMetrics(dbMetrics)");
    expect(hubSrc).toContain("metrics.total_products");
  });

  it("never blanks rows on empty input and fetches DEP-01 stock per row", () => {
    expect(hubSrc).not.toContain("setRows([])");
    expect(hubSrc).toContain("ipcGetStock(tok, product.product_id, depot)");
  });

  it("integrates both modals with the print-after-create handoff", () => {
    expect(hubSrc).toContain("<ProductCreateModal");
    expect(hubSrc).toContain("<ProductLabelModal");
    expect(hubSrc).toContain("onCreated={(product, printLabel)");
    expect(hubSrc).toContain("setLabelProduct(printLabel ? product : null)");
  });

  it("provisions stock with an explicit INCREASE and the creation reason", () => {
    expect(modalSrc).toContain('direction: "INCREASE"');
    expect(modalSrc).toContain('reason: PROVISION_REASON');
    expect(modalSrc).toContain("await deps.adjust(payload)");
  });

  it("exposes an accessible dialog with Escape handling", () => {
    expect(modalSrc).toContain('role="dialog"');
    expect(modalSrc).toContain('aria-modal="true"');
    expect(modalSrc).toContain('"Escape"');
  });
});

describe("computeKpis / filterProducts", () => {
  const stocked = product({});
  const ruptured = product({
    product_id: "p-2",
    sku: "RUPT",
    units: [
      {
        unit_id: "u-2",
        product_id: "p-2",
        unit_name_ar: "قطعة",
        unit_name_fr: "PCS",
        factor_micro: "10000",
        barcode: null,
        sale_price_centimes: "5000",
        purchase_price_centimes: "4000",
        wholesale_price_centimes: null,
        is_base_unit: true,
        is_default_selling: true,
      },
    ],
  });
  const dormant = product({ product_id: "p-3", sku: "OLD", is_active: false });
  const rows = [stocked, ruptured, dormant];
  const levels = { "p-1": "150000", "p-2": "0" };

  it("tallies total, in-stock, rupture and inactive rows", () => {
    expect(computeKpis(rows, levels)).toEqual({ total: 3, inStock: 1, outOfStock: 1, inactive: 1 });
    expect(computeKpis([], {})).toEqual({ total: 0, inStock: 0, outOfStock: 0, inactive: 0 });
  });

  it("treats missing stock as rupture and malformed micros as zero", () => {
    expect(computeKpis([stocked], {})).toEqual({ total: 1, inStock: 0, outOfStock: 1, inactive: 0 });
    expect(stockOfBigint(stocked, { "p-1": "nope" })).toBe(0n);
  });

  it("filters rows per KPI tab", () => {
    expect(filterProducts(rows, levels, "all").map((row) => row.sku)).toEqual([
      "VIS-8X40",
      "RUPT",
      "OLD",
    ]);
    expect(filterProducts(rows, levels, "in_stock").map((row) => row.sku)).toEqual(["VIS-8X40"]);
    expect(filterProducts(rows, levels, "out_of_stock").map((row) => row.sku)).toEqual(["RUPT"]);
    expect(filterProducts(rows, levels, "inactive").map((row) => row.sku)).toEqual(["OLD"]);
  });
});

describe("sortProducts / nextSort", () => {
  const cheap = product({ product_id: "a", sku: "B-SKU", name_fr: "Banane" });
  const pricey = product({
    product_id: "b",
    sku: "A-SKU",
    name_fr: "Abricot",
    units: [
      {
        unit_id: "ub",
        product_id: "b",
        unit_name_ar: "قطعة",
        unit_name_fr: "PCS",
        factor_micro: "10000",
        barcode: null,
        sale_price_centimes: "99900",
        purchase_price_centimes: "1000",
        wholesale_price_centimes: null,
        is_base_unit: true,
        is_default_selling: true,
      },
    ],
  });
  const rows = [cheap, pricey];
  const levels = { a: "10000", b: "50000" };

  it("keeps load order without a sort and never mutates the input", () => {
    const frozen = [...rows];
    expect(sortProducts(rows, levels, null).map((row) => row.product_id)).toEqual(["a", "b"]);
    expect(rows).toEqual(frozen);
  });

  it("orders SKUs and names alphabetically both ways", () => {
    expect(sortProducts(rows, levels, { field: "sku", direction: "asc" }).map((r) => r.sku)).toEqual([
      "A-SKU",
      "B-SKU",
    ]);
    expect(sortProducts(rows, levels, { field: "name", direction: "desc" }).map((r) => r.name_fr)).toEqual([
      "Banane",
      "Abricot",
    ]);
  });

  it("orders prices, margins and stock with BigInt math", () => {
    expect(
      sortProducts(rows, levels, { field: "price", direction: "asc" }).map((r) => r.product_id),
    ).toEqual(["a", "b"]);
    expect(
      sortProducts(rows, levels, { field: "margin", direction: "desc" }).map((r) => r.product_id),
    ).toEqual(["b", "a"]);
    expect(
      sortProducts(rows, levels, { field: "stock", direction: "desc" }).map((r) => r.product_id),
    ).toEqual(["b", "a"]);
    expect(marginOfBigint(pricey)).toBe(98900n);
  });

  it("toggles direction on the active column and resets on a new one", () => {
    expect(nextSort(null, "sku")).toEqual({ field: "sku", direction: "asc" });
    expect(nextSort({ field: "sku", direction: "asc" }, "sku")).toEqual({
      field: "sku",
      direction: "desc",
    });
    expect(nextSort({ field: "sku", direction: "desc" }, "price")).toEqual({
      field: "price",
      direction: "asc",
    });
  });
});

describe("KPI strip and export markup", () => {
  const noop = (): void => undefined;
  const view = (
    extra: Partial<{
      filter: "all" | "in_stock" | "out_of_stock" | "inactive";
      sort: { field: "sku" | "name" | "price" | "margin" | "stock"; direction: "asc" | "desc" } | null;
    }> = {},
  ) =>
    renderToStaticMarkup(
      <CatalogHubView
        locale="fr-DZ"
        products={[product({})]}
        stockLevels={{ "p-1": "150000" }}
        loading={false}
        error={null}
        query=""
        togglingId={null}
        filter={extra.filter ?? "all"}
        kpis={{ total: 3, inStock: 1, outOfStock: 1, inactive: 1 }}
        sort={extra.sort ?? null}
        onQuery={noop}
        onClear={noop}
        onRefresh={noop}
        onNew={noop}
        onPrintLabel={noop}
        onEdit={noop}
        onDelete={noop}
        onToggleActive={noop}
        onFilter={noop}
        onSort={noop}
      />,
    );

  it("renders KPI titles with counts and no data-exchange menu trigger", () => {
    const html = view();
    expect(html).toContain("Tous les produits");
    expect(html).toContain("En stock");
    expect(html).toContain("En rupture");
    expect(html).toContain("Inactifs");
    // DSN-053: exchange lives in Settings — the purified hub toolbar
    // carries search, refresh, and new-product only.
    expect(html).not.toContain("Échanges &amp; Données");
    expect(html).not.toContain("Exporter CSV");
  });

  it("marks the active filter and idle sort columns", () => {
    const html = view({ filter: "in_stock" });
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain("↕");
  });

  it("shows the active sort direction", () => {
    expect(view({ sort: { field: "price", direction: "asc" } })).toContain("▲");
    expect(view({ sort: { field: "price", direction: "desc" } })).toContain("▼");
  });

  it("declares the hub strings in both locales", () => {
    expect(t("fr-DZ", "cat.kpiInStock")).toBe("En stock");
    expect(t("ar-DZ", "cat.kpiInStock")).toBe("متوفر في المخزون");
    expect(t("fr-DZ", "cat.exportCsv")).toBe("Exporter CSV");
    expect(t("ar-DZ", "cat.exportCsv")).toBe("تصدير CSV");
  });
});
