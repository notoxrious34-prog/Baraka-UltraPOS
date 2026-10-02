# Viewport matrix

Status: v0.1 PROPOSED. Date: 2026-10-02. Baseline: a374cf10c6b04d33605c1c293c2fb726df842775. All values are nominal; nothing has been measured on the shop PC.

## 1. Method

- M1 Nominal CSS viewport = floor(physical pixels divided by the Windows display scale), width and height separately.
- M2 The real usable height is lower: the Windows taskbar and the window title bar are subtracted. This deduction is an ESTIMATE until measured on a real machine and is not applied in this table.
- M3 The app opens maximized (BRK-060).

## 2. Matrix

| # | Physical px | Windows scale | Nominal CSS viewport | Tier | Height class | Remark |
|---|---|---|---|---|---|---|
| 1 | 1024x600 | 100% | 1024x600 | T1 | short | Hypothetical small screen |
| 2 | 1280x720 | 100% | 1280x720 | T2 | normal | - |
| 3 | 1280x720 | 125% | 1024x576 | T1 | short | Smallest supported viewport |
| 4 | 1366x768 | 100% | 1366x768 | T2 | normal | - |
| 5 | 1366x768 | 125% | 1092x614 | T1 | short | Key case named in the roadmap |
| 6 | 1366x768 | 150% | 910x512 | T0 | short | Below minimum, degraded mode |
| 7 | 1600x900 | 100% | 1600x900 | T3 | normal | - |
| 8 | 1600x900 | 125% | 1280x720 | T2 | normal | - |
| 9 | 1920x1080 | 100% | 1920x1080 | T3 | normal | - |
| 10 | 1920x1080 | 125% | 1536x864 | T2 | normal | - |
| 11 | 1920x1080 | 150% | 1280x720 | T2 | normal | - |
| 12 | 2560x1440 | 150% | 1706x960 | T3 | normal | - |

## 3. Tiers and rules (PROPOSED)

- T0 Degraded: CSS width below 1024 or CSS height below 576. The app uses the T1 layout and shows one dismissible notice recommending a lower display scale. No guarantee beyond this.
- T1 Compact: CSS width from 1024 up to 1279, and height at least 576. The navigation rail is collapsed (icons only) by default.
- T2 Standard: CSS width from 1280 up to 1599. The navigation rail is expanded by default.
- T3 Wide: CSS width 1600 or more. The rail is expanded by default. Extra width goes to content (more catalogue columns), and panels such as the cart keep a maximum width.
- Height class: short means CSS height below 640 (compact density); normal means 640 or more (comfortable density). Density values are set later in the design tokens document.
- The user can toggle the rail in every tier; the choice is persisted (preference persistence task).

## 4. Invariants in every tier

- No page scroll (only inner lists and tables scroll).
- Logical CSS properties only.
- The layout mirrors for Arabic (RTL).
- Latin digits always.
- No position fixed except modal overlays, toasts and the command palette (ADR-067).

## 5. Reference screenshots for the prototypes

- Every static prototype screen is captured in French (LTR) at matrix rows 3, 5, 4, 10 and 9, and in Arabic (RTL) at rows 5 and 4; row 6 is captured once per prototype set to show degraded mode.

## 6. Unmeasured items

- Shop PC specification.
- Real display resolution and Windows scale on the shop PC.
- Real taskbar and title bar heights.
- Ctrl+digit conflicts inside the packaged webview.
- Real scanner keyboard-wedge behaviour.
