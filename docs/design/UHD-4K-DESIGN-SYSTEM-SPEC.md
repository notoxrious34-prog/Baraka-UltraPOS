# UHD 4K Design System Specification — Baraka UltraPOS

Authored under TASK-AUD-007 (read-only; zero production modifications).
Baseline: `a7f538c` (v1.0.0-rc.10). Companion: `docs/architecture/AUDIT-UI-BLUEPRINT-2026.md`.

## 1. Font asset audit (on-disk truth)

### 1.1 Web UI fonts — `public/fonts/` (10 files, 860 384 bytes total)

| File | Format | Weight | Bytes |
| --- | --- | --- | --- |
| `Inter-Regular.woff2` | woff2 | 400 | 111 268 |
| `Inter-Medium.woff2` | woff2 | 500 | 114 348 |
| `Inter-SemiBold.woff2` | woff2 | 600 | 114 812 |
| `Inter-Bold.woff2` | woff2 | 700 | 114 840 |
| `Cairo-Regular.ttf` | truetype | 400 | 41 180 |
| `Cairo-SemiBold.ttf` | truetype | 600 | 41 356 |
| `Cairo-Bold.ttf` | truetype | 700 | 41 288 |
| `JetBrainsMono-Regular.woff2` | woff2 | 400 | 92 380 |
| `JetBrainsMono-Medium.woff2` | woff2 | 500 | 94 284 |
| `JetBrainsMono-Bold.woff2` | woff2 | 700 | 94 628 |

Declared in `src/index.css:1-73` with `font-display: swap`. Consumed via
Tailwind tokens `font-sans` (Inter-first) / `font-arabic` (Cairo-first) /
`font-mono` (`tailwind.config.js:19-23`).

### 1.2 Print-engine fonts — `assets/fonts/` (NOT web UI)

`Inter-Regular.ttf` + `NotoNaskhArabic-Regular.ttf`, referenced by
`crates/infrastructure/src/printing/font_cache.rs` for ESC/POS + PDF
rendering. Out of scope for screen typography; listed here only to prevent
confusion during asset procurement.

### 1.3 Gap analysis

- **No 800 or 900 face exists anywhere in the repository** (`find` for
  `*.woff2|*.ttf|*.otf` outside `target/`, `node_modules/`, `dist/`
  returns only the files above).
- `grep -c "font-black|font-extrabold" src/**/*.tsx` = **387 call sites**,
  every one of which currently renders as algorithmically thickened
  faux-bold (blur + stem color fringing at 150–200% DPI scaling).
- Cairo jumps 400 → 600 → 700 (no 500); Inter jumps to 700 max.
- **Procurement requirement** (offline, zero-dependency): authentic static
  instances, kept in `public/fonts/`, wired as two new `@font-face` blocks:
  - `Inter-ExtraBold.woff2` (weight 800, latin + latin-ext subsets to cover
    French diacritics; expect ~115 KB by analogy with siblings).
  - `Cairo-ExtraBold.ttf` (weight 800, Arabic + latin; expect ~42 KB) and,
    if the Owner approves the extra ~42 KB, `Cairo-Black.ttf` (900) for
    hero numerals only.
  - Sources: Inter 4.x static builds (rsms), Cairo Play families (Google
    Fonts static TTFs, OFL-licensed like the current set). No CDN, no
    variable-font payload (keeps the offline handbook-equivalent guarantee
    for the app shell).
- Until procured, the stopgap is `font-extrabold` (700 = real Bold) in new
  4K-critical components; new code must never rely on `font-black`.

## 2. Design tokens architecture

### 2.1 Palette (extends `tailwind.config.js:7-18`)

| Token | Value | Usage |
| --- | --- | --- |
| Industrial Trust Blue / `primary-900` | `#0F3D5C` | Nav rail, headers, primary surfaces |
| Deep Navy | `#0B192C` | 4K backdrop for modal scrims (60% alpha), POS shell surround |
| Primary action blue / `primary-600` | `#1E6091` | Links, focus rings, hover borders |
| Amber Glow / `accent-500` | `#F59E0B` | Active nav pill, primary CTAs, install/progress |
| Amber hover / `accentDark` | `#D97706` | CTA hover, warning text/icons |
| Surfaces | `#FFFFFF` + `border-slate-200/80` + `shadow-[0_1px_3px_rgba(0,0,0,0.04)]` | Crisp high-DPI cards |
| Ink / paper | `#0F172A` / `#F8FAFC` | Body text / app background (`index.css:94-97`) |
| Success | `#16803C` (emerald-700) | Installed badges, confirm notes (`role="status"`) |
| Danger | `#DC2626` (red-600) | Delete confirm `#C62828` CTA, blocked cards |
| Warning | `#D97706` | Not-installed pack border, amber accents |
| Info | `#2563EB` (blue-600) | Discovery hints, informational notes |

Dark-mode mappings already established (`dark:bg-slate-900`,
`dark:border-slate-700`) are preserved verbatim.

### 2.2 Shape, spacing, elevation

