# TASK-BRK-101 REPORT — BITMAP silent-print pipeline (raster only)

## 1. FAIL-before

- Owner verdict on trial 6 (574d363): ALL-RED on P1–P3 — 4 TEXT modes
  (normal/no_codepage/ascii_only/font1_only) x label + ticket, every cell
  BLANK. TEXT/BARCODE TSPL is rejected regardless of CODEPAGE, font, or
  payload. The E-matrix (E1 yes / E2 no / E3 yes / E4 no) proves the driver
  raster path always prints.
- Codebase: no `render_bitmap` existed. `grep BITMAP tspl.rs` returned a
  single comment-only line (the AUD-047 deferred note). `ports.rs:1483-1485`
  already had `PrinterTransportPort::send_raster(&[u8])` (opaque bytes) —
  NO transport change needed.

## 2. Pipeline + byte math

- Domain pure fn `render_bitmap_job(media, x, y, w_bytes, h_rows, data,
  copies)` (`crates/domain/src/printing/tspl.rs`): preamble
  SIZE -> GAP -> DENSITY -> SPEED -> DIRECTION -> CLS (**NO CODEPAGE —
  zero text bytes anywhere**), then
  `BITMAP x,y,w,h,0,<raw binary>`, then `PRINT 1,n`. SIZE derived from
  raster geometry by integer math (width_bytes -> mm; height
  div_ceil(rows/8) -> mm). No floats. Refuses, never truncates:
  `len==w*h`, `w*8<=640`, `h<=1638`, `total<=131072`, `1<=copies<=99`,
  rejects all-`0x00`/all-`0xFF` frames (anti-blank guard).
- App: `PrintRawBitmapUseCase` (`tspl_printing.rs`) mirrors the
  `print_tspl_*` seam (POS-sell auth + spooler-transport required;
  probe switch ignored — bitmap has no text) -> existing `send_raster`.
- Commands: `RawBitmapRequest { session_token, pixels: Vec<u8>,
  width_bytes/height_rows/copies: u32 }` (dto.rs) + `print_raw_bitmap`
  Tauri command (ipc.rs, registered in `src-tauri/src/lib.rs`).
  `pixels` rides a JSON number array; caps enforced pre-send.
- Frontend pure packer `src/lib/bitmap-pack.ts` (RGBA -> 1-bit MSB-first,
  dark = `r+g+b<384` integer-only, blank guard, `BITMAP_PACK_*` codes) +
  thin glue `src/lib/bitmap-capture.ts`: ticket via measured
  `buildTicketPrintHtml` DOM at 640 dots; label via fixed 320x160
  `buildLabelRasterDom` (name/price/barcode/code, mirrors modal order).
  Path: offscreen host -> SVG `foreignObject` -> `Image` -> 2d canvas
  (`imageSmoothingEnabled=false`, system fonts only, zero webfonts) ->
  `getImageData` (taint caught -> `BITMAP_CANVAS_TAINTED`) -> packer.
- Facade: `ipcPrintRawBitmap(sessionToken, pixels, wBytes, hRows,
  copies)` (`src/lib/ipc.ts`).
- Byte math (posted): label 40x160 dots = 40x160 B = **6,400 B**;
  ticket 80x672 (84 mm) = 80x672 B = **53,760 B (~52.5 KiB)**;
  max frame 80x1638 = **131,040 B <= 128 KiB** — USB-trivial.
- Capture-forced findings (harness-only, production code unaffected
  except where stated):
  (a) `outerHTML` re-emits `<br>` which is fatal inside an SVG image ->
      glue uses `XMLSerializer` (self-closes voids). Kept in production
      file with comment.
  (b) Chromium taints canvas on `foreignObject` SVG drawn from a **blob**
      URL (`SecurityError`, pinned in-browser) but NOT from a **data:**
      URL -> glue uses
      `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`.
      Pinned in code comment 2026-10-31.
  (c) Label human code at 10px Courier measured **0.00% dark** after
      thresholding (content lost) -> raster builder carries the code at
      **12px bold** (one step larger than screen; `buildLabelRasterDom`
      only). Lock-in should unify modal + raster at 12px.

## 3. Pin lines

- Domain `printing::tspl`: **28 passed / 0 failed** incl. 4 BRK-101 pins:
  2-byte fixture byte-exact
  (`SIZE 2 mm,1 mm...BITMAP 0,0,2,1,0,<AA 55>...PRINT 1,1`, CODEPAGE
  absent); probe-switch-ignored (3 modes -> identical bytes); refuses
  (length/zero-dim/too-wide/too-tall/blank x2/copies 0+100/neg-origin);
  geometry (label `SIZE 40 mm,20 mm` + `PRINT 1,2`; ticket
  `SIZE 80 mm,84 mm`).
