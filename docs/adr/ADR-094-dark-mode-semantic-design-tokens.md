# ADR-094: Dark Mode via Semantic Surface/Ink/Line/Status Design Tokens

## Status
Accepted

## Context
AUD-015 found that `src/index.css` hardcodes body background/text colors
with no dark-mode variant, while `tailwind.config.js` sets
`darkMode: "class"` and `src/components/pos/Topbar.tsx` toggles `.dark` on
`document.documentElement`. As a result, dark-mode styling across the
codebase is applied ad hoc, per component, via unpaired `dark:text-*` /
`dark:bg-*` Tailwind utilities (685 occurrences across 76 files), with no
shared contract guaranteeing that a dark text color is ever paired with a
compatible dark background. Empirically, 6 of 7 sampled already-shipped
screens failed WCAG AA contrast in dark theme (ratios as low as 1.0:1).
TASK-DSN-054-C1 independently discovered the same defect in two new
components before this root cause was understood.

## Decision
Adopt a three-tier semantic design token system per Charter §7.1.6:
primitive hex values defined once as CSS custom properties at `:root`
(light) and `:root.dark` (dark); semantic token names (surface-canvas,
surface-raised, surface-sunken, ink-primary, ink-secondary, ink-inverse,
line, line-strong, success/warning/danger/info bg+fg) consumed through
Tailwind's `theme.extend.colors`, generating utility classes
(`bg-surface-canvas`, `text-ink-primary`, etc.). The application's root
shell consumes these tokens directly so that toggling `.dark` on
`documentElement` darkens the entire canvas without requiring any
individual component to separately declare a dark background. Light-theme
token values are fixed to the pre-existing hardcoded values to guarantee
zero visual regression to the Owner-approved default theme.

## Consequences
- Positive: a single source of truth prevents future text/background
  contrast mismatches; migration of existing ad hoc pairs becomes
  mechanical and verifiable.
- Negative / accepted debt: this ADR's implementing task (TASK-DSN-055)
  only establishes the foundation and migrates 2 of 76 affected files.
  The remaining ~74 files require separate, small, sequenced migration
  tasks, prioritized by the severity data in `audit/AUD-015/REPORT.md`.
  Until that migration completes, dark mode remains PARTIALLY correct and
  must never be described as fully verified or production-ready.
- Measured token contrast ratios (light / dark), recorded after
  TASK-DSN-055 Step 1 verification: ink-primary on surface-canvas
  17.06 / 17.19; ink-primary on surface-raised 17.85 / 16.19;
  ink-secondary on surface-canvas 9.90 / 7.34; ink-secondary on
  surface-raised 10.40 / 6.92; success-fg on success-bg 4.57 / 8.00;
  warning-fg on warning-bg 4.51 / 8.30; danger-fg on danger-bg 4.60 / 5.86;
  info-fg on info-bg 5.49 / 5.93 (light info-fg adjusted #2563eb → #1d4ed8
  to clear 4.5:1; light ink-secondary adjusted #475569 → #334155 for
  pixel-identical continuity with existing slate-700 copy).
