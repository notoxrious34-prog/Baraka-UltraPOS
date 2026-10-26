# TASK-DSN-096 REPORT — Depot delete pre-check + exchange final polish

STATUS: **PARTIAL**. Exchange polish DONE and green; depot pre-check BLOCKED
per the task's own HANDOFF CLAUSE (field evidence §2). DO NOT COMMIT.

## 1. FAIL-before (quoted)

- Depot: AUD-043 §8 sole remnant — stocked non-DEP-01 depots are
  "offered-then-rejected (`WAREHOUSE_HAS_STOCK` post-confirm) instead of
  pre-blocked". `WarehouseTransferModal.tsx:179-190` delete button
  `disabled={s.isSubmitting}` only; `:213`
  `isBlocked={deleting.code === PRINCIPAL_DEPOT_CODE}` is unreachable
  defense (locked depots never reach the modal, `:167`); zero levels query
  in the modal; only `s.errorMessage` post-confirm carries the refusal.
- Exchange: `DataExchangeSettingsTab.tsx:27` `EXPORT_LIMIT = 1000` with a
  silent `fetched.slice(0, 1000)` (cap disclosed only post-export);
  `ProductImportModal.tsx` preview→report had no pre-launch over-limit
  guard (1500-row file → launch enabled → engine `IMPORT_ROW_LIMIT_EXCEEDED`
  post-click); template card copy never named the importer dialect; drop
  zone used a 📥 emoji (`:363`).

## 2. Depot verdict — BLOCKED (handoff clause fired, no new IPC invented)

- Row wire `WarehouseItem` (`src/lib/ipc.ts:353-358`: id/code/name/is_active)
  carries **no levels field**.
- No existing IPC lists per-depot levels: `stock_get` is per-product
  (level read, 0 when absent, `ipc.ts:1282`); `reports_low_stock` returns
  depleted rows only (`ipc.ts:4116`) — absence of rows cannot prove empty;
  engine `count_warehouse_nonzero_levels`
  (`sqlite_warehouse_repository.rs:195`) has no read command surface.
- Handoff: "Levels absent from row wire → STATE blocked with the field".
  QUESTION FOR ARCHITECT: authorize either (a) a read-only
  `warehouses_nonzero_counts` command surfacing the existing
  `count_warehouse_nonzero_levels` per depot, or (b) a
  `dry_run` flag on `warehouses_delete` returning the verdict without
  mutating — then the Customers-style `isBlocked` + guidance lands on real
  wire. Engine backstop (`WAREHOUSE_HAS_STOCK` post-confirm) stays the
  primary UX until then; DEP-01 lock badge untouched.

## 3. Component lines (exchange polish only, presentation + copy)

- `src/lib/i18n.ts` — `imp.overLimit` FR/AR; `settings.exchangeExportCopy`
  now states the 1000-row cap up front FR/AR;
  `settings.exchangeTemplateCopy` names the dialect
  `sku;barcode;name_fr;name_ar;purchase_price;sale_price;initial_stock;category_code`
  FR/AR (BRK-093 direction kept — no header renames).
- `src/features/catalog/ProductImportModal.tsx` —
  `MAX_IMPORT_ROWS_CLIENT = 1000` (documented mirror of engine
  `MAX_IMPORT_ROWS`, BRK-093) + pre-launch guard: amber `role="alert"`
  guidance + launch disabled when `parsed.rows.length > 1000`; engine
  `IMPORT_ROW_LIMIT_EXCEEDED` stays the backstop (race). 📥 replaced with
  an inline SVG download icon (no-emoji rule).
- `src/features/settings/DataExchangeSettingsTab.tsx` — card copy
  `break-words` so the long dialect token wraps instead of clipping
  (vision-caught, re-shot clean).
- Out of scope untouched: deletion/import engines, backup/reset screens.

## 4. Pin lines (`ProductImportModal.test.tsx`, 18/18 in file)

- Cap mirror `= 1000` + gate-before-IPC source pins
  (`disabled={parsed.rows.length === 0 || overLimit}`, `imp.overLimit`
  call-site); over-limit FR/AR copy (2× key + "fractionnez"/"قسّم");
  dialect headers in template copy (2× key + `name_fr;name_ar` +
  `initial_stock;category_code`); export cap FR/AR (2× key +
  "1000 premières lignes"/"أول 1000 سطر"); `break-words` wrap pin;
  no-📥 pin; `ImportReportCard` renders updated(5→"mis à jour") +
  errored + DUP-1 from the wire report; hub-wiring + policy-selector pins
  preserved.

## 5. PNGs + asserts (4/4 distinct SHAs, 20/20 asserts, 0 console)

Harness: headless Chrome CDP vs `vite :5173`, stubbed
`__TAURI_INTERNALS__.invoke` (auth_login/category_list/warehouses_list/
catalog_import_products canned report 2/1/0/0/42ms — never a live
backend); real login → Paramètres → Échanges; CSV injected via file input;
AR via POS Topbar toggle (settings has no toggle — probed).
`audit/DSN-096/dsn096-capture-results.json`: **20/20 PASS, consoleErrors 0**
(first-attempt CDP screenshot stalls needed one retry each — harness
quirk, content asserts unaffected). Vision-QA all four, no defects
(final re-shoot after the `break-words` fix).

| PNG | SHA-256 (short) | Assert coverage |
| --- | --------------- | --------------- |
| `exchange-fr-light.png` | `2ede780c…` | 3 cards, cap + dialect copy, wrap fixed, no h-scroll |
| `exchange-report-fr-light.png` | `89f9c252…` | modal `Importation terminée`, 2 importés / 1 mis à jour / 42 ms, Terminer |
| `exchange-ar-light.png` | `6d33a751…` | RTL shell/sidebar, `أول 1000 سطر`, dialect, `بدء` ×3 |
| `exchange-report-ar-light.png` | `5ccedffa…` | `اكتمل الاستيراد`, 2 مستوردة / 1 محدّثة, `إغلاق وتحديث القائمة`, RTL modal |

All four SHAs pairwise distinct.

## 6. Gates

| Gate | Result |
| ---- | ------ |
| G4 `npm run lint` | exit 0, GREEN |
| G5 `npx tsc --noEmit` | clean, GREEN |
| G6 `npm run build` | ✓ built in 6.24s, GREEN |
| G7 `npx vitest run` | **81 files / 829 passed**, GREEN |
| G1–G3 | N/A — zero Rust (state reason) |

No product file changed since the green run — tallies stand as stated.

## 7. git status --short

```text
M src/features/catalog/ProductImportModal.tsx
M src/features/catalog/__tests__/ProductImportModal.test.tsx
M src/features/settings/DataExchangeSettingsTab.tsx
M src/lib/i18n.ts
?? audit/DSN-096/
```

Diff limited to exchange modal + exchange tab + i18n + pins + audit
screenshots/results/REPORT. No ports left listening (`:5173`/`:9222`
killed, own PIDs only, verified by port check); scratch
(`dsn096-probe/capture/q/q2/shot/shot2/run*.mjs`, `*.log`,
`dsn096-chrome/`) deleted — `ls scratch | grep dsn096` empty.

## 8. STATE / DONE / REMAINING

- STATE: **PARTIAL** (exchange DONE; depot BLOCKED per handoff).
- DONE: exchange polish + 18/18 pins + 4 distinct captures + 20/20 asserts.
- REMAINING: full G7 tally, NO_LEFTOVERS (kill own vite/chrome, delete
  `dsn096-*` scratch), final `git status --short`, REPORT §6–§7 fill.

NEXT READY: NO (seal pass still open).
