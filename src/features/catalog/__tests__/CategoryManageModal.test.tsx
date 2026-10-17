/**
 * Category suite tests (TASK-DSN-044).
 *
 * Pure helpers (code normalization, form validation, pill labels, hub
 * filtering/counts/lookup) are unit-tested directly; the pill strip, hub
 * category column, and edit-modal selector render via `renderToStaticMarkup`
 * (node-only harness, no DOM library — same pattern as the hub/edit view
 * tests); container wiring (taxonomy load, seed flow, modal mount, payload
 * paths) is pinned via source-text assertions, per repo convention.
 */

import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { invoke } from "@tauri-apps/api/core";
import {
  CategoryPillStrip,
  categoryLabel,
  countForCategory,
} from "@/features/catalog/CategoryPillStrip";
import {
  normalizeCategoryCode,
  validateCategoryForm,
} from "@/features/catalog/CategoryManageModal";
import {
  CatalogHubView,
  categoryById,
  countProductsByCategory,
  filterByCategory,
} from "@/features/catalog/CatalogModule";
import { buildModalWire } from "@/features/catalog/ProductCreateModal";
import {
  buildUpdatePayload,
  formFromProduct,
  ProductEditModalView,
} from "@/features/catalog/ProductEditModal";
import { ipcSeedStarterCategories } from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { CatalogProduct, Category, SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

declare const process: { cwd(): string };

function category(overrides: Partial<Category>): Category {
  return {
    id: "cat-1",
    code: "PLOMB",
    name_fr: "Plomberie",
    name_ar: "الترصيص",
    parent_id: null,
    is_active: true,
    ...overrides,
  };
}

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

describe("normalizeCategoryCode", () => {
  it("trims and uppercases the code", () => {
    expect(normalizeCategoryCode(" plomb ")).toBe("PLOMB");
    expect(normalizeCategoryCode("Visserie")).toBe("VISSERIE");
    expect(normalizeCategoryCode("")).toBe("");
  });
});

describe("validateCategoryForm", () => {
  it("accepts a bilingual form with an uppercaseable code", () => {
    expect(
      validateCategoryForm({ code: "plomb", nameFr: "Plomberie", nameAr: "الترصيص" }),
    ).toEqual({});
  });

  it("requires code, French and Arabic names", () => {
    const errors = validateCategoryForm({ code: "   ", nameFr: "", nameAr: "  " });
    expect(errors.code).toBe("cat.required");
    expect(errors.nameFr).toBe("cat.required");
    expect(errors.nameAr).toBe("cat.required");
  });
});

describe("categoryLabel / countForCategory", () => {
  const row = category({});

  it("picks the locale-aware name", () => {
    expect(categoryLabel(row, "fr-DZ")).toBe("Plomberie");
    expect(categoryLabel(row, "ar-DZ")).toBe("الترصيص");
  });

  it("reads counts defensively", () => {
    expect(countForCategory({ "cat-1": 4 }, "cat-1")).toBe(4);
    expect(countForCategory({}, "cat-1")).toBe(0);
    expect(countForCategory(undefined, "cat-1")).toBe(0);
  });
});

describe("filterByCategory / countProductsByCategory / categoryById", () => {
  const rows = [
    product({}),
    product({ product_id: "p-2", sku: "CLOU-5", category_id: "cat-1" }),
    product({ product_id: "p-3", sku: "MAR-3", category_id: "cat-2" }),
  ];

  it("keeps every row when no category is selected", () => {
    expect(filterByCategory(rows, null)).toHaveLength(3);
  });

  it("filters displayed products to the selected category only", () => {
    const visible = filterByCategory(rows, "cat-1");
    expect(visible).toHaveLength(1);
    expect(visible[0]?.sku).toBe("CLOU-5");
  });

  it("tallies per-category counts skipping unassigned rows", () => {
    expect(countProductsByCategory(rows)).toEqual({ "cat-1": 1, "cat-2": 1 });
  });

  it("resolves categories by id", () => {
    const rows2 = [category({}), category({ id: "cat-2", code: "ELEC" })];
    expect(categoryById(rows2, "cat-2")?.code).toBe("ELEC");
    expect(categoryById(rows2, null)).toBeNull();
    expect(categoryById(rows2, "missing")).toBeNull();
  });
});

describe("CategoryPillStrip markup", () => {
  const noopSelect = (): void => undefined;
  const noopManage = (): void => undefined;
  const rows = [
    category({}),
    category({ id: "cat-2", code: "ELEC", name_fr: "Électricité", name_ar: "الكهرباء" }),
    category({ id: "cat-3", code: "OLD", is_active: false }),
  ];

  it("renders Tous, active pills with counts, and the manage CTA", () => {
    const html = renderToStaticMarkup(
      <CategoryPillStrip
        locale="fr-DZ"
        categories={rows}
        selectedCategoryId={null}
        onSelectCategory={noopSelect}
        onManageCategories={noopManage}
        productCounts={{ "cat-1": 2 }}
        totalProductsCount={3}
      />,
    );
    expect(html).toContain("Tous");
    expect(html).toContain("Plomberie");
    expect(html).toContain("Électricité");
    expect(html).not.toContain("OLD");
    expect(html).toContain("Gérer");
    expect(html).toContain('aria-pressed="true"');
  });

  it("marks the selected pill and renders Arabic labels", () => {
    const html = renderToStaticMarkup(
      <CategoryPillStrip
        locale="ar-DZ"
        categories={rows}
        selectedCategoryId="cat-2"
        onSelectCategory={noopSelect}
        onManageCategories={noopManage}
        totalProductsCount={3}
      />,
    );
    expect(html).toContain("الكل");
    expect(html).toContain("الكهرباء");
    expect(html).toContain("إدارة الأقسام");
  });
});

describe("CatalogHubView category column", () => {
  const noop = (): void => undefined;
  const noopEdit = (): void => undefined;

  it("renders the Rayon header and badges, with an em dash when unassigned", () => {
    const html = renderToStaticMarkup(
      <CatalogHubView
        locale="fr-DZ"
        products={[product({ category_id: "cat-1" }), product({ product_id: "p-2", sku: "NU" })]}
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
        kpis={{ total: 2, inStock: 0, outOfStock: 0, inactive: 0 }}
        sort={null}
        onFilter={noopEdit}
        onSort={noopEdit}
        categories={[category({})]}
        selectedCategoryId={null}
        onSelectCategory={noopEdit}
      />,
    );
    expect(html).toContain("Rayon");
    expect(html).toContain("Plomberie");
    expect(html).toContain("—");
  });
});

describe("category product payloads", () => {
  it("saves category_id on creation when a rayon is selected", () => {
    const wire = buildModalWire(
      {
        sku: "VIS-8X40",
        barcode: "",
        nameAr: "",
        nameFr: "Vis 8x40",
        preset: "PCS",
        customAr: "",
        customFr: "",
        salePrice: "",
        costPrice: "",
        wholesalePrice: "",
        initialStock: "",
        printAfter: true,
      },
      "cat-1",
    );
    expect(wire.category_id).toBe("cat-1");
  });

  it("stays unassigned on creation without a rayon", () => {
    const wire = buildModalWire({
      sku: "VIS-8X40",
      barcode: "",
      nameAr: "",
      nameFr: "Vis 8x40",
      preset: "PCS",
      customAr: "",
      customFr: "",
      salePrice: "",
      costPrice: "",
      wholesalePrice: "",
      initialStock: "",
      printAfter: true,
    });
    expect(wire.category_id).toBeNull();
  });

  it("reassigns or clears the category on update", () => {
    const base = product({ category_id: "cat-9" });
    const reassigned = buildUpdatePayload("t", base, {
      ...formFromProduct(base),
      categoryId: "cat-1",
    });
    expect(reassigned.category_id).toBe("cat-1");
    const cleared = buildUpdatePayload("t", base, {
      ...formFromProduct(base),
      categoryId: "",
    });
    expect(cleared.category_id).toBeNull();
  });
});

describe("ProductEditModalView category selector", () => {
  it("pre-selects the current rayon and lists active categories", () => {
    const html = renderToStaticMarkup(
      <ProductEditModalView
        product={product({ category_id: "cat-1" })}
        form={formFromProduct(product({ category_id: "cat-1" }))}
        locale="fr-DZ"
        attempted={false}
        busy={false}
        submitError={null}
        cardRef={{ current: null }}
        onPatch={() => undefined}
        onClose={() => undefined}
        onSubmit={() => undefined}
        categories={[category({}), category({ id: "cat-2", code: "ELEC" })]}
        categoryId="cat-1"
        onCategory={() => undefined}
      />,
    );
    expect(html).toContain("Rayon");
    expect(html).toContain("— Aucun rayon —");
    expect(html).toContain("Plomberie");
  });
});

describe("starter pack seeding (bridge)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const globals = globalThis as unknown as Record<string, unknown>;
    globals.window = globalThis;
    globals.__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    const globals = globalThis as unknown as Record<string, unknown>;
    delete globals.window;
    delete globals.__TAURI_INTERNALS__;
    useSessionStore.setState({ session: null });
  });

  it("resolves the seven seeded rows via category_seed_starter", async () => {
    const session: SessionInfo = {
      token: "tok-seed-1",
      userId: "55555555-5555-5555-5555-555555555555",
      username: "gerant1",
      role: "GERANT",
      expiresAt: "2026-01-01T00:00:00Z",
    };
    useSessionStore.setState({ session });
    const seeded = Array.from({ length: 7 }, (_, index) =>
      category({ id: `seed-${index}`, code: `RAYON-${index}` }),
    );
    invokeMock.mockResolvedValueOnce(seeded);
    const rows = await ipcSeedStarterCategories("tok-seed-1");
    expect(rows).toHaveLength(7);
    expect(invokeMock).toHaveBeenCalledWith("category_seed_starter", {
      session_token: "tok-seed-1",
    });
  });
});

