/**
 * CSV import modal tests (TASK-DSN-045).
 *
 * Pure helpers (category lookup, preview slicing, money formatting) are
 * unit-tested directly; the preview table, report card, and drop-step modal
 * render via `renderToStaticMarkup` (node-only harness, no DOM library);
 * the `catalog_import_products` wire contract is pinned through a mocked
 * `invoke`; container wiring (hub button, modal mount, policy mapping) is
 * pinned via source-text assertions, per repo convention.
 */

import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { invoke } from "@tauri-apps/api/core";
import {
  buildCategoryLookup,
  categoryLabelFor,
  formatCentimes,
  formatMicros,
  ImportPreviewTable,
  ImportReportCard,
  previewRows,
  ProductImportModal,
} from "@/features/catalog/ProductImportModal";
import { parseCatalogCsv } from "@/features/catalog/catalog-import-parser";
import { ipcImportProducts } from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import { usePosUiStore } from "@/stores/pos-ui-store";
import type {
  Category,
  ImportProductRow,
  ImportProductsReport,
  SessionInfo,
} from "@/types/pos";

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

function row(overrides: Partial<ImportProductRow>): ImportProductRow {
  return {
    sku: "VIS-1",
    barcode: null,
    name_fr: "Vis Placo",
    name_ar: null,
    category_id: null,
    purchase_price_centimes: 10010,
    sale_price_centimes: 15020,
    initial_quantity_micro: 100_000,
    ...overrides,
  };
}

function report(overrides: Partial<ImportProductsReport>): ImportProductsReport {
  return {
    total_rows: 3,
    imported_count: 2,
    skipped_count: 1,
    errors: [{ row_index: 1, sku: "DUP-1", name_fr: "Doublon", error: "DUPLICATE" }],
    duration_ms: 42,
    ...overrides,
  };
}

function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function clearTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  delete globals.window;
  delete globals.__TAURI_INTERNALS__;
}

function primeSession(token: string): void {
  const session: SessionInfo = {
    token,
    userId: "55555555-5555-5555-5555-555555555555",
    username: "gerant1",
    role: "GERANT",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

describe("import pure helpers", () => {
  it("builds a code-or-id category lookup", () => {
    const lookup = buildCategoryLookup([category({ id: "uuid-1", code: "plomb" })]);
    expect(lookup.get("uuid-1")).toBe("uuid-1");
    expect(lookup.get("PLOMB")).toBe("uuid-1");
  });

  it("slices the first five rows for the preview", () => {
    const parsed = parseCatalogCsv(
      [
        "sku;name_fr;sale_price",
        "A;Vis;10",
        "B;Coude;20",
        "C;Robinet;30",
        "D;Cable;40",
        "E;Boulon;50",
        "F;Peinture;60",
        "G;Marteau;70",
      ].join("\n"),
    );
    expect(previewRows(parsed)).toHaveLength(5);
    expect(previewRows(parsed)[0]?.sku).toBe("A");
  });

  it("formats centimes and micros without floats", () => {
    expect(formatCentimes(15020)).toBe("150.20");
    expect(formatCentimes(null)).toBe("—");
    expect(formatMicros(500_000)).toBe("50");
    expect(formatMicros(25_000)).toBe("2.5");
    expect(formatMicros(undefined)).toBe("—");
  });

  it("labels row categories per locale", () => {
    const categories = [category({})];
    expect(categoryLabelFor(categories, "cat-1", "fr-DZ")).toBe("Plomberie");
    expect(categoryLabelFor(categories, "cat-1", "ar-DZ")).toBe("الترصيص");
    expect(categoryLabelFor(categories, null, "fr-DZ")).toBe("—");
  });
});

describe("ProductImportModal static rendering", () => {
  beforeEach(() => {
    usePosUiStore.setState({ locale: "fr-DZ" });
  });

  it("renders the file drop area and template button", () => {
    const html = renderToStaticMarkup(
      <ProductImportModal
        isOpen={true}
        token="tok"
        onClose={() => {}}
        onSuccess={() => {}}
        categories={[]}
      />,
    );
    expect(html).toContain("Glissez-déposez");
    expect(html).toContain("Télécharger le modèle CSV standard");
    expect(html).toContain(".csv");
  });

  it("renders the dry-run preview table with parsed rows", () => {
    const html = renderToStaticMarkup(
      <ImportPreviewTable
        rows={[row({}), row({ sku: "COU-1", name_fr: "Coude PVC" })]}
        categories={[category({})]}
        locale="fr-DZ"
      />,
    );
    expect(html).toContain("Vis Placo");
    expect(html).toContain("Coude PVC");
    expect(html).toContain("150.20");
  });

  it("renders the execution report with counts and duration", () => {
    const html = renderToStaticMarkup(
      <ImportReportCard report={report({})} locale="fr-DZ" />,
    );
    expect(html).toContain("42");
    expect(html).toContain("DUP-1");
    expect(html).toContain("importés");
    expect(html).toContain("ignorés (doublons)");
  });
});

describe("ipcImportProducts wire contract", () => {
  beforeEach(() => {
    fakeTauriWindow();
    invokeMock.mockReset();
  });
  afterEach(() => {
    clearTauriWindow();
    useSessionStore.setState({ session: null });
  });

  it("invokes catalog_import_products with the mapped payload", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      total_rows: 1,
      imported_count: 1,
      skipped_count: 0,
      errors: [],
      duration_ms: 7,
    });
    const result = await ipcImportProducts(
      "tok",
      [row({})],
      undefined,
      "error",
    );
    expect(invokeMock).toHaveBeenCalledWith("catalog_import_products", {
      request: {
        session_token: "tok",
        rows: [row({})],
        warehouse_id: null,
        on_duplicate_sku: "error",
      },
      session_token: "tok",
    });
    expect(result.imported_count).toBe(1);
    useSessionStore.setState({ session: null });
  });

  it("defaults the duplicate policy to skip", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      total_rows: 0,
      imported_count: 0,
      skipped_count: 0,
      errors: [],
      duration_ms: 1,
    });
    await ipcImportProducts("tok", []);
    expect(invokeMock).toHaveBeenCalledWith("catalog_import_products", {
      request: {
        session_token: "tok",
        rows: [],
        warehouse_id: null,
        on_duplicate_sku: "skip",
      },
      session_token: "tok",
    });
    useSessionStore.setState({ session: null });
  });
});

describe("import modal hub wiring (source pins)", () => {
  it("mounts the modal from the settings exchange tab", () => {
    const tab = readFileSync(
      join(process.cwd(), "src", "features", "settings", "DataExchangeSettingsTab.tsx"),
      "utf8",
    ) as string;
    expect(tab).toContain("ProductImportModal");
    expect(tab).toContain("onSuccess");
    expect(tab).toContain("buildCatalogCsvContent");
    expect(tab).toContain("downloadSampleCsvTemplate");
    const menu = readFileSync(
      join(process.cwd(), "src", "features", "catalog", "components", "DataExchangeMenu.tsx"),
      "utf8",
    ) as string;
    expect(menu).toContain("cat.exchangeImport");
    expect(menu).toContain("cat.exchangeExport");
  });

  it("maps the policy selector to the IPC payload", () => {
    const modal = readFileSync(
      join(process.cwd(), "src", "features", "catalog", "ProductImportModal.tsx"),
      "utf8",
    ) as string;
    expect(modal).toContain("ipcImportProducts(token, parsed.rows, undefined, policy)");
    expect(modal).toContain('value="skip"');
    expect(modal).toContain('value="error"');
  });
});
