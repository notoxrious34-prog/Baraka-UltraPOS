# TASK-AUD-023 — Theme Architecture, Topbar Desync & AUD-015 Status: REPORT

READ-ONLY audit. No production/test source modified. Artifact: this file only.
HEAD at start: `93b0d09`; `git status --porcelain` clean. Method: static inspection +
headless-Chrome CDP (fetch+WebSocket, vite dev, stubbed Tauri `invoke`, 100% synthetic
mock data, 1280×720, `fr`), same pipeline as DSN-063. No live database touched.

## 1. SUMMARY OF FINDINGS

**Theme architecture.** Exactly ONE theme control exists repo-wide:
`src/components/pos/Topbar.tsx:30-36`. State is a view-local `useState(false)`; the
effect writes the *global* `document.documentElement` class. No store, no persistence,
no second toggle. The Topbar renders ONLY inside the POS view (`PosShellView.tsx:23`);
every other module (stock, catalog, settings, customers, suppliers, documents, reports,
users) renders its own header with NO toggle. Root cause of the logged desync debt:
view-local state × global effect × conditional mount, plus an effect with no cleanup.

**AUD-015 re-measurement: 25 target elements examined.**
DIRECT CDP (in-page computed fg/bg + ratio, light & dark): 17. ANALYTIC (CDP-measured
background + authored static classes + standard relative-luminance math, method labeled
per element): 5 trigger/conditional-gated elements. ABSENT/CONDITIONAL: 2. COSMETIC: 1.
- `CONFIRMED-STILL-BROKEN` (dark < 4.5:1): **5** — POS empty text (3.75), TotalsFooter
  rows (3.75), ParkedSalesModal sub-text (3.75), DiscountModal labels (3.75 analytic),
  DocumentTable thead (3.07). Single unifying family: unmarked `text-slate-500` inside
  self-darkened containers (`dark:bg-slate-900` / `dark:bg-slate-800`).
- `ALREADY-RESOLVED` (dark ≥ 4.5:1): **17** (12 DIRECT incl. DSN-063 trio citation, 5 ANALYTIC).
- `ELEMENT-ABSENT` / `CONDITIONAL`: 2. Cosmetic-only: 1 (dossier decorative icon 1.82).

## 2. THEME STATE & DESYNC DIAGNOSIS (with exact excerpts)

`src/components/pos/Topbar.tsx:30-36` (entire mechanism, only occurrence of
`classList.toggle("dark"` under `src/` — repo-wide grep confirms):
```tsx
function useDarkMode(): { dark: boolean; toggle: () => void } {
  const [dark, setDark] = useState(false);
  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
  }, [dark]);
  return { dark, toggle: () => setDark((value) => !value) };
}
```
Toggle button (`Topbar.tsx:141-148`): `onClick={toggle}`, `aria-label={t(locale,
"topbar.theme")}`, icon `{dark ? "☀" : "☾"}`. Header root (`:85`) is always-dark
`bg-primary-900 text-white` — theme-independent by design.
- Representation: local `useState`, default `false`. No Zustand, no `useTheme`, no
  `darkMode`/`theme` store key anywhere; `localStorage` under `src/` is used ONLY by
  `src/lib/starter-pack.ts` (installed flag). No SQLite settings persistence of theme.
- Click effect: flips local state → effect toggles `document.documentElement`
  (document root — the correct scope for `darkMode: "class"`, `tailwind.config.js:4`).
- Mount scope: `Topbar` renders only in `PosShellView.tsx:23`, which renders only for
  the POS view (`App.tsx:161` else-branch). Navigating to any manager module unmounts
  PosShell → Topbar unmounts → local `dark` state is destroyed — while the `dark`
  class persists on `documentElement` (effect has NO cleanup). Returning to POS
  remounts with `useState(false)` → effect runs `toggle("dark", false)` → dark mode is
  silently stripped. No other module offers a toggle (grep for `topbar.theme` /
  `useDarkMode` outside `Topbar.tsx`: zero hits), so off-POS screens cannot display or
  change theme state at all.
- Observable consequences: (a) dark does not survive logout/login or app restart —
  always boots light; (b) leaving POS keeps dark visuals with no visible control;
  (c) returning to POS resets to light without user action — the reported
  "view-local state desync across screens". Icon-vs-DOM desync additionally occurs if
  anything external (e.g. an audit harness) toggles the class, since the icon reads
  only the local boolean.
