/**
 * Canvas raster glue (BRK-101 — thin, no invention).
 *
 * Turns the EXISTING print DOM into device pixels at EXACT printer geometry:
 * - ticket: `buildTicketPrintHtml` output at 640 dots wide, height measured;
 * - label: `buildLabelRasterDom` fixed 320x160 frame mirroring the
 *   SilentLabelModal preview content order (name / price / barcode / code).
 *
 * Path: offscreen host → SVG `foreignObject` (self-contained: system fonts
 * only — Arial / Courier New, zero webfonts, zero external resources) →
 * `Image` → 2d canvas (`imageSmoothingEnabled = false`) → `getImageData`
 * (taint caught → `BITMAP_CANVAS_TAINTED`) → `packBitmapRgba` (pure).
 *
 * Smoothing off keeps barcodes pixel-exact. No console output anywhere.
 */

import { packBitmapRgba, type PackedBitmap } from "./bitmap-pack";

/** Exact device geometry (dots @203 dpi). */
export const BITMAP_LABEL_WIDTH_DOTS = 320;
export const BITMAP_LABEL_HEIGHT_DOTS = 160;
export const BITMAP_TICKET_WIDTH_DOTS = 640;
/** Mirrors the domain `BITMAP_MAX_HEIGHT_DOTS` cap (fail fast, same code). */
export const BITMAP_MAX_RASTER_ROWS = 1638;

export interface RasterFrame extends PackedBitmap {
  /** Alias kept for call-site readability (dots, multiple of 8). */
  widthDots: number;
}

/** Load an SVG document into an image (decode errors → explicit code). */
function loadSvgImage(svg: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      resolve(img);
    };
    img.onerror = () => {
      reject(new Error("BITMAP_RASTERIZE_FAILED"));
    };
    // data: URL (NOT a blob URL): Chromium treats blob-URL SVG images
    // containing foreignObject as cross-origin and taints the canvas on
    // draw; data: URLs stay origin-clean (pinned in-capture 2026-10-31).
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  });
}

/**
 * Rasterize an attached element at exact device pixels.
 * The element must carry its own inline/<style> CSS (no external sheets).
 */
export async function rasterizeElement(
  element: HTMLElement,
  widthDots: number,
  heightDots: number,
): Promise<RasterFrame> {
  if (widthDots % 8 !== 0) {
    throw new Error("BITMAP_PACK_WIDTH_NOT_BYTE_ALIGNED");
  }
  if (heightDots <= 0 || heightDots > BITMAP_MAX_RASTER_ROWS) {
    throw new Error("BITMAP_TOO_TALL");
  }
  const wrapper =
    `<div xmlns="http://www.w3.org/1999/xhtml" style="margin:0;padding:0;width:${widthDots}px;">` +
    // XMLSerializer (not outerHTML): HTML serialization would re-emit void
    // elements (`<br>`) that are fatal inside an SVG image; the XML
    // serializer self-closes them (`<br/>`).
    `${new XMLSerializer().serializeToString(element)}</div>`;
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${widthDots}" height="${heightDots}">` +
    `<foreignObject x="0" y="0" width="${widthDots}" height="${heightDots}">${wrapper}</foreignObject></svg>`;
  const img = await loadSvgImage(svg);
  const canvas = document.createElement("canvas");
  canvas.width = widthDots;
  canvas.height = heightDots;
  const ctx = canvas.getContext("2d");
  if (ctx === null) {
    throw new Error("BITMAP_RASTERIZE_FAILED");
  }
  ctx.imageSmoothingEnabled = false;
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, widthDots, heightDots);
  ctx.drawImage(img, 0, 0, widthDots, heightDots);
  let data: ImageData;
  try {
    data = ctx.getImageData(0, 0, widthDots, heightDots);
  } catch {
    throw new Error("BITMAP_CANVAS_TAINTED");
  }
  const packed = packBitmapRgba(data.data, widthDots, heightDots);
  return { ...packed, widthDots };
}

/** Split a standalone ticket document into its <style> and <body> parts. */
function splitTicketDocument(html: string): { style: string; body: string } {
  const styleStart = html.indexOf("<style>");
  const styleEnd = html.indexOf("</style>");
  const bodyStart = html.indexOf("<body>");
  const bodyEnd = html.indexOf("</body>");
  if (styleStart < 0 || styleEnd < 0 || bodyStart < 0 || bodyEnd < 0) {
    throw new Error("BITMAP_RASTERIZE_FAILED");
  }
  return {
    style: html.slice(styleStart + "<style>".length, styleEnd),
    body: html
      .slice(bodyStart + "<body>".length, bodyEnd)
      // XHTML repair: the template emits HTML void elements.
      .replace(/<br>/g, "<br/>"),
  };
}

/**
 * Rasterize a `buildTicketPrintHtml` document at 640 dots wide.
 * Height is measured from layout (offsetHeight at 1:1 CSS px == dots).
 */
export async function rasterizeTicketHtml(html: string): Promise<RasterFrame> {
  const { style, body } = splitTicketDocument(html);
  const host = document.createElement("div");
  host.setAttribute(
    "style",
    "margin:0;padding:0;width:640px;background:#ffffff;color:#000000;overflow:hidden;",
  );
  const styleEl = document.createElement("style");
  styleEl.textContent = style;
  host.appendChild(styleEl);
  const content = document.createElement("div");
  content.setAttribute("xmlns", "http://www.w3.org/1999/xhtml");
  content.innerHTML = body;
  host.appendChild(content);
  document.body.appendChild(host);
  try {
    const heightDots = host.offsetHeight;
    if (heightDots <= 0 || heightDots > BITMAP_MAX_RASTER_ROWS) {
      throw new Error("BITMAP_TOO_TALL");
    }
    return await rasterizeElement(host, BITMAP_TICKET_WIDTH_DOTS, heightDots);
  } finally {
    host.remove();
  }
}

/** Label raster source (content order mirrors the modal preview). */
export interface LabelRasterInput {
  name: string;
  priceText: string;
  /** Inline Code128 SVG (existing generator, no external refs). */
  barcodeSvg: string;
  humanCode: string;
}

/**
 * Build the fixed 320x160 label frame (name / price / barcode / code).
 * Inline styles only — the frame must render identically inside
 * `foreignObject` with zero stylesheets.
 */
export function buildLabelRasterDom(input: LabelRasterInput): HTMLElement {
  const frame = document.createElement("div");
  frame.setAttribute(
    "style",
    "margin:0;padding:4px 8px;width:320px;height:160px;background:#ffffff;color:#000000;" +
      "font-family:Arial,sans-serif;text-align:center;overflow:hidden;box-sizing:border-box;",
  );
  const name = document.createElement("p");
  name.setAttribute(
    "style",
    "margin:0;font-size:11px;font-weight:800;white-space:nowrap;overflow:hidden;",
  );
  name.textContent = input.name;
  const price = document.createElement("p");
  price.setAttribute("style", "margin:0;font-size:13px;font-weight:900;");
  price.textContent = input.priceText;
  const band = document.createElement("div");
  band.setAttribute("style", "margin:0;width:304px;");
  band.innerHTML = input.barcodeSvg;
  const code = document.createElement("p");
  // Human code at 12px bold: 10px Courier strokes fall entirely under the
  // 1-bit threshold (measured 0.00% dark — content lost), so the raster
  // carries the code one step larger than screen. Lock-in should unify.
  code.setAttribute("style", "margin:0;font-size:12px;font-weight:700;font-family:'Courier New',monospace;");
  code.textContent = input.humanCode;
  frame.append(name, price, band, code);
  return frame;
}
