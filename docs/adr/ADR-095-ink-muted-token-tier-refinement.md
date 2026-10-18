# ADR-095: Introduce the `ink-muted` Token — Refining the Semantic Ink Tier

## Status
Accepted

## Context
TASK-DSN-055 (ADR-094) established a two-level semantic "ink" tier
(`ink-primary`, `ink-secondary`) for text color, darkening the application's
root canvas for the first time. This exposed a genuine WCAG AA regression in
`ReportsHomeScreen.tsx`'s card subtitle (`text-slate-500`, unpaired,
non-token), which had only ever "passed" contrast by accident, relying on
the old static light-mode body background that no longer applies once the
canvas genuinely darkens (measured: 4.55:1 before TASK-DSN-055's root-canvas
change, 3.96:1 after — a FAIL against the 4.5:1 threshold).

Mechanically reusing the existing `ink-secondary` token to fix this would
have introduced a DIFFERENT regression: `ink-secondary`'s light value
(`#334155`) was tuned for TASK-DSN-054's higher-emphasis body copy, and is
visibly darker than the `slate-500` (`#64748b`) this report subtitle
currently and correctly uses in light mode. Forcing every "secondary-ish"
text treatment across the codebase into one token would repeat the precise
one-size-fits-all mistake the three-tier token architecture (Charter
§7.1.6) is designed to prevent, and would introduce unauthorized light-theme
visual drift across components never reviewed for that change.

## Decision
Introduce a third ink-emphasis level, `ink-muted`, scoped specifically for
lower-emphasis/caption-like text (e.g., report card subtitles, helper
captions). Its LIGHT value is pinned EXACTLY to the pre-existing
`text-slate-500` resolved color (zero light-mode visual change for any
future consumer of this token that currently uses that exact shade). Its
DARK value is computed independently to satisfy WCAG AA (>= 4.5:1) against
the dark canvas established in ADR-094, rather than being derived from
`ink-secondary`'s dark value.

This task applies the new token ONLY to the specific regressed
`ReportsHomeScreen.tsx` subtitle element. It does NOT retroactively migrate
every other `text-slate-500` usage in the codebase (AUD-015 found several);
those remain tracked for the forthcoming batched dark-mode migration tasks,
which now have `ink-muted` available as a correctly-calibrated tool for any
element whose current light-mode treatment matches this shade.

## Consequences
- Positive: the regression introduced by ADR-094's root-canvas change is
  closed without introducing a new, unauthorized light-theme visual change
  elsewhere; the token taxonomy now has three graduated emphasis levels
  (primary/secondary/muted) instead of forcing two levels to cover three
  real design intents.
- Negative / accepted debt: other files using `text-slate-500` (or visually
  similar ad hoc muted-text classes) remain unmigrated and may exhibit the
  SAME latent regression pattern once the root-canvas change reaches them
  through normal usage; they are explicitly NOT fixed by this ADR and must
  not be described as resolved. They are prioritized for the next batched
  migration tasks using AUD-015's inventory.
- Measured final values: `--color-ink-muted` light = `#64748b`, dark =
  `#94a3b8`, light-vs-original-identity proof = computed subtitle color
  `rgb(100, 116, 139)` in light theme, identical to the pre-existing
  `text-slate-500` resolved value, dark contrast ratio against
  surface-canvas dark = 7.34:1 (≥ 4.5:1 ✓).
