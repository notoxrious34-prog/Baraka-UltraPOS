# TASK-AUD-NEXT (AUD-015) — Dark-Theme Contrast Defect Scope Audit + ADR Index Verification

READ-ONLY audit. Nothing was fixed, no source file modified, no dependency added.
Evidence: `audit/AUD-015/screenshots/` (7 PNG + `contrast.json`), script
`audit/AUD-015/scripts/contrast-scan.mjs` (CDP mechanism reused from
`audit/AUD-007/scripts/cdp.mjs` via `audit/DSN-054/scripts/dsn054-capture.mjs`;
Node fetch + WebSocket, zero deps). All captures use 100% synthetic mock data
through a Tauri `invoke` stub (browser has no backend); no live database, no
`%APPDATA%/com.baraka.ultrapos` access. Dark mode toggled through the real
Topbar theme control (`button[aria-label="Thème"]`, verified
`documentElement.classList.contains('dark') === true`, `DARK_VIA_USER_TOGGLE=ok`).

Baseline (reconfirmed at start AND end): cargo 462 passed / 0 failed,
vitest 597 passed / 0 failed.

## (c) Global dark-mode surface question — ANSWER FIRST (it frames everything)

- `tailwind.config.js:4`: `darkMode: "class",` (exact line).
- Toggle site: `src/components/pos/Topbar.tsx:33`
  `document.documentElement.classList.toggle("dark", dark);` — local `useState`
  default `false`; the Topbar (and therefore the toggle) renders ONLY inside
  the POS view.
- `src/index.css` global base (lines ~94-97):
  `body { background-color: #f8fafc; color: #0f172a; }` — NO `.dark` variant,
  no `color-scheme`, no CSS custom properties for surfaces anywhere in the file.
  No `.dark` selector, `[data-theme]`, or `prefers-color-scheme` exists anywhere
  under `src/` (grep, excl. tests: zero hits).
- **Definitive conclusion: the codebase has NO mechanism by which the
  root/body background changes in dark mode.** `body` stays `#f8fafc` with
  `#0f172a` text in both themes. Only per-component `dark:` utilities change
  anything. Consequences (both directions observed, see (b)):
  1. `dark:text-*` on an element whose effective background stays light
     (inherited body) → near-white on near-white (the DSN-054-C1 pattern,
     measured 1.05–1.18:1).
  2. Unmarked text (inherits body `#0f172a`) on a component that DOES darken
     its own background → near-black on near-black (measured 1.0:1 repeatedly).
  Dark styling is ad hoc per-component; there is no shared surface/token contract.

## (a) Static inventory — every file using `dark:` utilities

Method: repo-wide grep for `dark:` (76 source files, 685 occurrences, tests
excluded). Mechanical pre-sort: every `className="…"` / `className={`…`}`
literal containing `dark:text-*` without `dark:bg-*` in the SAME literal
(101 literals, ~30 files), PLUS a line complement for ternary/const strings
outside `className=` (all paired, SAFE). Each candidate was then resolved by
reading the element's own and nearest-ancestor classes in the same file.
Rule: SAFE = same element (or same-file ancestor) paints its own dark bg;
SUSPECT = dark text with no same-file dark bg (ambient body surface);
UNKNOWN = bg comes from a wrapper in another file (resolution file named;
several pre-resolved with 1 read each, marked accordingly).

### SUSPECT (defect family 1: light text, undarkened ambient surface)

