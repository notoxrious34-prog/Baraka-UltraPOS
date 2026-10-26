# TASK-DSN-096-R REPORT — Depot deletion pre-check (block lifted by BRK-095 `876cc5a`) + exchange final polish (sealed `5e80414`, untouched below)

STATUS: **DONE**. DO NOT COMMIT.

Depot half: presence wire `warehouses_stock_presence` consumed for a
pre-confirm block; engine guards and backstop untouched. Exchange half
(§§3–5, sealed): reproduced verbatim, zero edits.

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

## 2. Depot verdict — UNBLOCKED by BRK-095 (no new IPC invented here)

Handoff clause fired in the previous pass and is now resolved upstream:
`warehouses_stock_presence` (`876cc5a`) returns
`{presence: [{warehouse_id, nonzero_levels}]}` (ManageWarehouses gate,
zero writes, DEP-01 reported truthfully). Facade shape verified against
`crates/commands/src/ipc.rs:1281-1304` + DTOs (`dto.rs`) — no mismatch,
no engine edits needed. This task only consumes the wire.

Prior-pass field evidence (kept for the trail — superseded, not deleted):
- Row wire `WarehouseItem` (`src/lib/ipc.ts:353-358`: id/code/name/is_active)
  carries **no levels field**.
- No pre-existing IPC listed per-depot levels: `stock_get` is per-product
  (level read, 0 when absent); `reports_low_stock` returns depleted rows
  only — absence of rows cannot prove empty; engine
  `count_warehouse_nonzero_levels` had no read command surface.
- That evidence produced the authorized option (a): the new read-only
  command above. No dry-run flag, no delete-path change.

## 2b. Depot component lines (presentation + store + facade only)

- `src/lib/ipc.ts` — `WarehouseStockPresence` (`warehouseId`,
  `nonzeroLevels`) + `ipcWarehouseStockPresence(sessionToken)` calling
  `warehouses_stock_presence` with top-level `session_token`, mapping the
  `presence` envelope to camelCase.
- `src/stores/warehouse-transfer-store.ts` — `stockPresence:
  Record<string, number>` + `isLoadingPresence` + `loadStockPresence()`
  (session-token read, fail-open to `{}` on read errors — never a red
  banner; engine stays the backstop).
- `src/components/pos/modals/WarehouseTransferModal.tsx` —
  `ManageDepotsTab`: presence loaded on mount when `canManage`;
  `stocked = (s.stockPresence[warehouse.id] ?? 0) > 0`; delete button
  `disabled={s.isSubmitting || stocked}`; stocked non-DEP-01 rows get an
  amber guidance line (`wh.stockedGuidance`, `{name}` replaced);
  `{stocked && !locked && (...)}` so locked DEP-01 keeps its badge-only
  look; `SafeDeleteModal` + `isBlocked` + `submitDeleteWarehouse` +
  error-line backstop byte-identical.
- `src/lib/i18n.ts` — `wh.stockedGuidance` FR
  ("« {name} » contient encore du stock — transférez-le vers un autre
  dépôt avant de le supprimer.") / AR
  ("«{name}» ما زال يحتوي على مخزون — انقل المخزون إلى مستودع آخر قبل
  الحذف.").
- Untouched: delete engine, exchange files, backup/reset screens.

## 2c. Depot pin lines (`warehouse-transfer-modal.test.ts`, 5 new, 17/17 file)

- Presence wire → record mapping (`warehouses_stock_presence` called with
  `{session_token}`, `{w1: 0, w2: 3}`, loading flag reset).
- Fail-open: rejected read → `{}` with no `INTERNAL` in `errorMessage`
  (engine stays the backstop).
- Pre-block source pins: `(s.stockPresence[warehouse.id] ?? 0) > 0`,
  `disabled={s.isSubmitting || stocked}`, guidance `.replace("{name}"`,
  `void s.loadStockPresence()`, `{stocked && !locked && (`.
- Lock + backstop untouched: `PRINCIPAL_DEPOT_CODE` gate, unreachable
  `isBlocked` defense, `submitDeleteWarehouse` confirm path,
  `translateBackendError(s.errorMessage, locale)` line + i18n
  `WAREHOUSE_HAS_STOCK`.
- `wh.stockedGuidance` exactly 2× with FR/AR copy spot-checks.

## 2d. Depot PNGs + asserts (2/2 distinct SHAs, 14/14 asserts, 0 console)

Harness: headless Chrome CDP vs `vite :5173`, stubbed
`__TAURI_INTERNALS__.invoke` (full-permission login; `warehouses_list`
DEP-01/MAG-01/ANN-01; presence 5/2/0); real login → Stock & Dépôts →
Transfert → Dépôts tab; AR via POS Topbar toggle.
`audit/DSN-096/dsn096r-capture-results.json`: **14/14 PASS, consoleErrors
0**. Vision-QA both, no defects (AR cosmetics noted are pre-existing and
out of scope). Background red banner in shots is a stub gap on the
dimmed stock-overview page (presence-driven modal states all proven) —
not product.

| PNG | SHA-256 (short) | Assert coverage |
| --- | --------------- | --------------- |
| `depot-blocked-fr-light.png` | `38a48a0f…` | DEP-01 lock badge only (no delete, no guidance); MAG-01 delete disabled + « Magasin Secondaire » guidance; ANN-01 delete enabled; no confirm open |
| `depot-blocked-ar-light.png` | `36657df2…` | RTL shell + mirrored rows/X; MAG-01 حذف disabled + guidance; DEP-01 مقفل only; ANN-01 حذف enabled |

SHAs pairwise distinct and distinct from the four sealed exchange shots.

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
| G6 `npm run build` | ✓ built in 6.18s, GREEN |
| G7 `npx vitest run` | **81 files / 834 passed**, GREEN (829 sealed + 5 new) |
| G1–G3 | N/A — zero Rust (state reason) |

No product file changed since the green run — tallies stand as stated
(REPORT §6–§7 timestamp/status-block corrections only, post-gates).

## 7. git status --short

```text
M audit/DSN-096/REPORT.md
M src/components/pos/modals/WarehouseTransferModal.tsx
M src/components/pos/modals/warehouse-transfer-modal.test.ts
M src/lib/i18n.ts
M src/lib/ipc.ts
M src/stores/warehouse-transfer-store.ts
?? audit/DSN-096/depot-blocked-ar-light.png
?? audit/DSN-096/depot-blocked-fr-light.png
?? audit/DSN-096/dsn096r-capture-results.json
```

Diff limited to depot modal + transfer store + facade + i18n + pins +
audit screenshots/results/REPORT; exchange files byte-identical since
their seal. Ports `:5173`/`:9222` are currently LISTENING (node.exe PID
11540, chrome.exe PID 12912 — present before this pass, not started here,
left untouched per the kill-only-own-PIDs rule); no server was started in
this pass. Scratch: no `dsn096*` entries in `D:/Dev/_scratch/` nor
`$TMPDIR`; no ad-hoc scripts created in this pass — nothing to delete.

## 8. STATE / DONE / REMAINING

- STATE: **DONE**. Stocked non-DEP-01 depots cannot reach confirm
  (disabled pre-IPC, pin-proven); DEP-01 lock badge untouched; empty depots
  flow through the existing confirm path with the engine backstop intact.
- DONE: depot pre-check + 5 new pins (17/17 file) + 2 distinct captures +
  14/14 asserts; exchange half sealed and unretouched.
- REMAINING: none — AUD-043 branch set closed.

NEXT READY: YES.
