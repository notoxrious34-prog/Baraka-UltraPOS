# Design tokens — single source of truth (ADR-094, TASK-DSN-055)

Consumed via Tailwind utilities (`bg-surface-canvas`, `text-ink-primary`,
`border-line`, `bg-success-bg`, `text-success-fg`, …). Ratios computed by
relative luminance (same method as AUD-015). Two light values were adjusted
from the draft (noted below); all pairs meet their threshold.

| Token | Light | Dark | Pair (ratio → threshold) | Light | Dark |
|---|---|---|---|---|---|
| surface-canvas | `#f8fafc` | `#0b1120` | ink-primary on canvas (≥7) | 17.06 ✓ | 17.19 ✓ |
| surface-raised | `#ffffff` | `#111827` | ink-primary on raised (≥7) | 17.85 ✓ | 16.19 ✓ |
| surface-sunken | `#eef2f6` | `#0a0f1a` | — (structural only) | — | — |
| ink-primary | `#0f172a` | `#f1f5f9` | see above | — | — |
| ink-secondary | `#334155`¹ | `#94a3b8` | on canvas (≥4.5) | 9.90 ✓ | 7.34 ✓ |
| ink-secondary | `#334155`¹ | `#94a3b8` | on raised (≥4.5) | 10.40 ✓ | 6.92 ✓ |
| ink-inverse | `#f8fafc` | `#0f172a` | — (on accent/primary fills) | — | — |
| line-default | `#e2e8f0` | `#1e293b` | — (decorative) | — | — |
| line-strong | `#94a3b8` | `#334155` | — (decorative/icons) | — | — |
| success-bg / fg | `#dcfce7` / `#16803c` | `#14321f` / `#4ade80` | fg on bg (≥4.5) | 4.57 ✓ | 8.00 ✓ |
| warning-bg / fg | `#fef3c7` / `#b45309` | `#3a2a0a` / `#fbbf24` | fg on bg (≥4.5) | 4.51 ✓ | 8.30 ✓ |
| danger-bg / fg | `#fee2e2` / `#c62828` | `#3b1414` / `#f87171` | fg on bg (≥4.5) | 4.60 ✓ | 5.86 ✓ |
| info-bg / fg | `#dbeafe` / `#1d4ed8`² | `#0f2744` / `#60a5fa` | fg on bg (≥4.5) | 5.49 ✓ | 5.93 ✓ |
| ink-muted | `#64748b`³ | `#94a3b8` | on canvas (≥4.5) | 4.76 ✓ | 7.34 ✓ |

¹ Draft was `#475569`; adjusted to `#334155` (existing slate-700 body copy)
to keep light rendering pixel-identical. ² Draft `#2563eb` measured 4.24:1
(below AA); darkened to `#1d4ed8` (same hue, 5.49:1). ³ TASK-DSN-056: light
pinned exactly to resolved `text-slate-500` (zero visual change); dark
computed independently (ADR-095).

Rule for future migrations: a text token may only be used on its paired
surface token (or a surface it was measured against); decorative tokens
(`line-*`) are never body copy.
