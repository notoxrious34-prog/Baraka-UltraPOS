/**
 * Engine-faithful label preview (DOC-03): the modal renders the backend
 * plan lines verbatim — engine spelling, spool order, no phantom unit.
 */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SilentLabelPreviewView } from "@/features/catalog/SilentLabelModal";
import type { TsplLabelPreview } from "@/lib/ipc";

function enginePreview(): TsplLabelPreview {
  return {
    lines: [
      { slot: "brand", text: "BARAKA QUINCAILLERIE", x: 72, y: 2, cellH: 12 },
      { slot: "name1", text: "JILI ISPANA", x: 8, y: 16, cellH: 20 },
      { slot: "price", text: "350.00", x: 8, y: 58, cellH: 48 },
      { slot: "currency", text: "DZD", x: 272, y: 94, cellH: 12 },
      { slot: "human", text: "AR02", x: 268, y: 146, cellH: 12 },
    ],
    barcodeX: 8,
    barcodeY: 106,
    barcodeWidth: 158,
    barcodeHeight: 38,
    jobLen: 361,
  };
}

describe("SilentLabelPreviewView (DOC-03)", () => {
  it("renders engine texts verbatim in spool order", () => {
    const html = renderToStaticMarkup(
      <SilentLabelPreviewView preview={enginePreview()} barcode="AR02" />,
    );
    for (const text of ["BARAKA QUINCAILLERIE", "JILI ISPANA", "350.00", "DZD", "AR02"]) {
      expect(html).toContain(text);
    }
    const order = ["BARAKA QUINCAILLERIE", "JILI ISPANA", "350.00", "DZD", "AR02"].map((t) =>
      html.indexOf(t),
    );
    expect(order).toEqual([...order].sort((a, b) => a - b));
  });

  it("uses the engine price spelling, never the locale one", () => {
    const html = renderToStaticMarkup(
      <SilentLabelPreviewView preview={enginePreview()} barcode="AR02" />,
    );
    expect(html).toContain("350.00");
    expect(html).not.toContain("350,00");
  });

  it("never shows the unit — the engine never prints it", () => {
    const html = renderToStaticMarkup(
      <SilentLabelPreviewView preview={enginePreview()} barcode="AR02" />,
    );
    expect(html).not.toContain("Pièce");
    expect(html).not.toContain("PCS");
  });

  it("sizes the barcode band from the engine geometry", () => {
    const html = renderToStaticMarkup(
      <SilentLabelPreviewView preview={enginePreview()} barcode="AR02" />,
    );
    // 158/320 dots → 49.375%.
    expect(html).toContain("49.375%");
  });
});
