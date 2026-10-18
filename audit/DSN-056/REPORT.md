# TASK-DSN-056 REPORT — ink-muted regression closure

Baseline (start): cargo 462/0, vitest 597/597, tree as specified (plus one
probe-caused `pnpm-lock.yaml` bump from the earlier `hermes verify` attempt,
restored via `git checkout -- pnpm-lock.yaml` before any task work; reported
explicitly, not task-related).

## Step 0 — path + element
File: `src/features/reports/ReportsHomeScreen.tsx` (confirmed). Regressed
element: line 76, the per-view subtitle
`<p className="text-xs font-bold text-slate-500">{t(locale,"rep.subtitle")}</p>`.
It is the ONLY `text-slate-500` usage in the file (line 55 is a self-paired
button, untouched). Card title (line ~75 h2, unmarked, fixed by DSN-055 root
effect) untouched. No other file touched.

## Step A — light hex identity
No slate override in `tailwind.config.js` → Tailwind default slate-500 =
`#64748b`, confirmed empirically: AUD-015/DSN-055 measured the element's
computed fg as `rgb(100, 116, 139)` = `#64748b`. Token light pinned to
`#64748b`. Dark `#94a3b8` (candidate from spec) verified 7.34:1 on
`#0b1120` (surface-canvas dark re-read from `src/index.css`, unchanged).

## Step 4 measurements (verify-ink-muted.mjs, 1280×720, fr)
- Subtitle dark: 3.96 → **7.34** PASS (≥4.5 ✓). Regression closed.
- Card title dark recheck: **16.3** PASS (undisturbed); full-card shot proves
  title + subtitle pass simultaneously.
- Subtitle light: computed `rgb(100, 116, 139)`, ratio **4.55** — numerically
  identical to the element's original pre-regression contrast (4.55),
  class now `text-ink-muted`. Zero light change (no EDGE CASE triggered).
- 7-screen recheck: every element IDENTICAL to DSN-055 "after" numbers
  (pos 3.75/9.39, settings 16.3/8.4, catalog 16.3/17.19, stock 16.3,
  customers 17.19/11.28, suppliers 17.19, reports h2 17.19/card 16.3).
  Zero blast radius beyond the one fixed element.
- Scroll: `1280/1280/720/720` on all 10 captures (no page scroll).

## Evidence
`audit/DSN-056/screenshots/`: 7 `dsn056-*-dark-after.png` +
`dsn056-reports-subtitle-dark-after.png` +
`dsn056-reports-full-regression-recheck-dark.png` +
`dsn056-reports-subtitle-light-unchanged.png` + `contrast-ink-muted.json`.
