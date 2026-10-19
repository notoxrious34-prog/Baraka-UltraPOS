# TASK-AUD-025 — Quick-Product Palette & Catalog KPI Ribbon Pre-Verification: REPORT

READ-ONLY audit. HEAD: `5f8fb86`. No source file modified; no dev server, no
temp scripts (pure static inspection — nothing to delete).

## 1. EXECUTIVE SUMMARY — SHORTCUTS & Alt+1..8

Single cockpit handler: `src/hooks/use-pos-keyboard.ts` (window `keydown`,
added/removed in one effect, ~:270). Full registry: **F1** search-focus,
**F2** customer, **F3** cycle-line-unit, **F4** quantity (needs lines),
**F5** discount (needs lines), **F6** parked list, **F8** park-sale,
**F10** cycle-doc-type, **F11** swallowed (no action, anti-fullscreen),
**F12** checkout (needs lines + OPEN cash session + no credit block),
**Escape** modal-aware close (:121), **Delete/Backspace** remove-line,
**ArrowUp/Down** selection, **Ctrl/⌘+K** search-focus, printable-key scanner
wedge → search input. **F7/F9 unbound. `Alt+Digit` COMPLETELY FREE** — zero
`altKey` consumers in `src/` (only two guards that explicitly exclude Alt:
`use-pos-keyboard.ts:43`, `use-catalog-search.ts:132`); the Alt path can never
collide, and the wedge can never swallow it. Placement note: new binding must
sit with the F-keys (before the `inInput`/`modalOpen` early-outs at ~:217 if it
should work from search, or after them if modal-excluded like F5/F12).

## 2. CART ADD SIGNATURE

`src/stores/cart-store.ts:68` —
`addProduct: (product: CatalogProduct, unitIndex?: number) => void`.
Payload = a FULL `CatalogProduct` (units[] with `unit_id`, `is_default_selling`,
`is_base_unit`, prices micro-strings). Unit omitted → `defaultUnitIndex`
(:52-59: default-selling → base → 0); unknown unit → silent no-op.
Repeat add bumps `quantityMicro` by `DEFAULT_QTY_MICRO`; entry needs no
session/cash (checkout alone is gated). Palette sourcing options (no new
mechanism exists — §4): resolve pinned ids via existing `ipcGetProduct`, or
`catalog_search` by SKU; nothing to reuse, nothing to collide with.

## 3. PALETTE SLOT (PosShellView, 12-col grid)

`src/components/pos/PosShellView.tsx:33-37`: Topbar / cash banner / KeyFooter
are fixed chrome; `main.grid-cols-12` holds **ProductDiscovery `col-span-7`**
(:159, `min-h-0 flex-col`, scroll confined to inner list :216) and **CartPanel
`col-span-5`** (:33, own inner scroll :55). Designated slot: a
`shrink-0`-capped horizontal strip INSIDE the discovery column between the
search row (:160) and the results list (:216) — e.g. under the chip row
(:192) — `flex-wrap`, max-height + `overflow-y-auto`, never in CartPanel,
never in the footer. Cart/TOTAL-F12 live exclusively in `col-span-5` +
KeyFooter, so intersection is structurally impossible; no root-scroll risk
(shell is `min-h-0 flex-1`). Config storage: NO generic settings IPC exists
(only `fiscal_settings_*`, `printer_*`); viable = `localStorage` key
(precedents: `baraka-theme`, starter-pack flag) or a future backend table —
decision for the execution task.

## 4. KPI RIBBON — ALREADY EXISTS (do not rebuild)

`src/features/catalog/CatalogModule.tsx` already ships the ribbon:
`CatalogKpis` (:78), pure `computeKpis` (:106), four filter-cards
(total/in-stock/out-of-stock/inactive, :361-385) above the search input, plus
`CategoryPillStrip` (:473, counts-less pills). Gaps vs the planned spec:
(a) metrics cover only the LOADED window (`SEARCH_LIMIT = 100`, :250, via
`ipcSearchCatalog` + per-row `ipcGetStock` against depot) — global counts
would need a new backend summary (NONE exists: no `catalog_*summary*` command;
`crates/commands` has only reports/printer/fiscal getters); (b) "low-stock
alert" ≠ threshold — backend doctrine ADR-037 pins low-stock AS stockout
(`quantity_base_micro <= 0`,
`sqlite_reporting_repository.rs:517-531`; `reports_low_stock` IPC :2294,
per-warehouse); no reorder-point column exists anywhere; (c) category
distribution counts are not shown (pills only; `category_list` IPC :254
available). Recommendation: execution task = EXTEND (global-summary command
+ optional distribution counts), never rebuild.

## 5. FILES

- [CREATED] `D:/Dev/Baraka UltraPOS/audit/AUD-025/REPORT.md`

STATE complete / DONE survey (Alt+1..8 FREE; slot designated; KPI exists) /
REMAINING none.
NEXT READY: YES
