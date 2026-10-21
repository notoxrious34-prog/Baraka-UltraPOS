# AUD-040 TRACE — Barcode/label printing path, end to end

## L1. UI affordance (CatalogModule row, always enabled)

- `src/features/catalog/CatalogModule.tsx:632-644` — 🏷️ button per product row,
  `aria-label={t(locale, "cat.printLabel")}` (`Imprimer étiquette` / `طباعة ملصق`,
  `src/lib/i18n.ts:872,1929`). No `disabled={!canManage}` (unlike edit/delete
  beside it) — DSN-077 deliberately left label printing ungated.
- Click → `props.onPrintLabel(product)` → `setLabelProduct` (`:978`) →
  `ProductLabelModal product={labelProduct}` (`:1000-1009`). Also fired from
  `ProductCreateModal` via `onCreated(product, printLabel)` (`:994-996`).

## L2. Label designer (pure frontend, zero IPC)

- `src/features/catalog/ProductLabelModal.tsx` (DSN-039, XP-410B 203 DPI):
  - Presets `LabelFormat` (`:20-28`): `50x30`, `40x25`, `100x50` — **no 39x20**.
  - `labelBarcodeSource` (`:46-52`): product `barcode`, else SKU fallback.
  - `labelPriceUnit` (`:59-66`): base-unit sale price + unit name.
  - Qty stepper `clampLabelQuantity` 1..500 (`:31-43`).
  - `buildLabelPrintHtml` (`:77-86`): one `.sticker` div per copy,
    `@page { size: Wmm Hmm; margin: 0; }`, fonts 8/9/12pt, `overflow: hidden`.
  - `printLabelDocument` (`:96-129`): hidden iframe → `doc.write` → `win.print()`
    → OS spooler → label-printer driver. `window.print()`, **not** Tauri IPC:
    no `print_jobs` row, no PENDING job, no worker, no audit event.
  - Preview (`ProductLabelModalView`, `:155-259`): aspect-ratio box, same
    fixed-width SVG via `dangerouslySetInnerHTML`, `cat.labelPreview`
    `Aperçu de l'étiquette (Xprinter 203 DPI)` (`i18n.ts:876,1933`).

## L3. Barcode engine (frontend, no EAN13 branch)

- `src/lib/barcode-128.ts` (233 lines, zero-dep): Code 128 **Subset B only**,
  Mod-103 checksum, quiet zone 10 modules/side (`CODE128_QUIET_MODULES`, `:18`),
  pattern table 0..106 (`PATTERNS`), ASCII 32..127 enforced (throws otherwise).
- `generateCode128Svg(text, { moduleWidth: 2, height: 56 })` (`:199-233`):
  fixed `width="${totalWidth}"` + `viewBox` (`:232`) — no `width:100%`, so the
  SVG prints at `totalWidth` CSS px = `totalWidth/96` inch, clipped by the
  sticker's `overflow: hidden`.
- Module budget: `code128ModuleCount(n) = 11*(n+2)+13` (`:132-134`) + 20 quiet
  → total `11n+55` modules; example 13-char code → 198 modules → **396 px**
  (≈104.8 mm) at `moduleWidth: 2`.
- No EAN13/GTIN selection logic exists anywhere in `src/` (only hit is an
  unrelated starter-pack test fixture). Master §18's "EAN13 commercial" is
  aspirational: a 13-digit payload today encodes as 13 Subset-B symbols, never
  as compact Code128-C pairs.
- Tests: `src/lib/barcode-128.test.ts` (encode vectors incl. Mod-103
  `[104,48,16,16,17,94,106]`, module-sum, reject rules — no print goldens);
  `src/features/catalog/product-label-modal.test.tsx` (`@page` size per preset,
  sticker-per-copy count, render smoke — no `.snap`/golden files repo-wide).

## L4. What the label path does NOT touch (contrast: receipt path)

- Receipt thermal path: `PaperWidth` enum (`crates/domain/src/printing/
  paper.rs:12-26`, 80mm→576 / 58mm→384 dots, `MARGIN_DOTS = 8.0` in
  `crates/infrastructure/src/printing/rasterizer.rs:28,34,199,224`);
  `dot_width()` consumed by `sqlite_printer_settings.rs:64,86`,
  worker via `print_worker_service.rs:154`; persisted `paper_width_mm`
  validated to 58|80 (`printing.rs:414-438`, `paper.rs:57-63`); UI selector
  `PrinterSettingsTab.tsx:32,142-151` (no label-printer section — serial/COM +
  paper 58/80 + test print only).
- Labels bypass ALL of it: no dots math in code (CSS mm only), no settings
  persistence, no serial transport, no test-print, no error surfacing.

## L5. 39x20 projection (203 dpi = 7.992 dots/mm ≈ 8)

- Full-bleed: 39mm → **312 dots**; 20mm → **160 dots**.
- With 2 mm/side printer margins: printable ≈ 35mm → **~280 dots** ≈ **132 CSS px**.
- Current smallest preset 40x25 at `moduleWidth: 2`: 13-char code = 396 px ≫
  132 px → hard clip today (even 50x30 clips long codes). Height 56px bars +
  12pt price + caption ≈ fits 20mm only after font cuts (12pt ≈ 4.2mm/line;
  5 lines + bars ≈ 22-24mm > 20mm printable ~16mm).
