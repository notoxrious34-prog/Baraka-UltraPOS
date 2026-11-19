/**
 * Engine-faithful studio preview (DSN-107, DOC-03): the modal renders the
 * backend plan lines verbatim, with the Code128 band inserted where the
 * engine puts it — textile bars sit below every line (band last),
 * hardware hero bars sit above every line (band first).
 */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { StudioLabelPreviewView } from "@/features/catalog/StudioLabelModal";
import type { TsplLabelPreview } from "@/lib/ipc";

function textilePreview(): TsplLabelPreview {
  return {
    lines: [
      { slot: "studio_model", text: "COTON ML", x: 16, y: 16, cellH: 20 },
      { slot: "studio_size", text: "XL", x: 74, y: 40, cellH: 48 },
      { slot: "studio_color", text: "NOIR", x: 196, y: 44, cellH: 12 },
      { slot: "studio_pack", text: "PACK 6 PCS", x: 196, y: 60, cellH: 12 },
      { slot: "studio_price", text: "250.00 DZD", x: 16, y: 92, cellH: 32 },
    ],
    barcodeX: 59,
    barcodeY: 126,
    barcodeWidth: 202,
    barcodeHeight: 28,
    jobLen: 402,
  };
}

function hardwarePreview(): TsplLabelPreview {
  return {
    lines: [
      { slot: "studio_hw_name", text: "CHEVILLE MOLLY M8", x: 16, y: 88, cellH: 20 },
      { slot: "studio_hw_dim", text: "PCS · O8X32MM", x: 16, y: 110, cellH: 12 },
      { slot: "studio_hw_price", text: "45.00 DZD", x: 16, y: 124, cellH: 20 },
      { slot: "human", text: "HW0001", x: 16, y: 74, cellH: 12 },
    ],
    barcodeX: 59,
    barcodeY: 16,
    barcodeWidth: 202,
    barcodeHeight: 56,
    jobLen: 388,
  };
}

describe("StudioLabelPreviewView (DOC-03)", () => {
  it("renders textile lines verbatim with the band last (bars y126)", () => {
    const html = renderToStaticMarkup(
      <StudioLabelPreviewView preview={textilePreview()} barcode="TXL001" />,
    );
    for (const text of ["COTON ML", "XL", "NOIR", "PACK 6 PCS", "250.00 DZD"]) {
      expect(html).toContain(text);
    }
    const bandAt = html.indexOf("<svg");
    expect(bandAt).toBeGreaterThan(-1);
    // Every TEXT line precedes the band: bars close the label.
    for (const text of ["COTON ML", "250.00 DZD"]) {
      expect(html.indexOf(text)).toBeLessThan(bandAt);
    }
  });

  it("renders hardware with the hero band first (bars y16)", () => {
    const html = renderToStaticMarkup(
      <StudioLabelPreviewView preview={hardwarePreview()} barcode="HW0001" />,
    );
    for (const text of ["CHEVILLE MOLLY M8", "PCS · O8X32MM", "45.00 DZD", "HW0001"]) {
      expect(html).toContain(text);
    }
    const bandAt = html.indexOf("<svg");
    expect(bandAt).toBeGreaterThan(-1);
    // The hero bars open the label: every TEXT line follows the band.
    for (const text of ["CHEVILLE MOLLY M8", "HW0001"]) {
      expect(html.indexOf(text)).toBeGreaterThan(bandAt);
    }
  });

  it("uses the engine price spelling, never the locale one", () => {
    const html = renderToStaticMarkup(
      <StudioLabelPreviewView preview={textilePreview()} barcode="TXL001" />,
    );
    expect(html).toContain("250.00 DZD");
    expect(html).not.toContain("250,00");
  });

  it("renders the giant size badge, never a unit line", () => {
    const html = renderToStaticMarkup(
      <StudioLabelPreviewView preview={textilePreview()} barcode="TXL001" />,
    );
    expect(html).toContain("text-[26px]");
    expect(html).not.toContain("/ Pièce");
  });
});
