/**
 * Unit tests for the topbar Operations menu (TASK-DSN-023, ADR-067).
 *
 * Role gating is verified through the pure `operationsForRole` map, action
 * wiring through the pure `runOperation` dispatcher with mocked deps, and
 * trigger/dropdown rendering through `renderToStaticMarkup` on the
 * presentational `OperationsMenuView` with explicit props (the repo harness
 * is node-only with no DOM library, and zustand v5 server snapshots always
 * render store initial state — verified by probe — so the store-connected
 * container itself is only a thin store-to-props bridge).
 */

import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  OperationsMenuView,
  operationsForRole,
  runOperation,
  type OperationDeps,
  type OperationId,
} from "@/components/pos/OperationsMenu";
import { t } from "@/lib/i18n";

function mockDeps(): OperationDeps & Record<string, ReturnType<typeof vi.fn>> {
  return {
    openPosModal: vi.fn(),
    openConversion: vi.fn(),
    openReturn: vi.fn(),
    openWarehouseTransfer: vi.fn(),
    openStockAdjust: vi.fn(),
    openBackup: vi.fn(),
  };
}

function allItems(role: string): OperationId[] {
  return operationsForRole(role).flatMap((section) => [...section.items]);
}

describe("OperationsMenuView rendering", () => {
  /** Static markup HTML-escapes `&`/`'` — mirror it before comparing. */
  function esc(value: string): string {
    return value.replace(/&/g, "&amp;").replace(/'/g, "&#x27;");
  }

  function renderView(role: string, defaultOpen = false): string {
    return renderToStaticMarkup(
      <OperationsMenuView
        role={role}
        locale="fr-DZ"
        defaultOpen={defaultOpen}
        onSelect={() => {}}
      />,
    );
  }

  it("renders the trigger button and stays closed by default", () => {
    const markup = renderView("CAISSIER");
    expect(markup).toContain(esc(t("fr-DZ", "topbar.operations")));
    // Closed: no dropdown item labels leak into the markup.
    expect(markup).not.toContain(esc(t("fr-DZ", "conversion.open")));
    expect(markup).not.toContain(esc(t("fr-DZ", "return.returnModalTitle")));
  });

  it("ADMIN open renders all 6 actions across 3 section headers", () => {
    const markup = renderView("ADMIN", true);
    expect(markup).toContain(esc(t("fr-DZ", "topbar.operationsDocuments")));
    expect(markup).toContain(esc(t("fr-DZ", "topbar.operationsStock")));
    expect(markup).toContain(esc(t("fr-DZ", "topbar.operationsSystem")));
    expect(markup).toContain(esc(t("fr-DZ", "conversion.open")));
    expect(markup).toContain(esc(t("fr-DZ", "return.returnModalTitle")));
    expect(markup).toContain(esc(t("fr-DZ", "warehousesTitle")));
    expect(markup).toContain(esc(t("fr-DZ", "stockAdjust.title")));
    expect(markup).toContain(esc(t("fr-DZ", "backups.title")));
    expect(markup).toContain(esc(t("fr-DZ", "settings.identity.title")));
  });

  it("CAISSIER open renders documents only, no stock or system", () => {
    const markup = renderView("CAISSIER", true);
    expect(markup).toContain(esc(t("fr-DZ", "topbar.operationsDocuments")));
    expect(markup).not.toContain(esc(t("fr-DZ", "topbar.operationsStock")));
    expect(markup).not.toContain(esc(t("fr-DZ", "topbar.operationsSystem")));
    expect(markup).toContain(esc(t("fr-DZ", "conversion.open")));
    expect(markup).toContain(esc(t("fr-DZ", "return.returnModalTitle")));
    expect(markup).not.toContain(esc(t("fr-DZ", "warehousesTitle")));
    expect(markup).not.toContain(esc(t("fr-DZ", "stockAdjust.title")));
    expect(markup).not.toContain(esc(t("fr-DZ", "backups.title")));
    expect(markup).not.toContain(esc(t("fr-DZ", "settings.identity.title")));
  });

  it("GERANT open renders stock actions but no system actions", () => {
    const markup = renderView("GERANT", true);
    expect(markup).toContain(esc(t("fr-DZ", "topbar.operationsStock")));
    expect(markup).not.toContain(esc(t("fr-DZ", "topbar.operationsSystem")));
    expect(markup).toContain(esc(t("fr-DZ", "warehousesTitle")));
    expect(markup).toContain(esc(t("fr-DZ", "stockAdjust.title")));
    expect(markup).not.toContain(esc(t("fr-DZ", "backups.title")));
    expect(markup).not.toContain(esc(t("fr-DZ", "settings.identity.title")));
  });
});

describe("operationsForRole", () => {
  it("CAISSIER sees only conversion and return (documents section)", () => {
    const sections = operationsForRole("CAISSIER");
    expect(sections).toHaveLength(1);
    expect(sections[0]?.key).toBe("documents");
    expect(allItems("CAISSIER")).toEqual(["conversion", "return"]);
    const items = allItems("CAISSIER");
    expect(items).not.toContain("warehouse-transfer");
    expect(items).not.toContain("stock-adjust");
    expect(items).not.toContain("backup");
    expect(items).not.toContain("identity");
  });

  it("VENDEUR sees only conversion and return (documents section)", () => {
    expect(allItems("VENDEUR")).toEqual(["conversion", "return"]);
  });

  it("GERANT sees documents plus stock, but no system section", () => {
    const sections = operationsForRole("GERANT");
    expect(sections.map((section) => section.key)).toEqual([
      "documents",
      "stock",
    ]);
    const items = allItems("GERANT");
    expect(items).toEqual([
      "conversion",
      "return",
      "warehouse-transfer",
      "stock-adjust",
    ]);
    expect(items).not.toContain("backup");
    expect(items).not.toContain("identity");
  });

  it("ADMIN sees all 6 actions across all 3 sections", () => {
    const sections = operationsForRole("ADMIN");
    expect(sections.map((section) => section.key)).toEqual([
      "documents",
      "stock",
      "system",
    ]);
    expect(allItems("ADMIN")).toEqual([
      "conversion",
      "return",
      "warehouse-transfer",
      "stock-adjust",
      "backup",
      "identity",
    ]);
  });
});

describe("runOperation dispatch", () => {
  it("routes every operation to its payload opener and pos-ui modal key", () => {
    const expectations: ReadonlyArray<{
      readonly id: OperationId;
      readonly opener: keyof OperationDeps;
      readonly modal: string;
    }> = [
      { id: "conversion", opener: "openConversion", modal: "conversion" },
      { id: "return", opener: "openReturn", modal: "return" },
      {
        id: "warehouse-transfer",
        opener: "openWarehouseTransfer",
        modal: "warehouse-transfer",
      },
      { id: "stock-adjust", opener: "openStockAdjust", modal: "stock-adjust" },
      { id: "backup", opener: "openBackup", modal: "backup" },
    ];
    for (const { id, opener, modal } of expectations) {
      const deps = mockDeps();
      runOperation(id, deps);
      expect(deps[opener]).toHaveBeenCalledTimes(1);
      expect(deps.openPosModal).toHaveBeenCalledTimes(1);
      expect(deps.openPosModal).toHaveBeenCalledWith(modal);
    }
  });

  it("identity dispatches openModal without any payload-store opener", () => {
    const deps = mockDeps();
    runOperation("identity", deps);
    expect(deps.openPosModal).toHaveBeenCalledTimes(1);
    expect(deps.openPosModal).toHaveBeenCalledWith("identity");
    expect(deps.openConversion).not.toHaveBeenCalled();
    expect(deps.openReturn).not.toHaveBeenCalled();
    expect(deps.openWarehouseTransfer).not.toHaveBeenCalled();
    expect(deps.openStockAdjust).not.toHaveBeenCalled();
    expect(deps.openBackup).not.toHaveBeenCalled();
  });
});