- App: `raw_bitmap_spools_validated_bytes_and_rejects_before_send`
  **1 passed** (valid frame spooled, `SIZE 2 mm,1 mm`, no CODEPAGE;
  oversize + blank rejected pre-send, transport untouched).
- Pack: `bitmap-pack.test.ts` **7 passed / 0 failed** (checkerboard
  `AA 55`, threshold 127-dark/128-white, alpha-ignored, 4 guards).
- G3 workspace total: **71 suites, 587 passed, 0 failed** (582 + 4
  domain + 1 app). G7 total: **86 files, 889 passed**
  (882 + 7 pack).

## 4. G-exits + counts

- G1 `cargo fmt --check`: EXIT 0 (after `cargo fmt --all`).
- G2 `cargo clippy --all-targets --all-features -- -D warnings`: EXIT 0
  (5 self-hits fixed during C1: `manual_range_contains`,
  `manual_div_ceil`, `assertions_on_constants`, 2x `useless_vec`).
- G3 `cargo test --workspace -j 2`: EXIT 0 — 71 suites / 587 / 0.
- G4 `npm run lint`: EXIT 0.
- G5 `npm run typecheck` (`tsc --noEmit`): EXIT 0.
- G7 `npx vitest run`: 86 files / 889 passed / 0 failed, EXIT 0.
- G6 build: `npm run build` EXIT 0 (5.90s, 812.12 kB js) + `npx tauri
  build` EXIT 0 -> `target/release/bundle/nsis/Baraka UltraPOS_1.0.0_x64-setup.exe`.
  No red at any gate; no fix-forward beyond the listed trivial lints.

## 5. PNGs + asserts + vision

- `audit/BRK-101/raster-ticket.png` (21,666 B): live in-browser verdict
  `OK ticket=80x512 label=40x160 errors=0`, `__errs []`. Vision PASS:
  header fully legible (BARAKA ULTRAPOS, Alger, NIF, T-2026-000123,
  depot, date); `2 pc 10,00 DZD 20,00 DZD` / `1 pc 7,00 DZD 7,00 DZD`;
  `remise 1.00 DZD`; sous-total `27,00 DZD`, TVA `0.00`; lane
  alignment clean, zero overlaps/smears, no placeholders. (Barcode +
  footer below the viewport fold — present in the 512-row frame.)
- `audit/BRK-101/raster-label.png` (4,663 B): vision PASS — `Vis 8x40`
  / `10.00 DZD / pc` / clean Code128 bars / exactly ONE legible
  `VIS-001` code line (12px bold fix), no duplication, no defects.
- Asserts pasted: `ticket.widthBytes === 80`, height measured 512 rows;
  `label 40x160`; canvases painted 640x512 / 320x160; dark-pixel census
  price 14.03% / barcode 17.83%; 0 console errors.
- Honest note: human code survives thresholding with thin strokes at
  12px bold (black cores, min=0); the machine-readable barcode is the
  authoritative carrier — judge scans, not the code line, in trial 7.

## 6. Leftovers

- Deleted: `src/__brk101-harness.tsx`, `vite.__brk101.config.ts`,
  `D:/Dev/_scratch/BRK-101/` (harness build + 16 chunks + css),
  `brk101-build.log`, `brk101-build2.log`, `brk101-g3.log`,
  `brk101-tauri.log`. Verified: `ls src/__brk101-* vite.__brk101*`
  -> No such file; `ls D:/Dev/_scratch/` -> only pre-existing
  `AUD-005`, `TASK-DSN-012`.

## 7. Diff

- M: `crates/domain/src/printing/tspl.rs` (+renderer, +4 pins),
  `crates/domain/src/printing/mod.rs` (+exports),
  `crates/application/src/use_cases/tspl_printing.rs`
  (+PrintRawBitmapUseCase), `crates/application/tests/tspl_print_tests.rs`
  (+1 pin), `crates/commands/src/dto.rs` (+RawBitmapRequest),
  `crates/commands/src/ipc.rs` (+print_raw_bitmap),
  `src-tauri/src/lib.rs` (+registration), `src/lib/ipc.ts`
  (+ipcPrintRawBitmap).
- New: `src/lib/bitmap-pack.ts`, `src/lib/bitmap-pack.test.ts`,
  `src/lib/bitmap-capture.ts`, `audit/BRK-101/` (REPORT + 2 PNGs).
- DO NOT COMMIT (installer never; REPORT staged by architect).

STATE: DONE / REMAINING: owner trial 7 (BITMAP acceptance) -> lock-in
(TEXT removal + 12px code unify) or, if trial-7 all-red, emulation AUD.
NEXT READY: YES.
