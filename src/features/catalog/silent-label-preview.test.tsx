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
      { slot: "name1", text: "JILI ISPANA", x: 100, y: 16, cellH: 20 },
      { slot: "price", text: "350.00", x: 88, y: 44, cellH: 32 },
      { slot: "currency", text: "DZD", x: 280, y: 54, cellH: 12 },
      { slot: "human", text: "AR02", x: 144, y: 132, cellH: 12 },
    ],
    barcodeX: 81,
    barcodeY: 86,
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
    for (const text of ["JILI ISPANA", "350.00", "DZD", "AR02"]) {
      expect(html).toContain(text);
    }
    const order = ["JILI ISPANA", "350.00", "DZD", "AR02"].map((t) =>
      html.indexOf(t),
    );
    expect(order).toEqual([...order].sort((a, b) => a - b));
  });

  it("mirrors the centered redesign: no brand, centered rows, smaller price", () => {
    const html = renderToStaticMarkup(
      <SilentLabelPreviewView preview={enginePreview()} barcode="AR02" />,
    );
    expect(html).not.toContain("BARAKA QUINCAILLERIE");
    expect(html).toContain("text-[12px]");
    expect(html).not.toContain("text-[15px]");
    expect(html.match(/text-center/g)?.length).toBeGreaterThanOrEqual(3);
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