describe("category wiring (source pins)", () => {
  const root = process.cwd();
  const hubSrc = readFileSync(
    join(root, "src", "features", "catalog", "CatalogModule.tsx"),
    "utf8",
  ) as string;
  const manageSrc = readFileSync(
    join(root, "src", "features", "catalog", "CategoryManageModal.tsx"),
    "utf8",
  ) as string;
  const createSrc = readFileSync(
    join(root, "src", "features", "catalog", "ProductCreateModal.tsx"),
    "utf8",
  ) as string;
  const editSrc = readFileSync(
    join(root, "src", "features", "catalog", "ProductEditModal.tsx"),
    "utf8",
  ) as string;

  it("loads the taxonomy on hub mount and filters rows by pill", () => {
    expect(hubSrc).toContain("ipcListCategories(activeToken, true)");
    expect(hubSrc).toContain("filterByCategory(");
    expect(hubSrc).toContain("selectedCategoryId");
    expect(hubSrc).toContain("<CategoryPillStrip");
    // DSN-053: config surfaces live in Settings — the hub keeps a pure
    // filter strip with no manage trigger and no modal mount.
    expect(hubSrc).not.toContain("<CategoryManageModal");
    expect(hubSrc).not.toContain("isCategoryModalOpen");
    expect(hubSrc).not.toContain("onManageCategories");
    expect(hubSrc).not.toContain("<DataExchangeMenu");
    expect(hubSrc).not.toContain("StarterPackBanner");
    expect(manageSrc).toContain("CategoryManagePanel");
  });

  it("seeds the starter pack and refreshes the list in place", () => {
    expect(manageSrc).toContain("ipcSeedStarterCategories(token)");
    expect(manageSrc).toContain("onClick={seedStarter}");
    expect(manageSrc).toContain("t(locale, \"cat.installStarter\")");
    expect(manageSrc).toContain("ipcCreateCategory(token,");
    expect(manageSrc).toContain("ipcUpdateCategory(token,");
    expect(manageSrc).toContain("is_active: !category.is_active");
  });

  it("creates categories with an uppercased code and bilingual names", () => {
    expect(manageSrc).toContain("normalizeCategoryCode(code)");
    expect(manageSrc).toContain("name_fr: nameFr.trim()");
    expect(manageSrc).toContain("name_ar: nameAr.trim()");
  });

  it("selects the rayon in both product modals and saves category_id", () => {
    expect(createSrc).toContain("ipcListCategories(token, false)");
    expect(createSrc).toContain("cat.noCategoryOption");
    expect(createSrc).toContain("buildModalWire(form, categoryId)");
    expect(editSrc).toContain("ipcListCategories(token, false)");
    expect(editSrc).toContain("product?.category_id ?? \"\"");
    expect(editSrc).toContain("categoryId");
  });
});