| File:lines | Evidence |
|---|---|
| `CustomerDossierPane.tsx:75,78` | PROVEN by measurement (C1): 1.05/1.18:1 |
| `SupplierListScreen.tsx:199,205` | PROVEN by measurement (C1): 1.18:1 |
| `CustomerListPane.tsx:145` | PROVEN by measurement (this audit): debtor balance `dark:text-amber-400` `#fbbf24` on `#f8fafc` = **1.6:1 FAIL** |
| `CustomersModule.tsx:186` | deleteNote `dark:text-emerald-300` on ambient (module root has no `dark:bg`) |
| `SupplierListScreen.tsx:371` | deleteNote `dark:text-emerald-300`, same pattern |
| `CustomerDossierPane.tsx:42` | HudCard `dark:text-amber-400`, card root line 38 has NO `dark:bg` (sibling StockOverviewPane HudCard DOES — copy-paste miss) |
| `CustomerDossierPane.tsx:128`, `ProductEditModal.tsx:488`, `StarterPackBanner.tsx:238` | delete/destructive buttons: `dark:text-red-200` with only `dark:hover:bg-*`, NO resting dark bg. Resting state ≈ red-200 on red-50 ≈ 1.3:1. (ProductEditModal: card root IS dark, so its text is fine but the BUTTON face stays light — same resting defect; StarterPackBanner: card dark, same.) |
| `LowStockReport.tsx:100` | `dark:text-red-300` table cells, `ReportsHomeScreen` root (line 10) has no `dark:bg` |
| `CreditOutstandingReport.tsx:134` | `dark:text-red-300` cells, same ambient root |
| `StockMovementReport.tsx:64-65`, `GrossMarginReport.tsx:50-51` | `signedClass`/`marginClass` red/emerald-700→300 helpers, same ambient root |
| `CategoryPillStrip.tsx:99` | manage button `dark:text-slate-400` on transparent resting bg; `CatalogModule` roots (392, 856) have no `dark:bg` |
| `ProductDiscovery.tsx:184` | scan-success banner `dark:text-emerald-400` on `bg-emerald-500/10` (NO dark variant — tint stays light) ≈ 2:1. Plus unmarked empty text (see (b): 3.75:1 FAIL) |
| `TotalsFooter.tsx:25-33` | footer paints own `dark:bg-slate-900` (line 24, same file) but rows stay `text-slate-500` unmarked → ≈3.7:1 est. (needs measurement to confirm) |
| `DiscountModal.tsx:61,73` | labels `text-slate-500` unmarked inside ModalShell dark card → ≈3.7:1 est. |

### SUSPECT (defect family 2, inverse: unmarked dark text on darkened surface)

| File:lines | Evidence |
|---|---|
| `CatalogModule.tsx` product cards (~450) | PROVEN: product-name SPAN `#0f172a` on `#0f172a` = **1.0:1 FAIL** (card has `dark:bg-slate-900`, name unmarked) |
| `ReportsHomeScreen.tsx:86` card | PROVEN: card title "Marges brutes" `#0f172a` on `#0f172a` = **1.0:1 FAIL** |
| `SettingsModule.tsx` h1 | PROVEN: "Paramètres & Configuration" `#0f172a` on `#0f172a` = **1.0:1 FAIL** (root darkens line 192, h1 unmarked) |
| `StockModule.tsx` h1 | PROVEN: "Stock & Dépôts" 1.0:1 FAIL, same pattern (root darkens line 149) |
| `ParkedSalesModal.tsx:204,211,223,224,229` | text nodes with NO `dark:text` at all inside ModalShell dark card (only `dark:` in file is a border, line 220) |
| `DocumentTable.tsx` (5× `dark:bg`, 0× `dark:text`) | unmarked rows; parent `DocumentsModule` root darkens (line 96) — static finding, needs measurement |
| `CustomerDossierPane.tsx:68` | icon `dark:text-slate-500` on ambient — decorative, cosmetic only |

### UNKNOWN (bg from another file; resolution pre-done, likely fine)

