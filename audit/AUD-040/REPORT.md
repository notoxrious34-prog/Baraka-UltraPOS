# AUD-040 REPORT — Label/barcode printing map + 39x20 support gap

## (1) Trace one-liner

Catalogue 🏷️ row button (`CatalogModule.tsx:632`, ungated) → `ProductLabelModal`
(XP-410B browser-print, presets 50x30/40x25/100x50) → zero-dep Code128-B SVG
(`barcode-128.ts`, fixed `moduleWidth: 2`) → hidden-iframe `window.print()` →
OS spooler. No IPC command, no engine renderer, no audit row, no EAN13 branch.

## (2) Dots-math rows (203 dpi = 7.992 dots/mm)

| Format | Full-bleed dots | Code assumption today | 39x20 need | Delta |
|--------|-----------------|----------------------|------------|-------|
| Receipt 80mm | 640 | 576 (`paper.rs:23`) | n/a (separate path) | — |
| Receipt 58mm | 464 | 384 (`paper.rs:24`) | n/a | — |
| Label 50x30 | 400 × 240 | fits only short codes (396 px SVG for 13 chars ≈ 105 mm) | — | already clips long codes |
| Label 40x25 | 320 × 200 | same fixed SVG, clipped | — | — |
| **Label 39x20** | **312 × 160** (printable ≈280 × 128 @2mm margins ≈132 CSS px) | **no preset, no layout** | 132 px budget vs 396 px barcode; 5 text lines + 56 px bars vs ~16 mm printable height | **−264 px width, −6-8 mm height** |

## (3) Hardcoded size constants (9)

`barcode-128.ts`: `CODE128_START_B=104`, `CODE128_STOP=106`,
`CODE128_QUIET_MODULES=10`, `moduleWidth: 2` / `height: 56` defaults
(`generateCode128Svg:201-202`, call sites `ProductLabelModal.tsx:81,158`);
`ProductLabelModal.tsx:22-28` preset table (50x30/40x25/100x50), fonts
8/9/12/8/8pt + `overflow:hidden` (`:85`), qty clamp 1..500 (`:31-43`);
receipt side (context only): `576/384` (`paper.rs:23-24`),
`MARGIN_DOTS=8.0` (`rasterizer.rs:28`), `42/32` cols (`paper.rs:31-32`).

## Proposal + size

- **S (frontend only)**: add `39x20` preset + compact sticker layout (drop shop
  line, 7/8/10pt fonts, bars height ~40) + `moduleWidth` prop threaded from
  format (1 for 39x20, 2 otherwise) + `width:100%` SVG scaling guard with a
  max-width cap so short codes don't blow up; pins: `@page 39mm 20mm`,
  sticker count, `moduleWidth` per preset, overflow assertion on a 13-char
  fixture.
- **M (if S fails scan tests)**: numeric-payload Code128-C pairing (halves
  digit runs; touches `barcode-128.ts` encoder + vectors) and/or SKU-shortening
  rule for labels.
- **Out of scope**: EAN13 symbology (no selection logic exists; master §18
  exceeds the 39x20 ask), engine/serial label path, audit rows for labels.

## Cannot be proven without the XP-410B hardware (owner gate)

Scannability at 1.4–2 dots/module, driver `@page 39x20` handling (spooler may
fall back to a default form), darkness/speed vs ribbon, peel gap calibration,
Arabic glyph shaping (Arial fallback) at 7pt, real clip behavior of
`overflow:hidden` vs driver scaling. All provable in-repo: layout math,
pins, screenshots of the designer — not a single successful scan.
