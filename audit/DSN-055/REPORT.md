# TASK-DSN-055 REPORT — Semantic Dark-Mode Token Foundation

Baseline (reconfirmed at start): cargo 462/0, vitest 597/597, tree as expected.
Method: `audit/DSN-055/scripts/verify-tokens.mjs` (thin copy of frozen
`audit/AUD-015/scripts/contrast-scan.mjs`; zero deps, synthetic mocks only,
dark via the real Topbar toggle). 1280×720, fr. Scroll state
`scrollW/clientW/scrollH/clientH` = `1280/1280/720/720` for ALL 11 captures
(no page scroll introduced).

## Step 3 finding — root wrapper
`src/App.tsx:125-128` root div is `h-full font-sans` — NO explicit background
(transparent, body shows through). **"Body background is unobstructed; no root
wrapper change needed."** No file changed for Step 3.

## Token adjustments (both explicitly authorized, measured)
1. `--color-ink-secondary` LIGHT `#475569` → `#334155` (existing slate-700 body
   copy; preserves pixel-identical light rendering). Re-verified: 9.90 on canvas,
   10.40 on raised (≥4.5 ✓).
2. `--color-info-fg` LIGHT `#2563eb` → `#1d4ed8` (draft measured 4.24:1, below
   AA; same hue, now 5.49:1 ✓).
No Tailwind collision: no top-level `surface/ink/line/success/warning/danger/info`
key existed (`baraka.ink` is namespaced, untouched) — no rename needed.

## Full token pair table (relative luminance, both themes)
- ink-primary on canvas: 17.06 / 17.19 (≥7 ✓)
- ink-primary on raised: 17.85 / 16.19 (≥7 ✓)
- ink-secondary on canvas: 9.90 / 7.34 (≥4.5 ✓)
- ink-secondary on raised: 10.40 / 6.92 (≥4.5 ✓)
- success fg/bg: 4.57 / 8.00 ✓ · warning: 4.51 / 8.30 ✓
- danger: 4.60 / 5.86 ✓ · info: 5.49 / 5.93 ✓

## C1 defect closure (dark, fr)
- Customers teaching: title 1.05 → **17.19** ✓, body 1.18 → **7.34** ✓
- Suppliers no-match body: 1.18 → **7.34** ✓
Defect formally closed. (Decorative icons moved to `text-line-strong`,
light-identical `#94a3b8`; dark value noted as decorative-only.)

## Before (AUD-015) → After, per element
| Element | Before | After | Class |
|---|---|---|---|
| pos discovery-empty (unmarked slate-500) | 3.75 F | 3.75 F | UNCHANGED-UNMIGRATED |
| pos nav / buttons | 9.39 P | 9.39 P | PASS |
| settings-h1 | 1.0 F | 16.3 P | FIXED (root canvas effect) |
| about-body | 8.4 P | 8.4 P | PASS |
| catalog product names ×2 | 1.0 F | 16.3 P | FIXED (root canvas effect) |
| catalog-h2 | 17.06 P | 17.19 P | PASS |
| stock-h1 | 1.0 F | 16.3 P | FIXED (root canvas effect) |
| customers h1 / row / suppliers rows | 17.06 P | 17.19 P | PASS |
| customers debtor balance (amber-400) | 1.6 F | 11.28 P | FIXED (root canvas effect) |
| reports card title | 1.0 F | 16.3 P | FIXED (root canvas effect) |
| reports subtitle (slate-500) | 4.55 P | 3.96 F | **REGRESSED — see below** |

"Root canvas effect": darkening the body canvas fixed every family-2
(dark-on-dark) site AND ambient-surface family-1 sites whose text already
carried a light dark-mode color (amber-400 balance) — without touching those
files. No file outside SCOPE-IN was modified to achieve this.

## NEWLY INTRODUCED REGRESSION (EDGE CASE — reported, NOT patched)
`src/features/reports/ReportsHomeScreen.tsx` subtitle ("Analyses d’activité",
`text-slate-500`, unmarked): 4.55 → **3.96** (below 4.5). Cause: it relied on
the old static light body color showing through; the canvas now darkens while
the text has no dark variant. Same latent class (unmarked `text-slate-500`
secondary copy on ambient surfaces, e.g. `UserListTable.tsx:87`,
`CustomerListPane` phone lines) will need dark variants in follow-up migration
tasks — the token (`text-ink-secondary`) now exists for exactly that.
File is SCOPE-OUT; left untouched per the EDGE CASE clause.

## Observation (not patched, SCOPE-OUT)
Topbar theme toggle (`Topbar.tsx:30-33`) is view-local `useState`: leaving the
POS view unmounts it and the toggle desyncs from the persisted `.dark` class.
Pre-existing behavior, unaffected by this task; flagged for a follow-up.

## Light-theme-unchanged proof
`LIGHT_BODY=rgb(248, 250, 252)|rgb(15, 23, 42)` — byte-identical to pre-task
`#f8fafc` / `#0f172a`. Light samples: pos body text 4.76 PASS, settings h1
17.06 PASS. Screenshots: `dsn055-pos-light-unchanged.png`,
`dsn055-settings-light-unchanged.png`.

## Evidence files
`audit/DSN-055/screenshots/`: 7 `dsn055-*-dark-after.png` + 
`dsn055-customers-empty-dark-after.png` + `dsn055-suppliers-nomatch-dark-after.png`
+ 2 light-unchanged + `contrast-after.json`.