| File | Resolves to (verified) |
|---|---|
| `AboutTab.tsx:20,26,29` | `SettingsModule.tsx:192` dark root ✓ + measured `about-body` 8.4:1 PASS |
| `BackupSettingsTab.tsx:64,109,118-130,144,157` | same dark root ✓ (thead self-painted) |
| `UserListTable.tsx:28,39-54,65,87` | `UsersModule.tsx:61` dark root ✓ (thead self-painted same-file) |
| `StockOverviewPane.tsx:98` | `StockModule.tsx:149` dark root ✓ (HudCard self-painted same-file) |
| `CartTable.tsx:85,91,94,147,216` | `PosShell.tsx:54` (`bg-slate-100 dark:bg-slate-950` ✓) / `CartPanel.tsx:55` dark surface ✓ (thead self-painted) |
| All 12 ModalShell consumers (`Backup/Checkout/Quantity/RecordPayment/Return/StockAdjust/WarehouseTransfer/CustomerSelect/DocumentConversion/CloseCashSession/CredentialReset` modals) | `ModalShell.tsx:12` `dark:bg-slate-900` ✓ verified (1 read, no rabbit hole) |

### SAFE (representative; full list: every other file's `dark:text` pairs with `dark:bg` in the same literal, and helpers return paired pill strings)

`SafeDeleteModal` (own card :126), `CategoryManageModal` (:240), `ProductCreateModal`
(:521), `ProductEditModal` content (:301), `ProductImportModal` (:307),
`OperationsMenu` (:255), `ProductCreateForm` (:314), `StarterPackBanner` cards
(:162/:263), `UnitSelectPopover` (:78), `CreditHud` + `CustomerTierBadge`
(self-painted pills), `DataExchangeMenu` (:93), `UsersModule` (:61),
`StockModule`/`DocumentsModule` roots, `CartPanel` (:55), `ProductDiscovery`
list/cards (:170/:216/:277), `StockOverviewPane` HudCard (:47),
`DocumentFilterBar`, `App.tsx:155` (self-painted error panel),
`document-helpers.ts` / `user-helpers.ts` / `stock-helpers.ts` /
`settings-helpers.ts` (all return `bg-* + dark:bg-* + dark:text-*` triples),
`KeyFooter` (always-dark footer, explicit text colors — theme-independent).

## (b) Empirical contrast sample (dark, fr, 1280×720, user-toggle path)

Screenshots: `audit/AUD-015/screenshots/aud015-{pos,settings,catalog,stock,customers,suppliers,reports}-dark.png` + `contrast.json` (machine log with fg/bg/size/weight per element).

| Screen | Element (tag, text) | fg on bg | Ratio / threshold | Verdict |
|---|---|---|---|---|
| pos | P "Catalogue vide — ajoutez…" | `#64748b` on `#0f172a` | 3.75 / 4.5 | FAIL (unmarked empty text on darkened POS surface) |
| pos | BUTTON "Caisse" (active nav) | `#020617` on `#f59e0b` | 9.39 / 4.5 | PASS |
| settings | H1 "Paramètres & Configuration" | `#0f172a` on `#0f172a` | 1.0 / 4.5 | FAIL (inverse) |
| settings | P "Version 1.0.0 — moteur…" (About) | `#e2e8f0` on `#334155` | 8.4 / 4.5 | PASS |
| catalog | SPAN "Savon de Marseille" | `#0f172a` on `#0f172a` | 1.0 / 4.5 | FAIL (inverse, product card) |
| catalog | SPAN "Ciment CPJ 45" | `#0f172a` on `#0f172a` | 1.0 / 4.5 | FAIL (inverse) |
| catalog | H2 "Catalogue des Produits" | `#0f172a` on `#f8fafc` | 17.06 / 4.5 | PASS (ambient root, inherited text) |
| stock | H1 "Stock & Dépôts" | `#0f172a` on `#0f172a` | 1.0 / 4.5 | FAIL (inverse) |
| customers | H1 "Clients & Crédit" | `#0f172a` on `#f8fafc` | 17.06 / 4.5 | PASS |
| customers | P "Amine Benali" (row) | `#0f172a` on `#f8fafc` | 17.06 / 4.5 | PASS (inherits — no dark:text, correct here) |
| customers | BDI "1 250,50 DZD" (debtor bal.) | `#fbbf24` on `#f8fafc` | 1.6 / 4.5 | FAIL (family 1) |
| suppliers | H2 "Fournisseurs" | `#0f172a` on `#f8fafc` | 17.06 / 4.5 | PASS |
| suppliers | TD "Fournisseur Test Alpha" | `#0f172a` on `#f8fafc` | 17.06 / 4.5 | PASS |
| suppliers | TD "2 500,00 DZD" | `#0f172a` on `#f8fafc` | 17.06 / 4.5 | PASS |
| reports | H2 "Rapports" | `#0f172a` on `#f8fafc` | 17.06 / 4.5 | PASS |
| reports | SPAN "Marges brutes" (card) | `#0f172a` on `#0f172a` | 1.0 / 4.5 | FAIL (inverse) |
| reports | P "Analyses d’activité…" | `#64748b` on `#f8fafc` | 4.55 / 4.5 | PASS (borderline) |

