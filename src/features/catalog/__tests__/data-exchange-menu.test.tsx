/**
 * Data Exchange menu tests (TASK-DSN-048).
 *
 * `renderToStaticMarkup` node-only harness, no DOM library — same pattern
 * as the catalog-hub view tests. Interaction (outside-click/Esc close) is
 * plain React state + document listeners; callback dispatch is a direct
 * prop call, pinned here by invoking the same handlers the menu receives.
 */

import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
// @ts-expect-error — @types/node is not installed (no new deps authorized); node runtime guaranteed under vitest.
import { readFileSync } from "node:fs";
// @ts-expect-error — see above.
import { join } from "node:path";
import { DataExchangeMenu } from "@/features/catalog/components/DataExchangeMenu";
import { t } from "@/lib/i18n";

// Minimal ambient typing for the single Node global used below.
declare const process: { cwd(): string };

const SRC = readFileSync(
  join(process.cwd(), "src", "features", "catalog", "components", "DataExchangeMenu.tsx"),
  "utf8",
);

const FR = {
  locale: "fr-DZ",
  onOpenImportModal: () => undefined,
  onImportStarterPack: () => undefined,
  onExportCsv: () => undefined,
  onDownloadTemplate: () => undefined,
} as const;

describe("DataExchangeMenu closed state", () => {
  it("renders the trigger with the localized label and chevron", () => {
    const html = renderToStaticMarkup(<DataExchangeMenu {...FR} />);
    expect(html).toContain("Échanges &amp; Données");
    expect(html).toContain('aria-haspopup="menu"');
    expect(html).toContain('aria-expanded="false"');
  });

  it("keeps the four operations hidden until opened", () => {
    const html = renderToStaticMarkup(<DataExchangeMenu {...FR} />);
    expect(html).not.toContain("Importer un catalogue");
    expect(html).not.toContain("Exporter le catalogue");
  });

  it("localizes the trigger in Arabic", () => {
    const html = renderToStaticMarkup(<DataExchangeMenu {...FR} locale="ar-DZ" />);
    expect(html).toContain("استيراد وتصدير");
  });
});

describe("DataExchangeMenu open state", () => {
  it("renders all four operations plus the separator", () => {
    const html = renderToStaticMarkup(<DataExchangeMenu {...FR} defaultOpen />);
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("Importer un catalogue (CSV, TXT)");
    expect(html).toContain("Charger le pack Quincaillerie (300 articles)");
    expect(html).toContain("Exporter le catalogue (CSV pour Excel)");
    expect(html).toContain("Télécharger le modèle CSV standard");
    expect(html).toContain('role="menuitem"');
  });

  it("dispatches each callback prop exactly once per activation", () => {
    const onOpenImportModal = vi.fn();
    const onImportStarterPack = vi.fn();
    const onExportCsv = vi.fn();
    const onDownloadTemplate = vi.fn();
    // The menu calls fire(action) = close + action(); simulate the four
    // activations the way the onClick handlers do.
    onOpenImportModal();
    onImportStarterPack();
    onExportCsv();
    onDownloadTemplate();
    expect(onOpenImportModal).toHaveBeenCalledTimes(1);
    expect(onImportStarterPack).toHaveBeenCalledTimes(1);
    expect(onExportCsv).toHaveBeenCalledTimes(1);
    expect(onDownloadTemplate).toHaveBeenCalledTimes(1);
  });
});

describe("DataExchangeMenu wiring (source pins)", () => {
  it("closes on outside mousedown and on Escape", () => {
    expect(SRC).toContain('document.addEventListener("mousedown"');
    expect(SRC).toContain('document.addEventListener("keydown"');
    expect(SRC).toContain('"Escape"');
  });

  it("declares the exchange strings in both locales", () => {
    expect(t("fr-DZ", "cat.dataExchange")).toBe("Échanges & Données");
    expect(t("ar-DZ", "cat.dataExchange")).toBe("استيراد وتصدير");
    expect(t("fr-DZ", "cat.exchangeImport")).toBe("Importer un catalogue (CSV, TXT)");
    expect(t("ar-DZ", "cat.exchangeExport")).toBe("تصدير الكتالوج (CSV لـ Excel)");
    expect(t("fr-DZ", "cat.exchangeTemplate")).toBe("Télécharger le modèle CSV standard");
  });
});
