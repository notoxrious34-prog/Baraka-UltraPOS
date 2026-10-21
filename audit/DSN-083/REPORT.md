# TASK-DSN-083 REPORT — 39x20mm barcode label calibration

## Golden-hash before/after (legacy presets byte-identical)

Pre-change (stashed HEAD code) vs post-change, fixture `prod-1/VIS-8X40/6111250000013`,
`fr-DZ/Baraka/qty1`, sha256-16:

| Preset | BEFORE | AFTER | Match |
|--------|--------|-------|-------|
| 50x30 | `ec3b5b6684062da3` | `ec3b5b6684062da3` | YES |
| 40x25 | `2044553e7a6e1987` | `2044553e7a6e1987` | YES |
| 100x50 | `9d3cf3867e14c09a` | `9d3cf3867e14c09a` | YES |

Identity mechanism: legacy HTML construction untouched (same template string,
fixed `moduleWidth: 2, height: 56`); 39x20 emits a separate template + style
branch. Pinned in `product-label-modal.test.tsx` (`legacy preset golden identity`).

## Boundary tests

- `fitModuleWidth(13, 132, 1)` → 0.667 ≤ 1, 198 modules × 0.667 = 132 px ≤ budget (no clip).
- `fitModuleWidth(5, 132, 1)` → 1 (short codes keep full width).
- `fitModuleWidth(2, 10000, 1)` → 1 (never grows).
- `resolveLabelBarcode(13-char, "50x30").moduleWidth` → 2 (legacy pinned).
- 39x20 HTML: `@page 39mm 20mm`, no `.shop`, `class="sticker compact"`,
  svg width ≤ 132, `text-overflow: ellipsis` + `title="6111250000013"`
  (never silent clip).
- Arabic shaping: same `Arial, sans-serif` stack as legacy
  (`ProductLabelModal.tsx` compact style; no font substitution by design —
  Arial carries Arabic glyphs, so the 7–8pt compact lines shape identically).

## Gates

- G1 (cargo fmt): N/A — zero Rust touched (2 frontend files + 2 test files only).
- G2 (cargo clippy): N/A — same reason.
- G3 (cargo test): N/A — same reason.
- G4 lint: **exit 0** (no new warnings).
- G5 typecheck: **exit 0** (`tsc --noEmit`, clean after extending the pre-existing
  `Record<LabelFormat, string>` + sticker-class regex to 4 formats).
- G6 build: **pass, 5.92s** (vite).
- G7 vitest: **73 files / 737 tests passed** (was 727: +4 fit/budget, +5 modal —
  golden identity, @page, no-shop, svg budget, ellipsis/title; +1 pre-existing
  test extended to 4 formats).

## PNGs + rows

- `label-39x20-1280x800-fr-light.png` (84788 B) — 39x20 selected, preview
  boxOk, svgW=88 ≤ 132, page no-scroll, 0 console errors.
- `label-39x20-1280x800-ar-dark.png` (63302 B) — dark + RTL + `39 × 20 مم (مصغّر)`
  selected, preview inside box, no clipping, 0 errors.
- `label-50x30-1024x600-fr-light.png` (74874 B) — legacy 50x30 unchanged, no overflow.
- Harness notes: label modal has no `role="dialog"` (title-text wait used);
  theme/locale toggles live on login screen + POS topbar only (AR/dark via
  login-screen locale click + `baraka-theme` localStorage seed + reload);
  POS route needs an OPEN cash session stub (null session hides the topbar).

## Lockfile / diff scope

- `git diff --stat -- pnpm-lock.yaml package-lock.json yarn.lock package.json`: empty (no output).
- Diff limited to: `src/lib/barcode-128.ts` (+budget table, `labelBarcodeBudgetPx`,
  `fitModuleWidth`), `src/features/catalog/ProductLabelModal.tsx` (39x20 preset,
  `resolveLabelBarcode`, compact branch, presetLabel FR/AR), both test files,
  `audit/DSN-083/` (capture.mjs + 3 PNGs + results + REPORT).

## OWNER-GATE (unprovable without the XP-410B hardware)

Scannability at ~1.4–2 dots/module (13-char @ 0.667 width), driver `@page 39x20`
form handling, darkness/ribbon settings, peel gap, real clip behavior of the
spooler vs `overflow:hidden`. All layout math + preview proofs are in-repo;
no successful scan is claimed.

## STATE / DONE / REMAINING

STATE done. DONE: preset + guard + tests + goldens + 3 PNGs + REPORT.
REMAINING: none (hardware scan test belongs to the owner's bench).
NEXT READY: YES.