- Radius: `rounded-lg` (8 px) controls, `rounded-xl` (12 px) cards,
  `rounded-full` pills/badges, `rounded-md` inline notices.
- Chrome: `h-10` (40 px) nav rail + toolbars; content gutters `p-4`,
  cockpit density `p-2 gap-2`.
- Elevation: cards `shadow-sm` → hover `shadow`; modals `shadow-2xl` on
  `bg-black/60` scrim, `max-w-md`, `border-slate-200 dark:border-slate-700`.
- Icons: inline SVG, `strokeWidth 1.75`, `round` caps/joins, no emoji.

## 3. UHD 4K typography matrix

| Role | Size / leading | Weight (post-procurement) | Tracking |
| --- | --- | --- | --- |
| Hero module title (`h1`) | 18 px / 1.4 | 800 (Inter-E cried) | default |
| Card title (`h2`) | 14 px / 1.4 | 800 | default |
| Body / list rows | 12–13 px / 1.55 | 400–700 | default |
| Micro labels / badges | 11 px / 1.4 | 700 | default |
| Money / counts / SKUs | 12–13 px mono | 500–700, `tabular-nums` | default |

Mandatory rules:

1. `-webkit-font-smoothing: antialiased` (already `index.css:98`) **plus**
   `-moz-osx-font-smoothing: grayscale` (to add — missing today).
2. Faux-bold ban: no `font-black`/`font-extrabold` in new or touched code
   until §1.3 faces land; then `font-extrabold` maps to real 800.
3. Numerics: `tabular-nums` (or `.cockpit-nums`) **and**
   `<bdi dir="ltr">` on every currency, quantity, SKU/barcode, date, and
   `X / N (P %)` progress readout — RTL isolation is load-bearing.
4. No `position: fixed` outside modal overlays/toasts; no
   `w-screen`/`h-screen` outside the app root (charter carry-over).
5. 4K scaling without plugins: layout stays flex/grid-fluid; type steps up
   only via `xl:`/`2xl:` utilities (no container-query plugin authorized).

## 4. Screen blueprints

### 4.1 Settings — two-tier sidebar (4 groups, 8 master tabs)

Left rail groups: **Magasin** (Identité, Imprimante), **Données**
(Catégories & Rayons [new], Échanges & Import [new], Sauvegardes),
**Pack** (Pack Quincaillerie [new], Fiscalité), **Système** (À propos).
Implementation: replace the pill strip (`SettingsModule.tsx:37-54`) with a
`w-56 shrink-0` sidebar (`flex-row` shell, content `flex-1`); tab type gains
`categories | pack | exchange`; branches per §2 of the blueprint doc.
Session-gating preserved: token-backed tabs render only when
`session !== null`; write actions stay ADMIN-only server-side with the
switch disabled client-side (FiscalSettingsTab precedent).

### 4.2 Catalog — purified operational surface

Remove banner slot, category-modal trigger, and exchange menu (§3 of the
blueprint); header becomes a **3-stat KPI ribbon** (Articles, Stock faible,
Valeur stock — reuse `kpis` at `CatalogModule.tsx:~380-410`); `CategoryPillStrip`
stays as a read-only filter; table keeps edit/delete/toggle + depot
`<select>` in `ProductCreateModal` (operational necessity, not config).

### 4.3 Suppliers — instant load, zero desert

Mount triggers `runSearch()` immediately (today the list waits for input);
empty branch (`SupplierListScreen.tsx:159-161`) becomes a card with a
`Nouveau fournisseur` CTA opening the existing `panel`/`formMode` flow;
rows gain a debt dot (red when balance ≠ 0, reusing `hasOpenBalance`
semantics from Customers).

### 4.4 Customers — Credit Intelligence HUD

`selectedId === null` renders, in the dossier pane, aggregate delinquency:
count of debtors, total outstanding (tabular mono, `<bdi>`), top-5 debtors
shortlist that selects on click — all computable from `details`
(`CustomersModule.tsx:54`) with zero new IPC.

### 4.5 Reports — 9 executive tiles

`TILES` (`ReportsHomeScreen.tsx:29-41`) gain `icon` (1.75px SVG) +
`preview` strings; tile layout becomes icon + label + live micro-metric
(today: label only). Routing (`active: ReportKey | null`, `:45`) unchanged.

### 4.6 POS cockpit — dropdown isolation

`ProductDiscovery` (`col-span-7`) unit popover (`UnitSelectPopover`) and
`OperationsMenu` render in a portal-scoped layer (`z-50`) with backdrop
dismiss, so open menus never occlude the `#stock-search-input`/F1 flow;
`CartPanel` + `TotalsFooter` + `KeyFooter` geometry unchanged.

## 5. Audit integrity

- Only file written: `docs/design/UHD-4K-DESIGN-SYSTEM-SPEC.md`.
- Production sources, tests, packages, lockfiles: untouched (verify with
  `git status` — sole entries must be this spec plus the AUD-006 blueprint,
  both untracked docs).
- Gates undisturbed by a read-only pass: Rust 462/0, Vitest 574/574
  (recorded at `a7f538c`; no code changed since).
