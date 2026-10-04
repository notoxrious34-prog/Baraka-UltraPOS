# Design tokens

Status: v0.1 PROPOSED. Date: 2026-10-04. Baseline: 716c4a0a75aed3aa385753a95fc8ade2d44e506d. All values are proposals pending owner review with the first prototypes and real screens. No code reads this document yet; it is the contract for the later tokens-implementation task.

## 1. Principles

- TK-P1 Logical CSS properties only (margin-inline, padding-block, inset-inline-start, and so on); no left/right physical properties, so the Arabic mirror needs no separate tokens.
- TK-P2 No floating layout: these tokens never produce position: fixed, except modal overlays, toasts and the command palette (ADR-067).
- TK-P3 No floats for money: amount cells use a tabular alignment token (see section 4), never CSS float.
- TK-P4 Two density modes only, tied to the viewport-matrix height class: compact (short) and comfortable (normal). No third density.
- TK-P5 Two themes only at v1: light (default) and dark (available). No per-module theming.
- TK-P6 Colour is never the sole signal (state-catalog rule S-R7); every semantic colour token has a paired icon or text convention described in the component inventory.

## 2. Colour tokens (PROPOSED hex, pending owner review)

| Token | Role | Light | Dark |
|---|---|---|---|
| color-bg | Page background | #F7F7F5 | #15171A |
| color-surface | Card and panel background | #FFFFFF | #1E2024 |
| color-surface-raised | Modal and popover background | #FFFFFF | #26292E |
| color-border | Default border | #D9D9D6 | #34373C |
| color-text-primary | Main text | #1A1A18 | #ECECEA |
| color-text-secondary | Secondary text | #5B5B57 | #A8A8A4 |
| color-text-disabled | Disabled text | #9A9A96 | #6B6E73 |
| color-accent | Primary action | #2F6F4F | #4E9C74 |
| color-accent-contrast | Text on accent | #FFFFFF | #0B1210 |
| color-success | Success state | #2F6F4F | #4E9C74 |
| color-warning | Warning state | #9A6B12 | #D9A64B |
| color-danger | Danger and destructive | #A3312B | #E0675F |
| color-info | Informational | #2A5B8C | #6FA4DD |
| color-focus-ring | Keyboard focus outline | #2A5B8C | #6FA4DD |
| color-overlay-scrim | Modal backdrop | rgba(20,20,18,0.45) | rgba(0,0,0,0.6) |
| color-credit-allowed | Credit HUD Autorisé | #2F6F4F | #4E9C74 |
| color-credit-refused | Credit HUD Refusé | #A3312B | #E0675F |

- TK-C1 Contrast ratios are not yet verified against WCAG AA; this is an open point.
- TK-C2 color-credit-allowed and color-credit-refused reuse color-success and color-danger on purpose: one meaning, one colour, plus a word and an icon from glossary rows 53 and 54 (rule R6).

## 3. Spacing scale

| Token | Value |
|---|---|
| space-0 | 0px |
| space-1 | 4px |
| space-2 | 8px |
| space-3 | 12px |
| space-4 | 16px |
| space-5 | 24px |
| space-6 | 32px |
| space-7 | 48px |
| space-8 | 64px |

- TK-S1 Compact density (short viewport) uses space-1 through space-5 for component padding; comfortable density (normal viewport) uses space-2 through space-6. The exact mapping per component is set in the component inventory.

## 4. Typography tokens

| Token | French/Latin | Arabic |
|---|---|---|
| font-family-base | Inter, system-ui fallback | Noto Naskh Arabic, system-ui fallback |
| font-size-sm | 13px | 14px |
| font-size-md | 15px | 16px |
| font-size-lg | 18px | 19px |
| font-size-xl | 24px | 25px |
| line-height-base | 1.4 | 1.7 |
| font-feature-money | tabular-nums | tabular-nums |

- TK-T1 Arabic sizes and line heights are slightly larger because the script needs more vertical room; values are PROPOSED and unmeasured on the shop PC.
- TK-T2 Money always uses font-feature-money (tabular figures) so amounts align in columns without floats (principle TK-P3).
- TK-T3 No font has been licensed or chosen yet; the family names above are placeholders for the later tokens-implementation task.

## 5. Radius tokens

| Token | Value |
|---|---|
| radius-sm | 4px |
| radius-md | 8px |
| radius-lg | 12px |
| radius-full | 9999px |

## 6. Elevation tokens

| Token | Usage | Shadow |
|---|---|---|
| elevation-0 | Flat surfaces, cards, panels | none |
| elevation-1 | Dropdown, popover | 0 2px 8px rgba(0,0,0,0.12) |
| elevation-2 | Toast | 0 4px 16px rgba(0,0,0,0.16) |
| elevation-3 | Modal overlay, command palette | 0 8px 32px rgba(0,0,0,0.24) |

- TK-E1 Elevation tokens are used only where ADR-067 allows position: fixed (modal overlays, toasts, command palette); every other surface stays in normal flow at elevation-0.

## 7. Z-index tokens

| Token | Value | Usage |
|---|---|---|
| z-base | 0 | Normal document flow |
| z-dropdown | 10 | Inline dropdown and popover |
| z-overlay-scrim | 100 | Modal backdrop |
| z-modal | 110 | Modal dialog, including Autorisation du gérant |
| z-toast | 120 | Toast notifications |
| z-command-palette | 130 | Command palette (ADR-067) |

## 8. Motion tokens

| Token | Value |
|---|---|
| duration-fast | 100ms |
| duration-normal | 180ms |
| duration-slow | 260ms |
| easing-standard | cubic-bezier(0.2, 0, 0, 1) |

- TK-M1 Every transition respects prefers-reduced-motion: reduce by dropping to duration-fast with no change in meaning.

## 9. Density tokens

| Token | Compact (short) | Comfortable (normal) |
|---|---|---|
| density-row-height | 36px | 44px |
| density-control-height | 32px | 40px |
| density-padding-inline | space-2 | space-3 |
| density-padding-block | space-1 | space-2 |

- TK-D1 Compact density applies at viewport-matrix height class short (rows 1, 3, 5, 6); comfortable applies at normal (rows 2, 4, 7-12).

## 10. Breakpoint tokens

| Token | Value | Matches viewport-matrix tier |
|---|---|---|
| bp-compact | 1024px | T1 |
| bp-standard | 1280px | T2 |
| bp-wide | 1600px | T3 |

## 11. Open points

- O1 Colour values, contrast ratios and both font families are PROPOSED and unreviewed.
- O2 Dark theme has not been checked against any accessibility tool.
- O3 Whether a third, larger density is needed for very tall screens (row 12 of the viewport matrix) is undecided.
