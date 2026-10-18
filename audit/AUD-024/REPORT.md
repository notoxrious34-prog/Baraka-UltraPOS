# TASK-AUD-024 — Secondary Search Inputs & Family-2 Contrast Targets: REPORT

READ-ONLY audit. No source file modified. HEAD: `ea9712f`; `git status` clean at
start. Method: static inspection (class/trigger tracing per input) + math
verification. No CDP rerun: all 5 contrast files are byte-untouched since `6b06f93`
(`git log 6b06f93..HEAD` on the five files: empty), so AUD-023's DIRECT CDP ratios
stand; line numbers re-pinned below against current HEAD.

## 1. EXECUTIVE SUMMARY

**Secondary search inputs: 9 surveyed → 6 ALREADY-INSTANT, 3 MANUAL-TRIGGER-CONFIRMED.**
Manual: ReturnModal document lookup (Enter + Search button), WarehouseTransferModal
product picker (Enter + button — the exact pre-DSN-064 pattern), PurchaseOrderFormScreen
product search (Enter + `sup.search` button). Instant: CustomerSelectModal (150ms),
StockAdjustModal picker (175ms, DSN-064), DocumentConversionModal (per-keystroke, no
debounce), DocumentFilterBar (250ms debounce), plus primary bars (out of scope, noted).
Non-search inputs (credential/amount/qty editors, rename drafts, lists without
inputs) excluded with reason per item.

**Contrast targets: all 5 re-pinned with exact lines/classes.** Token verification
yields one split decision: four elements fix cleanly with `text-ink-muted`
(light-identical, dark 6.96); DocumentTable thead MUST use `text-ink-secondary`
(`text-ink-muted` would still fail 4.34 in LIGHT on its slate-100 header).

## 2. SECONDARY SEARCH TABLE

| File | Component | Context | Trigger | Classification |
|---|---|---|---|---|
| `src/components/pos/modals/CustomerSelectModal.tsx:62-96,232` | customer search input | F2 customer select | `onChange`→state→`useEffect` + `clearTimeout`, `DEBOUNCE_MS = 150` (:25) | ALREADY-INSTANT |
| `src/components/pos/modals/StockAdjustModal.tsx` | product picker | adjust modal (DSN-064) | 175ms debounce effect + ticket guard; Enter fallback; button removed | ALREADY-INSTANT |
| `src/components/pos/modals/DocumentConversionModal.tsx:174` | doc search input | convert-to-... picker | `onChange`→store `setSearchQuery`→`runSearch()` immediately per keystroke; no Enter/button/debounce | ALREADY-INSTANT (undebounced — observation, not a trigger defect) |
| `src/features/documents/DocumentFilterBar.tsx:30` + `DocumentsModule.tsx:42-48` | documents query | module filter bar | `onChange` lifts; 250ms debounce effect before server query | ALREADY-INSTANT |
| `src/components/pos/modals/ReturnModal.tsx:156-179` | doc lookup input | return against finalized doc | `onChange` buffers only; fetch on **Enter** (`:161-165`) or explicit **Search button** (`:171-179`) | MANUAL-TRIGGER-CONFIRMED |
| `src/components/pos/modals/WarehouseTransferModal.tsx:391-409` | product picker | transfer lines | `onChange` buffers; fetch on **Enter** or button (`addItemToTransfer`); `runPickerSearch` has no timer | MANUAL-TRIGGER-CONFIRMED |
| `src/features/purchases/PurchaseOrderFormScreen.tsx:72-91` | product search | PO line entry | `onChange` buffers; fetch on **Enter** (`:78`) or `sup.search` button | MANUAL-TRIGGER-CONFIRMED |
| `src/components/pos/ParkedSalesModal.tsx` | — | parked list | NO search input exists (static list) | N/A (no input) |
| `src/components/pos/CartTable.tsx` | — | cart hint | no input (`cart.hintSearch` is hint text) | N/A |
| `RecordPaymentModal` (`:360,409,422`), `CategoryManageModal` (`:287`), `CredentialResetModal`, `UserFormModal`, `PurchaseOrderFormScreen` (`:116,125`), transfer rename/checkbox (`:137,152`) | — | amount/method/code/name/qty/cost editors, credential fields | direct state patch, no lookup | N/A (editors, not search) |
| `DataExchangeSettingsTab`, `StarterPackSettingsTab` | — | settings | no search inputs (only timer/callback matches) | N/A |

Primary bars (POS discovery, Catalog, Customers, Suppliers, Stock table, conversion
excluded above) already live — out of secondary scope, no action.

## 3. FAMILY-2 TARGET TABLE (all `text-slate-500` #64748b, unmarked, dark bg)

| Element | Exact path:line | Current class | Dark pair → ratio | Target token → light / dark |
|---|---|---|---|---|
| POS empty text — VARIANCE: task names `Aucun article dans le panier`, which appears NOWHERE in `src/` (grep empty). Actual: `search.catalogEmpty` "Catalogue vide — …" | `src/components/pos/ProductDiscovery.tsx:231` (wrapper div; copy in `p` :233-236) `flex flex-col items-center justify-center p-8 text-center text-sm text-slate-500` | same | on POS dark surface #0f172a → **3.75** (AUD-023 DIRECT) | `text-ink-muted` → 4.76 / **6.96** ✓ |
| TotalsFooter sub/discount/TVA rows | `src/components/pos/TotalsFooter.tsx:25,29,33` `flex items-center justify-between text-xs font-semibold text-slate-500` (root self-painted `dark:bg-slate-900`) | same | → **3.75** (DIRECT) | `text-ink-muted` → 4.76 / **6.96** ✓ |
| ParkedSales empty (211) + draft sub-lines | `src/components/pos/ParkedSalesModal.tsx:211` (div), `:224`, `:229` (`p.text-xs text-slate-500`; ModalShell card `dark:bg-slate-900` CDP-confirmed) | same | → **3.75** (DIRECT) | `text-ink-muted` → 4.76 / **6.96** ✓ |
| DiscountModal field labels | `src/components/pos/modals/DiscountModal.tsx:61,73` `text-xs font-bold text-slate-500` (same ModalShell dark card) | same | → **3.75** (ANALYTIC, CDP-measured bg) | `text-ink-muted` → 4.76 / **6.96** ✓ |
| DocumentTable thead row | `src/features/documents/DocumentTable.tsx:66` `text-start text-slate-500` (thead `dark:bg-slate-800`, light `bg-slate-100`) | same | light 4.34 / dark **3.07** (DIRECT) | `text-ink-secondary` → **9.45** / **5.71** ✓ — `text-ink-muted` REJECTED here (4.34 light on slate-100, still < 4.5) |

`text-ink-muted` preserves light rendering pixel-identically (light value IS
`#64748b` = slate-500) wherever the light bg is white/canvas; the thead's light
slate-100 bg forces `text-ink-secondary` instead. Adjacent same-family instances
for scheduling (not in AUD-015's five): `DocumentTable.tsx:50,57,100`,
`ProductDiscovery.tsx:194,218,223,303`, `SettingsModule.tsx:200` (AUD-023 observation).

## 4. HYGIENE

No temp scripts were created (pure static inspection) — nothing to delete.
`git status --porcelain` shows only `?? audit/AUD-024/`.

## 5. FILES

- [CREATED] `D:/Dev/Baraka UltraPOS/audit/AUD-024/REPORT.md`

STATE complete / DONE inventory + target re-confirmation / REMAINING none (remediation is future DSN scope).
NEXT READY: YES