6 of 7 screens reproduce a defect (pos, settings, catalog, stock, customers,
reports); suppliers-populated is clean. Stock low-stock ROWS were not reachable:
the overview's data path errors (`Erreur interne du noyau` — an unmapped command
in the mock, likely the depot-summary call) and renders its empty state instead;
per the task's substitution allowance the stock H1 finding stands and row-level
sampling is deferred (overview needs a real backend or a fuller mock).
Visual note (aud015-stock-dark.png): HudCard "0" values and disabled
depot/transfer buttons are near-invisible in the error state — consistent with
the unmarked-text-on-dark-card family, recorded as observation (not computed).

## (d) ADR index verification

`docs/adr/` EXISTS. Complete file list (26 ADRs + INDEX.md + README.md):

ADR-066 viewport-baseline-and-window-policy · ADR-067 single-top-bar-no-floating-buttons ·
ADR-068 tax-regime-ifu · ADR-069 locales-fr-ar-latin-digits · ADR-070 output-formats-58-80-a4 ·
ADR-071 deployment-topology-single-device · ADR-072 catalog-migration-from-dolisoft ·
ADR-073 canonical-repository-location · ADR-074 destructive-action-confirmation ·
ADR-075 catalog-import-scope-products-only · ADR-076 isolated-data-directory-override ·
ADR-077 product-entry-and-generic-import-export · ADR-078 information-architecture-and-navigation ·
ADR-079 roles-in-use-and-manager-approval-in-place · **(080, 081 absent — numbering gap, no files)** ·
ADR-082 debt-receipt-numbering · ADR-083 debt-receipt-separate-pipeline ·
ADR-084 ctp-persistence-language · ADR-085 ctp-persistence-language-reratified ·
ADR-086 v1.0.0-release-approval · ADR-087 artisan-wholesale-pricing-tiers-and-brand-identity ·
ADR-088 single-instance-guard-and-starter-catalog · ADR-089 data-retention-and-upgrade-preservation ·
ADR-090 data-exchange-hub-and-cockpit-optimization · ADR-091 dual-width-thermal-printing-and-shaping-calibration ·
ADR-092 dual-track-deletion-and-configurable-timbre · ADR-093 settings-command-center-and-catalog-purification.

**Confirmed latest ADR number: ADR-093.** The commit-message hint was correct;
no ADR-094+ exists anywhere in the repository.

## Design input for the next task (factual, no prescription)

- A single root-level change (dark body background) would fix family 1 but
  BREAK every family-2 site that currently relies on inheriting dark body text
  on a darkened card — and vice versa. The inventory above is the blast radius.
- Cheapest structural fix consistent with the data: give the four ambient-root
  views (`CustomersModule`, `SupplierListScreen` section, `CatalogModule`,
  `ReportsHomeScreen`) the same `bg-slate-50 dark:bg-slate-900` root the other
  four views already have, then re-measure; family-2 sites need per-element
  `dark:text-*` additions (each listed above with line numbers).