- AUD-015's-era claim "no mechanism changes root/body background" is STALE: since
  `6b06f93`, `src/index.css:143-147` sets `body { background:
  var(--color-surface-canvas); color: var(--color-ink-primary); }` with `:root.dark`
  overrides (`#0b1120` / `#f1f5f9`). This body-level change is what resolved every
  inheritance-based defect below — the same side-effect class as the ReportsHome
  hazard (DSN-063).

## 3. AUD-015 ELEMENT TABLE (re-located to real paths; ratios CDP-measured)

Method DIRECT = computed `color` + nearest non-transparent ancestor bg, both themes.
Method ANALYTIC = CDP-measured routable background + static authored classes + WCAG
math (used only where UI triggers are unreachable in a stubbed harness: delete-result
notes, scan-success banner, credit-statement error, uninstall CTA, discount labels —
each labeled).

| Element | Real path | Light ratio | Dark ratio | Classification |
|---|---|---|---|---|
| POS empty "Catalogue vide" | `src/components/pos/ProductDiscovery.tsx:231-236` (`div.text-slate-500`) | 4.76 (#64748b on #ffffff) | **3.75** (#64748b on #0f172a) | CONFIRMED-STILL-BROKEN (DIRECT) |
| TotalsFooter subtotal/discount/TVA rows | `src/components/pos/TotalsFooter.tsx:26-34` (`text-slate-500`, root self-painted `dark:bg-slate-900`) | 4.76 | **3.75** | CONFIRMED-STILL-BROKEN (DIRECT) |
| ParkedSale sub-line "3 articles · …" | `src/components/pos/ParkedSalesModal.tsx:226-230` (`p.text-xs text-slate-500`, ModalShell card `dark:bg-slate-900` CDP-confirmed) | 4.76 | **3.75** | CONFIRMED-STILL-BROKEN (DIRECT) |
| DiscountModal field labels | `src/components/pos/modals/DiscountModal.tsx:61,73` (`span.text-slate-500`, same ModalShell dark card bg rgb(15,23,42) CDP-measured) | 4.76 | **3.75** | CONFIRMED-STILL-BROKEN (ANALYTIC: modal unopenable without cart flow; bg + classes measured) |
| DocumentTable thead row | `src/features/documents/DocumentTable.tsx:66` (`tr.text-slate-500`, thead `dark:bg-slate-800`) | 4.34 (#64748b on #f1f5f9 — fails even in light) | **3.07** (#64748b on #1e293b) | CONFIRMED-STILL-BROKEN (DIRECT) |
| Scan-success banner | `src/components/pos/ProductDiscovery.tsx:185` (`dark:text-emerald-400` on `bg-emerald-500/10`, banner untriggerable without scan IPC) | 4.99 (emerald-700 on 10%-tint over white #e7f8f2) | 8.55 (emerald-400 #34d399 on 10%-tint over #0b1120 = #0c222a) | ALREADY-RESOLVED (ANALYTIC; container bg from CDP hint-box probe) |
| Hint pill (control) | ProductDiscovery barcode hint `span ▮▮▮` (`bg-slate-200…`, has dark bg) | 6.15 | 9.85 | PASS (DIRECT control) |
| ParkedSale doc number (unmarked) | `ParkedSalesModal.tsx:225` (`p.truncate font-extrabold`, inherits) | 17.85 | 16.30 | ALREADY-RESOLVED (DIRECT — body ink token) |
| Catalog delete button (resting) | `src/features/catalog/ProductEditModal.tsx:488` (`dark:text-red-200`, transparent on dark card) | 6.47 (#b91c1c on #ffffff) | 12.34 (#fecaca on #0f172a) | ALREADY-RESOLVED (DIRECT) |
| Customer debtor balance BDI | `src/features/customers/CustomerListPane.tsx:145` (`text-amber-600 dark:text-amber-400`, "1 250,50 DZD") | 3.04 (amber-600 on #f8fafc — light-side note, was 1.6 in AUD-015 on same family) | 11.28 (#fbbf24 on #0b1120) | ALREADY-RESOLVED (DIRECT — body canvas token flipped the bg) |
| Dossier HudCard alert value | `src/features/customers/CustomerDossierPane.tsx:42` (same amber pair, "1 250,50 DZD", after row select) | 3.04 | 11.28 | ALREADY-RESOLVED (DIRECT) |
| Dossier delete button "Supprimer" | `CustomerDossierPane.tsx:128` (dark: variants present) | 6.18 | 13.02 | ALREADY-RESOLVED (DIRECT) |
| Dossier decorative icon | `CustomerDossierPane.tsx:66` (`svg.text-line-strong`, `stroke=currentColor`, aria-hidden) | 2.45 (#94a3b8 on #f8fafc) | 1.82 (#334155 on #0b1120) | COSMETIC-ONLY (DIRECT; non-text decoration, AUD-015 agreed) |
| Customers delete-result note | `src/features/customers/CustomersModule.tsx:186` (`dark:text-emerald-300`, transparent on module root → body) | 5.24 (#047857 on #f8fafc) | 12.35 (#6ee7b7 on #0b1120) | ALREADY-RESOLVED (ANALYTIC: note needs delete flow; classes + CDP body bg) |
| Suppliers delete-result note | `src/features/suppliers/SupplierListScreen.tsx:403` (same pair, same site) | 5.24 | 12.35 | ALREADY-RESOLVED (ANALYTIC, same basis) |
| Supplier row (control) | SupplierListScreen TD "Quincaillerie El Amel" | 17.06 | 17.19 | PASS (DIRECT control) |
| LowStock qty cell | `src/features/reports/LowStockReport.tsx:100` (`dark:text-red-300`, rows transparent → body; Apply-gated fetch) | 6.18 (#b91c1c on #f8fafc) | 9.92 (#fca5a5 on #0b1120) | ALREADY-RESOLVED (DIRECT element + CDP bg; ratio via identical math path as CDP lum) |
| Movement signed qty | `src/features/reports/StockMovementReport.tsx:64` (same pair/site) | 6.18 | 9.92 | ALREADY-RESOLVED (same basis) |
| Margin signed cell | `src/features/reports/GrossMarginReport.tsx:50` (same pair/site) | 6.18 | 9.92 | ALREADY-RESOLVED (same basis) |
| Credit statement error line | `src/features/reports/CreditOutstandingReport.tsx:134` (`dark:text-red-300`, expanded `tr.bg-slate-50 dark:bg-slate-800/50`; expansion untriggerable in stub) | 6.18 | 8.87 (#fca5a5 on 50%-blend #151d2e) | ALREADY-RESOLVED (ANALYTIC) |
| Reports card title "Marges brutes" | `src/features/reports/ReportsHomeScreen.tsx:88` (unmarked on `dark:bg-slate-900` card) | 17.85 | 16.30 | ALREADY-RESOLVED (DIRECT — ReportsHome side-effect fix reconfirmed) |
| Reports subtitle (control) | `ReportsHomeScreen.tsx:76` (`text-ink-muted`) | 4.55 | 7.34 | PASS (DIRECT control — also settles §5: doc value 4.76 is wrong) |
| Document row (unmarked) | DocumentTable `TD FA-2026-101` (container `bg-white dark:bg-slate-950`) | 17.85 | 18.41 | ALREADY-RESOLVED (DIRECT) |
| Pack uninstall CTA | `src/features/catalog/StarterPackBanner.tsx:238` via `StarterPackSettingsTab` (renders only when pack installed — backend state; mock shows install UI) | — | — (class-identical to measured ProductEditModal delete: 12.34 dark) | CONDITIONAL-NOT-RENDERED (ANALYTIC by class identity) |
| Catalog "Gérer" manage button | `src/features/catalog/CategoryPillStrip.tsx:99` | — | — | ELEMENT-ABSENT: `CatalogModule.tsx:473-480` renders `CategoryPillStrip` WITHOUT `onManageCategories`, so the button never mounts in this composition |
| DSN-063 trio (catalog title, settings H1, stock H1) | `src/features/*/…` | 17.85/17.06 | 16.30 | ALREADY-RESOLVED (cited, not re-measured) |
| Migrated controls (dossier empty h2/p, supplier empty p) | `CustomerDossierPane.tsx:75,78`, `SupplierListScreen.tsx:205,243,246` (`text-ink-primary/secondary`) | — | — | STATIC-CONFIRMED migrated (token classes present) |

Residual defect family (the entire BROKEN set): unmarked `text-slate-500` (#64748b)
resting on a self-darkened container (`dark:bg-slate-900` → 3.75:1;
`dark:bg-slate-800` thead → 3.07:1). Occurrences of `text-slate-500` in the six
touched files: SettingsModule 1, ParkedSalesModal 3, DocumentTable 4, TotalsFooter 3,
DiscountModal 2, ProductDiscovery 5. New same-family observation (not in AUD-015):
`SettingsModule.tsx:200` group-header labels (`text-slate-500` on `dark:bg-slate-900`
nav) — same 3.75 predicted, unmeasured (nav H1 measured PASS; headers share its bg).

## 4. `src/` DIRECTORY TAXONOMY

Depth-1: `__tests__/ assets/ components/ features/ hooks/ lib/ stores/ types/
App.tsx main.tsx index.css vite-env.d.ts`.
- `src/features/` (12 modules: auth, catalog, customers, documents, health, pos,
  purchases, reports, settings, stock, suppliers, users) — routed view modules,
  each owning its screens/panes/tabs; the unit of navigation in `App.tsx:136-161`
  (no router; `app-view-store` switch). **This is where feature code lives.**
- `src/components/` — shared/cross-cutting presentation: `pos/` (POS cockpit shell:
  PosShell, PosShellView, Topbar, CartPanel/Table, TotalsFooter, ProductDiscovery,
  OperationsMenu, modals incl. ModalShell) + `common/` (SafeDeleteModal) +
  `status-card.tsx`. I.e. the POS cockpit kit + shared primitives, NOT feature screens.
- `src/stores/` — Zustand client state (session, cart, pos-ui incl. locale/modals,
  app-view, modal/cash/payment/transfer/supplier/store-identity/backup stores).
- `src/lib/` — framework-free domain helpers (ipc.ts Tauri bridge, i18n.ts,
  money.ts, format.ts, starter-pack.ts, barcode-128.ts) + colocated `*.test.ts`.
- `src/types/` (DTO mirrors e.g. `pos.ts`), `src/hooks/`, `src/assets/`.
- Historical confusion cause: feature files previously lived under `src/components/`;
  AUD-015/DSN-063 task texts still use the old prefix. Current truth: features are
  under `src/features/`; `src/components/pos/` is the cockpit, not a feature module.

## 5. DESIGN-TOKENS 4.76 vs 4.55 DISCREPANCY

`docs/design/design-tokens.md:23`:
`| ink-muted | `#64748b`³ | `#94a3b8` | on canvas (≥4.5) | 4.76 ✓ | 7.34 ✓ |`
Exact recomputation (WCAG relative luminance, node one-liner, same method as AUD-015):
`#64748b` on `#f8fafc` = **4.55** (matches AUD-015's browser-measured rep.subtitle
4.55 AND this audit's CDP-measured 4.55, §3 control row); `#94a3b8` on `#0b1120` =
**7.34** (doc correct). The "4.76" is a transcription error — note 4.76 IS the genuine
ratio of `#64748b` on pure `#ffffff` (POS/white-card surfaces, measured §3), i.e. the
wrong-background value was copied into the canvas row.
Exact one-line correction: line 23 `| 4.76 ✓ | 7.34 ✓ |` → `| 4.55 ✓ | 7.34 ✓ |`
(read-only audit: NOT applied here).

## 6. EVIDENCE & HYGIENE

- Stray-port: `STRAY_CHECK=clear` before each of 3 CDP runs (ports 5173/5174/9222/9223
  all free afterwards; vite + headless Chrome killed per-run via cleanup).
- Ad-hoc scripts (`node_modules/.aud023-measure.mjs`, `.aud023-dbg.mjs`,
  `.aud023-run3.mjs`, `.aud023-raw*.json`, vite logs, chrome profiles): DELETED before
  commit (NO_LEFTOVERS; node_modules is git-ignored regardless).
- `git status --porcelain` at commit: only `?? audit/AUD-023/` (report); no modified
  files under `src/ crates/ docs/` or root configs. Commit: `docs(audit): theme
  architecture, toggle desync, and AUD-015 defect status [TASK-AUD-023]` — report-only.
- Analysis computations used the corrected hex parser (`[0,2,4]` slices); an early
  draft used `[1,3,5]` and was discarded after giving absurd values — no reported
  number above comes from it (all cross-checked against CDP-measured values where
  available: 4.55/7.34/11.28/16.30/17.19 all match CDP exactly).

## 7. FILES

- [CREATED] `D:/Dev/Baraka UltraPOS/audit/AUD-023/REPORT.md`

STATE: complete (theme diagnosis + 25-element disposition + taxonomy + tokens math).
DONE: read-only audit, report written, hygiene verified.
REMAINING: 5-element `text-slate-500`-on-dark-container remediation (queued DSN scope,
not this task); Topbar persistence/centralization fix (queued); design-tokens.md
one-line correction (1 word, needs a docs task).
NEXT READY: YES
